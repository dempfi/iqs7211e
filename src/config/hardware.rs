#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 4)]
pub struct Hardware {
  #[bits(16)]
  pub trackpad: TrackpadHardware,
  #[bits(16)]
  pub alp: AlpHardware,
}

impl Hardware {
  pub const fn new(trackpad: TrackpadHardware, alp: AlpHardware) -> Self {
    Self { trackpad, alp }
  }

  pub const fn with_trackpad(mut self, trackpad: TrackpadHardware) -> Self {
    self.trackpad = trackpad;
    self
  }

  pub const fn with_alp(mut self, alp: AlpHardware) -> Self {
    self.alp = alp;
    self
  }
}

impl Default for Hardware {
  fn default() -> Self {
    Self::new(TrackpadHardware::default(), AlpHardware::default())
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct TrackpadHardware {
  #[bits(2)]
  pub init_delay: InitDelay,
  #[skip(6)]
  #[bits(2)]
  pub max_count: MaxCount,
  #[bits(2)]
  pub opamp_bias: OpampBias,
  #[bits(1)]
  pub cs_cap: CSCap,
  pub rf_filter: bool,
  #[bits(1)]
  pub cs_discharge: CSDischarge,
  pub nm_in_static: bool,
}

impl TrackpadHardware {
  pub const fn new(
    init_delay: InitDelay,
    max_count: MaxCount,
    opamp_bias: OpampBias,
    cs_cap: CSCap,
    rf_filter: bool,
    cs_discharge: CSDischarge,
    nm_in_static: bool,
  ) -> Self {
    Self { init_delay, max_count, opamp_bias, cs_cap, rf_filter, cs_discharge, nm_in_static }
  }

  const fn default() -> Self {
    Self::new(
      InitDelay::Cycles64,
      MaxCount::Count1023,
      OpampBias::Microamp10,
      CSCap::Picofarad80,
      false,
      CSDischarge::To0v,
      true,
    )
  }
}

impl Default for TrackpadHardware {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct AlpHardware {
  #[bits(2)]
  pub init_delay: InitDelay,
  #[bits(3)]
  pub lp1_auto_prox_cycles: AutoProxCycles,
  #[bits(3)]
  pub lp2_auto_prox_cycles: AutoProxCycles,
  #[bits(2)]
  pub max_count: MaxCount,
  #[bits(2)]
  pub opamp_bias: OpampBias,
  #[bits(1)]
  pub cs_cap: CSCap,
  pub rf_filter: bool,
  #[bits(1)]
  pub cs_discharge: CSDischarge,
  pub nm_in_static: bool,
}

impl AlpHardware {
  #[allow(clippy::too_many_arguments)]
  pub const fn new(
    init_delay: InitDelay,
    lp1_auto_prox_cycles: AutoProxCycles,
    lp2_auto_prox_cycles: AutoProxCycles,
    max_count: MaxCount,
    opamp_bias: OpampBias,
    cs_cap: CSCap,
    rf_filter: bool,
    cs_discharge: CSDischarge,
    nm_in_static: bool,
  ) -> Self {
    Self {
      init_delay,
      lp1_auto_prox_cycles,
      lp2_auto_prox_cycles,
      max_count,
      opamp_bias,
      cs_cap,
      rf_filter,
      cs_discharge,
      nm_in_static,
    }
  }

  const fn default() -> Self {
    Self::new(
      InitDelay::Cycles64,
      AutoProxCycles::Cycles8,
      AutoProxCycles::Cycles32,
      MaxCount::Count1023,
      OpampBias::Microamp10,
      CSCap::Picofarad80,
      false,
      CSDischarge::To0v,
      true,
    )
  }

  pub const fn with_init_delay(mut self, value: InitDelay) -> Self {
    self.init_delay = value;
    self
  }

  pub const fn with_lp1_auto_prox_cycles(mut self, value: AutoProxCycles) -> Self {
    self.lp1_auto_prox_cycles = value;
    self
  }

  pub const fn with_lp2_auto_prox_cycles(mut self, value: AutoProxCycles) -> Self {
    self.lp2_auto_prox_cycles = value;
    self
  }

  pub const fn with_max_count(mut self, value: MaxCount) -> Self {
    self.max_count = value;
    self
  }

  pub const fn with_opamp_bias(mut self, value: OpampBias) -> Self {
    self.opamp_bias = value;
    self
  }

  pub const fn with_cs_cap(mut self, value: CSCap) -> Self {
    self.cs_cap = value;
    self
  }

  pub const fn with_rf_filter(mut self, enabled: bool) -> Self {
    self.rf_filter = enabled;
    self
  }

  pub const fn with_cs_discharge(mut self, value: CSDischarge) -> Self {
    self.cs_discharge = value;
    self
  }

  pub const fn with_nm_in_static(mut self, enabled: bool) -> Self {
    self.nm_in_static = enabled;
    self
  }
}

impl Default for AlpHardware {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
pub enum InitDelay {
  Cycles4 = 0b00,
  Cycles16 = 0b01,
  Cycles32 = 0b10,
  Cycles64 = 0b11,
}

impl TryFrom<u8> for InitDelay {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b00 => Ok(Self::Cycles4),
      0b01 => Ok(Self::Cycles16),
      0b10 => Ok(Self::Cycles32),
      0b11 => Ok(Self::Cycles64),
      _ => Err(()),
    }
  }
}

impl From<InitDelay> for u8 {
  fn from(v: InitDelay) -> Self {
    v as u8
  }
}

#[derive(Debug, Clone, Copy)]
pub enum AutoProxCycles {
  Cycles4 = 0b000,
  Cycles8 = 0b001,
  Cycles16 = 0b010,
  Cycles32 = 0b011,
  Disabled = 0b100,
}

impl TryFrom<u8> for AutoProxCycles {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b111 {
      0b000 => Ok(Self::Cycles4),
      0b001 => Ok(Self::Cycles8),
      0b010 => Ok(Self::Cycles16),
      0b011 => Ok(Self::Cycles32),
      0b100 => Ok(Self::Disabled),
      _ => Err(()),
    }
  }
}

impl From<AutoProxCycles> for u8 {
  fn from(v: AutoProxCycles) -> Self {
    v as u8
  }
}

#[derive(Debug, Clone, Copy)]
pub enum MaxCount {
  Count1023 = 0b00,
  Count2047 = 0b01,
  Count4095 = 0b10,
  Count16384 = 0b11,
}

impl TryFrom<u8> for MaxCount {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b00 => Ok(Self::Count1023),
      0b01 => Ok(Self::Count2047),
      0b10 => Ok(Self::Count4095),
      0b11 => Ok(Self::Count16384),
      _ => Err(()),
    }
  }
}

impl From<MaxCount> for u8 {
  fn from(v: MaxCount) -> Self {
    v as u8
  }
}

#[derive(Debug, Clone, Copy)]
pub enum OpampBias {
  Microamp2 = 0b00,
  Microamp5 = 0b01,
  Microamp7 = 0b10,
  Microamp10 = 0b11,
}

impl TryFrom<u8> for OpampBias {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b00 => Ok(Self::Microamp2),
      0b01 => Ok(Self::Microamp5),
      0b10 => Ok(Self::Microamp7),
      0b11 => Ok(Self::Microamp10),
      _ => Err(()),
    }
  }
}

impl From<OpampBias> for u8 {
  fn from(v: OpampBias) -> Self {
    v as u8
  }
}

#[derive(Debug, Clone, Copy)]
pub enum CSCap {
  Picofarad40 = 0b0,
  Picofarad80 = 0b1,
}

impl TryFrom<u8> for CSCap {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b1 {
      0b0 => Ok(Self::Picofarad40),
      0b1 => Ok(Self::Picofarad80),
      _ => Err(()),
    }
  }
}

impl From<CSCap> for u8 {
  fn from(v: CSCap) -> Self {
    v as u8
  }
}

#[derive(Debug, Clone, Copy)]
pub enum CSDischarge {
  To0v = 0b0,
  To0_5v = 0b1,
}

impl TryFrom<u8> for CSDischarge {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b1 {
      0b0 => Ok(Self::To0v),
      0b1 => Ok(Self::To0_5v),
      _ => Err(()),
    }
  }
}

impl From<CSDischarge> for u8 {
  fn from(v: CSDischarge) -> Self {
    v as u8
  }
}
