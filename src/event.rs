use defmt::info;
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{defs::*, Error, Iqs7211e};

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Report {
  pub gesture: Option<Gesture>,
  pub info: InfoFlags,
  pub fingers: (Finger, Finger),
}

impl Report {
  /// Build a new event snapshot with the supplied payload.
  pub fn new(gesture: Option<Gesture>, info: InfoFlags, fingers: (Finger, Finger)) -> Self {
    Self { gesture, info, fingers }
  }

  /// Get the primary finger (first finger) snapshot.
  pub fn primary_finger(&self) -> Finger {
    self.fingers.0
  }

  /// Get the secondary finger (second finger) snapshot.
  pub fn secondary_finger(&self) -> Finger {
    self.fingers.1
  }

  /// Get both fingers as a tuple (primary, secondary).
  pub fn fingers(&self) -> (Finger, Finger) {
    self.fingers
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::convert::TryFrom;

  #[test]
  fn finger_is_present_flag() {
    assert!(Finger::new(10, 20, 30, 40).is_present());
    assert!(!Finger::new(0xFFFF, 0xFFFF, 0, 0).is_present());
  }

  #[test]
  fn finger_packbits_roundtrip() {
    let original = Finger::new(0x0102, 0x0304, 0x0506, 0x0708);
    let packed: [u8; 8] = original.into();
    assert_eq!(packed, [0x02, 0x01, 0x04, 0x03, 0x06, 0x05, 0x08, 0x07]);

    let decoded = Finger::try_from(packed).expect("finger decode");
    assert_eq!(decoded, original);
  }

  #[test]
  fn gesture_try_from_enforces_single_bit() {
    assert_eq!(Gesture::try_from(0b0000_0000_0000_0001u16).ok(), Some(Gesture::SingleTap));
    assert_eq!(Gesture::try_from(0b0000_0001_0000_0000u16).ok(), Some(Gesture::SwipeXPositive));
    assert!(Gesture::try_from(0b0000_0000_0000_0011u16).is_err());
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 8)]
pub struct Finger {
  pub x: u16,
  pub y: u16,
  pub strength: u16,
  pub area: u16,
}

impl Finger {
  pub const fn new(x: u16, y: u16, strength: u16, area: u16) -> Self {
    Self { x, y, strength, area }
  }

  /// Returns a sentinel finger representing "no touch".
  pub const fn absent() -> Self {
    Self::new(0xFFFF, 0xFFFF, 0, 0)
  }

  /// Returns `true` if the finger represents an active touch.
  pub const fn is_present(&self) -> bool {
    self.x != 0xFFFF && self.y != 0xFFFF
  }
}

impl Default for Finger {
  fn default() -> Self {
    Self::absent()
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u16)]
pub enum Gesture {
  SingleTap = 0b0000_0000_0000_0001,
  DoubleTap = 0b0000_0000_0000_0010,
  TripleTap = 0b0000_0000_0000_0100,
  PressHold = 0b0000_0000_0000_1000,
  Palm = 0b0000_0000_0001_0000,
  SwipeXPositive = 0b0000_0001_0000_0000,
  SwipeXNegative = 0b0000_0010_0000_0000,
  SwipeYPositive = 0b0000_0100_0000_0000,
  SwipeYNegative = 0b0000_1000_0000_0000,
  SwipeHoldXPositive = 0b0001_0000_0000_0000,
  SwipeHoldXNegative = 0b0010_0000_0000_0000,
  SwipeHoldYPositive = 0b0100_0000_0000_0000,
  SwipeHoldYNegative = 0b1000_0000_0000_0000,
}

impl From<Gesture> for u16 {
  fn from(g: Gesture) -> Self {
    g as u16
  }
}

impl TryFrom<u16> for Gesture {
  type Error = ();

  fn try_from(bits: u16) -> Result<Self, Self::Error> {
    match bits {
      0b0000_0000_0000_0001 => Ok(Self::SingleTap),
      0b0000_0000_0000_0010 => Ok(Self::DoubleTap),
      0b0000_0000_0000_0100 => Ok(Self::TripleTap),
      0b0000_0000_0000_1000 => Ok(Self::PressHold),
      0b0000_0000_0001_0000 => Ok(Self::Palm),
      0b0000_0001_0000_0000 => Ok(Self::SwipeXPositive),
      0b0000_0010_0000_0000 => Ok(Self::SwipeXNegative),
      0b0000_0100_0000_0000 => Ok(Self::SwipeYPositive),
      0b0000_1000_0000_0000 => Ok(Self::SwipeYNegative),
      0b0001_0000_0000_0000 => Ok(Self::SwipeHoldXPositive),
      0b0010_0000_0000_0000 => Ok(Self::SwipeHoldXNegative),
      0b0100_0000_0000_0000 => Ok(Self::SwipeHoldYPositive),
      0b1000_0000_0000_0000 => Ok(Self::SwipeHoldYNegative),
      _ => Err(()),
    }
  }
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub async fn read_report(&mut self) -> Result<Report, Error<E>> {
    self.wait_for_comm_window().await?;

    let gesture = self.gesture().await?;
    if let Some(g) = gesture {
      info!("IQS7211E: Gesture detected: {:?}", g);
    } else {
      info!("IQS7211E: Gesture status clear");
    }

    let info_flags = self.info_flags().await?;
    info!(
      "CHRG: {:?}   FNGRS: {:?}   TP MVMNT: {:?}   ALP OUT: {:?}",
      info_flags.charge_mode,
      NumFingers::from_bits(info_flags.num_fingers),
      info_flags.trackpad_movement,
      info_flags.low_power_output
    );

    let finger1 = self.primary_finger().await?;
    let finger2 = self.secondary_finger().await?;
    info!("Finger 1: x: {}, y: {}, strength: {}, area: {}", finger1.x, finger1.y, finger1.strength, finger1.area);
    info!("Finger 2: x: {}, y: {}, strength: {}, area: {}", finger2.x, finger2.y, finger2.strength, finger2.area);

    if !self.initialized {
      if self.initialize().await? {
        info!("IQS7211E: Settings updated, device is initialized");
        self.initialized = true;
      }
    } else if info_flags.show_reset {
      self.initialized = false;
    }

    Ok(Report::new(gesture, info_flags, (finger1, finger2)))
  }

  /// Read the current gesture, if any.
  pub async fn gesture(&mut self) -> Result<Option<Gesture>, Error<E>> {
    let raw = self.read_u16(Reg::Gestures).await?;
    if raw == 0 {
      return Ok(None);
    }
    Ok(Gesture::try_from(raw).ok())
  }

  /// Read the primary finger snapshot (absolute position, strength, and area).
  pub async fn primary_finger(&mut self) -> Result<Finger, Error<E>> {
    self.read::<8, Finger>(Reg::Finger1X).await
  }

  /// Read the secondary finger snapshot (absolute position, strength, and area).
  pub async fn secondary_finger(&mut self) -> Result<Finger, Error<E>> {
    self.read::<8, Finger>(Reg::Finger2X).await
  }

  /// Convenience helper returning both finger snapshots in one call.
  pub async fn fingers(&mut self) -> Result<(Finger, Finger), Error<E>> {
    let primary = self.primary_finger().await?;
    let secondary = self.secondary_finger().await?;
    Ok((primary, secondary))
  }
}
