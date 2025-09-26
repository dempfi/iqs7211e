//! Minimal configuration example.
#![allow(unused)]
use embedded_hal_async::{
  digital::Wait,
  i2c::{I2c, SevenBitAddress},
};
use iqs7211e::{Config, Iqs7211e, Pin, Pinout};

#[allow(dead_code)]
async fn main_async<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
where
  I2C: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  let config = Config::default().with_pinout(
    Pinout::default()
      .with_rxtx([Pin::RxTx0, Pin::RxTx2, Pin::RxTx4], [Pin::Tx8, Pin::Tx9])
      .with_alp_rxtx([Pin::RxTx0], [Pin::Tx8]),
  );

  let mut dev = Iqs7211e::new(i2c, rdy, config);
  let _ = dev.initialize().await?;
  Ok(())
}

fn main() {}
