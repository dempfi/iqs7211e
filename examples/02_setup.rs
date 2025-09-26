//! Manual setup session example.
#![allow(unused)]
use embedded_hal_async::{
  digital::Wait,
  i2c::{I2c, SevenBitAddress},
};
use iqs7211e::{Config, Iqs7211e, SetupSnapshot};

#[allow(dead_code)]
async fn main_async<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
where
  I2C: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  let mut dev = Iqs7211e::new(i2c, rdy, Config::default());
  let mut session = dev.begin_setup();

  session.initialize().await?;
  session.enter_manual_control().await?;

  let snapshot: SetupSnapshot = session.snapshot().await?;
  let channels = snapshot.rx_count * snapshot.tx_count;
  let _deltas = &snapshot.trackpad_deltas[..channels];
  let _bases = &snapshot.trackpad_base_targets[..channels];

  session.finish().await?;
  Ok(())
}

fn main() {}
