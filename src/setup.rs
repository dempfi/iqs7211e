use defmt::info;
use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

use crate::config;

use super::{Error, Iqs7211e, control, defs};

#[derive(PartialEq, Eq, defmt::Format)]
pub enum SetupStep {
  Step1,
  Step2,
  Step3,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Step1 — Determine AtiDivMul
  /// Step2 — ALP ATI Setup
  pub async fn setup(&mut self, step: SetupStep) -> Result<SetupStep, Error<E>> {
    self.wait_for_comm_window().await?;

    match step {
      SetupStep::Step1 => {
        self.config.interrupt_mode = control::InterruptMode::Stream;
        self
          .config
          .alp_hw_settings
          .set_lp1_auto_prox_cycles(config::AutoProxCycles::Disabled);

        if self.init().await? {
          info!("INITIALIZED");
        }

        Ok(SetupStep::Step2)
      }

      SetupStep::Step2 => {
        self.config_settings(|x| x.set_manual_control(true))?;
        self.sys_control(|x| x.set_charge_mode(control::ChargeMode::LowPower1))?;

        Ok(SetupStep::Step3)
      }

      SetupStep::Step3 => {
        let info_flags = self.info_flags()?;
        let base = self.get_extended(0xE100)?;
        let deltas = self.get_extended(0xE200)?;
        let alp_channel_lta = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_CHANNEL_LTA)?);
        let alp_channel_count = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_CHANNEL_COUNT)?);
        let alp_count_a = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_CHANNEL_COUNT_A)?);
        let alp_count_b = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_CHANNEL_COUNT_B)?);
        let alp_comp_a = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_ATI_COMP_A)?);
        let alp_comp_b = u16::from_le_bytes(self.read_two_bytes(defs::IQS7211E_MM_ALP_ATI_COMP_B)?);

        info!(
          "\x1B[9F\x1B[J CHRG: {:?}   FNGRS: {:?}   TP MVMNT: {:?}   ALP OUT: {:?}   ALP LTA: {:?}   ALP CNT: {:?}   ALP CNT A: {:?}    ALP CNT B: {:?}    ALP COMP A: {:?}    ALP COMP B: {:?}",
          info_flags.charge_mode(),
          info_flags.num_fingers(),
          info_flags.tp_movement(),
          info_flags.alp_output(),
          alp_channel_lta,
          alp_channel_count,
          alp_count_a,
          alp_count_b,
          alp_comp_a,
          alp_comp_b
        );

        info!("DELTAS:");
        self.print_counts(&deltas);

        info!("BASE TARGETS:");
        self.print_counts(&base);
        Ok(SetupStep::Step3)
      }
    }
  }

  fn print_counts(&self, counts: &[u16; 9]) {
    for i in 0..3 {
      info!("{:?} {:?} {:?}", counts[i * 3 + 0], counts[i * 3 + 1], counts[i * 3 + 2]);
    }
  }

  fn get_extended(&mut self, from: u16) -> Result<[u16; 9], Error<E>> {
    let mut ret = [0u16; 9];

    for i in 0..=8 {
      let addr = from + i as u16;
      let regs = addr.to_be_bytes();
      let mut buf = [0u8; 2];
      self
        .i2c
        .write_read(defs::IQS7211E_I2C_ADDR, &regs, &mut buf)
        .map_err(Error::I2c)?;

      ret[i] = u16::from_le_bytes(buf);
    }

    Ok(ret)
  }

  pub async fn force_i2c(&mut self) -> Result<(), Error<E>> {
    _ = self.rdy.wait_for_high().await;
    self.write_bytes(0xFF, &[0x00])
  }
}
