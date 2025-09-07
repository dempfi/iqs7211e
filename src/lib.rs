#![no_std]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! Async, `no_std` driver for the Azoteq IQS7211E capacitive touch and gesture
//! controller.
//!
//! The IQS7211E is an advanced multi-channel capacitive sensor that provides
//! rich gesture detection in addition to traditional proximity sensing. This
//! crate exposes a strongly typed API on top of the raw register map, with
//! helpers for:
//!
//! - Applying the reference configuration sequence recommended by Azoteq
//! - Managing the complex Rx/Tx matrix allocation and sensing cycles
//! - Tuning ATI, debounce, gesture, and idle behaviour via descriptive
//!   structures instead of raw bit-twiddling
//! - Using `embedded-hal` / `embedded-hal-async` 1.0 traits so the driver works
//!   across MCU families
//! - Querying firmware details and live gesture/touch status without manual
//!   register juggling
//! - Optional high-level touchpad event façade (enable the `touchpad` Cargo feature)
//!
//! ```no_run
//! use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
//! use iqs7211e::{Config, Iqs7211e, SensorPinMapping};
//!
//! async fn example<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
//! where
//!   I2C: I2c<SevenBitAddress, Error = E>,
//!   RDY: Wait,
//! {
//!   let mapping = SensorPinMapping::new(&[0, 1, 2, 3, 4], &[0, 1], &[0, 1], &[0]);
//!   let config = Config::builder()
//!     .sensor_pin_mapping(mapping)
//!     .build();
//!
//!   let mut controller = Iqs7211e::new(i2c, rdy, config);
//!   _ = controller.initialize().await?;
//!   Ok(())
//! }
//! ```
mod config;
mod control;
mod defs;
mod event;
mod setup;
#[cfg(feature = "touchpad")]
mod touchpad;

use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

pub use config::*;
use defs::*;
pub use defs::{ChargeMode, ConfigSettings, InfoFlags, InterruptMode, NumFingers, SysControl, Version};
pub use event::{Finger, Gesture, Report};
pub use setup::*;
#[cfg(feature = "touchpad")]
pub use touchpad::*;

/// Errors that can occur while interacting with the controller.
#[derive(Debug, defmt::Format)]
pub enum Error<E> {
  /// I²C bus transaction failed with the underlying driver error.
  I2c(E),
  /// The device reported an unexpected chip identifier during bring-up.
  InvalidChipId(u8),
  /// An operation attempted to write a buffer larger than the protocol allows.
  BufferOverflow,
}

/// High-level state machine for the Azoteq IQS7211E controller.
///
/// The driver owns the I²C peripheral and RDY pin and offers strongly typed
/// configuration helpers and control functions. Create an instance with
/// [`Iqs7211e::new`], provide a [`config::Config`], and then call
/// [`Iqs7211e::initialize`] to stage the desired setup on the device.
pub struct Iqs7211e<I, RDY> {
  i2c: I,
  rdy: RDY,
  initialized: bool,
  config: config::Config,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Create a new driver instance with the provided peripherals and
  /// configuration template.
  ///
  /// The configuration is not transmitted to the device until
  /// [`Iqs7211e::initialize`] is called. This allows the caller to adjust fields
  /// after construction if desired.
  pub fn new(i2c: I, rdy: RDY, config: config::Config) -> Self {
    Self { i2c, rdy, initialized: false, config }
  }

  /// Initialize the touchpad controller.
  ///
  /// This validates the product identifier, handles chip reset if needed,
  /// pushes the staged configuration, and triggers the ATI calibration routine.
  /// Returns `true` if a configuration update occurred during initialization.
  pub async fn initialize(&mut self) -> Result<bool, Error<E>> {
    // Verify chip ID
    self.wait_for_comm_window().await?;
    let prod_num = self.app_version().await?.number;
    if prod_num != PRODUCT_NUMBER {
      return Err(Error::InvalidChipId(prod_num as u8));
    }

    // Reset if needed
    self.wait_for_comm_window().await?;
    if !self.info_flags().await?.show_reset {
      self.software_reset().await?;
      // @TODO: Wait for reset to complete
      // embassy_time::Timer::after_millis(100).await;
    }

    // Configure device
    self.wait_for_comm_window().await?;
    self.write_config(self.config).await?;

    self.wait_for_comm_window().await?;
    self.acknowledge_reset().await?;

    // Trigger ATI and wait for completion
    self.wait_for_comm_window().await?;
    self.trigger_retune().await?;

    loop {
      self.wait_for_comm_window().await?;
      if self.info_flags().await?.re_auto_tuning_occurred {
        break;
      }
    }

    // Set final interrupt mode
    self.wait_for_comm_window().await?;
    self.set_interrupt_mode(self.config.interrupt_mode).await?;

    Ok(true)
  }

  async fn wait_for_comm_window(&mut self) -> Result<(), Error<E>> {
    self.rdy.wait_for_falling_edge().await.map_err(|_| unreachable!())
  }

  // Typed helpers
  async fn read<const N: usize, T: TryFrom<[u8; N]>>(&mut self, reg: Reg) -> Result<T, Error<E>> {
    let mut b = [0u8; N];
    self.read_bytes(reg, &mut b).await?;
    T::try_from(b).map_err(|_| Error::BufferOverflow)
  }

  async fn read_u16(&mut self, reg: Reg) -> Result<u16, Error<E>> {
    let buf: [u8; 2] = self.read(reg).await?;
    Ok(u16::from_le_bytes(buf))
  }

  async fn write<const N: usize, T: TryInto<[u8; N]>>(&mut self, reg: Reg, v: T) -> Result<(), Error<E>> {
    let b = v.try_into().map_err(|_| Error::BufferOverflow)?;
    self.write_bytes(reg, &b).await
  }

  async fn read_bytes(&mut self, reg: Reg, buf: &mut [u8]) -> Result<(), Error<E>> {
    let addr = [reg as u8];
    self.i2c.write_read(I2C_ADDR, &addr, buf).await.map_err(Error::I2c)
  }

  async fn write_bytes(&mut self, reg: Reg, data: &[u8]) -> Result<(), Error<E>> {
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
  async fn read_ext_bytes(&mut self, addr: u16, buf: &mut [u8]) -> Result<(), Error<E>> {
    let regs = addr.to_be_bytes();
    self.i2c.write_read(I2C_ADDR, &regs, buf).await.map_err(Error::I2c)
  }

  async fn read_u16_ext(&mut self, addr: u16) -> Result<u16, Error<E>> {
    let mut buf = [0u8; 2];
    self.read_ext_bytes(addr, &mut buf).await?;
    Ok(u16::from_le_bytes(buf))
  }
}
