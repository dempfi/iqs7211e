use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Reg, Error, Iqs7211e};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Update the interrupt delivery mode (Event or Stream).
  pub async fn set_interrupt_mode(&mut self, mode: InterruptMode) -> Result<(), Error<E>> {
    self.modify_config_settings(|cfg| cfg.interrupt_mode = mode).await
  }

  /// Toggle the manual control bit.
  pub async fn set_manual_control(&mut self, enable: bool) -> Result<(), Error<E>> {
    self.modify_config_settings(|cfg| cfg.manual_control = enable).await
  }

  async fn modify_config_settings<F: FnOnce(&mut ConfigSettings)>(&mut self, f: F) -> Result<(), Error<E>> {
    let mut settings = self.read(Reg::ConfigSettings).await?;
    f(&mut settings);
    self.write(Reg::ConfigSettings, settings).await
  }

  /// Fetch the current config delivery settings from the device.
  pub async fn config_settings(&mut self) -> Result<ConfigSettings, Error<E>> {
    self.read(Reg::ConfigSettings).await
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct ConfigSettings {
  #[skip(2)]
  pub trackpad_autotune: bool,
  pub alp_autotune: bool,
  pub comms_request: bool,
  pub watchdog: bool,
  pub end_comms: bool,
  pub manual_control: bool,
  #[bits(1)]
  pub interrupt_mode: InterruptMode,
  #[bits(6)]
  pub event_triggers: EventTriggers,
}

impl Default for ConfigSettings {
  fn default() -> Self {
    Self {
      trackpad_autotune: true,
      alp_autotune: true,
      comms_request: false,
      watchdog: true,
      end_comms: false,
      manual_control: false,

      interrupt_mode: InterruptMode::Stream,
      event_triggers: EventTriggers::new(true, true, false, false, false),
    }
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u8)]
pub struct EventTriggers {
  pub gesture: bool,
  pub trackpad: bool,
  pub retuning: bool,
  #[skip(1)]
  pub alp: bool,
  pub trackpad_touch: bool,
}

impl EventTriggers {
  pub const fn new(gesture: bool, trackpad: bool, retuning: bool, alp: bool, trackpad_touch: bool) -> Self {
    Self { gesture, trackpad, retuning, alp, trackpad_touch }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptMode {
  /// I2C is presented each cycle (except auto-prox cycles)
  Stream = 0b0,
  /// I2C is only initiated when an enabled event occurs
  Event = 0b1,
}

impl From<InterruptMode> for u8 {
  fn from(v: InterruptMode) -> Self {
    v as u8
  }
}

impl TryFrom<u8> for InterruptMode {
  type Error = ();

  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b1 {
      0b0 => Ok(Self::Stream),
      0b1 => Ok(Self::Event),
      _ => Err(()),
    }
  }
}
