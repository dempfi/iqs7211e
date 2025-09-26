use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Error, Iqs7211e, Reg};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Set the ACK_RESET bit which clears the SHOW_RESET flag in [`crate::Info`].
  pub async fn ack_reset(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.ack_reset = true).await
  }

  /// Trigger a fresh trackpad ATI routine.
  pub async fn trigger_autotune(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.trackpad_retune = true).await
  }

  /// Trigger a fresh ALP ATI routine.
  pub async fn trigger_autotune_for_alp(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.alp_retune = true).await
  }

  /// Issue a software reset (SW_RESET bit) to the controller.
  pub async fn software_reset(&mut self) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.sw_reset = true).await
  }

  /// Change the charge/sensing mode used by the controller.
  pub async fn set_charge_mode(&mut self, mode: ChargeMode) -> Result<(), Error<E>> {
    self.modify_sys_control(|sys| sys.charge_mode = mode).await
  }

  async fn modify_sys_control<F: FnOnce(&mut SysControl)>(&mut self, f: F) -> Result<(), Error<E>> {
    let mut control = self.read(Reg::SysControl).await?;
    f(&mut control);
    self.write(Reg::SysControl, control).await
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct SysControl {
  #[bits(3)]
  pub charge_mode: ChargeMode,
  pub trackpad_reseed: bool,
  pub alp_reseed: bool,
  pub trackpad_retune: bool,
  pub alp_retune: bool,
  pub ack_reset: bool,
  #[skip(1)]
  pub sw_reset: bool,
  #[skip(1)]
  pub suspend: bool,
  #[skip(3)]
  pub tx_test: bool,
}

impl Default for SysControl {
  fn default() -> Self {
    Self {
      charge_mode: ChargeMode::LowPower1,
      trackpad_reseed: false,
      alp_reseed: false,
      trackpad_retune: false,
      alp_retune: false,
      ack_reset: false,
      sw_reset: false,
      suspend: false,
      tx_test: false,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeMode {
  Active = 0b000,
  IdleTouch = 0b001,
  Idle = 0b010,
  LowPower1 = 0b011,
  LowPower2 = 0b100,
}

impl From<ChargeMode> for u8 {
  fn from(v: ChargeMode) -> Self {
    v as u8
  }
}

impl TryFrom<u8> for ChargeMode {
  type Error = ();

  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b111 {
      0b000 => Ok(Self::Active),
      0b001 => Ok(Self::IdleTouch),
      0b010 => Ok(Self::Idle),
      0b011 => Ok(Self::LowPower1),
      0b100 => Ok(Self::LowPower2),
      _ => Err(()),
    }
  }
}
