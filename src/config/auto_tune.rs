#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 18)]
pub struct AutoTune {
  #[bits(32)]
  pub alp_compensation: AlpCompensation,
  #[bits(48)]
  pub tune: Tune,
  pub retune_threshold: u16,
  #[bits(48)]
  pub alp_tune: Tune,
}

impl AutoTune {
  pub const fn new(alp_compensation: AlpCompensation, tune: Tune, retune_threshold: u16, alp_tune: Tune) -> Self {
    Self { alp_compensation, tune, retune_threshold, alp_tune }
  }

  pub const fn with_tuning(mut self, tune: Tune, retune_threshold: u16) -> Self {
    self.tune = tune;
    self.retune_threshold = retune_threshold;
    self
  }

  pub const fn with_alp_tuning(mut self, tune: Tune, compensation: AlpCompensation) -> Self {
    self.alp_tune = tune;
    self.alp_compensation = compensation;
    self
  }
}

impl Default for AutoTune {
  fn default() -> Self {
    Self::new(AlpCompensation::default(), Tune::new(1, 15, 24, 9, 50, 300), 50, Tune::new(1, 15, 24, 4, 20, 200))
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u32)]
pub struct AlpCompensation {
  pub engine_a: u16,
  pub engine_b: u16,
}

impl AlpCompensation {
  pub const fn new(engine_a: u16, engine_b: u16) -> Self {
    Self { engine_a, engine_b }
  }

  const fn default() -> Self {
    Self::new(441, 466)
  }
}

impl Default for AlpCompensation {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u64)]
pub struct Tune {
  #[bits(5)]
  pub coarse_divider: u8,
  #[bits(4)]
  pub coarse_multiplier: u8,
  #[bits(5)]
  pub fine_divider: u8,
  #[skip(2)]
  pub compensation_divider: u8,
  pub drift_limit: u8,
  pub target: u16,
}

impl Tune {
  pub const fn new(
    coarse_divider: u8,
    coarse_multiplier: u8,
    fine_divider: u8,
    compensation_divider: u8,
    drift_limit: u8,
    target: u16,
  ) -> Self {
    Self { coarse_divider, coarse_multiplier, fine_divider, compensation_divider, drift_limit, target }
  }

  pub const fn with_target(mut self, target: u16) -> Self {
    self.target = target;
    self
  }

  pub const fn with_drift_limit(mut self, drift_limit: u8) -> Self {
    self.drift_limit = drift_limit;
    self
  }

  pub const fn with_dividier_multiplier(mut self, divider: u8, multiplier: u8, fine_divider: u8) -> Self {
    self.coarse_divider = divider;
    self.coarse_multiplier = multiplier;
    self.fine_divider = fine_divider;
    self
  }

  pub const fn with_compensation_divider(mut self, divider: u8) -> Self {
    self.compensation_divider = divider;
    self
  }
}

impl Default for Tune {
  fn default() -> Self {
    Self::new(1, 15, 24, 9, 50, 300)
  }
}
