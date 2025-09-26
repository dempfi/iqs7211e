#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 4)]
pub struct Alp {
  pub(crate) rx: u8,
  #[bits(1)]
  pub sensing_mode: AlpSensingMode,
  pub count_filter: bool,
  #[skip(6)]
  #[bits(13)]
  pub(crate) tx: u16,
}

impl Alp {
  pub const fn new(sensing_mode: AlpSensingMode, count_filter: bool, rx: u8, tx: u16) -> Self {
    Self { sensing_mode, count_filter, rx, tx }
  }

  pub const fn with_mode(mut self, mode: AlpSensingMode) -> Self {
    self.sensing_mode = mode;
    self
  }

  pub const fn with_count_filter(mut self, enabled: bool) -> Self {
    self.count_filter = enabled;
    self
  }
}

impl Default for Alp {
  fn default() -> Self {
    Self::new(AlpSensingMode::ProjectedCapacitance, true, 0, 0)
  }
}

#[derive(Debug, Clone, Copy)]
pub enum AlpSensingMode {
  SelfCapacitance = 0,
  ProjectedCapacitance = 1,
}

impl From<AlpSensingMode> for u8 {
  fn from(v: AlpSensingMode) -> Self {
    v as u8
  }
}

impl TryFrom<u8> for AlpSensingMode {
  type Error = ();

  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b1 {
      0 => Ok(Self::SelfCapacitance),
      1 => Ok(Self::ProjectedCapacitance),
      _ => Err(()),
    }
  }
}
