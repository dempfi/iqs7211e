use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Error, Iqs7211e, Reg};

mod config_settings;
mod system_control;

pub use config_settings::*;
pub use system_control::*;

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Fetch the product number and firmware revision as reported by the device.
  pub async fn app_version(&mut self) -> Result<Version, Error<E>> {
    self.read(Reg::AppVersion).await
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[packbits::pack(bytes = 10)]
pub struct Version {
  pub number: u16,
  pub major: u8,
  #[skip(8)]
  pub minor: u8,
  #[skip(8)]
  pub commit: u32,
}
