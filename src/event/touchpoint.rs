use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Reg, Error, Iqs7211e, Point};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub async fn touchpoints(&mut self) -> Result<Touchpoints, Error<E>> {
    self.read(Reg::Finger1X).await
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u128)]
pub struct Touchpoints {
  #[bits(64)]
  pub primary: Touchpoint,
  #[bits(64)]
  pub secondary: Touchpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[packbits::pack(u64)]
pub struct Touchpoint {
  #[bits(32)]
  pub point: Point,
  pub strength: u16,
  pub area: u16,
}

impl Touchpoint {
  pub fn is_empty(&self) -> bool {
    self.point.x == 0xFFFF && self.point.y == 0xFFFF
  }
}
