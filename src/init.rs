use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

use super::{Error, Iqs7211e, defs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum InitState {
  VerifyProduct,
  ReadReset,
  ChipReset,
  UpdateSettings,
  AckReset,
  Ati,
  WaitForAti,
  SetInterruptMode,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub(super) async fn init(&mut self) -> Result<bool, Error<E>> {
    let mut state = InitState::VerifyProduct;

    loop {
      self.wait_for_comm_window().await?;

      match state {
        InitState::VerifyProduct => {
          let prod_num = self.product_num()?;
          if prod_num == defs::IQS7211E_PRODUCT_NUM {
            state = InitState::ReadReset;
          } else {
            return Err(Error::InvalidChipId(prod_num as u8));
          }
        }

        InitState::ReadReset => {
          // Returns `true` if a reset has occurred the device settings should be reloaded using
          // the begin function. After new device settings have been reloaded the acknowledge
          // reset function can be used to clear the reset flag
          if self.info_flags()?.show_reset() {
            state = InitState::UpdateSettings;
          } else {
            state = InitState::ChipReset;
          }
        }

        InitState::ChipReset => {
          self.sys_control(|x| x.set_sw_reset(true))?;
          // @TODO: Wait for reset to complete
          // embassy_time::Timer::after_millis(100).await;
          state = InitState::ReadReset;
        }

        InitState::UpdateSettings => {
          self.write_config(self.config)?;
          state = InitState::AckReset;
        }

        InitState::AckReset => {
          self.sys_control(|x| x.set_ack_reset(true))?;
          state = InitState::Ati;
        }

        InitState::Ati => {
          self.sys_control(|x| x.set_tp_re_ati(true))?;
          state = InitState::WaitForAti;
        }

        InitState::WaitForAti => {
          // If the ATI routine is active the channel states (NONE, PROX, TOUCH)
          // might exhibit unwanted behaviour. Thus it is advised to wait for the
          // routine to complete before continuing.
          if self.info_flags()?.re_ati_occurred() {
            state = InitState::SetInterruptMode;
          }
        }

        InitState::SetInterruptMode => {
          let mode = self.config.interrupt_mode;
          self.config_settings(|x| x.set_interrupt_mode(mode))?;
          return Ok(true);
        }
      }
    }
  }

  fn product_num(&mut self) -> Result<u16, Error<E>> {
    let buf = self.read_two_bytes(defs::IQS7211E_MM_PROD_NUM)?;
    Ok(u16::from_le_bytes(buf))
  }
}
