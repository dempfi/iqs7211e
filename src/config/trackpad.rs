#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 18)]
pub struct Trackpad {
  #[bits(2)]
  pub axes: Axes,
  #[bits(3)]
  pub filters: Filters,
  #[skip(3)]
  pub(crate) total_rx: u8,
  pub(crate) total_tx: u8,
  #[bits(2)]
  pub max_simultaneous_touches: MaxTouches,
  #[skip(6)]
  #[bits(32)]
  pub resolution: Resolution,
  #[bits(40)]
  pub dynamic_filter: DynamicFilterConfig,
  pub static_filter_beta: u8,
  pub stationary_touch_threshold: u8,
  pub multitouch_finger_split_factor: u8,
  #[bits(16)]
  pub axes_inset: AxesInset,
}

impl Trackpad {
  pub const fn new() -> Self {
    Self {
      axes: Axes::default(),
      filters: Filters::dynamic(),
      total_rx: 0,
      total_tx: 0,
      max_simultaneous_touches: MaxTouches::default(),
      resolution: Resolution::default(),
      dynamic_filter: DynamicFilterConfig::default(),
      static_filter_beta: 128,
      stationary_touch_threshold: 20,
      multitouch_finger_split_factor: 3,
      axes_inset: AxesInset::default(),
    }
  }

  pub const fn with_axes(mut self, axes: Axes, resolution: Resolution, inset: AxesInset) -> Self {
    self.axes = axes;
    self.resolution = resolution;
    self.axes_inset = inset;
    self
  }

  pub const fn without_filters(mut self) -> Self {
    self.filters = Filters::disabled();
    self
  }

  pub const fn with_dynamic_filter(mut self, config: DynamicFilterConfig) -> Self {
    self.filters = Filters::dynamic();
    self.dynamic_filter = config;
    self
  }

  pub const fn with_static_filter(mut self, mav: bool, beta: u8) -> Self {
    self.filters = Filters::fixed();
    self.filters.moving_average = mav;
    self.static_filter_beta = beta;
    self
  }

  pub const fn single_touch(mut self) -> Self {
    self.max_simultaneous_touches = MaxTouches::One;
    self
  }

  pub const fn multi_touch(mut self, finger_split_factor: u8) -> Self {
    self.max_simultaneous_touches = MaxTouches::Two;
    self.multitouch_finger_split_factor = finger_split_factor;
    self
  }
}

impl Default for Trackpad {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u8)]
pub struct Axes {
  pub flip_x: bool,
  pub flip_y: bool,
  pub swap_axes: bool,
}

impl Axes {
  pub const fn new(flip_x: bool, flip_y: bool, swap_axes: bool) -> Self {
    Self { flip_x, flip_y, swap_axes }
  }

  const fn default() -> Self {
    Self::new(false, false, false)
  }
}

impl Default for Axes {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u8)]
pub struct Filters {
  #[bits(2)]
  pub irr: IrrFilter,
  pub moving_average: bool,
}

impl Filters {
  pub const fn new(irr: IrrFilter, moving_average: bool) -> Self {
    Self { irr, moving_average }
  }

  pub const fn dynamic() -> Self {
    Self::new(IrrFilter::Dynamic, true)
  }

  pub const fn fixed() -> Self {
    Self::new(IrrFilter::Fixed, true)
  }

  pub const fn disabled() -> Self {
    Self::new(IrrFilter::Disable, false)
  }
}

impl Default for Filters {
  fn default() -> Self {
    Self::dynamic()
  }
}

#[derive(Debug, Clone, Copy)]
pub enum IrrFilter {
  Disable = 0b00,
  Dynamic = 0b01,
  Fixed = 0b10,
}

impl From<IrrFilter> for u8 {
  fn from(filter: IrrFilter) -> Self {
    filter as u8
  }
}

impl TryFrom<u8> for IrrFilter {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b00 => Ok(Self::Disable),
      0b01 => Ok(Self::Dynamic),
      0b10 => Ok(Self::Fixed),
      _ => Err(()),
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum MaxTouches {
  One = 0b01,
  Two = 0b10,
}

impl From<MaxTouches> for u8 {
  fn from(touches: MaxTouches) -> Self {
    touches as u8
  }
}

impl TryFrom<u8> for MaxTouches {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b01 => Ok(Self::One),
      0b10 => Ok(Self::Two),
      _ => Err(()),
    }
  }
}

impl MaxTouches {
  const fn default() -> Self {
    Self::Two
  }
}

impl Default for MaxTouches {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u32)]
pub struct Resolution {
  pub x: u16,
  pub y: u16,
}

impl Resolution {
  pub const fn new(x: u16, y: u16) -> Self {
    Self { x, y }
  }

  const fn default() -> Self {
    Self::new(1000, 1000)
  }
}

impl Default for Resolution {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u64)]
pub struct DynamicFilterConfig {
  pub bottom_speed: u16,
  pub top_speed: u16,
  pub bottom_beta: u8,
}

impl DynamicFilterConfig {
  pub const fn new(bottom_speed: u16, top_speed: u16, bottom_beta: u8) -> Self {
    Self { bottom_speed, top_speed, bottom_beta }
  }

  const fn default() -> Self {
    Self::new(6, 124, 7)
  }
}

impl Default for DynamicFilterConfig {
  fn default() -> Self {
    Self::default()
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct AxesInset {
  pub x: u8,
  pub y: u8,
}

impl AxesInset {
  pub const fn new(x: u8, y: u8) -> Self {
    Self { x, y }
  }

  pub const fn uniform(axis: u8) -> Self {
    Self::new(axis, axis)
  }

  const fn default() -> Self {
    Self::new(20, 20)
  }
}

impl Default for AxesInset {
  fn default() -> Self {
    Self::default()
  }
}
