use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{defs::*, Error, Iqs7211e};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Fetch the product number and firmware revision as reported by the device.
  pub async fn app_version(&mut self) -> Result<Version, Error<E>> {
    self.read(Reg::AppVersion).await
  }

  /// Read the real-time [`InfoFlags`] block.
  pub async fn info_flags(&mut self) -> Result<InfoFlags, Error<E>> {
    self.read(Reg::InfoFlags).await
  }

  /// Fetch the current [`SysControl`] structure from the device.
  pub async fn read_sys_control(&mut self) -> Result<SysControl, Error<E>> {
    self.read(Reg::SysControl).await
  }

  /// Write the provided [`SysControl`] structure to the device.
  pub async fn write_sys_control(&mut self, sys: SysControl) -> Result<(), Error<E>> {
    self.write(Reg::SysControl, sys).await
  }

  /// Fetch the current [`ConfigSettings`] structure from the device.
  pub async fn read_config_settings(&mut self) -> Result<ConfigSettings, Error<E>> {
    self.read(Reg::ConfigSettings).await
  }

  /// Write the provided [`ConfigSettings`] structure to the device.
  pub async fn write_config_settings(&mut self, settings: ConfigSettings) -> Result<(), Error<E>> {
    self.write(Reg::ConfigSettings, settings).await
  }

  /// Set the ACK_RESET bit which clears the SHOW_RESET flag in [`InfoFlags`].
  pub async fn acknowledge_reset(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.ack_reset = true).await
  }

  /// Trigger a fresh trackpad ATI routine.
  pub async fn trigger_retune(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.trackpad_retune = true).await
  }

  /// Trigger a fresh ALP ATI routine.
  pub async fn trigger_retune_for_low_power(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.low_power_retune = true).await
  }

  /// Issue a software reset (SW_RESET bit) to the controller.
  pub async fn software_reset(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.sw_reset = true).await
  }

  /// Change the charge/sensing mode used by the controller.
  pub async fn set_charge_mode(&mut self, mode: ChargeMode) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.charge_mode = mode).await
  }

  /// Update the interrupt delivery mode (Event or Stream).
  pub async fn set_interrupt_mode(&mut self, mode: InterruptMode) -> Result<(), Error<E>> {
    self.modify_config_settings(|cfg| cfg.interrupt_mode = mode).await
  }

  /// Toggle the manual control bit.
  pub async fn set_manual_control(&mut self, enable: bool) -> Result<(), Error<E>> {
    self.modify_config_settings(|cfg| cfg.manual_control = enable).await
  }

  async fn modify_sys_control<F: FnOnce(&mut SysControl)>(&mut self, f: F) -> Result<(), Error<E>> {
    let mut sys_control = self.read_sys_control().await?;

    f(&mut sys_control);

    self.write_sys_control(sys_control).await
  }

  async fn modify_config_settings<F: FnOnce(&mut ConfigSettings)>(&mut self, f: F) -> Result<(), Error<E>> {
    let mut config_settings = self.read_config_settings().await?;

    f(&mut config_settings);

    self.write_config_settings(config_settings).await
  }
}
