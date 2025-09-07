use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{config::AutoProxCycles, defs::*, Error, Iqs7211e};

const MAX_TRACKPAD_CHANNELS: usize = 42;

/// Snapshot of the live measurements that are typically reviewed while tuning
/// a new hardware design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct SetupSnapshot {
  /// Raw [`InfoFlags`] block reported by the device.
  pub info_flags: InfoFlags,
  /// Flattened view into the trackpad delta counters (0xE200). Use
  /// `rx_count * tx_count` to determine how many entries are populated.
  pub trackpad_deltas: [u16; MAX_TRACKPAD_CHANNELS],
  /// Flattened view into the trackpad base targets (0xE100). Use
  /// `rx_count * tx_count` to determine how many entries are populated.
  pub trackpad_base_targets: [u16; MAX_TRACKPAD_CHANNELS],
  /// Number of Rx electrodes captured in the snapshot.
  pub rx_count: usize,
  /// Number of Tx electrodes captured in the snapshot.
  pub tx_count: usize,
  /// Latest reported ALP channel long-term average.
  pub low_power_channel_lta: u16,
  /// Latest reported ALP channel count.
  pub low_power_channel_count: u16,
  /// ALP A/B channel counts.
  pub low_power_count_a: u16,
  pub low_power_count_b: u16,
  /// ALP A/B compensation values.
  pub low_power_comp_a: u16,
  pub low_power_comp_b: u16,
}

/// State machine helper that guides the operator through the manual setup
/// described in the Azoteq reference documentation.
pub struct SetupSession<'a, I, RDY> {
  device: &'a mut Iqs7211e<I, RDY>,
  original_interrupt_mode: InterruptMode,
  original_lp1_auto_prox_cycles: AutoProxCycles,
  manual_control_enabled: bool,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Begin an interactive setup sequence.
  ///
  /// The returned [`SetupSession`] stages the controller so that live
  /// measurements can be collected and presented to the user. When the session
  /// is finished, call [`SetupSession::finish`] to leave the device in a clean
  /// state.
  pub fn begin_setup(&mut self) -> SetupSession<'_, I, RDY> {
    let original_interrupt_mode = self.config.interrupt_mode;
    let original_lp1_auto_prox_cycles = self.config.hardware.low_power.lp1_auto_prox_cycles;

    SetupSession { device: self, original_interrupt_mode, original_lp1_auto_prox_cycles, manual_control_enabled: false }
  }
}

impl<'a, I, E, RDY> SetupSession<'a, I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Perform the one-time initialisation required before tuning.
  ///
  /// This mirrors the "basic setup" portion of the Azoteq documentation by
  /// switching the device to stream mode, disabling automatic ALP cycles, and
  /// reloading the staged [`Config`](crate::Config). The controller stays in
  /// manual-friendly stream mode until [`finish`](Self::finish) is invoked.
  pub async fn initialize(&mut self) -> Result<(), Error<E>> {
    self.device.config.interrupt_mode = InterruptMode::Stream;
    self.device.config.hardware.low_power.lp1_auto_prox_cycles = AutoProxCycles::Disabled;

    // Run the regular device bring-up path with the staged parameters.
    self.device.initialize().await?;

    // Restore the in-memory configuration so that callers can continue editing
    // their preferred interrupt mode once the session has ended.
    self.device.config.interrupt_mode = self.original_interrupt_mode;
    self.device.config.hardware.low_power.lp1_auto_prox_cycles = self.original_lp1_auto_prox_cycles;
    Ok(())
  }

  /// Enable manual control and force the device into LP1 charge mode.
  ///
  /// This mirrors the GUI steps where the operator enables manual control under
  /// "Control and Config" and selects the LP1 charge mode before capturing
  /// counters.
  pub async fn enter_manual_control(&mut self) -> Result<(), Error<E>> {
    self.device.set_manual_control(true).await?;
    self.device.set_charge_mode(ChargeMode::LowPower1).await?;
    self.manual_control_enabled = true;
    Ok(())
  }

  /// Capture the live counters that are typically recorded while tuning.
  pub async fn snapshot(&mut self) -> Result<SetupSnapshot, Error<E>> {
    let rx_count = self.device.config.pin_mapping.rx_pins().len();
    let tx_count = self.device.config.pin_mapping.tx_pins().len();
    let populated = rx_count * tx_count;
    debug_assert!(populated <= MAX_TRACKPAD_CHANNELS);

    let info_flags = self.device.info_flags().await?;
    let base = self.read_measurement_block(0xE100, populated).await?;
    let deltas = self.read_measurement_block(0xE200, populated).await?;
    let low_power_channel_lta = self.device.read_u16(Reg::LowPowerChannelLta).await?;
    let low_power_channel_count = self.device.read_u16(Reg::LowPowerChannelCount).await?;
    let low_power_count_a = self.device.read_u16(Reg::LowPowerChannelCountA).await?;
    let low_power_count_b = self.device.read_u16(Reg::LowPowerChannelCountB).await?;
    let low_power_comp_a = self.device.read_u16(Reg::LowPowerAutoTuningCompA).await?;
    let low_power_comp_b = self.device.read_u16(Reg::LowPowerAutoTuningCompB).await?;

    Ok(SetupSnapshot {
      info_flags,
      trackpad_deltas: deltas,
      trackpad_base_targets: base,
      rx_count,
      tx_count,
      low_power_channel_lta,
      low_power_channel_count,
      low_power_count_a,
      low_power_count_b,
      low_power_comp_a,
      low_power_comp_b,
    })
  }

  /// Leave manual control and restore the interrupt configuration that was
  /// active prior to the setup session.
  pub async fn finish(mut self) -> Result<(), Error<E>> {
    if self.manual_control_enabled {
      self.device.set_manual_control(false).await?;
      self.manual_control_enabled = false;
    }

    self.device.set_interrupt_mode(self.original_interrupt_mode).await?;
    self.device.config.hardware.low_power.lp1_auto_prox_cycles = self.original_lp1_auto_prox_cycles;
    Ok(())
  }

  async fn read_measurement_block(
    &mut self,
    from: u16,
    populated: usize,
  ) -> Result<[u16; MAX_TRACKPAD_CHANNELS], Error<E>> {
    let mut out = [0u16; MAX_TRACKPAD_CHANNELS];

    for (idx, entry) in out.iter_mut().take(populated).enumerate() {
      let addr = from + idx as u16;
      *entry = self.device.read_u16_ext(addr).await?;
    }

    Ok(out)
  }
}

impl<'a, I, RDY> Drop for SetupSession<'a, I, RDY> {
  fn drop(&mut self) {
    // If the session terminates early without calling `finish`, the staged
    // configuration should still be restored in-memory so that a subsequent
    // call to `initialize` behaves as expected.
    self.device.config.interrupt_mode = self.original_interrupt_mode;
    self.device.config.hardware.low_power.lp1_auto_prox_cycles = self.original_lp1_auto_prox_cycles;
  }
}
