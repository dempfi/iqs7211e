use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Error, Iqs7211e, Reg, I2C_ADDR};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Wait for the IQS7211E to open a communication window by asserting RDY low.
  ///
  /// **Important**: Call this once before a sequence of register operations, not before
  /// each individual I²C transaction. Multiple reads/writes can occur within a single
  /// RDY window. The window remains open until the I²C bus is idle (STOP condition).
  ///
  /// This matches the pattern used in the official Arduino driver where RDY is checked
  /// once at the start of higher-level operations (e.g., `queueValueUpdates()`), allowing
  /// multiple register accesses within that window.
  pub(crate) async fn wait_for_comm_window(&mut self) -> Result<(), Error<E>> {
    self.rdy.wait_for_low().await.map_err(|_| unreachable!())
  }

  /// Force a communication request when RDY is HIGH (per datasheet 11.9.2).
  ///
  /// In Event Mode, if no events are occurring, RDY stays HIGH. This method
  /// writes to address 0xFF with 0x00 to request the device open the next
  /// communication window. Required during initialization or when polling
  /// is necessary while in Event Mode.
  pub(crate) async fn force_comms_request(&mut self) -> Result<(), Error<E>> {
    // Write 0x00 to address 0xFF (without waiting for RDY first)
    let buf = [0xFFu8, 0x00];
    self.i2c.write(I2C_ADDR, &buf).await.map_err(Error::I2c)?;
    // Now wait for the RDY window that the device will open
    self.wait_for_comm_window().await
  }

  // Typed helpers
  pub(crate) async fn read<const N: usize, T: TryFrom<[u8; N]>>(&mut self, reg: Reg) -> Result<T, Error<E>> {
    let mut b = [0u8; N];
    self.read_bytes(reg, &mut b).await?;
    T::try_from(b).map_err(|_| Error::BufferOverflow)
  }

  pub(crate) async fn read_u16(&mut self, reg: Reg) -> Result<u16, Error<E>> {
    let buf: [u8; 2] = self.read(reg).await?;
    Ok(u16::from_le_bytes(buf))
  }

  pub(crate) async fn write<const N: usize, T: TryInto<[u8; N]>>(&mut self, reg: Reg, v: T) -> Result<(), Error<E>> {
    let b = v.try_into().map_err(|_| Error::BufferOverflow)?;
    self.write_bytes(reg, &b).await
  }

  pub(crate) async fn read_bytes(&mut self, reg: Reg, buf: &mut [u8]) -> Result<(), Error<E>> {
    let addr = [reg as u8];
    self.i2c.write_read(I2C_ADDR, &addr, buf).await.map_err(Error::I2c)
  }

  pub(crate) async fn write_bytes(&mut self, reg: Reg, data: &[u8]) -> Result<(), Error<E>> {
    let len = data.len();
    if len > 31 {
      return Err(Error::BufferOverflow);
    }
    let mut buf = [0u8; 32];
    buf[0] = reg.into();
    buf[1..=len].copy_from_slice(data);
    self.i2c.write(I2C_ADDR, &buf[..=len]).await.map_err(Error::I2c)
  }

  // Extended (16-bit addressed) reads for diagnostic pages
  pub(crate) async fn read_ext_bytes(&mut self, addr: u16, buf: &mut [u8]) -> Result<(), Error<E>> {
    let regs = addr.to_be_bytes();
    self.i2c.write_read(I2C_ADDR, &regs, buf).await.map_err(Error::I2c)
  }

  pub(crate) async fn read_u16_ext(&mut self, addr: u16) -> Result<u16, Error<E>> {
    let mut buf = [0u8; 2];
    self.read_ext_bytes(addr, &mut buf).await?;
    Ok(u16::from_le_bytes(buf))
  }
}
