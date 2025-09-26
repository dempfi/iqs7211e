use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{Error, Iqs7211e, Point, Reg};

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub async fn gesture(&mut self) -> Result<Gesture, Error<E>> {
    self.read(Reg::GestureX).await
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
  Tap(TapCount, Point),
  PressHold(Point),
  Palm,
  Swipe(SwipeDirection, Vector),
  SwipeHold(SwipeDirection, Vector),
}

impl Gesture {
  pub fn is_swipe(&self) -> bool {
    matches!(self, Gesture::Swipe(_, _) | Gesture::SwipeHold(_, _))
  }

  pub fn is_tap(&self) -> bool {
    matches!(self, Gesture::Tap(_, _))
  }
}

impl TryFrom<[u8; 6]> for Gesture {
  type Error = ();

  fn try_from(data: [u8; 6]) -> Result<Self, Self::Error> {
    let kind = u16::from_le_bytes([data[4], data[5]]);
    match kind {
      0b0000_0000_0000_0001 => Ok(Self::Tap(TapCount::One, Point::from(&data[0..4]))),
      0b0000_0000_0000_0010 => Ok(Self::Tap(TapCount::Two, Point::from(&data[0..4]))),
      0b0000_0000_0000_0100 => Ok(Self::Tap(TapCount::Three, Point::from(&data[0..4]))),
      0b0000_0000_0000_1000 => Ok(Self::PressHold(Point::from(&data[0..4]))),
      0b0000_0000_0001_0000 => Ok(Self::Palm),
      0b0000_0001_0000_0000 => Ok(Self::Swipe(SwipeDirection::XPositive, Vector::from(&data[0..4]))),
      0b0000_0010_0000_0000 => Ok(Self::Swipe(SwipeDirection::XNegative, Vector::from(&data[0..4]))),
      0b0000_0100_0000_0000 => Ok(Self::Swipe(SwipeDirection::YPositive, Vector::from(&data[0..4]))),
      0b0000_1000_0000_0000 => Ok(Self::Swipe(SwipeDirection::YNegative, Vector::from(&data[0..4]))),
      0b0001_0000_0000_0000 => Ok(Self::SwipeHold(SwipeDirection::XPositive, Vector::from(&data[0..4]))),
      0b0010_0000_0000_0000 => Ok(Self::SwipeHold(SwipeDirection::XNegative, Vector::from(&data[0..4]))),
      0b0100_0000_0000_0000 => Ok(Self::SwipeHold(SwipeDirection::YPositive, Vector::from(&data[0..4]))),
      0b1000_0000_0000_0000 => Ok(Self::SwipeHold(SwipeDirection::YNegative, Vector::from(&data[0..4]))),
      _ => Err(()),
    }
  }
}

impl Point {
  fn from(data: &[u8]) -> Self {
    Self::new(u16::from_le_bytes([data[0], data[1]]), u16::from_le_bytes([data[2], data[3]]))
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vector {
  pub dx: i16,
  pub dy: i16,
}

impl Vector {
  pub fn new(dx: i16, dy: i16) -> Self {
    Self { dx, dy }
  }

  fn from(data: &[u8]) -> Self {
    Self::new(i16::from_le_bytes([data[0], data[1]]), i16::from_le_bytes([data[2], data[3]]))
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TapCount {
  One = 1,
  Two = 2,
  Three = 3,
}

impl core::fmt::Debug for TapCount {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}", *self as u8)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
  XPositive,
  XNegative,
  YPositive,
  YNegative,
}

impl SwipeDirection {
  pub fn is_horizontal(&self) -> bool {
    matches!(self, SwipeDirection::XPositive | SwipeDirection::XNegative)
  }

  pub fn is_vertical(&self) -> bool {
    matches!(self, SwipeDirection::YPositive | SwipeDirection::YNegative)
  }
}
