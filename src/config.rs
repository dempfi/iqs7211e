use bitfield_struct::bitfield;
use defmt::info;
use embedded_hal::i2c::{I2c, SevenBitAddress};
use embedded_hal_async::digital::Wait;

use super::{Error, Iqs7211e, defs};

static MAX_CYCLES: usize = 21;
static MAX_PINS: usize = 13;
static UNUSED: u8 = 255;
static PROX_A_PINS: [u8; 4] = [0, 1, 2, 3];
static PROX_B_PINS: [u8; 4] = [4, 5, 6, 7];

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
struct Cycle {
  tx: u8,
  prox_a: u8, // Channel index or 255
  prox_b: u8, // Channel index or 255
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct RxTxMap {
  rx_pins: &'static [u8],
  tx_pins: &'static [u8],
  alp_rx_pins: &'static [u8],
  alp_tx_pins: &'static [u8],
}

impl RxTxMap {
  pub fn new(
    rx_pins: &'static [u8],
    tx_pins: &'static [u8],
    alp_rx_pins: &'static [u8],
    alp_tx_pins: &'static [u8],
  ) -> Self {
    assert!((rx_pins.len() + tx_pins.len()) <= MAX_PINS, "There are 13 Rx/Tx mapping slots available");
    assert!(alp_rx_pins.iter().all(|&p| rx_pins.contains(&p)), "ALP Rx pins must be a subset of Rx pins");
    assert!(alp_tx_pins.iter().all(|&p| tx_pins.contains(&p)), "ALP Tx pins must be a subset of Tx pins");
    Self { rx_pins, tx_pins, alp_rx_pins, alp_tx_pins }
  }

  /// Generate up to 21 sensing cycles for the IQS7211E controller.
  ///
  /// Each cycle allows simultaneous sensing of:
  /// - One Prox A channel (from `PROX_A_PINS`: Rx0–Rx3)
  /// - One Prox B channel (from `PROX_B_PINS`: Rx4–Rx7)
  ///
  /// Both channels in a cycle must share the same Tx pin. If no matching channel
  /// is available from one of the blocks, the corresponding entry is set to `UNUSED` (255).
  ///
  /// The generated cycles are ordered such that:
  /// - Prox A and Prox B channels are paired when possible, sharing the same Tx
  /// - Remaining unpaired channels are assigned to separate cycles
  /// - A maximum of 21 cycles is generated (hardware limit)
  ///
  /// The `channel_index` values assigned to `prox_a` and `prox_b` fields represent
  /// a linear index into the full Rx × Tx matrix. These can be used to configure
  /// the channel allocation registers accordingly.
  ///
  /// # Returns
  ///
  /// An array of 21 [`Cycle`] structs. Any unused cycle entries will be filled with
  /// `Cycle { tx: 0, prox_a: 255, prox_b: 255 }`.
  ///
  /// # Example
  ///
  /// ```ignore
  /// use iqs7211e::RxTxMap;
  ///
  /// let mapping = RxTxMap::new(
  ///     &[0, 1, 2, 3, 4, 5], // rx pins
  ///     &[0, 1],             // tx pins
  ///     &[],                 // alp rx pins
  ///     &[],                 // alp tx pins
  /// );
  ///
  /// let cycles = mapping.cycles();
  ///
  /// for (i, c) in cycles.iter().enumerate() {
  ///     defmt::info!("Cycle {}: Tx={}, A={}, B={}", i, c.tx, c.prox_a, c.prox_b);
  /// }
  /// ```
  fn cycles(&self) -> [Cycle; MAX_CYCLES] {
    let mut out = [Cycle { tx: 0, prox_a: UNUSED, prox_b: UNUSED }; MAX_CYCLES];
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
        for i in 0..cycle_index {
          if out[i].tx == tx {
            if is_a && out[i].prox_a == UNUSED {
              out[i].prox_a = channel_index;
              backfilled = true;
              break;
            } else if is_b && out[i].prox_b == UNUSED {
              out[i].prox_b = channel_index;
              backfilled = true;
              break;
            }
          }
        }

        // if we didn't find a matching cycle to backfill, create a new one
        if !backfilled {
          out[cycle_index] = Cycle {
            tx,
            prox_a: if is_a { channel_index } else { UNUSED },
            prox_b: if is_b { channel_index } else { UNUSED },
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
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub struct AtiDivMul {
  #[bits(5, default = 1)]
  pub coarse_divider: u8,
  #[bits(4, default = 15)]
  pub coarse_multiplier: u8,
  #[bits(5, default = 30)]
  pub fine_divider: u8,
  #[bits(2)]
  __: u2,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub struct GestureEnable {
  #[bits(default = true)]
  pub tap: bool,
  #[bits(default = true)]
  pub double_tap: bool,
  #[bits(default = true)]
  pub triple_tap: bool,
  #[bits(default = true)]
  pub press_hold: bool,
  #[bits(default = true)]
  pub palm: bool,
  #[bits(3)]
  __: u8,
  #[bits(default = true)]
  pub swipe_x_pos: bool,
  #[bits(default = true)]
  pub swipe_x_neg: bool,
  #[bits(default = true)]
  pub swipe_y_pos: bool,
  #[bits(default = true)]
  pub swipe_y_neg: bool,
  #[bits(default = true)]
  pub swipe_hold_x_pos: bool,
  #[bits(default = true)]
  pub swipe_hold_x_neg: bool,
  #[bits(default = true)]
  pub swipe_hold_y_pos: bool,
  #[bits(default = true)]
  pub swipe_hold_y_neg: bool,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
struct AlpSetup {
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
  #[bits(6)]
  __: u8,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
struct AlpTxEnable {
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
  #[bits(3)]
  __: u8,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub(crate) struct AlpHwSettings {
  #[bits(2, default = InitDelay::Cycles64)]
  pub(crate) init_delay: InitDelay,
  #[bits(3, default = AutoProxCycles::Cycles8)]
  pub(crate) lp1_auto_prox_cycles: AutoProxCycles,
  #[bits(3, default = AutoProxCycles::Cycles32)]
  pub(crate) lp2_auto_prox_cycles: AutoProxCycles,
  #[bits(2, default = MaxCount::Count1023)]
  pub(crate) max_count: MaxCount,
  #[bits(2, default = OpampBias::Microamp10)]
  pub(crate) opamp_bias: OpampBias,
  #[bits(1, default = CSCap::Picofarad80)]
  pub(crate) cs_cap: CSCap,
  #[bits(default = false)]
  pub(crate) rf_filter: bool,
  #[bits(1, default = CSDischarge::To0v)]
  pub(crate) cs_discharge: CSDischarge,
  #[bits(default = true)]
  pub(crate) nm_in_static: bool,
}

#[bitfield(u16)]
#[derive(PartialEq, Eq, defmt::Format)]
pub(crate) struct TpHwSettings {
  #[bits(2, default = InitDelay::Cycles64)]
  init_delay: InitDelay,
  #[bits(6)]
  __: u8,
  #[bits(2, default = MaxCount::Count1023)]
  max_count: MaxCount,
  #[bits(2, default = OpampBias::Microamp10)]
  opamp_bias: OpampBias,
  #[bits(1, default = CSCap::Picofarad80)]
  cs_cap: CSCap,
  #[bits(default = false)]
  rf_filter: bool,
  #[bits(1, default = CSDischarge::To0v)]
  cs_discharge: CSDischarge,
  #[bits(default = true)]
  nm_in_static: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
enum InitDelay {
  Cycles4 = 0b00,
  Cycles16 = 0b01,
  Cycles32 = 0b10,
  Cycles64 = 0b11,
}

impl InitDelay {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b00 => Self::Cycles4,
      0b01 => Self::Cycles16,
      0b10 => Self::Cycles32,
      0b11 => Self::Cycles64,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub(crate) enum AutoProxCycles {
  Cycles4 = 0b000,
  Cycles8 = 0b001,
  Cycles16 = 0b010,
  Cycles32 = 0b011,
  Disabled = 0b100,
}

impl AutoProxCycles {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b000 => Self::Cycles4,
      0b001 => Self::Cycles8,
      0b010 => Self::Cycles16,
      0b011 => Self::Cycles32,
      _ => Self::Disabled,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
enum MaxCount {
  Count1023 = 0b00,
  Count2047 = 0b01,
  Count4095 = 0b10,
  Count16384 = 0b11,
}

impl MaxCount {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b00 => Self::Count1023,
      0b01 => Self::Count2047,
      0b10 => Self::Count4095,
      0b11 => Self::Count16384,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
enum OpampBias {
  Microamp2 = 0b00,
  Microamp5 = 0b01,
  Microamp7 = 0b10,
  Microamp10 = 0b11,
}

impl OpampBias {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b00 => Self::Microamp2,
      0b01 => Self::Microamp5,
      0b10 => Self::Microamp7,
      0b11 => Self::Microamp10,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
enum CSCap {
  Picofarad40 = 0b0,
  Picofarad80 = 0b1,
}

impl CSCap {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b0 => Self::Picofarad40,
      0b1 => Self::Picofarad80,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
enum CSDischarge {
  To0v = 0b0,
  To0_5v = 0b1,
}

impl CSDischarge {
  pub const fn into_bits(self) -> u8 {
    self as u8
  }

  pub const fn from_bits(bits: u8) -> Self {
    match bits {
      0b0 => Self::To0v,
      0b1 => Self::To0_5v,
      _ => unreachable!(),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct TouchThreshold {
  pub set: u8,
  pub clear: u8,
}

impl TouchThreshold {
  pub const fn new(set: u8, clear: u8) -> Self {
    Self { set, clear }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub struct Config {
  pub interrupt_mode: super::control::InterruptMode,
  pub rxtx_map: RxTxMap,
  pub tp_ati_div_mul: AtiDivMul,
  pub tp_ati_target: u16,
  pub tp_ati_comp_div: u8,
  pub gesture_enable: GestureEnable,
  pub touch_threshold: TouchThreshold,
  pub(crate) tp_hw_settings: TpHwSettings,
  pub(crate) alp_hw_settings: AlpHwSettings,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      interrupt_mode: super::control::InterruptMode::Event,
      rxtx_map: RxTxMap::new(&[], &[], &[], &[]),
      tp_ati_div_mul: AtiDivMul::default(),
      tp_ati_target: 300,
      tp_ati_comp_div: 10,
      gesture_enable: GestureEnable::default(),
      touch_threshold: TouchThreshold::new(50, 20),
      tp_hw_settings: TpHwSettings::default(),
      alp_hw_settings: AlpHwSettings::default(),
    }
  }
}

impl<I, E, RDY> Iqs7211e<I, RDY>
where
  I: I2c<SevenBitAddress, Error = E>,
  RDY: Wait,
{
  pub(crate) fn write_config(&mut self, config: Config) -> Result<(), Error<E>> {
    let mut buf = [0u8; 30]; // Temporary array which holds the bytes to be transferred.

    /* Change the ALP ATI Compensation */
    /* Memory Map Position 0x1F - 0x20 */
    buf[0] = ALP_COMPENSATION_A_0;
    buf[1] = ALP_COMPENSATION_A_1;
    buf[2] = ALP_COMPENSATION_B_0;
    buf[3] = ALP_COMPENSATION_B_1;

    self.write_bytes(defs::IQS7211E_MM_ALP_ATI_COMP_A, buf[0..4].as_ref())?;
    info!("1. Write ALP Compensation");

    /* Change the ATI Settings */
    /* Memory Map Position 0x21 - 0x27 */
    buf[0..=1].copy_from_slice(&config.tp_ati_div_mul.into_bits().to_le_bytes());

    buf[2] = config.tp_ati_comp_div;
    buf[3] = TP_REF_DRIFT_LIMIT;

    buf[4..=5].copy_from_slice(config.tp_ati_target.to_le_bytes().as_ref());

    buf[6] = TP_MIN_COUNT_REATI_0;
    buf[7] = TP_MIN_COUNT_REATI_1;
    buf[8] = ALP_ATI_MULTIPLIERS_DIVIDERS_0;
    buf[9] = ALP_ATI_MULTIPLIERS_DIVIDERS_1;
    buf[10] = ALP_COMPENSATION_DIV;
    buf[11] = ALP_LTA_DRIFT_LIMIT;
    buf[12] = ALP_ATI_TARGET_0;
    buf[13] = ALP_ATI_TARGET_1;

    self.write_bytes(defs::IQS7211E_MM_TP_GLOBAL_MIRRORS, buf[0..14].as_ref())?;
    info!("2. Write ATI Settings");

    /* Change the RR and Timing Settings */
    /* Memory Map Position 0x28 - 0x32 */
    buf[0] = ACTIVE_MODE_REPORT_RATE_0;
    buf[1] = ACTIVE_MODE_REPORT_RATE_1;
    buf[2] = IDLE_TOUCH_MODE_REPORT_RATE_0;
    buf[3] = IDLE_TOUCH_MODE_REPORT_RATE_1;
    buf[4] = IDLE_MODE_REPORT_RATE_0;
    buf[5] = IDLE_MODE_REPORT_RATE_1;
    buf[6] = LP1_MODE_REPORT_RATE_0;
    buf[7] = LP1_MODE_REPORT_RATE_1;
    buf[8] = LP2_MODE_REPORT_RATE_0;
    buf[9] = LP2_MODE_REPORT_RATE_1;
    buf[10] = ACTIVE_MODE_TIMEOUT_0;
    buf[11] = ACTIVE_MODE_TIMEOUT_1;
    buf[12] = IDLE_TOUCH_MODE_TIMEOUT_0;
    buf[13] = IDLE_TOUCH_MODE_TIMEOUT_1;
    buf[14] = IDLE_MODE_TIMEOUT_0;
    buf[15] = IDLE_MODE_TIMEOUT_1;
    buf[16] = LP1_MODE_TIMEOUT_0;
    buf[17] = LP1_MODE_TIMEOUT_1;
    buf[18] = REATI_RETRY_TIME;
    buf[19] = REF_UPDATE_TIME;
    buf[20] = I2C_TIMEOUT_0;
    buf[21] = I2C_TIMEOUT_1;

    self.write_bytes(defs::IQS7211E_MM_ACTIVE_MODE_RR, buf[0..22].as_ref())?;
    info!("3. Write Report rates and timings");

    /* Change the System Settings */
    /* Memory Map Position 0x33 - 0x35 */
    buf[0] = SYSTEM_CONTROL_0;
    buf[1] = SYSTEM_CONTROL_1;
    buf[2] = CONFIG_SETTINGS0;
    buf[3] = CONFIG_SETTINGS1;
    buf[4] = OTHER_SETTINGS_0;
    buf[5] = OTHER_SETTINGS_1;
    self.write_bytes(defs::IQS7211E_MM_SYS_CONTROL, buf[0..6].as_ref())?;
    info!("4. Write System control settings");

    /* Change the ALP Settings */
    /* Memory Map Position 0x36 - 0x37 */
    let alp_setup = AlpSetup::new()
      .with_rx0(config.rxtx_map.alp_rx_pins.contains(&0))
      .with_rx1(config.rxtx_map.alp_rx_pins.contains(&1))
      .with_rx2(config.rxtx_map.alp_rx_pins.contains(&2))
      .with_rx3(config.rxtx_map.alp_rx_pins.contains(&3))
      .with_rx4(config.rxtx_map.alp_rx_pins.contains(&4))
      .with_rx5(config.rxtx_map.alp_rx_pins.contains(&5))
      .with_rx6(config.rxtx_map.alp_rx_pins.contains(&6))
      .with_rx7(config.rxtx_map.alp_rx_pins.contains(&7))
      .with_cap_self_proj(true)
      .with_count_filter(true);

    let alp_tx_enable = AlpTxEnable::new()
      .with_tx0(config.rxtx_map.alp_tx_pins.contains(&0))
      .with_tx1(config.rxtx_map.alp_tx_pins.contains(&1))
      .with_tx2(config.rxtx_map.alp_tx_pins.contains(&2))
      .with_tx3(config.rxtx_map.alp_tx_pins.contains(&3))
      .with_tx4(config.rxtx_map.alp_tx_pins.contains(&4))
      .with_tx5(config.rxtx_map.alp_tx_pins.contains(&5))
      .with_tx6(config.rxtx_map.alp_tx_pins.contains(&6))
      .with_tx7(config.rxtx_map.alp_tx_pins.contains(&7))
      .with_tx8(config.rxtx_map.alp_tx_pins.contains(&8))
      .with_tx9(config.rxtx_map.alp_tx_pins.contains(&9))
      .with_tx10(config.rxtx_map.alp_tx_pins.contains(&10))
      .with_tx11(config.rxtx_map.alp_tx_pins.contains(&11))
      .with_tx12(config.rxtx_map.alp_tx_pins.contains(&12));

    buf[0..=1].copy_from_slice(&alp_setup.into_bits().to_le_bytes());
    buf[2..=3].copy_from_slice(&alp_tx_enable.into_bits().to_le_bytes());
    self.write_bytes(defs::IQS7211E_MM_ALP_SETUP, buf[0..4].as_ref())?;
    info!("5. Write ALP Settings");

    /* Change the Threshold Settings */
    /* Memory Map Position 0x38 - 0x3A */
    buf[0] = config.touch_threshold.set;
    buf[1] = config.touch_threshold.clear;
    buf[2] = ALP_THRESHOLD_0;
    buf[3] = ALP_THRESHOLD_1;
    buf[4] = ALP_SET_DEBOUNCE;
    buf[5] = ALP_CLEAR_DEBOUNCE;

    self.write_bytes(defs::IQS7211E_MM_TP_TOUCH_SET_CLEAR_THR, buf[0..6].as_ref())?;
    info!("6. Write Threshold settings");

    /* Change the Button and ALP count and LTA betas */
    /* Memory Map Position 0x3B - 0x3C */
    buf[0] = ALP_COUNT_BETA_LP1;
    buf[1] = ALP_LTA_BETA_LP1;
    buf[2] = ALP_COUNT_BETA_LP2;
    buf[3] = ALP_LTA_BETA_LP2;

    self.write_bytes(defs::IQS7211E_MM_LP1_FILTERS, buf[0..4].as_ref())?;
    info!("7. Write Filter Betas");

    /* Change the Hardware Settings */
    /* Memory Map Position 0x3D - 0x40 */
    buf[0] = TP_CONVERSION_FREQUENCY_UP_PASS_LENGTH;
    buf[1] = TP_CONVERSION_FREQUENCY_FRACTION_VALUE;
    buf[2] = ALP_CONVERSION_FREQUENCY_UP_PASS_LENGTH;
    buf[3] = ALP_CONVERSION_FREQUENCY_FRACTION_VALUE;

    buf[4..=5].copy_from_slice(&config.tp_hw_settings.into_bits().to_le_bytes());
    buf[6..=7].copy_from_slice(&config.alp_hw_settings.into_bits().to_le_bytes());

    self.write_bytes(defs::IQS7211E_MM_TP_CONV_FREQ, buf[0..8].as_ref())?;
    info!("8. Write Hardware settings");

    /* Change the TP Setup */
    /* Memory Map Position 0x41 - 0x49 */
    buf[0] = TRACKPAD_SETTINGS_0_0;
    buf[1] = config.rxtx_map.rx_pins.len() as u8; // Total Rxs
    buf[2] = config.rxtx_map.tx_pins.len() as u8; // Total Txs
    buf[3] = TRACKPAD_SETTINGS_1_1;
    buf[4] = X_RESOLUTION_0;
    buf[5] = X_RESOLUTION_1;
    buf[6] = Y_RESOLUTION_0;
    buf[7] = Y_RESOLUTION_1;
    buf[8] = XY_DYNAMIC_FILTER_BOTTOM_SPEED_0;
    buf[9] = XY_DYNAMIC_FILTER_BOTTOM_SPEED_1;
    buf[10] = XY_DYNAMIC_FILTER_TOP_SPEED_0;
    buf[11] = XY_DYNAMIC_FILTER_TOP_SPEED_1;
    buf[12] = XY_DYNAMIC_FILTER_BOTTOM_BETA;
    buf[13] = XY_DYNAMIC_FILTER_STATIC_FILTER_BETA;
    buf[14] = STATIONARY_TOUCH_MOV_THRESHOLD;
    buf[15] = FINGER_SPLIT_FACTOR;
    buf[16] = X_TRIM_VALUE;
    buf[17] = Y_TRIM_VALUE;

    self.write_bytes(defs::IQS7211E_MM_TP_RX_SETTINGS, buf[0..18].as_ref())?;
    info!("9. Write TP Settings");

    /* Change the Settings Version Numbers */
    /* Memory Map Position 0x4A - 0x4A */
    buf[0] = MINOR_VERSION;
    buf[1] = MAJOR_VERSION;

    self.write_bytes(defs::IQS7211E_MM_SETTINGS_VERSION, buf[0..2].as_ref())?;
    info!("10. Write Version numbers");

    /* Change the Gesture Settings */
    /* Memory Map Position 0x4B - 0x55 */
    buf[0..=1].copy_from_slice(&config.gesture_enable.into_bits().to_le_bytes());
    buf[2] = TAP_TOUCH_TIME_0;
    buf[3] = TAP_TOUCH_TIME_1;
    buf[4] = TAP_WAIT_TIME_0;
    buf[5] = TAP_WAIT_TIME_1;
    buf[6] = TAP_DISTANCE_0;
    buf[7] = TAP_DISTANCE_1;
    buf[8] = HOLD_TIME_0;
    buf[9] = HOLD_TIME_1;
    buf[10] = SWIPE_TIME_0;
    buf[11] = SWIPE_TIME_1;
    buf[12] = SWIPE_X_DISTANCE_0;
    buf[13] = SWIPE_X_DISTANCE_1;
    buf[14] = SWIPE_Y_DISTANCE_0;
    buf[15] = SWIPE_Y_DISTANCE_1;
    buf[16] = SWIPE_X_CONS_DIST_0;
    buf[17] = SWIPE_X_CONS_DIST_1;
    buf[18] = SWIPE_Y_CONS_DIST_0;
    buf[19] = SWIPE_Y_CONS_DIST_1;
    buf[20] = SWIPE_ANGLE;
    buf[21] = PALM_THRESHOLD;

    self.write_bytes(defs::IQS7211E_MM_GESTURE_ENABLE, buf[0..22].as_ref())?;
    info!("11. Write Gesture Settings");

    /* Change the RxTx Mapping */
    /* Memory Map Position 0x56 - 0x5C */
    info!("rxtx pins: {:?}", config.rxtx_map.pins().as_ref());
    self.write_bytes(defs::IQS7211E_MM_RX_TX_MAPPING_0_1, config.rxtx_map.pins().as_ref())?;
    info!("12. Write Rx Tx Map Settings");

    /* Change the Allocation of channels into cycles 0-9 */
    /* Memory Map Position 0x5D - 0x6B */
    let cycles = config.rxtx_map.cycles();
    info!("Cycles: {:?}", cycles);
    buf[0] = 0x05;
    buf[1] = cycles[0].prox_a;
    buf[2] = cycles[0].prox_b;
    buf[3] = 0x05;
    buf[4] = cycles[1].prox_a;
    buf[5] = cycles[1].prox_b;
    buf[6] = 0x05;
    buf[7] = cycles[2].prox_a;
    buf[8] = cycles[2].prox_b;
    buf[9] = 0x05;
    buf[10] = cycles[3].prox_a;
    buf[11] = cycles[3].prox_b;
    buf[12] = 0x05;
    buf[13] = cycles[4].prox_a;
    buf[14] = cycles[4].prox_b;
    buf[15] = 0x05;
    buf[16] = cycles[5].prox_a;
    buf[17] = cycles[5].prox_b;
    buf[18] = 0x05;
    buf[19] = cycles[6].prox_a;
    buf[20] = cycles[6].prox_b;
    buf[21] = 0x05;
    buf[22] = cycles[7].prox_a;
    buf[23] = cycles[7].prox_b;
    buf[24] = 0x05;
    buf[25] = cycles[8].prox_a;
    buf[26] = cycles[8].prox_b;
    buf[27] = 0x05;
    buf[28] = cycles[9].prox_a;
    buf[29] = cycles[9].prox_b;

    self.write_bytes(defs::IQS7211E_MM_PROXA_CYCLE0, buf[0..30].as_ref())?;
    info!("13. Write Cycle 0 - 9 Settings");

    /* Change the Allocation of channels into cycles 10-19 */
    /* Memory Map Position 0x6C - 0x7A */
    buf[0] = 0x05;
    buf[1] = cycles[10].prox_a;
    buf[2] = cycles[10].prox_b;
    buf[3] = 0x05;
    buf[4] = cycles[11].prox_a;
    buf[5] = cycles[11].prox_b;
    buf[6] = 0x05;
    buf[7] = cycles[12].prox_a;
    buf[8] = cycles[12].prox_b;
    buf[9] = 0x05;
    buf[10] = cycles[13].prox_a;
    buf[11] = cycles[13].prox_b;
    buf[12] = 0x05;
    buf[13] = cycles[14].prox_a;
    buf[14] = cycles[14].prox_b;
    buf[15] = 0x05;
    buf[16] = cycles[15].prox_a;
    buf[17] = cycles[15].prox_b;
    buf[18] = 0x05;
    buf[19] = cycles[16].prox_a;
    buf[20] = cycles[16].prox_b;
    buf[21] = 0x05;
    buf[22] = cycles[17].prox_a;
    buf[23] = cycles[17].prox_b;
    buf[24] = 0x05;
    buf[25] = cycles[18].prox_a;
    buf[26] = cycles[18].prox_b;
    buf[27] = 0x05;
    buf[28] = cycles[19].prox_a;
    buf[29] = cycles[19].prox_b;

    self.write_bytes(defs::IQS7211E_MM_PROXA_CYCLE10, buf[0..30].as_ref())?;
    info!("14. Write Cycle 10 - 19 Settings");

    /* Change the Allocation of channels into cycles 20 */
    /* Memory Map Position 0x7B - 0x7C */
    buf[0] = 0x05;
    buf[1] = cycles[20].prox_a;
    buf[2] = cycles[20].prox_b;
    buf[3] = 0x01;

    self.write_bytes(defs::IQS7211E_MM_PROXA_CYCLE20, buf[0..4].as_ref())?;
    info!("15. Write Cycle 20  Settings");

    Ok(())
  }
}

/* Change the ALP ATI Compensation */
/* Memory Map Position 0x1F - 0x20 */
pub const ALP_COMPENSATION_A_0: u8 = 0xB9;
pub const ALP_COMPENSATION_A_1: u8 = 0x01;
pub const ALP_COMPENSATION_B_0: u8 = 0xD2;
pub const ALP_COMPENSATION_B_1: u8 = 0x01;

/* Change the ATI Settings */
/* Memory Map Position 0x21 - 0x27 */
pub const TP_REF_DRIFT_LIMIT: u8 = 0x32;
pub const TP_MIN_COUNT_REATI_0: u8 = 0x32;
pub const TP_MIN_COUNT_REATI_1: u8 = 0x00;
pub const ALP_ATI_MULTIPLIERS_DIVIDERS_0: u8 = 0x23;
pub const ALP_ATI_MULTIPLIERS_DIVIDERS_1: u8 = 0x02;
pub const ALP_COMPENSATION_DIV: u8 = 0x04;
pub const ALP_LTA_DRIFT_LIMIT: u8 = 0x14;
pub const ALP_ATI_TARGET_0: u8 = 0xC8;
pub const ALP_ATI_TARGET_1: u8 = 0x00;

/* Change the Report Rates and Timing */
/* Memory Map Position 0x28 - 0x32 */
pub const ACTIVE_MODE_REPORT_RATE_0: u8 = 0x0A;
pub const ACTIVE_MODE_REPORT_RATE_1: u8 = 0x00;
pub const IDLE_TOUCH_MODE_REPORT_RATE_0: u8 = 0x32;
pub const IDLE_TOUCH_MODE_REPORT_RATE_1: u8 = 0x00;
pub const IDLE_MODE_REPORT_RATE_0: u8 = 0x14;
pub const IDLE_MODE_REPORT_RATE_1: u8 = 0x00;
pub const LP1_MODE_REPORT_RATE_0: u8 = 0x50;
pub const LP1_MODE_REPORT_RATE_1: u8 = 0x00;
pub const LP2_MODE_REPORT_RATE_0: u8 = 0xA0;
pub const LP2_MODE_REPORT_RATE_1: u8 = 0x00;
pub const ACTIVE_MODE_TIMEOUT_0: u8 = 0x0A;
pub const ACTIVE_MODE_TIMEOUT_1: u8 = 0x00;
pub const IDLE_TOUCH_MODE_TIMEOUT_0: u8 = 0x3C;
pub const IDLE_TOUCH_MODE_TIMEOUT_1: u8 = 0x00;
pub const IDLE_MODE_TIMEOUT_0: u8 = 0x0A;
pub const IDLE_MODE_TIMEOUT_1: u8 = 0x00;
pub const LP1_MODE_TIMEOUT_0: u8 = 0x0A;
pub const LP1_MODE_TIMEOUT_1: u8 = 0x00;
pub const REATI_RETRY_TIME: u8 = 0x05;
pub const REF_UPDATE_TIME: u8 = 0x08;
pub const I2C_TIMEOUT_0: u8 = 0x64;
pub const I2C_TIMEOUT_1: u8 = 0x00;

/* Change the System Settings */
/* Memory Map Position 0x33 - 0x35 */
pub const SYSTEM_CONTROL_0: u8 = 0x03;
pub const SYSTEM_CONTROL_1: u8 = 0x00;
pub const CONFIG_SETTINGS0: u8 = 0x2C;
pub const CONFIG_SETTINGS1: u8 = 0x06;
pub const OTHER_SETTINGS_0: u8 = 0x00;
pub const OTHER_SETTINGS_1: u8 = 0x00;

/* Change the ALP Settings */
/* Memory Map Position 0x36 - 0x37 */

/* Change the Thresholds and Debounce Settings */
/* Memory Map Position 0x38 - 0x3A */
pub const ALP_THRESHOLD_0: u8 = 0x08;
pub const ALP_THRESHOLD_1: u8 = 0x00;
pub const ALP_SET_DEBOUNCE: u8 = 0x04;
pub const ALP_CLEAR_DEBOUNCE: u8 = 0x04;

/* Change the Button and ALP count and LTA betas */
/* Memory Map Position 0x3B - 0x3C */
pub const ALP_COUNT_BETA_LP1: u8 = 0xDC;
pub const ALP_LTA_BETA_LP1: u8 = 0x08;
pub const ALP_COUNT_BETA_LP2: u8 = 0xF0;
pub const ALP_LTA_BETA_LP2: u8 = 0x10;

/* Change the Hardware Settings */
/* Memory Map Position 0x3D - 0x40 */
pub const TP_CONVERSION_FREQUENCY_UP_PASS_LENGTH: u8 = 0x02;
pub const TP_CONVERSION_FREQUENCY_FRACTION_VALUE: u8 = 0x1A;
pub const ALP_CONVERSION_FREQUENCY_UP_PASS_LENGTH: u8 = 0x02;
pub const ALP_CONVERSION_FREQUENCY_FRACTION_VALUE: u8 = 0x1A;

/* Change the Trackpad Settings */
/* Memory Map Position 0x41 - 0x49 */
pub const TRACKPAD_SETTINGS_0_0: u8 = 0x2C;
pub const TRACKPAD_SETTINGS_1_1: u8 = 0x01;
pub const X_RESOLUTION_0: u8 = 0xE8;
pub const X_RESOLUTION_1: u8 = 0x03;
pub const Y_RESOLUTION_0: u8 = 0xE8;
pub const Y_RESOLUTION_1: u8 = 0x03;
pub const XY_DYNAMIC_FILTER_BOTTOM_SPEED_0: u8 = 0x06;
pub const XY_DYNAMIC_FILTER_BOTTOM_SPEED_1: u8 = 0x00;
pub const XY_DYNAMIC_FILTER_TOP_SPEED_0: u8 = 0x7C;
pub const XY_DYNAMIC_FILTER_TOP_SPEED_1: u8 = 0x00;
pub const XY_DYNAMIC_FILTER_BOTTOM_BETA: u8 = 0x07;
pub const XY_DYNAMIC_FILTER_STATIC_FILTER_BETA: u8 = 0x80;
pub const STATIONARY_TOUCH_MOV_THRESHOLD: u8 = 0x14;
pub const FINGER_SPLIT_FACTOR: u8 = 0x03;
pub const X_TRIM_VALUE: u8 = 0x14;
pub const Y_TRIM_VALUE: u8 = 0x14;

/* Change the Settings Version Numbers */
/* Memory Map Position 0x4A - 0x4A */
pub const MINOR_VERSION: u8 = 0x00;
pub const MAJOR_VERSION: u8 = 0x00;

/* Change the Gesture Settings */
/* Memory Map Position 0x4B - 0x55 */
pub const TAP_TOUCH_TIME_0: u8 = 0x96;
pub const TAP_TOUCH_TIME_1: u8 = 0x00;
pub const TAP_WAIT_TIME_0: u8 = 0x96;
pub const TAP_WAIT_TIME_1: u8 = 0x00;
pub const TAP_DISTANCE_0: u8 = 0x32;
pub const TAP_DISTANCE_1: u8 = 0x00;
pub const HOLD_TIME_0: u8 = 0x2C;
pub const HOLD_TIME_1: u8 = 0x01;
pub const SWIPE_TIME_0: u8 = 0x96;
pub const SWIPE_TIME_1: u8 = 0x00;
pub const SWIPE_X_DISTANCE_0: u8 = 0xC8;
pub const SWIPE_X_DISTANCE_1: u8 = 0x00;
pub const SWIPE_Y_DISTANCE_0: u8 = 0xC8;
pub const SWIPE_Y_DISTANCE_1: u8 = 0x00;
pub const SWIPE_X_CONS_DIST_0: u8 = 0x64;
pub const SWIPE_X_CONS_DIST_1: u8 = 0x00;
pub const SWIPE_Y_CONS_DIST_0: u8 = 0x64;
pub const SWIPE_Y_CONS_DIST_1: u8 = 0x00;
pub const SWIPE_ANGLE: u8 = 0x17;
pub const PALM_THRESHOLD: u8 = 0x1E;
