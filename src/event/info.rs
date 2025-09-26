use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{ChargeMode, Error, Iqs7211e, Reg};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub async fn info(&mut self) -> Result<Info, Error<E>> {
    self.read(Reg::InfoFlags).await
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[packbits::pack(u16)]
pub struct Info {
  #[bits(3)]
  pub charge_mode: ChargeMode,
  pub auto_tuning_error: bool,
  pub re_auto_tuning_occurred: bool,
  pub alp_auto_tuning_error: bool,
  pub alp_re_auto_tuning_occurred: bool,
  pub show_reset: bool,
  #[bits(2)]
  pub num_fingers: u8,
  pub trackpad_movement: bool,
  #[skip(1)]
  pub too_many_fingers: bool,
  #[skip(1)]
  pub alp_output: bool,
}
