use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Error, Iqs7211e, Reg};

mod gesture;
mod info;
mod touchpoint;

pub use gesture::*;
pub use info::*;
pub use touchpoint::*;

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  /// Await the next interesting event from the controller.
  ///
  /// Resolves only when either a gesture is present or the trackpad reports
  /// movement/contacts.
  pub async fn next_event(&mut self) -> Result<Event, Error<E>> {
    loop {
      // Take a coherent snapshot in a single RDY window
      self.wait_for_comm_window().await?;
      let gesture = self.gesture().await.ok();
      let info: Info = self.read(Reg::InfoFlags).await?;
      let touchpoints: Touchpoints = self.touchpoints().await?;

      if let Some(gesture) = gesture {
        return Ok(Event::Gesture(gesture, info));
      }

      // Classify movement by number of active contacts.
      if !touchpoints.primary.is_empty() && !touchpoints.secondary.is_empty() {
        return Ok(Event::MultiTouch(touchpoints.primary, touchpoints.secondary, info));
      } else if !touchpoints.primary.is_empty() {
        return Ok(Event::Touch(touchpoints.primary, info));
      }

      // Otherwise, keep waiting for the next RDY window
    }
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[packbits::pack(u32)]
pub struct Point {
  pub x: u16,
  pub y: u16,
}

impl Point {
  pub fn new(x: u16, y: u16) -> Self {
    Self { x, y }
  }
}

impl core::fmt::Debug for Point {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "({}, {})", self.x, self.y)
  }
}

/// High-level event emitted when the controller signals an update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
  Gesture(Gesture, Info),
  Touch(Touchpoint, Info),
  MultiTouch(Touchpoint, Touchpoint, Info),
}
