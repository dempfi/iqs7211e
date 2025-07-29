/******************************************************************************
 * Refer to IQS7211E datasheet for more information, available here:          *
 * - https://www.azoteq.com/design/datasheets/                                *
 * ========================================================================== *
 *                       IQS7211E - Registers & Memory Map                    *
*******************************************************************************/
#![allow(dead_code)]

pub const IQS7211E_I2C_ADDR: u8 = 0x56;

/* Device Information - Read Only */

/* All Banks: 0x00 - 0x09 */
pub const IQS7211E_MM_PROD_NUM: u8 = 0x00;
pub const IQS7211E_MM_MAJOR_VERSION_NUM: u8 = 0x01;
pub const IQS7211E_MM_MINOR_VERSION_NUM: u8 = 0x02;

/* GESTURE_DATA: 0x0A - 0x17 */
pub const IQS7211E_MM_RELATIVE_X: u8 = 0x0A;
pub const IQS7211E_MM_RELATIVE_Y: u8 = 0x0B;
pub const IQS7211E_MM_GESTURE_X: u8 = 0x0C;
pub const IQS7211E_MM_GESTURE_Y: u8 = 0x0D;
pub const IQS7211E_MM_GESTURES: u8 = 0x0E;
pub const IQS7211E_MM_INFO_FLAGS: u8 = 0x0F;
pub const IQS7211E_MM_FINGER_1_X: u8 = 0x10;
pub const IQS7211E_MM_FINGER_1_Y: u8 = 0x11;
pub const IQS7211E_MM_FINGER_1_TOUCH_STRENGTH: u8 = 0x12;
pub const IQS7211E_MM_FINGER_1_AREA: u8 = 0x13;
pub const IQS7211E_MM_FINGER_2_X: u8 = 0x14;
pub const IQS7211E_MM_FINGER_2_Y: u8 = 0x15;
pub const IQS7211E_MM_FINGER_2_TOUCH_STRENGTH: u8 = 0x16;
pub const IQS7211E_MM_FINGER_2_AREA: u8 = 0x17;

/* CHANNEL STATES & COUNTS: 0x18 - 0x1E */
pub const IQS7211E_MM_TOUCH_STATE_0: u8 = 0x18;
pub const IQS7211E_MM_TOUCH_STATE_1: u8 = 0x19;
pub const IQS7211E_MM_TOUCH_STATE_2: u8 = 0x1A;
pub const IQS7211E_MM_ALP_CHANNEL_COUNT: u8 = 0x1B;
pub const IQS7211E_MM_ALP_CHANNEL_LTA: u8 = 0x1C;
pub const IQS7211E_MM_ALP_CHANNEL_COUNT_A: u8 = 0x1D;
pub const IQS7211E_MM_ALP_CHANNEL_COUNT_B: u8 = 0x1E;

/* ALP & TP ATI SETTINGS: 0x1F - 0x27 */
pub const IQS7211E_MM_ALP_ATI_COMP_A: u8 = 0x1F;
pub const IQS7211E_MM_ALP_ATI_COMP_B: u8 = 0x20;
pub const IQS7211E_MM_TP_GLOBAL_MIRRORS: u8 = 0x21;
pub const IQS7211E_MM_TP_REF_DRIFT: u8 = 0x22;
pub const IQS7211E_MM_TP_TARGET: u8 = 0x23;
pub const IQS7211E_MM_TP_REATI_COUNTS: u8 = 0x24;
pub const IQS7211E_MM_ALP_MIRRORS: u8 = 0x25;
pub const IQS7211E_MM_ALP_REF_DRIFT: u8 = 0x26;
pub const IQS7211E_MM_ALP_TARGET: u8 = 0x27;

/* REPORT RATES AND TIMINGS: 0x28 - 0x32 */
pub const IQS7211E_MM_ACTIVE_MODE_RR: u8 = 0x28;
pub const IQS7211E_MM_IDLE_TOUCH_MODE_RR: u8 = 0x29;
pub const IQS7211E_MM_IDLE_MODE_RR: u8 = 0x2A;
pub const IQS7211E_MM_LP1_MODE_RR: u8 = 0x2B;
pub const IQS7211E_MM_LP2_MODE_RR: u8 = 0x2C;
pub const IQS7211E_MM_ACTIVE_MODE_TIMEOUT: u8 = 0x2D;
pub const IQS7211E_MM_IDLE_TOUCH_MODE_TIMEOUT: u8 = 0x2E;
pub const IQS7211E_MM_IDLE_MODE_TIMEOUT: u8 = 0x2F;
pub const IQS7211E_MM_LP1_MODE_TIMEOUT: u8 = 0x30;
pub const IQS7211E_MM_REF_UPDATE_REATI_TIME: u8 = 0x31;
pub const IQS7211E_MM_I2C_TIMEOUT: u8 = 0x32;

/* SYSTEM AND ALP SETUP SETTINGS: 0x33 - 0x37 */
pub const IQS7211E_MM_SYS_CONTROL: u8 = 0x33;
pub const IQS7211E_MM_CONFIG_SETTINGS: u8 = 0x34;
pub const IQS7211E_MM_OTHER_SETTINGS: u8 = 0x35;
pub const IQS7211E_MM_ALP_SETUP: u8 = 0x36;
pub const IQS7211E_MM_ALP_TX_ENABLE: u8 = 0x37;

/* TRACKPAD AND ALP THRESHOLDS: 0x38 - 0x3A */
pub const IQS7211E_MM_TP_TOUCH_SET_CLEAR_THR: u8 = 0x38;
pub const IQS7211E_MM_ALP_THRESHOLD: u8 = 0x39;
pub const IQS7211E_MM_ALP_SET_CLEAR_DEBOUNCE: u8 = 0x3A;

/* ALP CHANNEL SETUP: 0x3B - 0x3C */
pub const IQS7211E_MM_LP1_FILTERS: u8 = 0x3B;
pub const IQS7211E_MM_LP2_FILTERS: u8 = 0x3C;

/* CHANNEL SETUP: 0x3D - 0x40 */

pub const IQS7211E_MM_TP_CONV_FREQ: u8 = 0x3D;
pub const IQS7211E_MM_ALP_CONV_FREQ: u8 = 0x3E;
pub const IQS7211E_MM_TP_HARDWARE: u8 = 0x3F;
pub const IQS7211E_MM_ALP_HARDWARE: u8 = 0x40;

/* TP SETUP: 0x41 - 0x49 */
pub const IQS7211E_MM_TP_RX_SETTINGS: u8 = 0x41;
pub const IQS7211E_MM_MAX_TOUCHES_TX: u8 = 0x41;
pub const IQS7211E_MM_X_RESOLUTION: u8 = 0x43;
pub const IQS7211E_MM_Y_RESOLUTION: u8 = 0x44;
pub const IQS7211E_MM_XY_FILTER_BOTTOM_SPEED: u8 = 0x45;
pub const IQS7211E_MM_XY_FILTER_TOPSPEED: u8 = 0x46;
pub const IQS7211E_MM_STATIC_FILTER: u8 = 0x47;
pub const IQS7211E_MM_FINGER_SPLIT_MOVEMENT: u8 = 0x48;
pub const IQS7211E_MM_TRIM_VALUES: u8 = 0x49;

/*SETTINGS VERSIONS: 0x4A */

pub const IQS7211E_MM_SETTINGS_VERSION: u8 = 0x4A;

/* GESTURE SETTINGS: 0x4B - 0x55 */
pub const IQS7211E_MM_GESTURE_ENABLE: u8 = 0x4B;
pub const IQS7211E_MM_TAP_TIME: u8 = 0x4C;
pub const IQS7211E_MM_AIR_TIME: u8 = 0x4D;
pub const IQS7211E_MM_TAP_DISTANCE: u8 = 0x4E;
pub const IQS7211E_MM_HOLD_TIME: u8 = 0x4F;
pub const IQS7211E_MM_SWIPE_TIME: u8 = 0x50;
pub const IQS7211E_MM_X_INITIAL_DISTANCE: u8 = 0x51;
pub const IQS7211E_MM_Y_INITIAL_DISTANCE: u8 = 0x52;
pub const IQS7211E_MM_X_CONSECUTIVE_DISTANCE: u8 = 0x53;
pub const IQS7211E_MM_Y_CONSECUTIVE_DISTANCE: u8 = 0x54;
pub const IQS7211E_MM_THRESHOLD_ANGLE: u8 = 0x55;

/* GESTURE SETTINGS: 0x56 - 0x5C */
pub const IQS7211E_MM_RX_TX_MAPPING_0_1: u8 = 0x56;
pub const IQS7211E_MM_RX_TX_MAPPING_2_3: u8 = 0x57;
pub const IQS7211E_MM_RX_TX_MAPPING_4_5: u8 = 0x58;
pub const IQS7211E_MM_RX_TX_MAPPING_6_7: u8 = 0x59;
pub const IQS7211E_MM_RX_TX_MAPPING_8_9: u8 = 0x5A;
pub const IQS7211E_MM_RX_TX_MAPPING_10_11: u8 = 0x5B;
pub const IQS7211E_MM_RX_TX_MAPPING_12: u8 = 0x5C;

/* CYCLE SETTINGS: 0x5D - 0x6B */
pub const IQS7211E_MM_PROXA_CYCLE0: u8 = 0x5D;
pub const IQS7211E_MM_PROXB_CYCLE0: u8 = 0x5E;
pub const IQS7211E_MM_CYCLE1: u8 = 0x5F;
pub const IQS7211E_MM_PROXA_CYCLE2: u8 = 0x60;
pub const IQS7211E_MM_PROXB_CYCLE2: u8 = 0x61;
pub const IQS7211E_MM_CYCLE3: u8 = 0x62;
pub const IQS7211E_MM_PROXA_CYCLE4: u8 = 0x63;
pub const IQS7211E_MM_PROXB_CYCLE4: u8 = 0x64;
pub const IQS7211E_MM_CYCLE5: u8 = 0x65;
pub const IQS7211E_MM_PROXA_CYCLE6: u8 = 0x66;
pub const IQS7211E_MM_PROXB_CYCLE6: u8 = 0x67;
pub const IQS7211E_MM_CYCLE7: u8 = 0x68;
pub const IQS7211E_MM_PROXA_CYCLE8: u8 = 0x69;
pub const IQS7211E_MM_PROXB_CYCLE8: u8 = 0x6A;
pub const IQS7211E_MM_CYCLE9: u8 = 0x6B;

/* CYCLE SETTINGS2: 0x6C - 0x7C */
pub const IQS7211E_MM_PROXA_CYCLE10: u8 = 0x6C;
pub const IQS7211E_MM_PROXB_CYCLE10: u8 = 0x6D;
pub const IQS7211E_MM_CYCLE11: u8 = 0x6E;
pub const IQS7211E_MM_PROXA_CYCLE12: u8 = 0x6F;
pub const IQS7211E_MM_PROXB_CYCLE12: u8 = 0x70;
pub const IQS7211E_MM_CYCLE13: u8 = 0x71;
pub const IQS7211E_MM_PROXA_CYCLE14: u8 = 0x72;
pub const IQS7211E_MM_PROXB_CYCLE14: u8 = 0x73;
pub const IQS7211E_MM_CYCLE15: u8 = 0x74;
pub const IQS7211E_MM_PROXA_CYCLE16: u8 = 0x75;
pub const IQS7211E_MM_PROXB_CYCLE16: u8 = 0x76;
pub const IQS7211E_MM_CYCLE17: u8 = 0x77;
pub const IQS7211E_MM_PROXA_CYCLE18: u8 = 0x78;
pub const IQS7211E_MM_PROXB_CYCLE18: u8 = 0x79;
pub const IQS7211E_MM_CYCLE19: u8 = 0x7A;
pub const IQS7211E_MM_PROXA_CYCLE20: u8 = 0x7B;
pub const IQS7211E_MM_PROXB_CYCLE20: u8 = 0x7C;

/* Device Info */
pub const IQS7211E_PRODUCT_NUM: u16 = 0x0458;

// Info Flags Byte Bits.
pub const IQS7211E_CHARGING_MODE_BIT_0: u8 = 0;
pub const IQS7211E_CHARGING_MODE_BIT_1: u8 = 1;
pub const IQS7211E_CHARGING_MODE_BIT_2: u8 = 2;
pub const IQS7211E_ACTIVE_BITS: u8 = 0b000;
pub const IQS7211E_IDLE_TOUCH_BITS: u8 = 0b001;
pub const IQS7211E_IDLE_BITS: u8 = 0b010;
pub const IQS7211E_LP1_BITS: u8 = 0b011;
pub const IQS7211E_LP2_BITS: u8 = 0b100;
pub const IQS7211E_ATI_ERROR_BIT: u8 = 3;
pub const IQS7211E_RE_ATI_OCCURRED_BIT: u8 = 4;
pub const IQS7211E_ALP_ATI_ERROR_BIT: u8 = 5;
pub const IQS7211E_ALP_RE_ATI_OCCURRED_BIT: u8 = 4;
pub const IQS7211E_SHOW_RESET_BIT: u8 = 7;
pub const IQS7211E_NUM_FINGERS_BIT_0: u8 = 0; // 8
pub const IQS7211E_NUM_FINGERS_BIT_1: u8 = 1; // 9
pub const IQS7211E_NO_FINGERS_BITS: u8 = 0b00;
pub const IQS7211E_1_FINGER_ACTIVE_BITS: u8 = 0b01;
pub const IQS7211E_2_FINGER_ACTIVE_BITS: u8 = 0b10;
pub const IQS7211E_TP_MOVEMENT_BIT: u8 = 2; // 10
pub const IQS7211E_TOO_MANY_FINGERS_BIT: u8 = 4; // 12
pub const IQS7211E_ALP_OUTPUT_BIT: u8 = 6; // 14

// System Control Bits
pub const IQS7211E_MODE_SELECT_BIT_0: u8 = 0;
pub const IQS7211E_MODE_SELECT_BIT_1: u8 = 1;
pub const IQS7211E_MODE_SELECT_BIT_2: u8 = 2;
pub const IQS7211E_TP_RESEED_BIT: u8 = 3;
pub const IQS7211E_ALP_RESEED_BIT: u8 = 4;
pub const IQS7211E_TP_RE_ATI_BIT: u8 = 5;
pub const IQS7211E_ALP_RE_ATI_BIT: u8 = 6;
pub const IQS7211E_ACK_RESET_BIT: u8 = 7;
pub const IQS7211E_SW_RESET_BIT: u8 = 1; // 9
pub const IQS7211E_SUSPEND_BIT: u8 = 3; // 11

// Config Settings Bits
pub const IQS7211E_EVENT_MODE_BIT: u8 = 0; // 8

// Gesture Bits
pub const IQS7211E_GESTURE_SINGLE_TAP_BIT: u8 = 0;
pub const IQS7211E_GESTURE_DOUBLE_TAP_BIT: u8 = 1;
pub const IQS7211E_GESTURE_TRIPLE_TAP_BIT: u8 = 2;
pub const IQS7211E_GESTURE_PRESS_HOLD_BIT: u8 = 3;
pub const IQS7211E_GESTURE_PALM_GESTURE_BIT: u8 = 4;
pub const IQS7211E_GESTURE_SWIPE_X_POSITIVE_BIT: u8 = 0; // 8
pub const IQS7211E_GESTURE_SWIPE_X_NEGATIVE_BIT: u8 = 1; // 9
pub const IQS7211E_GESTURE_SWIPE_Y_POSITIVE_BIT: u8 = 2; // 10
pub const IQS7211E_GESTURE_SWIPE_Y_NEGATIVE_BIT: u8 = 3; // 11
pub const IQS7211E_GESTURE_SWIPE_HOLD_X_POSITIVE_BIT: u8 = 4; // 12
pub const IQS7211E_GESTURE_SWIPE_HOLD_X_NEGATIVE_BIT: u8 = 5; // 13
pub const IQS7211E_GESTURE_SWIPE_HOLD_Y_POSITIVE_BIT: u8 = 6; // 14
pub const IQS7211E_GESTURE_SWIPE_HOLD_Y_NEGATIVE_BIT: u8 = 7; // 15

pub const FINGER_1: u8 = 1;
pub const FINGER_2: u8 = 2;
