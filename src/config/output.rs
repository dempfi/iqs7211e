#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 10)]
pub struct ChannelOutput {
  #[bits(16)]
  pub touch: TouchOutput,
  #[bits(32)]
  pub alp: AlpOutput,
  #[bits(16)]
  pub alp_filter_lp1: AlpFilterBetas,
  #[bits(16)]
  pub alp_filter_lp2: AlpFilterBetas,
}

impl ChannelOutput {
  pub const fn new(
    touch: TouchOutput,
    alp: AlpOutput,
    alp_filter_lp1: AlpFilterBetas,
    alp_filter_lp2: AlpFilterBetas,
  ) -> Self {
    Self { touch, alp, alp_filter_lp1, alp_filter_lp2 }
  }

  pub const fn with_touch(mut self, touch: TouchOutput) -> Self {
    self.touch = touch;
    self
  }

  pub const fn with_alp(mut self, low_power: AlpOutput) -> Self {
    self.alp = low_power;
    self
  }

  pub const fn with_alp_filter_lp1(mut self, filter: AlpFilterBetas) -> Self {
    self.alp_filter_lp1 = filter;
    self
  }

  pub const fn with_alp_filter_lp2(mut self, filter: AlpFilterBetas) -> Self {
    self.alp_filter_lp2 = filter;
    self
  }
}

impl Default for ChannelOutput {
  fn default() -> Self {
    Self::new(TouchOutput::default(), AlpOutput::default(), AlpFilterBetas::new(220, 8), AlpFilterBetas::new(240, 16))
  }
}

/// The trackpad touch output is set when a channel's count value increases by more than the selected threshold.
/// The touch threshold for a specific channel is calculated as `Threshold = Reference * (1 + Multiplier / 128)`
/// where Multiplier is 'set' and 'clear' threshold, allowing a hysteresis to provide improved touch detection. A
/// smaller fraction will thus be a more sensitive threshold.
#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct TouchOutput {
  pub set_multiplier: u8,
  pub clear_multiplier: u8,
}

impl TouchOutput {
  pub const fn new(set_multiplier: u8, clear_multiplier: u8) -> Self {
    Self { set_multiplier, clear_multiplier }
  }

  const fn default() -> Self {
    Self::new(2, 2)
  }
}

impl Default for TouchOutput {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u32)]
pub struct AlpOutput {
  pub threshold: u16,
  pub set_debounce: u8,
  pub clear_debounce: u8,
}

impl AlpOutput {
  pub const fn new(threshold: u16, set_debounce: u8, clear_debounce: u8) -> Self {
    Self { threshold, set_debounce, clear_debounce }
  }

  const fn default() -> Self {
    Self::new(8, 4, 4)
  }
}

impl Default for AlpOutput {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct AlpFilterBetas {
  count: u8,
  long_term_average: u8,
}

impl AlpFilterBetas {
  pub const fn new(count: u8, long_term_average: u8) -> Self {
    Self { count, long_term_average }
  }
}
