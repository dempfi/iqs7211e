#![no_std]

mod config;
mod control;
mod defs;
mod event;
mod init;
mod setup;

use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

pub use config::*;
pub use control::InterruptMode;
pub use setup::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeviceState {
  Init,
  CheckReset,
  Run,
}

/// All possible errors in this crate
#[derive(Debug, defmt::Format)]
pub enum Error<E> {
  I2c(E),
  InvalidChipId(u8),
  BufferOverflow,
}

pub struct Iqs7211e<I, RDY> {
  i2c: I,
  rdy: RDY,
  state: DeviceState,
  config: config::Config,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub fn new(i2c: I, rdy: RDY, config: config::Config) -> Self {
    Self { i2c, rdy, state: DeviceState::Init, config }
  }

  async fn wait_for_comm_window(&mut self) -> Result<(), Error<E>> {
    self.rdy.wait_for_falling_edge().await.map_err(|_| unreachable!())
  }

  fn read_two_bytes(&mut self, reg: u8) -> Result<[u8; 2], Error<E>> {
    let mut buf = [0u8; 2];
    self.read_bytes(reg, &mut buf)?;
    Ok(buf)
  }

  fn read_bytes(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Error<E>> {
    self
      .i2c
      .write_read(defs::IQS7211E_I2C_ADDR, &[reg], buf)
      .map_err(Error::I2c)
  }

  fn write_bytes(&mut self, reg: u8, data: &[u8]) -> Result<(), Error<E>> {
    let len = data.len();
    if len > 31 {
      return Err(Error::BufferOverflow);
    }

    let mut buf = [0u8; 32];
    buf[0] = reg;
    buf[1..=len].copy_from_slice(data);

    self
      .i2c
      .write(defs::IQS7211E_I2C_ADDR, &buf[..=len])
      .map_err(Error::I2c)
  }
}
