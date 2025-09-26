/******************************************************************************
 * Refer to IQS7211E datasheet for more information, available here:          *
 * - https://www.azoteq.com/design/datasheets/                                *
 * ========================================================================== *
 *                       IQS7211E - Registers & Memory Map                    *
*******************************************************************************/

pub(crate) const I2C_ADDR: u8 = 0x56;

#[allow(dead_code)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reg {
  // App version information (0x00..0x04)
  AppVersion = 0x00,
  // ROM lib version information (0x05..0x09)
  RomVersion = 0x05,

  // Gesture data (0x0A..0x17)
  RelativeX = 0x0A,
  RelativeY = 0x0B,
  GestureX = 0x0C,
  GestureY = 0x0D,
  Gestures = 0x0E,
  InfoFlags = 0x0F,
  Finger1X = 0x10,
  Finger1Y = 0x11,
  Finger1TouchStrength = 0x12,
  Finger1Area = 0x13,
  Finger2X = 0x14,
  Finger2Y = 0x15,
  Finger2TouchStrength = 0x16,
  Finger2Area = 0x17,

  // Channel states & counts (0x18..0x1E)
  TouchState0 = 0x18,
  TouchState1 = 0x19,
  TouchState2 = 0x1A,
  LowPowerChannelCount = 0x1B,
  LowPowerChannelLta = 0x1C,
  LowPowerChannelCountA = 0x1D,
  LowPowerChannelCountB = 0x1E,

  // ALP & TP ATI settings (0x1F..0x27)
  AlpAutoTuningCompA = 0x1F,
  AlpAutoTuningCompB = 0x20,
  TpAutoTuningMultipliers = 0x21,
  TpRefDriftLimit = 0x22,
  TpAutoTuningTarget = 0x23,
  TpMinCountReAutoTuning = 0x24,
  AlpAutoTuningMultipliers = 0x25,
  AlpLtaDriftLimit = 0x26,
  AlpAutoTuningTarget = 0x27,

  // Report rates and timings (0x28..0x32)
  ActiveModeReportRate = 0x28,
  IdleTouchReportRate = 0x29,
  IdleModeReportRate = 0x2A,
  Lp1ModeReportRate = 0x2B,
  Lp2ModeReportRate = 0x2C,
  ActiveModeTimeout = 0x2D,
  IdleTouchModeTimeout = 0x2E,
  IdleModeTimeout = 0x2F,
  Lp1ModeTimeout = 0x30,
  RefUpdateReatiTime = 0x31,
  I2cTimeout = 0x32,

  // System and ALP setup (0x33..0x37)
  SysControl = 0x33,
  ConfigSettings = 0x34,
  OtherSettings = 0x35,
  AlpSetup = 0x36,
  AlpTxEnable = 0x37,

  // Trackpad & ALP thresholds (0x38..0x3A)
  TouchSetClearMultipliers = 0x38,
  LowPowerThreshold = 0x39,
  LowPowerSetClearDebounce = 0x3A,

  // Button/ALP betas (0x3B..0x3C)
  Lp1Filters = 0x3B,
  Lp2Filters = 0x3C,

  // Channel setup (0x3D..0x40)
  TpConvFreq = 0x3D,
  AlpConvFreq = 0x3E,
  TpHardware = 0x3F,
  AlpHardware = 0x40,

  // TP setup (0x41..0x49)
  TpRxSettings = 0x41,
  XResolution = 0x43,
  YResolution = 0x44,
  XyFilterBottomSpeed = 0x45,
  XyFilterTopSpeed = 0x46,
  StaticFilter = 0x47,
  FingerSplitMovement = 0x48,
  TrimValues = 0x49,

  // Settings version (0x4A)
  SettingsVersion = 0x4A,

  // Gesture settings (0x4B..0x55)
  GestureEnable = 0x4B,
  TapTime = 0x4C,
  AirTime = 0x4D,
  TapDistance = 0x4E,
  HoldTime = 0x4F,
  SwipeTime = 0x50,
  XInitialDistance = 0x51,
  YInitialDistance = 0x52,
  XConsecutiveDistance = 0x53,
  YConsecutiveDistance = 0x54,
  ThresholdAngle = 0x55,

  // Rx/Tx mapping (0x56..0x5C)
  RxTxMapping0_1 = 0x56,
  RxTxMapping2_3 = 0x57,
  RxTxMapping4_5 = 0x58,
  RxTxMapping6_7 = 0x59,
  RxTxMapping8_9 = 0x5A,
  RxTxMapping10_11 = 0x5B,
  RxTxMapping12 = 0x5C,

  // Cycle allocation (0x5D..0x7C)
  ProxACycle0 = 0x5D,
  ProxBCycle0 = 0x5E,
  Cycle1 = 0x5F,
  ProxACycle2 = 0x60,
  ProxBCycle2 = 0x61,
  Cycle3 = 0x62,
  ProxACycle4 = 0x63,
  ProxBCycle4 = 0x64,
  Cycle5 = 0x65,
  ProxACycle6 = 0x66,
  ProxBCycle6 = 0x67,
  Cycle7 = 0x68,
  ProxACycle8 = 0x69,
  ProxBCycle8 = 0x6A,
  Cycle9 = 0x6B,

  ProxACycle10 = 0x6C,
  ProxBCycle10 = 0x6D,
  Cycle11 = 0x6E,
  ProxACycle12 = 0x6F,
  ProxBCycle12 = 0x70,
  Cycle13 = 0x71,
  ProxACycle14 = 0x72,
  ProxBCycle14 = 0x73,
  Cycle15 = 0x74,
  ProxACycle16 = 0x75,
  ProxBCycle16 = 0x76,
  Cycle17 = 0x77,
  ProxACycle18 = 0x78,
  ProxBCycle18 = 0x79,
  Cycle19 = 0x7A,
  ProxACycle20 = 0x7B,
  ProxBCycle20 = 0x7C,
}

impl From<Reg> for u8 {
  #[inline]
  fn from(r: Reg) -> Self {
    r as u8
  }
}
pub(crate) const PRODUCT_NUMBER: u16 = 0x0458;
