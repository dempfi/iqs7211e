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
//!
//! ```no_run
//! use embedded_hal_async::{digital::Wait, i2c::{I2c, SevenBitAddress}};
//! use iqs7211e::{Config, Iqs7211e, Pinout, Pin};
//!
//! async fn example<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
//! where
//!   I2C: I2c<SevenBitAddress, Error = E>,
//!   RDY: Wait,
//! {
//!   let config = Config::default()
//!     .with_pinout(
//!       Pinout::new(
//!         [Pin::RxTx0, Pin::RxTx1, Pin::RxTx2, Pin::RxTx3],
//!         [Pin::Tx8, Pin::Tx9, Pin::Tx10],
//!         [],
//!         []
//!       )
//!     );
//!
//!   let mut controller = Iqs7211e::new(i2c, rdy, config);
//!   _ = controller.initialize().await?;
//!   Ok(())
//! }
//! ```
mod config;
mod control;
mod event;
mod reg;
mod rw;
mod setup;

use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

pub use config::*;
pub use control::*;
pub use event::*;
use reg::*;
pub use setup::*;

/// Errors that can occur while interacting with the controller.
#[derive(Debug)]
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
    Self { i2c, rdy, config }
  }

  /// Initialize the touchpad controller.
  ///
  /// This validates the product identifier, handles chip reset if needed,
  /// pushes the staged configuration, and triggers the ATI calibration routine.
  /// Returns `true` if a configuration update occurred during initialization.
  pub async fn initialize(&mut self) -> Result<bool, Error<E>> {
    // Device boots in Event Mode with Show Reset set. Since no events are
    // happening yet, RDY stays HIGH. Force first communication window.
    self.force_comms_request().await?;

    // Verify chip ID
    let prod_num = self.app_version().await?.number;
    if prod_num != PRODUCT_NUMBER {
      return Err(Error::InvalidChipId(prod_num as u8));
    }

    // Check if reset occurred
    if !self.info().await?.show_reset {
      // No reset detected, request one
      self.software_reset().await?;
      // Wait for reset to complete - force comms since RDY won't pulse
      loop {
        self.wait_for_comm_window().await?;
        if self.info().await?.show_reset {
          break;
        }
      }
    }

    // Switch to Stream Mode for initialization so RDY pulses every cycle
    // This avoids having to force comms repeatedly
    self.set_interrupt_mode(InterruptMode::Stream).await?;

    // Configure device
    self.wait_for_comm_window().await?;
    let config = self.config;
    self.write_config(&config).await?;

    self.wait_for_comm_window().await?;
    self.ack_reset().await?;

    // Trigger ATI and wait for completion
    self.wait_for_comm_window().await?;
    self.trigger_autotune().await?;

    loop {
      self.wait_for_comm_window().await?;
      if self.info().await?.re_auto_tuning_occurred {
        break;
      }
    }

    // Set final interrupt mode from config (may switch back to Event Mode)
    self.wait_for_comm_window().await?;
    self.set_interrupt_mode(self.config.interrupt_mode).await?;

    Ok(true)
  }
}
