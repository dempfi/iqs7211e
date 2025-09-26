#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 22)]
pub struct Gestures {
  #[bits(16)]
  pub enable: GestureEnable,
  #[bits(48)]
  pub tap: TapConfig,
  /// The minimum time in milliseconds the finger must remain on the sensor. Used by both press-and-hold and swipe-and-hold.
  pub hold_duration: u16,
  #[bits(88)]
  pub swipe: SwipeConfig,
  /// Number of channels to detect touch simultaneously for the palm gesture to be recognized
  pub palm_threshold: u8,
}

impl Gestures {
  pub const fn new(
    enable: GestureEnable,
    tap: TapConfig,
    hold_duration: u16,
    swipe: SwipeConfig,
    palm_threshold: u8,
  ) -> Self {
    Self { enable, tap, hold_duration, swipe, palm_threshold }
  }

  pub const fn enable_tap(mut self, tap: Tap) -> Self {
    self.enable.tap = tap;
    self
  }

  pub const fn enable_press_and_hold(mut self) -> Self {
    self.enable.press_and_hold = true;
    self
  }

  pub const fn enable_palm(mut self) -> Self {
    self.enable.palm = true;
    self
  }

  pub const fn enable_swipe(mut self, swipe: Swipe) -> Self {
    self.enable.swipe = swipe;
    self
  }

  pub const fn enable_swipe_and_hold(mut self, swipe_and_hold: Swipe) -> Self {
    self.enable.swipe_and_hold = swipe_and_hold;
    self
  }

  pub const fn enable_all(mut self) -> Self {
    self.enable = GestureEnable::new(Tap::all(), true, true, Swipe::all(), Swipe::all());
    self
  }

  pub const fn use_tap_config(mut self, config: TapConfig) -> Self {
    self.tap = config;
    self
  }

  pub const fn use_swipe_config(mut self, config: SwipeConfig) -> Self {
    self.swipe = config;
    self
  }

  pub const fn use_hold_duration(mut self, duration: u16) -> Self {
    self.hold_duration = duration;
    self
  }

  pub const fn use_palm_threshold(mut self, threshold: u8) -> Self {
    self.palm_threshold = threshold;
    self
  }
}

impl Default for Gestures {
  fn default() -> Self {
    Self::new(GestureEnable::none(), TapConfig::default(), 300, SwipeConfig::default(), 30)
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct GestureEnable {
  #[bits(3)]
  pub tap: Tap,
  pub press_and_hold: bool,
  pub palm: bool,
  #[skip(3)]
  #[bits(4)]
  pub swipe: Swipe,
  #[bits(4)]
  pub swipe_and_hold: Swipe,
}

impl GestureEnable {
  pub const fn new(tap: Tap, press_and_hold: bool, palm: bool, swipe: Swipe, swipe_and_hold: Swipe) -> Self {
    Self { tap, press_and_hold, palm, swipe, swipe_and_hold }
  }

  const fn none() -> Self {
    Self::new(Tap::none(), false, false, Swipe::none(), Swipe::none())
  }
}

impl Default for GestureEnable {
  fn default() -> Self {
    Self::none()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u8)]
pub struct Tap {
  pub single: bool,
  pub double: bool,
  pub triple: bool,
}

impl Tap {
  pub const fn new(single: bool, double: bool, triple: bool) -> Self {
    Self { single, double, triple }
  }

  pub const fn single() -> Self {
    Self::new(true, false, false)
  }

  pub const fn double() -> Self {
    Self::new(false, true, false)
  }

  pub const fn triple() -> Self {
    Self::new(false, false, true)
  }

  pub const fn single_and_double() -> Self {
    Self::new(true, true, false)
  }

  pub const fn all() -> Self {
    Self::new(true, true, true)
  }

  const fn none() -> Self {
    Self::new(false, false, false)
  }
}

impl Default for Tap {
  fn default() -> Self {
    Self::none()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u8)]
pub struct Swipe {
  pub pos_x: bool,
  pub neg_x: bool,
  pub pos_y: bool,
  pub neg_y: bool,
}

impl Swipe {
  pub const fn new(pos_x: bool, neg_x: bool, pos_y: bool, neg_y: bool) -> Self {
    Self { pos_x, neg_x, pos_y, neg_y }
  }

  pub const fn all() -> Self {
    Self::new(true, true, true, true)
  }

  pub const fn horizontal() -> Self {
    Self::new(true, true, false, false)
  }

  pub const fn vertical() -> Self {
    Self::new(false, false, true, true)
  }

  const fn none() -> Self {
    Self::new(false, false, false, false)
  }
}

impl Default for Swipe {
  fn default() -> Self {
    Self::none()
  }
}

/// Configuration for the tap gesture
///
/// * `duration`: the maximum touch duration in milliseconds that will result in
///   a valid gesture. The period is measured from the moment a touch is registered and
///   the touch should be released before the tap duration has elapsed.
/// * `air_duration`: maximum duration (in milliseconds) that is allowed between taps
///   (thus while the finger is NOT touching the sensors) for double and triple taps to be detected
/// * `distance`: Maximum distance in pixels the finger can move during the tap
#[derive(Debug, Clone, Copy)]
#[packbits::pack(u64)]
pub struct TapConfig {
  pub duration: u16,
  pub air_duration: u16,
  pub distance: u16,
}

impl TapConfig {
  pub const fn new(duration: u16, air_duration: u16, distance: u16) -> Self {
    Self { duration, air_duration, distance }
  }

  const fn default() -> Self {
    Self::new(150, 150, 50)
  }
}

impl Default for TapConfig {
  fn default() -> Self {
    Self::default()
  }
}

/// Configuration for the swipe gestures
///
/// * `duration`: the maximum time in milliseconds that the swipe gesture can take
/// * `distance_x`: the minimum distance in pixels that must be traveled for a swipes in the X direction
/// * `distance_y`: the minimum distance in pixels that must be traveled for a swipes in the Y direction
/// * `consecutive_distance_x`: the minimum distance in pixels that must be traveled in the X direction
///   to trigger additional swipe without lifting the finger
/// * `consecutive_distance_y`: the minimum distance in pixels that must be traveled in the Y direction
///   to trigger additional swipe without lifting the finger
/// * `angle`: the maximum angle in degrees off the main axis (X or Y) that is allowed for a swipe to be recognized.
///   Calculated as `64 * tanθ` where `θ` is the angle in degrees.
#[derive(Debug, Clone, Copy)]
#[packbits::pack(u128)]
pub struct SwipeConfig {
  duration: u16,
  distance_x: u16,
  distance_y: u16,
  consecutive_distance_x: u16,
  consecutive_distance_y: u16,
  angle: u8,
}

impl SwipeConfig {
  pub const fn new(
    duration: u16,
    distance_x: u16,
    distance_y: u16,
    consecutive_distance_x: u16,
    consecutive_distance_y: u16,
    angle: u8,
  ) -> Self {
    Self { duration, distance_x, distance_y, consecutive_distance_x, consecutive_distance_y, angle }
  }

  pub const fn symmetric(duration: u16, distance: u16, consecutive_distance: u16, angle: u8) -> Self {
    Self::new(duration, distance, distance, consecutive_distance, consecutive_distance, angle)
  }

  pub const fn with_duration(mut self, duration: u16) -> Self {
    self.duration = duration;
    self
  }

  /// Set the minimum distance in pixels that must be traveled for a swipes in the X and Y directions
  pub const fn with_distance(mut self, distance_x: u16, distance_y: u16) -> Self {
    self.distance_x = distance_x;
    self.distance_y = distance_y;
    self
  }

  /// Set the maximum angle in degrees off the main axis (X or Y) that is allowed for a swipe to be recognized.
  /// Calculated as `64 * tanθ` where `θ` is the angle in degrees.
  pub const fn with_angle(mut self, angle: u8) -> Self {
    self.angle = angle;
    self
  }

  pub const fn with_consecutive_distance(mut self, distance_x: u16, distance_y: u16) -> Self {
    self.consecutive_distance_x = distance_x;
    self.consecutive_distance_y = distance_y;
    self
  }

  const fn default() -> Self {
    Self::new(150, 200, 200, 100, 100, 23)
  }
}

impl Default for SwipeConfig {
  fn default() -> Self {
    Self::default()
  }
}
