#[derive(Debug, Clone, Copy)]
#[packbits::pack(bytes = 4)]
pub struct ConversionFrequency {
  #[bits(16)]
  pub trackpad: Frequency,
  #[bits(16)]
  pub alp: Frequency,
}

impl ConversionFrequency {
  pub const fn new(trackpad: Frequency, alp: Frequency) -> Self {
    Self { trackpad, alp }
  }
}

impl Default for ConversionFrequency {
  fn default() -> Self {
    Self::new(Frequency::new(2, 26), Frequency::new(2, 26))
  }
}

#[derive(Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct Frequency {
  /// 128 / fraction − 2
  ///
  /// Range 0..=127. If Frequency fraction is fixed at 127, the following values of the conversion period will result
  /// in the corresponding charge transfer frequencies:
  /// - 1 -> 2MHz
  /// - 5 -> 1MHz
  /// - 12 -> 500kHz
  /// - 17 -> 350kHz
  /// - 26 -> 250kHz
  /// - 53 -> 125kHz
  pub period: u8,
  /// 256 ∗ f_convf / clk
  pub fraction: u8,
}

impl Frequency {
  pub const fn new(period: u8, fraction: u8) -> Self {
    Self { period, fraction }
  }
}
