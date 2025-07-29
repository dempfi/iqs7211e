use defmt::info;
use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

use super::{DeviceState, Error, Iqs7211e, defs};

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Event {
  pub gesture: Gesture,
}

impl Event {
  pub fn new(gesture: Gesture) -> Self {
    Self { gesture }
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

impl Gesture {
  pub(crate) const fn into_bits(self) -> u16 {
    self as _
  }

  pub(crate) const fn from_bits(bits: u16) -> Self {
    match bits {
      0b0000_0000_0000_0001 => Self::SingleTap,
      0b0000_0000_0000_0010 => Self::DoubleTap,
      0b0000_0000_0000_0100 => Self::TripleTap,
      0b0000_0000_0000_1000 => Self::PressHold,
      0b0000_0000_0001_0000 => Self::Palm,
      0b0000_0001_0000_0000 => Self::SwipeXPositive,
      0b0000_0010_0000_0000 => Self::SwipeXNegative,
      0b0000_0100_0000_0000 => Self::SwipeYPositive,
      0b0000_1000_0000_0000 => Self::SwipeYNegative,
      0b0001_0000_0000_0000 => Self::SwipeHoldXPositive,
      0b0010_0000_0000_0000 => Self::SwipeHoldXNegative,
      0b0100_0000_0000_0000 => Self::SwipeHoldYPositive,
      0b1000_0000_0000_0000 => Self::SwipeHoldYNegative,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Touch {
  pub x: u16,
  pub y: u16,
  pub strength: u16,
  pub area: u16,
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub async fn on_event(&mut self) -> Result<Event, Error<E>> {
    self.wait_for_comm_window().await?;
    let gesture = self.get_gesture()?;
    info!("IQS7211E: Gesture detected: {:?}", gesture);

    let info_flags = self.info_flags()?;
    info!(
      "CHRG: {:?}   FNGRS: {:?}   TP MVMNT: {:?}   ALP OUT: {:?}",
      info_flags.charge_mode(),
      info_flags.num_fingers(),
      info_flags.tp_movement(),
      info_flags.alp_output()
    );

    let finger = self.finger1()?;
    info!("Finger 1: x: {}, y: {}, strength: {}, area: {}", finger.x, finger.y, finger.strength, finger.area);

    match self.state {
      DeviceState::Init => {
        if self.init().await? {
          info!("IQS7211E: Settings updated, device is initialized");
          self.state = DeviceState::Run;
        }
      }
      DeviceState::CheckReset => {
        if self.info_flags()?.show_reset() {
          self.state = DeviceState::Init;
        } else {
          self.state = DeviceState::Run;
        }
      }
      DeviceState::Run => {
        self.state = DeviceState::CheckReset;
      }
    }

    Ok(Event::new(gesture.unwrap_or(Gesture::Palm)))
  }

  pub(crate) fn get_gesture(&mut self) -> Result<Option<Gesture>, Error<E>> {
    let buf = self.read_two_bytes(defs::IQS7211E_MM_GESTURES)?;
    if buf == [0, 0] {
      return Ok(None);
    } else {
      Ok(Some(Gesture::from_bits(u16::from_le_bytes(buf))))
    }
  }

  pub(crate) fn finger1(&mut self) -> Result<Touch, Error<E>> {
    let mut buf = [0u8; 8];
    self.read_bytes(defs::IQS7211E_MM_FINGER_1_X, &mut buf)?;
    let x = u16::from_le_bytes([buf[0], buf[1]]);
    let y = u16::from_le_bytes([buf[2], buf[3]]);
    let strength = u16::from_le_bytes([buf[4], buf[5]]);
    let area = u16::from_le_bytes([buf[6], buf[7]]);

    Ok(Touch { x, y, strength, area })
  }
}
