use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{ConfigSettings, Error, InterruptMode, Iqs7211e, Reg, SysControl};

mod alp;
mod auto_tune;
mod conversion;
mod gesture;
mod hardware;
mod output;
mod pinout;
mod timing;
mod trackpad;

pub use alp::*;
pub use auto_tune::*;
pub use conversion::*;
pub use gesture::*;
pub use hardware::*;
pub use output::*;
pub use pinout::*;
pub use timing::*;
pub use trackpad::*;

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub(crate) async fn write_config(&mut self, config: &Config) -> Result<(), Error<E>> {
    self.write(Reg::AlpAutoTuningCompA, config.auto_tune).await?;
    self.write(Reg::ActiveModeReportRate, config.timing).await?;
    self.write(Reg::SysControl, SystemSettings::default()).await?;
    self.write(Reg::AlpSetup, config.alp).await?;
    self.write(Reg::TouchSetClearMultipliers, config.channel_output).await?;
    self.write(Reg::TpConvFreq, config.conversion_frequency).await?;
    self.write(Reg::TpHardware, config.hardware).await?;
    self.write(Reg::TpRxSettings, config.trackpad).await?;
    self.write_bytes(Reg::SettingsVersion, &[0, 0]).await?;
    self.write(Reg::GestureEnable, config.gestures).await?;
    self.write_bytes(Reg::RxTxMapping0_1, &config.pinout.mapping()).await?;

    let cycles = config.pinout.cycles();
    self.write_bytes(Reg::ProxACycle0, &cycles[..30]).await?;
    self.write_bytes(Reg::ProxACycle10, &cycles[30..60]).await?;
    self.write_bytes(Reg::ProxACycle20, &cycles[60..63]).await?;

    Ok(())
  }
}

/// Complete touchpad configuration ready for device initialization.
///
/// This staged configuration mirrors the register windows documented in the
/// IQS7211E datasheet. Construct it using the fluent helpers to keep values
/// consistent with the on-wire representation.
///
/// # Example
/// ```no_run
/// use iqs7211e::{Config, Pinout, Pin, Resolution, Axes, Gestures, Tap, Swipe, Trackpad, AxesInset};
///
/// let layout = Pinout::new(
///   [Pin::RxTx0, Pin::RxTx2, Pin::RxTx4],
///   [Pin::Tx8, Pin::Tx9],
///   [Pin::RxTx0],
///   [Pin::Tx8],
/// );
/// let trackpad = Trackpad::new()
///   .with_axes(Axes::default(), Resolution::new(1000, 1000), AxesInset::default())
///   .multi_touch(3);
/// let gestures = Gestures::default().enable_tap(Tap::all()).enable_swipe(Swipe::horizontal());
/// let config = Config::default()
///   .with_pinout(layout)
///   .with_trackpad(trackpad)
///   .with_gestures(gestures);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Config {
  pub auto_tune: AutoTune,
  pub timing: Timing,
  pub alp: Alp,
  pub channel_output: ChannelOutput,
  pub conversion_frequency: ConversionFrequency,
  pub hardware: Hardware,
  pub trackpad: Trackpad,
  pub gestures: Gestures,
  pub pinout: Pinout,
  pub interrupt_mode: InterruptMode,
}

impl Config {
  /// Create a configuration seeded with the datasheet defaults.
  pub const fn new(
    auto_tune: AutoTune,
    timing: Timing,
    alp: Alp,
    channel_output: ChannelOutput,
    conversion_frequency: ConversionFrequency,
    hardware: Hardware,
    trackpad: Trackpad,
    gestures: Gestures,
    pinout: Pinout,
    interrupt_mode: InterruptMode,
  ) -> Self {
    Self {
      auto_tune,
      timing,
      alp,
      channel_output,
      conversion_frequency,
      hardware,
      trackpad,
      gestures,
      pinout,
      interrupt_mode,
    }
  }

  pub const fn with_auto_tune(mut self, auto_tune: AutoTune) -> Self {
    self.auto_tune = auto_tune;
    self
  }

  pub const fn with_timing(mut self, timing: Timing) -> Self {
    self.timing = timing;
    self
  }

  pub const fn with_alp(mut self, alp: Alp) -> Self {
    let rx = alp.rx;
    let tx = alp.tx;
    self.alp = alp;
    self.alp.rx = rx;
    self.alp.tx = tx;
    self
  }

  pub const fn with_channel_output(mut self, channel_output: ChannelOutput) -> Self {
    self.channel_output = channel_output;
    self
  }

  pub const fn with_conversion_frequency(mut self, conversion_frequency: ConversionFrequency) -> Self {
    self.conversion_frequency = conversion_frequency;
    self
  }

  pub const fn with_hardware(mut self, hardware: Hardware) -> Self {
    self.hardware = hardware;
    self
  }

  pub const fn with_trackpad(mut self, config: Trackpad) -> Self {
    self.trackpad = config;
    self.trackpad.total_rx = self.pinout.rx.len as u8;
    self.trackpad.total_tx = self.pinout.tx.len as u8;
    self
  }

  pub const fn with_gestures(mut self, gestures: Gestures) -> Self {
    self.gestures = gestures;
    self
  }

  pub const fn with_pinout(mut self, pinout: Pinout) -> Self {
    self.pinout = pinout;
    self.trackpad.total_rx = self.pinout.rx.len as u8;
    self.trackpad.total_tx = self.pinout.tx.len as u8;
    self.alp.rx = self.pinout.alp_rx();
    self.alp.tx = self.pinout.alp_tx();
    self
  }

  pub const fn with_interrupt_mode(mut self, interrupt_mode: InterruptMode) -> Self {
    self.interrupt_mode = interrupt_mode;
    self
  }
}

impl Default for Config {
  fn default() -> Self {
    Self::new(
      AutoTune::default(),
      Timing::default(),
      Alp::default(),
      ChannelOutput::default(),
      ConversionFrequency::default(),
      Hardware::default(),
      Trackpad::new(),
      Gestures::default(),
      Pinout::default(),
      InterruptMode::Event,
    )
  }
}

#[derive(Debug, Clone, Copy, Default)]
#[packbits::pack(bytes = 6)]
struct SystemSettings {
  #[bits(16)]
  sys_control: SysControl,
  #[bits(16)]
  config_settings: ConfigSettings,
  other_settings: u16,
}
