//! Event processing example: gestures, single-touch, multi-touch.
#![allow(unused)]
use embedded_hal_async::{
  digital::Wait,
  i2c::{I2c, SevenBitAddress},
};
use iqs7211e::{Config, Event, Iqs7211e};

#[allow(dead_code)]
async fn main_async<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
where
  I2C: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  let mut dev = Iqs7211e::new(i2c, rdy, Config::default());
  let _ = dev.initialize().await?;

  loop {
    match dev.next_event().await? {
      Event::Gesture(gesture, info) => {
        let _ = (gesture, info);
        // handle gesture
      }
      Event::Touch(primary, info) => {
        let _ = (primary, info);
        // handle single touch
      }
      Event::MultiTouch(primary, secondary, info) => {
        let _ = (primary, secondary, info);
        // handle two touches
      }
    }
  }
}

fn main() {}
