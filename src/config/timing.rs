#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 22)]
pub struct Timing {
  #[bits(80)]
  pub report_rate: ReportRate,
  #[bits(64)]
  pub timeouts: Timeouts,
  /// After the auto tune algorithm is performed, a check is done to see if there was any error with the algorithm.
  /// In case of an error, retune will be attempted again after delay (in seconds) has elapsed. Max value is 60 seconds.
  #[bits(6)]
  pub retune_retry_delay: u8,
  /// Interval in seconds for sampling the reference for the long term average (LTA) calculation when power mode
  /// switching is managed automatically. Max value is 60 seconds.
  #[skip(2)]
  #[bits(6)]
  pub long_term_average_reference_sampling_interval: u8,
  /// Time in which the communication window should be serviced by master. If breached, the system will move on,
  /// however the corresponding data will be lost.
  #[skip(2)]
  pub i2c_timeout: u16,
}

impl Timing {
  pub const fn new() -> Self {
    Self {
      report_rate: ReportRate::default(),
      timeouts: Timeouts::default(),
      retune_retry_delay: 5,
      long_term_average_reference_sampling_interval: 8,
      i2c_timeout: 100,
    }
  }
}

impl Default for Timing {
  fn default() -> Self {
    Self::new()
  }
}

/// The report rate for each mode is configured by selecting the cycle time (in milliseconds)
///
/// A faster report rate will have a higher current consumption but will give faster response to user interaction.
/// Active mode typically has the fastest report rate, and the other modes are configured according to the power budget
/// of the design, and the expected response time.
#[derive(Debug, Clone, Copy)]
#[packbits::pack(u128)]
pub struct ReportRate {
  pub active: u16,
  pub idle_touch: u16,
  pub idle: u16,
  pub lp1: u16,
  pub lp2: u16,
}

impl ReportRate {
  pub const fn new(active: u16, idle_touch: u16, idle: u16, lp1: u16, lp2: u16) -> Self {
    Self { active, idle_touch, idle, lp1, lp2 }
  }

  const fn default() -> Self {
    Self::new(10, 50, 20, 80, 160)
  }
}

impl Default for ReportRate {
  fn default() -> Self {
    Self::default()
  }
}

/// Timeouts to switch to the next power state
///
/// Once these times have elapsed, the system will change to the next state. These times are adjusted by selecting a
/// desired value (in seconds), for the specific timeout.
///
/// Note: A timeout value of 0 will result in a 'never' timeout condition.
#[derive(Debug, Clone, Copy)]
#[packbits::pack(u64)]
pub struct Timeouts {
  pub active: u16,
  pub idle_touch: u16,
  pub idle: u16,
  pub lp1: u16,
}

impl Timeouts {
  pub const fn new(active: u16, idle_touch: u16, idle: u16, lp1: u16) -> Self {
    Self { active, idle_touch, idle, lp1 }
  }

  const fn default() -> Self {
    Self::new(10, 60, 10, 10)
  }
}

impl Default for Timeouts {
  fn default() -> Self {
    Self::default()
  }
}
