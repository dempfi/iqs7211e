use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{AlpHardware, AutoProxCycles, ChargeMode, Error, Info, InterruptMode, Iqs7211e, Reg};

const MAX_TRACKPAD_CHANNELS: usize = 42;

/// Snapshot of the live measurements that are typically reviewed while tuning
/// a new hardware design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupSnapshot {
  /// Raw [`Info`] block reported by the device.
  pub info: Info,
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
  pub alp_channel_lta: u16,
  /// Latest reported ALP channel count.
  pub alp_channel_count: u16,
  /// ALP A/B channel counts.
  pub alp_count_a: u16,
  pub alp_count_b: u16,
  /// ALP A/B compensation values.
  pub alp_comp_a: u16,
  pub alp_comp_b: u16,
}

/// State machine helper that guides the operator through the manual setup
/// described in the Azoteq reference documentation.
pub struct SetupSession<'a, I, RDY> {
  device: &'a mut Iqs7211e<I, RDY>,
  original_interrupt_mode: InterruptMode,
  original_lp1_auto_prox_cycles: AutoProxCycles,
  original_lp2_auto_prox_cycles: AutoProxCycles,
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
    SetupSession {
      device: self,
      // Defaults are placeholders; real values are captured during initialize()
      original_interrupt_mode: InterruptMode::Stream,
      original_lp1_auto_prox_cycles: AutoProxCycles::Cycles16,
      original_lp2_auto_prox_cycles: AutoProxCycles::Cycles32,
      manual_control_enabled: false,
    }
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
    // Run the regular device bring-up path with the staged parameters.
    self.device.initialize().await?;
    // Capture current on-device settings required for restoration
    let settings = self.device.config_settings().await?;
    self.original_interrupt_mode = settings.interrupt_mode;

    // Read ALP HW register to snapshot LP auto-prox cycles
    let alp_hw: AlpHardware = self.device.read(Reg::AlpHardware).await?;
    self.original_lp1_auto_prox_cycles = alp_hw.lp1_auto_prox_cycles;
    self.original_lp2_auto_prox_cycles = alp_hw.lp2_auto_prox_cycles;

    // Switch to stream mode and disable ALP auto-prox during tuning
    self.device.set_interrupt_mode(InterruptMode::Stream).await?;
    let mut new_alp = alp_hw;
    new_alp.lp1_auto_prox_cycles = AutoProxCycles::Disabled;
    new_alp.lp2_auto_prox_cycles = AutoProxCycles::Disabled;
    self.device.write(Reg::AlpHardware, new_alp).await?;
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
    let rx_count = self.device.config.pinout.rx.len;
    let tx_count = self.device.config.pinout.tx.len;
    let populated = rx_count * tx_count;
    debug_assert!(populated <= MAX_TRACKPAD_CHANNELS);

    let info_flags = self.device.info().await?;
    let base = self.read_measurement_block(0xE100, populated).await?;
    let deltas = self.read_measurement_block(0xE200, populated).await?;
    let alp_channel_lta = self.device.read_u16(Reg::LowPowerChannelLta).await?;
    let alp_channel_count = self.device.read_u16(Reg::LowPowerChannelCount).await?;
    let alp_count_a = self.device.read_u16(Reg::LowPowerChannelCountA).await?;
    let alp_count_b = self.device.read_u16(Reg::LowPowerChannelCountB).await?;
    let alp_comp_a = self.device.read_u16(Reg::AlpAutoTuningCompA).await?;
    let alp_comp_b = self.device.read_u16(Reg::AlpAutoTuningCompB).await?;

    Ok(SetupSnapshot {
      info: info_flags,
      trackpad_deltas: deltas,
      trackpad_base_targets: base,
      rx_count,
      tx_count,
      alp_channel_lta,
      alp_channel_count,
      alp_count_a,
      alp_count_b,
      alp_comp_a,
      alp_comp_b,
    })
  }

  /// Leave manual control and restore the interrupt configuration that was
  /// active prior to the setup session.
  pub async fn finish(mut self) -> Result<(), Error<E>> {
    if self.manual_control_enabled {
      self.device.set_manual_control(false).await?;
      self.manual_control_enabled = false;
    }
    // Restore interrupt mode and ALP auto-prox cycles
    self.device.set_interrupt_mode(self.original_interrupt_mode).await?;
    let mut alp: AlpHardware = self.device.read(Reg::AlpHardware).await?;
    alp.lp1_auto_prox_cycles = self.original_lp1_auto_prox_cycles;
    alp.lp2_auto_prox_cycles = self.original_lp2_auto_prox_cycles;
    self.device.write(Reg::AlpHardware, alp).await?;
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
