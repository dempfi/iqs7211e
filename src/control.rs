use super::{Error, Iqs7211e, defs};
use bitfield_struct::bitfield;
use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub(crate) struct InfoFlags {
  #[bits(3)]
  pub(crate) charge_mode: ChargeMode,
  pub(crate) ati_error: bool,
  pub(crate) re_ati_occurred: bool,
  pub(crate) alp_ati_error: bool,
  pub(crate) alp_re_ati_occurred: bool,
  pub(crate) show_reset: bool,
  #[bits(2)]
  pub(crate) num_fingers: u8,
  pub(crate) tp_movement: bool,
  __: bool,
  pub(crate) too_many_fingers: bool,
  ___: bool,
  pub(crate) alp_output: bool,
  ____: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub(crate) enum ChargeMode {
  Active = 0b000,
  IdleTouch = 0b001,
  Idle = 0b010,
  LowPower1 = 0b011,
  LowPower2 = 0b100,
}

impl ChargeMode {
  pub(crate) const fn into_bits(self) -> u8 {
    self as _
  }

  pub(crate) const fn from_bits(bits: u8) -> Self {
    match bits {
      0b000 => Self::Active,
      0b001 => Self::IdleTouch,
      0b010 => Self::Idle,
      0b011 => Self::LowPower1,
      0b100 => Self::LowPower2,
      _ => unreachable!(),
    }
  }
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub(crate) struct SysControl {
  #[bits(3)]
  pub(crate) charge_mode: ChargeMode,
  pub(crate) tp_reseed: bool,
  pub(crate) alp_reseed: bool,
  pub(crate) tp_re_ati: bool,
  pub(crate) alp_re_ati: bool,
  pub(crate) ack_reset: bool,
  __: bool,
  pub(crate) sw_reset: bool,
  ___: bool,
  pub(crate) suspend: bool,
  #[bits(3)]
  ____: u8,
  pub(crate) tx_test: bool,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub(crate) struct ConfigSettings {
  #[bits(2)]
  __: u8,
  pub(crate) tp_re_ati_enable: bool,
  pub(crate) alp_re_ati_enable: bool,
  pub(crate) comms_request_enable: bool,
  pub(crate) watchdog_timer: bool,
  pub(crate) comms_end_cmd: bool,
  pub(crate) manual_control: bool,
  #[bits(1)]
  pub(crate) interrupt_mode: InterruptMode,
  pub(crate) gesture_event: bool,
  pub(crate) tp_event: bool,
  pub(crate) re_ati_event: bool,
  ___: bool,
  pub(crate) alp_event: bool,
  pub(crate) tp_touch_event: bool,
  ____: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum InterruptMode {
  /// I2C is presented each cycle (except auto-prox cycles)
  Stream = 0b0,
  /// I2C is only initiated when an enabled event occurs
  Event = 0b1,
}

impl InterruptMode {
  pub(crate) const fn into_bits(self) -> u8 {
    self as _
  }

  pub(crate) const fn from_bits(bits: u8) -> Self {
    match bits {
      0b0 => Self::Stream,
      0b1 => Self::Event,
      _ => unreachable!(),
    }
  }
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub(crate) fn info_flags(&mut self) -> Result<InfoFlags, Error<E>> {
    let buf = self.read_two_bytes(defs::IQS7211E_MM_INFO_FLAGS)?;
    Ok(InfoFlags::from_bits(u16::from_le_bytes(buf)))
  }

  pub(crate) fn sys_control<F: FnOnce(&mut SysControl)>(&mut self, f: F) -> Result<(), Error<E>> {
    let buf = self.read_two_bytes(defs::IQS7211E_MM_SYS_CONTROL)?;
    let mut sys_control = SysControl::from_bits(u16::from_le_bytes(buf));

    f(&mut sys_control);

    self.write_bytes(defs::IQS7211E_MM_SYS_CONTROL, &sys_control.into_bits().to_le_bytes())
  }

  pub(crate) fn config_settings<F: FnOnce(&mut ConfigSettings)>(&mut self, f: F) -> Result<(), Error<E>> {
    let buf = self.read_two_bytes(defs::IQS7211E_MM_CONFIG_SETTINGS)?;
    let mut config_settings = ConfigSettings::from_bits(u16::from_le_bytes(buf));

    f(&mut config_settings);

    self.write_bytes(defs::IQS7211E_MM_CONFIG_SETTINGS, &config_settings.into_bits().to_le_bytes())
  }
}
