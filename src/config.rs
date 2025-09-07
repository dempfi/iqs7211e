use defmt::info;
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::{I2c, SevenBitAddress};

use crate::{defs::*, Error, Iqs7211e};

const MAX_CYCLES: usize = 21;
const MAX_PINS: usize = 13;
const UNUSED: u8 = 255;
const PROX_A_PINS: [u8; 4] = [0, 1, 2, 3];
const PROX_B_PINS: [u8; 4] = [4, 5, 6, 7];

/// Helper describing a single IQS7211E sensing timeslot.
///
/// The controller scans the matrix by stepping through a list of cycles. Each
/// cycle pairs up to two channels that share the same Tx drive line: one from
/// Prox block A (Rx0-Rx3) and one from Prox block B (Rx4-Rx7). Entries that do
/// not have a partner on the opposite block are marked as unused so that the
/// firmware can skip them when programming the channel allocation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
struct Cycle {
  tx_line: u8,
  prox_a_channel: u8, // Channel index or 255
  prox_b_channel: u8, // Channel index or 255
}

/// Static routing information for the IQS7211E Rx/Tx sensing matrix.
///
/// The device exposes 13 shared pads that can operate as Rx (receive) or Tx (transmit) electrodes.
/// Trackpad designs normally dedicate Rx0-Rx3 to proximity block A and Rx4-Rx7 to
/// block B, with remaining pads used for additional Rxs or spare Txs. The alternate low-power
/// channel must be drawn from the same electrodes as the main trackpad. This helper keeps
/// the regular and alternate low-power maps together and checks that the supplied pins
/// respect those hardware limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct PinMapping {
  rx_pins: &'static [u8],
  tx_pins: &'static [u8],
  /// Pins used for low-power (ALP) receive channels
  low_power_rx_pins: &'static [u8],
  /// Pins used for low-power (ALP) transmit channels
  low_power_tx_pins: &'static [u8],
}

impl PinMapping {
  /// Construct a new mapping across the various pin groups.
  ///
  /// # Panics
  ///
  /// Panics if more than 13 pins are supplied in total, or if the ALP pin sets
  /// are not strict subsets of the Prox Rx/Tx sets.
  pub fn new(
    rx_pins: &'static [u8],
    tx_pins: &'static [u8],
    low_power_rx_pins: &'static [u8],
    low_power_tx_pins: &'static [u8],
  ) -> Self {
    assert!((rx_pins.len() + tx_pins.len()) <= MAX_PINS, "There are 13 Rx/Tx mapping slots available");
    assert!(low_power_rx_pins.iter().all(|&p| rx_pins.contains(&p)), "ALP Rx pins must be a subset of Rx pins");
    assert!(low_power_tx_pins.iter().all(|&p| tx_pins.contains(&p)), "ALP Tx pins must be a subset of Tx pins");
    Self { rx_pins, tx_pins, low_power_rx_pins: low_power_rx_pins, low_power_tx_pins: low_power_tx_pins }
  }

  /// Generate the sensing cycles used by the IQS7211E scan engine.
  ///
  /// Each cycle allows the controller to scan one Prox A channel (Rx0-Rx3 block)
  /// and one Prox B channel (Rx4-Rx7 block) that share the same Tx line. When a
  /// matching partner is unavailable the relevant entry is marked as `UNUSED`
  /// (255) so that it can be ignored while programming the channel allocation
  /// registers. The returned array is padded with unused entries once no further
  /// valid pairings are available, up to the device limit of 21 cycles.
  fn cycles(&self) -> [Cycle; MAX_CYCLES] {
    let mut out = [Cycle { tx_line: 0, prox_a_channel: UNUSED, prox_b_channel: UNUSED }; MAX_CYCLES];
    let mut cycle_index = 0;
    let mut channel_index = 0;

    for &tx in self.tx_pins {
      for &rx in self.rx_pins {
        if cycle_index >= MAX_CYCLES {
          break;
        }

        let is_a = PROX_A_PINS.contains(&rx);
        let is_b = PROX_B_PINS.contains(&rx);

        let mut backfilled = false;
        #[allow(clippy::needless_range_loop)]
        for i in 0..cycle_index {
          if out[i].tx_line == tx {
            if is_a && out[i].prox_a_channel == UNUSED {
              out[i].prox_a_channel = channel_index;
              backfilled = true;
              break;
            } else if is_b && out[i].prox_b_channel == UNUSED {
              out[i].prox_b_channel = channel_index;
              backfilled = true;
              break;
            }
          }
        }

        // if we didn't find a matching cycle to backfill, create a new one
        if !backfilled {
          out[cycle_index] = Cycle {
            tx_line: tx,
            prox_a_channel: if is_a { channel_index } else { UNUSED },
            prox_b_channel: if is_b { channel_index } else { UNUSED },
          };
          cycle_index += 1;
        }

        channel_index += 1;
      }
    }

    out
  }

  fn pins(&self) -> [u8; MAX_PINS] {
    let mut out = [0; MAX_PINS];
    let mut idx = 0;
    for &rx in self.rx_pins {
      out[idx] = rx;
      idx += 1;
    }
    for &tx in self.tx_pins {
      out[idx] = tx;
      idx += 1;
    }
    out
  }

  /// Total number of Rx pins configured in the mapping.
  pub fn rx_pins(&self) -> &[u8] {
    self.rx_pins
  }

  /// Total number of Tx pins configured in the mapping.
  pub fn tx_pins(&self) -> &[u8] {
    self.tx_pins
  }

  /// Low-power Rx subset used by the low-power channel.
  pub fn low_power_rx_pins(&self) -> &[u8] {
    self.low_power_rx_pins
  }

  /// Low-power Tx subset used by the low-power channel.
  pub fn low_power_tx_pins(&self) -> &[u8] {
    self.low_power_tx_pins
  }
}

/// Combined coarse/fine auto-tuning divider settings for the trackpad channels.
///
/// The packed bits match the `TRACKPAD_ATI_MULTIPLIERS_DIVIDERS` register pair. The
/// coarse divider (5 bits) and coarse multiplier (4 bits) choose the starting
/// conversion strength, while the fine divider (5 bits) adds a smaller adjustment
/// so that the auto-tuning algorithm can settle on a stable base signal.
///
/// Note: "ATI" in the IQS7211E datasheet refers to "Auto-Tuning".
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct AutoTuningScale {
  #[bits(5)]
  pub coarse_divider: u8,
  #[bits(4)]
  pub coarse_multiplier: u8,
  #[bits(5)]
  pub fine_divider: u8,
  // reserved 2 bits
}

impl AutoTuningScale {
  pub const fn new(coarse_divider: u8, coarse_multiplier: u8, fine_divider: u8) -> Self {
    Self { coarse_divider, coarse_multiplier, fine_divider }
  }
}

impl Default for AutoTuningScale {
  fn default() -> Self {
    Self::new(1, 15, 30)
  }
}

/// Bitfield toggle of the various gesture events produced by the controller.
///
/// Each flag corresponds to a gesture recogniser inside the firmware. Setting a
/// flag enables reporting for taps, swipes, palm rejection, or swipe-hold
/// sequences. The packed layout matches the `GESTURE_ENABLE` registers, so the
/// bitfield can be written directly to the device.
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct GestureMask {
  pub tap: bool,
  pub double_tap: bool,
  pub triple_tap: bool,
  pub press_hold: bool,
  pub palm: bool,
  #[skip(3)]
  pub swipe_x_pos: bool,
  pub swipe_x_neg: bool,
  pub swipe_y_pos: bool,
  pub swipe_y_neg: bool,
  pub swipe_hold_x_pos: bool,
  pub swipe_hold_x_neg: bool,
  pub swipe_hold_y_pos: bool,
  pub swipe_hold_y_neg: bool,
}

impl Default for GestureMask {
  fn default() -> Self {
    Self {
      tap: true,
      double_tap: true,
      triple_tap: true,
      press_hold: true,
      palm: true,
      swipe_x_pos: true,
      swipe_x_neg: true,
      swipe_y_pos: true,
      swipe_y_neg: true,
      swipe_hold_x_pos: true,
      swipe_hold_x_neg: true,
      swipe_hold_y_pos: true,
      swipe_hold_y_neg: true,
    }
  }
}

/// Bundle of tuning values for the trackpad auto-tuning engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 8)]
pub struct TrackpadAutoTuning {
  #[bits(16)]
  pub scale: AutoTuningScale,
  pub compensation_divider: u8,
  pub reference_drift_limit: u8,
  pub target: u16,
  pub min_re_auto_tuning_count: u16,
}

impl TrackpadAutoTuning {
  pub const fn new(
    scale: AutoTuningScale,
    target: u16,
    compensation_divider: u8,
    reference_drift_limit: u8,
    min_re_auto_tuning_count: u16,
  ) -> Self {
    Self { scale, target, compensation_divider, reference_drift_limit, min_re_auto_tuning_count }
  }
}

impl Default for TrackpadAutoTuning {
  fn default() -> Self {
    Self {
      scale: AutoTuningScale::default(),
      target: 300,
      compensation_divider: 10,
      reference_drift_limit: TRACKPAD_REF_DRIFT_LIMIT,
      min_re_auto_tuning_count: TRACKPAD_MIN_COUNT_REATI,
    }
  }
}

/// Bundle of tuning values for the low-power auto-tuning engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 6)]
pub struct LowPowerAutoTuning {
  #[bits(16)]
  pub scale: AutoTuningScale,
  pub compensation_divider: u8,
  pub lta_drift_limit: u8,
  pub target: u16,
}

impl LowPowerAutoTuning {
  pub const fn new(scale: AutoTuningScale, target: u16, compensation_divider: u8, lta_drift_limit: u8) -> Self {
    Self { scale, target, compensation_divider, lta_drift_limit }
  }
}

impl Default for LowPowerAutoTuning {
  fn default() -> Self {
    Self {
      scale: AutoTuningScale::new(3, 1, 1),
      target: LOW_POWER_ATI_TARGET,
      compensation_divider: LOW_POWER_COMPENSATION_DIV,
      lta_drift_limit: LOW_POWER_LTA_DRIFT_LIMIT,
    }
  }
}

/// Pre-programmed compensation values used for the low-power channel A/B pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(u32)]
pub struct LowPowerCompensation {
  pub channel_a: u16,
  pub channel_b: u16,
}

impl LowPowerCompensation {
  pub const fn new(channel_a: u16, channel_b: u16) -> Self {
    Self { channel_a, channel_b }
  }
}

impl Default for LowPowerCompensation {
  fn default() -> Self {
    Self { channel_a: LOW_POWER_COMPENSATION_A, channel_b: LOW_POWER_COMPENSATION_B }
  }
}

/// Report rates (scan intervals) for each power mode (0x28..0x2C).
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 10)]
pub struct ReportRates {
  pub active_scan_interval: u16,
  pub idle_touch_scan_interval: u16,
  pub idle_scan_interval: u16,
  pub lp1_scan_interval: u16,
  pub lp2_scan_interval: u16,
}

impl ReportRates {
  pub const fn new(
    active_scan_interval: u16,
    idle_touch_scan_interval: u16,
    idle_scan_interval: u16,
    lp1_scan_interval: u16,
    lp2_scan_interval: u16,
  ) -> Self {
    Self { active_scan_interval, idle_touch_scan_interval, idle_scan_interval, lp1_scan_interval, lp2_scan_interval }
  }
}

impl Default for ReportRates {
  fn default() -> Self {
    Self {
      active_scan_interval: ACTIVE_MODE_REPORT_RATE,
      idle_touch_scan_interval: IDLE_TOUCH_MODE_REPORT_RATE,
      idle_scan_interval: IDLE_MODE_REPORT_RATE,
      lp1_scan_interval: LP1_MODE_REPORT_RATE,
      lp2_scan_interval: LP2_MODE_REPORT_RATE,
    }
  }
}

/// Timeouts that dictate mode transitions (0x2D..0x30).
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 8)]
pub struct ModeTimeouts {
  pub active_timeout: u16,
  pub idle_touch_timeout: u16,
  pub idle_timeout: u16,
  pub lp1_timeout: u16,
}

impl ModeTimeouts {
  pub const fn new(active_timeout: u16, idle_touch_timeout: u16, idle_timeout: u16, lp1_timeout: u16) -> Self {
    Self { active_timeout, idle_touch_timeout, idle_timeout, lp1_timeout }
  }
}

impl Default for ModeTimeouts {
  fn default() -> Self {
    Self {
      active_timeout: ACTIVE_MODE_TIMEOUT,
      idle_touch_timeout: IDLE_TOUCH_MODE_TIMEOUT,
      idle_timeout: IDLE_MODE_TIMEOUT,
      lp1_timeout: LP1_MODE_TIMEOUT,
    }
  }
}

/// Background maintenance timers and bus timeout (0x31..0x32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 4)]
pub struct MaintenanceTimers {
  pub reference_update_interval: u8,
  pub retune_retry_interval: u8,
  pub i2c_timeout: u16,
}

impl MaintenanceTimers {
  pub const fn new(reference_update_interval: u8, retune_retry_interval: u8, i2c_timeout: u16) -> Self {
    Self { reference_update_interval, retune_retry_interval, i2c_timeout }
  }
}

impl Default for MaintenanceTimers {
  fn default() -> Self {
    Self {
      reference_update_interval: REF_UPDATE_TIME,
      retune_retry_interval: REATI_RETRY_TIME,
      i2c_timeout: I2C_TIMEOUT,
    }
  }
}

/// Encodes the system control flags located at memory map 0x33-0x35.
///
/// The control bits gate features such as streaming, event generation, and
/// power-management behaviour. Keeping the values in a dedicated block makes it
/// easy to mirror the device defaults before selectively overriding them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 6)]
struct SystemSettings {
  #[bits(16)]
  sys_control: u16,
  #[bits(16)]
  config_settings: u16,
  other_settings: u16,
}

impl Default for SystemSettings {
  fn default() -> Self {
    Self {
      sys_control: SYSTEM_CONTROL_DEFAULT,
      config_settings: CONFIG_SETTINGS_DEFAULT,
      other_settings: OTHER_SETTINGS_DEFAULT,
    }
  }
}

/// Trackpad and ALP threshold settings.
///
/// The touch values are stored as multipliers that the device applies to the
/// reference count. The ALP fields provide the same hysteresis and debounce
/// behaviour for the low-power sensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 6)]
struct ThresholdSettings {
  #[bits(16)]
  touch_threshold: TouchThreshold,
  low_power_threshold: u16,
  low_power_set_debounce: u8,
  low_power_clear_debounce: u8,
}

impl ThresholdSettings {
  fn new(config: &Config) -> Self {
    Self {
      touch_threshold: config.touch_threshold,
      low_power_threshold: LOW_POWER_THRESHOLD,
      low_power_set_debounce: LOW_POWER_SET_DEBOUNCE,
      low_power_clear_debounce: LOW_POWER_CLEAR_DEBOUNCE,
    }
  }
}

/// Low-power filter coefficients for the ALP channel (memory map 0x3B-0x3C).
///
/// The betas define how aggressively the ALP count and long-term average values
/// follow new measurements while the device cycles through LP1 and LP2. Smaller
/// betas slow the response for noise immunity; larger values track touch events
/// faster but allow more jitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 4)]
struct FilterBetas {
  low_power_count_beta_lp1: u8,
  low_power_lta_beta_lp1: u8,
  low_power_count_beta_lp2: u8,
  low_power_lta_beta_lp2: u8,
}

impl Default for FilterBetas {
  fn default() -> Self {
    Self {
      low_power_count_beta_lp1: LOW_POWER_COUNT_BETA_LP1,
      low_power_lta_beta_lp1: LOW_POWER_LTA_BETA_LP1,
      low_power_count_beta_lp2: LOW_POWER_COUNT_BETA_LP2,
      low_power_lta_beta_lp2: LOW_POWER_LTA_BETA_LP2,
    }
  }
}

/// Conversion-frequency and bias options for the trackpad and ALP engines.
///
/// Each pair of fields selects the up-pass length and fractional divider used
/// to derive the charge-transfer frequency. The remaining words configure
/// hardware traits such as maximum count, op-amp bias, charge sharing capacitor,
/// and noise-mitigation switches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 8)]
struct HardwareSettings {
  trackpad_conversion_up_pass_length: u8,
  trackpad_conversion_fraction_value: u8,
  low_power_conversion_up_pass_length: u8,
  low_power_conversion_fraction_value: u8,
  #[bits(16)]
  trackpad: HardwareControl,
  #[bits(16)]
  low_power: LowPowerHardware,
}

impl HardwareSettings {
  fn new(config: &Config) -> Self {
    Self {
      trackpad_conversion_up_pass_length: TRACKPAD_CONVERSION_FREQUENCY_UP_PASS_LENGTH,
      trackpad_conversion_fraction_value: TRACKPAD_CONVERSION_FREQUENCY_FRACTION_VALUE,
      low_power_conversion_up_pass_length: LOW_POWER_CONVERSION_FREQUENCY_UP_PASS_LENGTH,
      low_power_conversion_fraction_value: LOW_POWER_CONVERSION_FREQUENCY_FRACTION_VALUE,
      trackpad: config.hardware.trackpad,
      low_power: config.hardware.low_power,
    }
  }
}

#[packbits::pack(bytes = 18)]
pub struct TrackpadSettings {
  #[bits(2)]
  pub irr_filter: IrrFilter,
  pub enable_mav_filter: bool,
  #[skip(2)]
  pub total_rx: u8,
  pub total_tx: u8,
  #[bits(8)]
  pub max_touches: MaxTouches,
  #[bits(32)]
  pub resolution: Resolution,
}

pub struct AxisSettings {
  pub flip_x: bool,
  pub flip_y: bool,
  pub swap: bool,
}

/// Trackpad resolution in logical units reported by the firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(u32)]
pub struct Resolution {
  pub x: u16,
  pub y: u16,
}

impl Resolution {
  pub const fn new(x: u16, y: u16) -> Self {
    Self { x, y }
  }
}

impl Default for Resolution {
  fn default() -> Self {
    Self { x: X_RESOLUTION, y: Y_RESOLUTION }
  }
}

/// IIR filtering method for the XY data points
///
/// Dynamic — Damping factor for IIR filter is dynamically adjusted relative
/// to XY movement (recommended)
/// Fixed — Damping factor for IIR filter is fixed
pub enum IrrFilter {
  Disable = 0b00,
  Dynamic = 0b01,
  Fixed = 0b11,
}

impl From<IrrFilter> for u8 {
  fn from(v: IrrFilter) -> Self {
    v as u8
  }
}

impl TryFrom<u8> for IrrFilter {
  type Error = ();
  fn try_from(bits: u8) -> Result<Self, Self::Error> {
    match bits & 0b11 {
      0b00 => Ok(Self::Disable),
      0b01 => Ok(Self::Dynamic),
      0b11 => Ok(Self::Fixed),
      _ => Err(()),
    }
  }
}

/// Gesture timing, distance, and palm-detection configuration.
///
/// Contains the timing windows for single, double, and triple taps, the
/// distance thresholds that distinguish tap clusters, swipe speed limits, and
/// the palm rejection threshold. The structure mirrors the layout of the
/// `GESTURE_ENABLE` through `PALM_THRESHOLD` registers (0x4B-0x55) but names the
/// fields after their behavioural meaning instead of the datasheet labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(bytes = 22)]
pub struct GestureParameters {
  #[bits(16)]
  pub enabled_gestures: GestureMask,
  pub tap_touch_time: u16,
  pub tap_wait_time: u16,
  pub tap_distance: u16,
  pub hold_time: u16,
  pub swipe_time: u16,
  pub swipe_x_distance: u16,
  pub swipe_y_distance: u16,
  pub swipe_x_cons_dist: u16,
  pub swipe_y_cons_dist: u16,
  pub swipe_angle: u8,
  pub palm_threshold: u8,
}

impl GestureParameters {
  #[allow(clippy::too_many_arguments)]
  pub const fn new(
    enabled_gestures: GestureMask,
    tap_touch_time: u16,
    tap_wait_time: u16,
    tap_distance: u16,
    hold_time: u16,
    swipe_time: u16,
    swipe_x_distance: u16,
    swipe_y_distance: u16,
    swipe_x_cons_dist: u16,
    swipe_y_cons_dist: u16,
    swipe_angle: u8,
    palm_threshold: u8,
  ) -> Self {
    Self {
      enabled_gestures,
      tap_touch_time,
      tap_wait_time,
      tap_distance,
      hold_time,
      swipe_time,
      swipe_x_distance,
      swipe_y_distance,
      swipe_x_cons_dist,
      swipe_y_cons_dist,
      swipe_angle,
      palm_threshold,
    }
  }
}

impl Default for GestureParameters {
  fn default() -> Self {
    Self {
      enabled_gestures: GestureMask::default(),
      tap_touch_time: TAP_TOUCH_TIME,
      tap_wait_time: TAP_WAIT_TIME,
      tap_distance: TAP_DISTANCE,
      hold_time: HOLD_TIME,
      swipe_time: SWIPE_TIME,
      swipe_x_distance: SWIPE_X_DISTANCE,
      swipe_y_distance: SWIPE_Y_DISTANCE,
      swipe_x_cons_dist: SWIPE_X_CONS_DIST,
      swipe_y_cons_dist: SWIPE_Y_CONS_DIST,
      swipe_angle: SWIPE_ANGLE,
      palm_threshold: PALM_THRESHOLD,
    }
  }
}

/// Boolean map for selecting which Rx electrodes participate in the ALP channel.
///
/// A `true` flag enables the corresponding Rx (Rx0-Rx7) for the alternate
/// low-power scan. The remaining bits control whether the ALP engine uses
/// self-capacitance and whether the count filter should be applied.
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
struct LowPowerSetup {
  rx0: bool,
  rx1: bool,
  rx2: bool,
  rx3: bool,
  rx4: bool,
  rx5: bool,
  rx6: bool,
  rx7: bool,
  cap_self_proj: bool,
  count_filter: bool,
}

/// Low-power Tx enable mask complementing [`LowPowerSetup`].
///
/// Setting a flag keeps the associated Tx electrode active while the ALP channel
/// scans for wake gestures, ensuring the reduced electrode set still covers the
/// full pad.
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
struct LowPowerTxEnable {
  tx0: bool,
  tx1: bool,
  tx2: bool,
  tx3: bool,
  tx4: bool,
  tx5: bool,
  tx6: bool,
  tx7: bool,
  tx8: bool,
  tx9: bool,
  tx10: bool,
  tx11: bool,
  tx12: bool,
}

/// Hardware timing and bias controls for the ALP sensing engine.
///
/// Provides the same knobs as the main trackpad hardware settings: conversion
/// delay before sampling, the number of auto-prox cycles allowed in LP1/LP2, the
/// maximum count window, bias current, charge-share capacitor size, RF filter,
/// and discharge behaviour. Tuning these values balances response time against
/// power draw while the pad sleeps.
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct LowPowerHardware {
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

impl Default for LowPowerHardware {
  fn default() -> Self {
    Self {
      init_delay: InitDelay::Cycles64,
      lp1_auto_prox_cycles: AutoProxCycles::Cycles8,
      lp2_auto_prox_cycles: AutoProxCycles::Cycles32,
      max_count: MaxCount::Count1023,
      opamp_bias: OpampBias::Microamp10,
      cs_cap: CSCap::Picofarad80,
      rf_filter: false,
      cs_discharge: CSDischarge::To0v,
      nm_in_static: true,
    }
  }
}

/// Hardware timing and bias controls for the primary trackpad sensing engine.
///
/// Configuring the startup
/// delay, maximum count, bias current, charge-share capacitor size, RF filter,
/// and discharge path for the main trackpad conversions.
#[derive(PartialEq, Eq, defmt::Format, Debug, Clone, Copy)]
#[packbits::pack(u16)]
pub struct HardwareControl {
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

impl Default for HardwareControl {
  fn default() -> Self {
    Self {
      init_delay: InitDelay::Cycles64,
      max_count: MaxCount::Count1023,
      opamp_bias: OpampBias::Microamp10,
      cs_cap: CSCap::Picofarad80,
      rf_filter: false,
      cs_discharge: CSDischarge::To0v,
      nm_in_static: true,
    }
  }
}

/// Combined view of the hardware settings used for the trackpad and ALP engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct SensorHardware {
  pub trackpad: HardwareControl,
  pub low_power: LowPowerHardware,
}

impl SensorHardware {
  pub const fn new(trackpad: HardwareControl, low_power: LowPowerHardware) -> Self {
    Self { trackpad, low_power }
  }
}

impl Default for SensorHardware {
  fn default() -> Self {
    Self { trackpad: HardwareControl::default(), low_power: LowPowerHardware::default() }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
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

/// Maximum simultaneous contacts the firmware will report.
///
/// The IQS7211E can track up to two fingers. Limiting the reported slots to one
/// may be desirable for single-touch interfaces that still benefit from the
/// broader trackpad processing pipeline. The encoded value is written directly
/// to the `Max multi-touches` register (memory map address `0x42`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum MaxTouches {
  One = 0x01,
  Two = 0x02,
}

impl Default for MaxTouches {
  fn default() -> Self {
    Self::One
  }
}

impl From<MaxTouches> for u8 {
  fn from(value: MaxTouches) -> Self {
    value as u8
  }
}

impl TryFrom<u8> for MaxTouches {
  type Error = ();
  fn try_from(value: u8) -> Result<Self, Self::Error> {
    match value {
      0x01 => Ok(Self::One),
      0x02 => Ok(Self::Two),
      _ => Ok(Self::One), // default to one if invalid
    }
  }
}

/// Threshold hysteresis configuration for trackpad touch detection.
///
/// The device multiplies the running reference count by these values (expressed
/// in eighth-count units) to decide when a finger press should register or
/// release. Keeping a gap between `set` and `clear` suppresses noise around the
/// trigger point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[packbits::pack(u16)]
pub struct TouchThreshold {
  pub set_multiplier: u8,
  pub clear_multiplier: u8,
}

impl TouchThreshold {
  pub const fn new(set_multiplier: u8, clear_multiplier: u8) -> Self {
    Self { set_multiplier, clear_multiplier }
  }
}

/// Builder for [`Config`] that exposes a fluent API while keeping sensible defaults.
///
/// The defaults mirror the reference settings shipped with the Azoteq evaluation
/// kit, giving a safe starting point. Callers can override individual blocks to
/// match their electrode layout and performance targets before finalising the
/// [`Config`].
#[derive(Debug, Clone, Copy)]
pub struct ConfigBuilder {
  interrupt_mode: InterruptMode,
  sensor_pin_mapping: PinMapping,
  resolution: Resolution,
  trackpad_auto_tuning: TrackpadAutoTuning,
  low_power_auto_tuning: LowPowerAutoTuning,
  low_power_compensation: LowPowerCompensation,
  gesture_parameters: GestureParameters,
  report_rates: ReportRates,
  timeouts: ModeTimeouts,
  maintenance: MaintenanceTimers,
  touch_threshold: TouchThreshold,
  max_touches: MaxTouches,
  hardware: SensorHardware,
}

impl ConfigBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn interrupt_mode(mut self, interrupt_mode: InterruptMode) -> Self {
    self.interrupt_mode = interrupt_mode;
    self
  }

  pub fn sensor_pin_mapping(mut self, sensor_pin_mapping: PinMapping) -> Self {
    self.sensor_pin_mapping = sensor_pin_mapping;
    self
  }

  pub fn trackpad_auto_tuning(mut self, trackpad_auto_tuning: TrackpadAutoTuning) -> Self {
    self.trackpad_auto_tuning = trackpad_auto_tuning;
    self
  }

  pub fn resolution(mut self, resolution: Resolution) -> Self {
    self.resolution = resolution;
    self
  }

  pub fn low_power_auto_tuning(mut self, low_power_auto_tuning: LowPowerAutoTuning) -> Self {
    self.low_power_auto_tuning = low_power_auto_tuning;
    self
  }

  pub fn low_power_compensation(mut self, low_power_compensation: LowPowerCompensation) -> Self {
    self.low_power_compensation = low_power_compensation;
    self
  }

  pub fn gesture_mask(mut self, gesture_mask: GestureMask) -> Self {
    self.gesture_parameters.enabled_gestures = gesture_mask;
    self
  }

  pub fn gesture_parameters(mut self, gesture_parameters: GestureParameters) -> Self {
    self.gesture_parameters = gesture_parameters;
    self
  }

  pub fn report_rates(mut self, report_rates: ReportRates) -> Self {
    self.report_rates = report_rates;
    self
  }

  pub fn timeouts(mut self, timeouts: ModeTimeouts) -> Self {
    self.timeouts = timeouts;
    self
  }

  pub fn maintenance(mut self, maintenance: MaintenanceTimers) -> Self {
    self.maintenance = maintenance;
    self
  }

  pub fn touch_threshold(mut self, touch_threshold: TouchThreshold) -> Self {
    self.touch_threshold = touch_threshold;
    self
  }

  pub fn max_touches(mut self, max_touches: MaxTouches) -> Self {
    self.max_touches = max_touches;
    self
  }

  pub fn hardware(mut self, hardware: SensorHardware) -> Self {
    self.hardware = hardware;
    self
  }

  pub fn build(self) -> Config {
    Config {
      interrupt_mode: self.interrupt_mode,
      pin_mapping: self.sensor_pin_mapping,
      resolution: self.resolution,
      trackpad_auto_tuning: self.trackpad_auto_tuning,
      low_power_auto_tuning: self.low_power_auto_tuning,
      low_power_compensation: self.low_power_compensation,
      gesture_parameters: self.gesture_parameters,
      report_rates: self.report_rates,
      timeouts: self.timeouts,
      maintenance: self.maintenance,
      touch_threshold: self.touch_threshold,
      max_touches: self.max_touches,
      hardware: self.hardware,
    }
  }
}

impl Default for ConfigBuilder {
  fn default() -> Self {
    Self {
      interrupt_mode: InterruptMode::Event,
      sensor_pin_mapping: PinMapping::new(&[], &[], &[], &[]),
      resolution: Resolution::default(),
      trackpad_auto_tuning: TrackpadAutoTuning::default(),
      low_power_auto_tuning: LowPowerAutoTuning::default(),
      low_power_compensation: LowPowerCompensation::default(),
      gesture_parameters: GestureParameters::default(),
      report_rates: ReportRates::default(),
      timeouts: ModeTimeouts::default(),
      maintenance: MaintenanceTimers::default(),
      touch_threshold: TouchThreshold::new(50, 20),
      max_touches: MaxTouches::default(),
      hardware: SensorHardware::default(),
    }
  }
}

impl From<ConfigBuilder> for Config {
  fn from(builder: ConfigBuilder) -> Self {
    builder.build()
  }
}

/// High-level configuration consumed by [`Iqs7211e::initialize`](crate::Iqs7211e::initialize).
///
/// Each field maps directly onto a memory block in the IQS7211E configuration
/// map, allowing the driver to mirror the device registers in a single write
/// sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Config {
  pub interrupt_mode: InterruptMode,
  pub pin_mapping: PinMapping,
  pub resolution: Resolution,
  pub trackpad_auto_tuning: TrackpadAutoTuning,
  pub low_power_auto_tuning: LowPowerAutoTuning,
  pub low_power_compensation: LowPowerCompensation,
  pub gesture_parameters: GestureParameters,
  pub report_rates: ReportRates,
  pub timeouts: ModeTimeouts,
  pub maintenance: MaintenanceTimers,
  pub touch_threshold: TouchThreshold,
  pub max_touches: MaxTouches,
  pub hardware: SensorHardware,
}

impl Config {
  pub fn builder() -> ConfigBuilder {
    ConfigBuilder::default()
  }

  pub fn into_builder(self) -> ConfigBuilder {
    ConfigBuilder {
      interrupt_mode: self.interrupt_mode,
      sensor_pin_mapping: self.pin_mapping,
      resolution: self.resolution,
      trackpad_auto_tuning: self.trackpad_auto_tuning,
      low_power_auto_tuning: self.low_power_auto_tuning,
      low_power_compensation: self.low_power_compensation,
      gesture_parameters: self.gesture_parameters,
      report_rates: self.report_rates,
      timeouts: self.timeouts,
      maintenance: self.maintenance,
      touch_threshold: self.touch_threshold,
      max_touches: self.max_touches,
      hardware: self.hardware,
    }
  }
}

impl Default for Config {
  fn default() -> Self {
    ConfigBuilder::default().build()
  }
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub(crate) async fn write_config(&mut self, config: Config) -> Result<(), Error<E>> {
    self
      .write(Reg::LowPowerAutoTuningCompA, config.low_power_compensation)
      .await?;
    self
      .write(Reg::TpAutoTuningMultipliers, config.trackpad_auto_tuning)
      .await?;
    self
      .write(Reg::LowPowerAutoTuningMultipliers, config.low_power_auto_tuning)
      .await?;
    self.write(Reg::ActiveModeReportRate, config.report_rates).await?;
    self.write(Reg::ActiveModeTimeout, config.timeouts).await?;
    self.write(Reg::RefUpdateReatiTime, config.maintenance).await?;
    self.write(Reg::SysControl, SystemSettings::default()).await?;
    self.write_low_power_settings(&config).await?;

    let block = ThresholdSettings::new(&config);
    self.write(Reg::TouchSetClearMultipliers, block).await?;

    self.write(Reg::Lp1Filters, FilterBetas::default()).await?;
    self.write(Reg::TpConvFreq, HardwareSettings::new(&config)).await?;
    // Write the TP setup block (0x41..0x49) using logically split settings
    // Layout:
    // 0x41: TRACKPAD_SETTINGS0, total_rxs
    // 0x42: total_txs, max_multi_touches
    // 0x43: X_RESOLUTION (LE)
    // 0x44: Y_RESOLUTION (LE)
    // 0x45: XY_DYNAMIC_FILTER_BOTTOM_SPEED (LE)
    // 0x46: XY_DYNAMIC_FILTER_TOP_SPEED (LE)
    // 0x47: XY_DYNAMIC_FILTER_BOTTOM_BETA, XY_DYNAMIC_FILTER_STATIC_FILTER_BETA
    // 0x48: STATIONARY_TOUCH_MOV_THRESHOLD, FINGER_SPLIT_FACTOR
    // 0x49: X_TRIM_VALUE, Y_TRIM_VALUE
    let mut tp_block = [0u8; 18];
    tp_block[0] = TRACKPAD_SETTINGS0;
    tp_block[1] = config.pin_mapping.rx_pins().len() as u8;
    tp_block[2] = config.pin_mapping.tx_pins().len() as u8;
    tp_block[3] = config.max_touches.into();

    self.write(Reg::XResolution, config.resolution).await?;
    // tp_block[4..6].copy_from_slice(&config.resolution.x.to_le_bytes());
    // tp_block[6..8].copy_from_slice(&config.resolution.y.to_le_bytes());

    tp_block[8..10].copy_from_slice(&XY_DYNAMIC_FILTER_BOTTOM_SPEED.to_le_bytes());
    tp_block[10..12].copy_from_slice(&XY_DYNAMIC_FILTER_TOP_SPEED.to_le_bytes());
    tp_block[12] = XY_DYNAMIC_FILTER_BOTTOM_BETA;
    tp_block[13] = XY_DYNAMIC_FILTER_STATIC_FILTER_BETA;
    tp_block[14] = STATIONARY_TOUCH_MOV_THRESHOLD;
    tp_block[15] = FINGER_SPLIT_FACTOR;
    tp_block[16] = X_TRIM_VALUE;
    tp_block[17] = Y_TRIM_VALUE;
    self.write_bytes(Reg::TpRxSettings, &tp_block).await?;

    self
      .write_bytes(Reg::SettingsVersion, &[MINOR_VERSION, MAJOR_VERSION])
      .await?;

    self.write(Reg::GestureEnable, config.gesture_parameters).await?;

    let pins = config.pin_mapping.pins();
    self.write_bytes(Reg::RxTxMapping0_1, pins.as_ref()).await?;

    self.write_cycle_blocks(&config).await?;

    Ok(())
  }

  async fn write_low_power_settings(&mut self, config: &Config) -> Result<(), Error<E>> {
    let low_power_setup = LowPowerSetup {
      rx0: config.pin_mapping.low_power_rx_pins().contains(&0),
      rx1: config.pin_mapping.low_power_rx_pins().contains(&1),
      rx2: config.pin_mapping.low_power_rx_pins().contains(&2),
      rx3: config.pin_mapping.low_power_rx_pins().contains(&3),
      rx4: config.pin_mapping.low_power_rx_pins().contains(&4),
      rx5: config.pin_mapping.low_power_rx_pins().contains(&5),
      rx6: config.pin_mapping.low_power_rx_pins().contains(&6),
      rx7: config.pin_mapping.low_power_rx_pins().contains(&7),
      cap_self_proj: true,
      count_filter: true,
    };

    let low_power_tx_enable = LowPowerTxEnable {
      tx0: config.pin_mapping.low_power_tx_pins().contains(&0),
      tx1: config.pin_mapping.low_power_tx_pins().contains(&1),
      tx2: config.pin_mapping.low_power_tx_pins().contains(&2),
      tx3: config.pin_mapping.low_power_tx_pins().contains(&3),
      tx4: config.pin_mapping.low_power_tx_pins().contains(&4),
      tx5: config.pin_mapping.low_power_tx_pins().contains(&5),
      tx6: config.pin_mapping.low_power_tx_pins().contains(&6),
      tx7: config.pin_mapping.low_power_tx_pins().contains(&7),
      tx8: config.pin_mapping.low_power_tx_pins().contains(&8),
      tx9: config.pin_mapping.low_power_tx_pins().contains(&9),
      tx10: config.pin_mapping.low_power_tx_pins().contains(&10),
      tx11: config.pin_mapping.low_power_tx_pins().contains(&11),
      tx12: config.pin_mapping.low_power_tx_pins().contains(&12),
    };

    let setup_bytes: [u8; 2] = low_power_setup.into();
    let tx_bytes: [u8; 2] = low_power_tx_enable.into();
    let mut payload = [0u8; 4];
    payload[..2].copy_from_slice(&setup_bytes);
    payload[2..].copy_from_slice(&tx_bytes);

    self.write_bytes(Reg::LowPowerSetup, &payload).await?;
    info!("5. Write ALP Settings");
    Ok(())
  }

  async fn write_cycle_blocks(&mut self, config: &Config) -> Result<(), Error<E>> {
    const CYCLE_HEADER: u8 = 0x05;
    const CYCLE_TERMINATOR: u8 = 0x01;

    let cycles = config.pin_mapping.cycles();
    info!("Cycles: {:?}", cycles);

    self
      .write_cycle_block(Reg::ProxACycle0, &cycles, 0, 10, CYCLE_HEADER)
      .await?;
    info!("13. Write Cycle 0 - 9 Settings");

    self
      .write_cycle_block(Reg::ProxACycle10, &cycles, 10, 10, CYCLE_HEADER)
      .await?;
    info!("14. Write Cycle 10 - 19 Settings");

    let tail_payload = [
      CYCLE_HEADER,
      cycles[20].prox_a_channel,
      cycles[20].prox_b_channel,
      CYCLE_TERMINATOR,
    ];
    self.write_bytes(Reg::ProxACycle20, &tail_payload).await?;
    info!("15. Write Cycle 20  Settings");

    Ok(())
  }

  async fn write_cycle_block(
    &mut self,
    reg: Reg,
    cycles: &[Cycle; MAX_CYCLES],
    start: usize,
    count: usize,
    header: u8,
  ) -> Result<(), Error<E>> {
    let mut buf = [0u8; 30];
    for i in 0..count {
      let idx = start + i;
      let base = i * 3;
      buf[base] = header;
      buf[base + 1] = cycles[idx].prox_a_channel;
      buf[base + 2] = cycles[idx].prox_b_channel;
    }

    let used = count * 3;
    self.write_bytes(reg, &buf[..used]).await
  }
}

const LOW_POWER_COMPENSATION_A: u16 = 0x01B9;
const LOW_POWER_COMPENSATION_B: u16 = 0x01D2;

const TRACKPAD_REF_DRIFT_LIMIT: u8 = 0x32;
const TRACKPAD_MIN_COUNT_REATI: u16 = 0x0032;

const LOW_POWER_COMPENSATION_DIV: u8 = 0x04;
const LOW_POWER_LTA_DRIFT_LIMIT: u8 = 0x14;
const LOW_POWER_ATI_TARGET: u16 = 0x00C8;

const ACTIVE_MODE_REPORT_RATE: u16 = 0x000A;
const IDLE_TOUCH_MODE_REPORT_RATE: u16 = 0x0032;
const IDLE_MODE_REPORT_RATE: u16 = 0x0014;
const LP1_MODE_REPORT_RATE: u16 = 0x0050;
const LP2_MODE_REPORT_RATE: u16 = 0x00A0;
const ACTIVE_MODE_TIMEOUT: u16 = 0x000A;
const IDLE_TOUCH_MODE_TIMEOUT: u16 = 0x003C;
const IDLE_MODE_TIMEOUT: u16 = 0x000A;
const LP1_MODE_TIMEOUT: u16 = 0x000A;
const REATI_RETRY_TIME: u8 = 0x05;
const REF_UPDATE_TIME: u8 = 0x08;
const I2C_TIMEOUT: u16 = 0x0064;

const SYSTEM_CONTROL_DEFAULT: u16 = 0x0003;
const CONFIG_SETTINGS_DEFAULT: u16 = 0x062C;
const OTHER_SETTINGS_DEFAULT: u16 = 0x0000;

const LOW_POWER_THRESHOLD: u16 = 0x0008;
const LOW_POWER_SET_DEBOUNCE: u8 = 0x04;
const LOW_POWER_CLEAR_DEBOUNCE: u8 = 0x04;

/* Change the Button and ALP count and LTA betas */
/* Memory Map Position 0x3B - 0x3C */
pub const LOW_POWER_COUNT_BETA_LP1: u8 = 0xDC;
pub const LOW_POWER_LTA_BETA_LP1: u8 = 0x08;
pub const LOW_POWER_COUNT_BETA_LP2: u8 = 0xF0;
pub const LOW_POWER_LTA_BETA_LP2: u8 = 0x10;

/* Change the Hardware Settings */
/* Memory Map Position 0x3D - 0x40 */
const TRACKPAD_CONVERSION_FREQUENCY_UP_PASS_LENGTH: u8 = 0x02;
const TRACKPAD_CONVERSION_FREQUENCY_FRACTION_VALUE: u8 = 0x1A;
const LOW_POWER_CONVERSION_FREQUENCY_UP_PASS_LENGTH: u8 = 0x02;
const LOW_POWER_CONVERSION_FREQUENCY_FRACTION_VALUE: u8 = 0x1A;

/* Change the Trackpad Settings */
/* Memory Map Position 0x41 - 0x49 */
const TRACKPAD_SETTINGS0: u8 = 0x2C;
const X_RESOLUTION: u16 = 0x03E8;
const Y_RESOLUTION: u16 = 0x03E8;
const XY_DYNAMIC_FILTER_BOTTOM_SPEED: u16 = 0x0006;
const XY_DYNAMIC_FILTER_TOP_SPEED: u16 = 0x007C;
const XY_DYNAMIC_FILTER_BOTTOM_BETA: u8 = 0x07;
const XY_DYNAMIC_FILTER_STATIC_FILTER_BETA: u8 = 0x80;
const STATIONARY_TOUCH_MOV_THRESHOLD: u8 = 0x14;
const FINGER_SPLIT_FACTOR: u8 = 0x03;
const X_TRIM_VALUE: u8 = 0x14;
const Y_TRIM_VALUE: u8 = 0x14;

/* Change the Settings Version Numbers */
/* Memory Map Position 0x4A - 0x4A */
const MINOR_VERSION: u8 = 0x00;
const MAJOR_VERSION: u8 = 0x00;

/* Change the Gesture Settings */
/* Memory Map Position 0x4B - 0x55 */
const TAP_TOUCH_TIME: u16 = 0x0096;
const TAP_WAIT_TIME: u16 = 0x0096;
const TAP_DISTANCE: u16 = 0x0032;
const HOLD_TIME: u16 = 0x012C;
const SWIPE_TIME: u16 = 0x0096;
const SWIPE_X_DISTANCE: u16 = 0x00C8;
const SWIPE_Y_DISTANCE: u16 = 0x00C8;
const SWIPE_X_CONS_DIST: u16 = 0x0064;
const SWIPE_Y_CONS_DIST: u16 = 0x0064;
const SWIPE_ANGLE: u8 = 0x17;
const PALM_THRESHOLD: u8 = 0x1E;
