# IQS7211E Driver notes

This trimmed datasheet captures only what the crate needs to talk to the device: I2C protocol basics, register map, and bitfield definitions. For electrical, mechanical, and layout guidance, consult the official PDF in `docs/`.

## Quick reference

- I2C address: 0x56
- RDY pin: open-drain active-low "communication window." Wait for a falling edge, complete one or more transfers, then STOP to end the window. Optional comms end by writing any byte(s) to 0xFF then STOP if enabled.
- Addressing/data: Most registers use 8-bit addresses with 16-bit little-endian data words (two bytes per address). Diagnostic "extended" blocks use 16-bit addresses starting at 0xE000.
- Modes: Active, Idle-Touch, Idle, LP1, LP2. Mode is reported in Info Flags [2:0]. In manual control, select via System Control [2:0].
- Event vs Stream: Event mode asserts RDY only when enabled events occur (gestures, trackpad movement/touch, ALP, re-ATI). Event mode requires clearing Show Reset once after a reset (Ack Reset = 1).
- Not I3C compatible (clock stretching used).

## I2C interface details

- fSCL up to 1 MHz (Fast-mode Plus).
- Words are little-endian. Multi-bitfields are packed as documented below.
- RDY timeout: if not serviced, RDY deasserts and processing continues; data may be missed.

## Operating modes (names mapping)

- 000 Active, 001 Idle-Touch, 010 Idle, 011 LP1, 100 LP2.
- Report rates per mode and timeouts are configured via the memory map (Active/Idle-Touch/Idle rates at 0x30–0x32; LP behavior via hardware/ALP settings).

## 12 I2C Memory Map - Register Descriptions

Refer to Appendix A for bitfield definitions. Summary of address ranges:

- 0x00–0x09: Version details (RO)
- 0x0A–0x17: XY/Gesture/Finger metrics (RO)
- 0x18–0x1E: Touch status bitmaps and ALP counts/LTA (RO)
- 0x1F–0x27: ATI settings (R/W): ALP comps, TP/ALP multipliers, targets, drift limits
- 0x28–0x32: Report rates and timeouts (R/W)
- 0x33–0x37: System & ALP setup (R/W)
- 0x38–0x3C: Trackpad/ALP thresholds and LP filter betas (R/W)
- 0x3D–0x40: Channel setup and hardware (R/W)
- 0x41–0x49: Trackpad setup and trims (R/W)
- 0x4A: Settings version (R/W)
- 0x4B–0x55: Gesture enables/parameters (R/W)
- 0x56–0x5C: Rx/Tx mapping (R/W)
- 0x5D–0x7C: Cycle channel mapping (R/W)
- 0xE000–: Extended RO blocks (counts, references, deltas, ATI compensation)

### 12.1 Register map (addresses and names)

Exact 1:1 register list from the datasheet. Data is 16-bit little-endian unless noted. “(HIGH byte)/(LOW byte)” indicates the byte-level meaning of the 16-bit word at that address.

| Address   | Name / Description | Notes |
| --------- | ------------------- | ----- |
| 0x00–0x09 | Version details (RO) | See Table A.1 |
| 0x0A | Relative X (RO) | See Section 7.2.2 |
| 0x0B | Relative Y (RO) | See Section 7.2.2 |
| 0x0C | Gesture X (RO) | See Sections 8.1, 8.3 |
| 0x0D | Gesture Y (RO) | See Sections 8.1, 8.3 |
| 0x0E | Gestures (RO) | See Table A.2 |
| 0x0F | Info flags (RO) | See Table A.3 |
| 0x10 | Finger 1 X-coordinate (RO) | See Section 7.2.3 |
| 0x11 | Finger 1 Y-coordinate (RO) | See Section 7.2.3 |
| 0x12 | Finger 1 touch strength (RO) | See Section 7.2.4 |
| 0x13 | Finger 1 area (RO) | See Section 7.2.5 |
| 0x14 | Finger 2 X-coordinate (RO) | See Section 7.2.3 |
| 0x15 | Finger 2 Y-coordinate (RO) | See Section 7.2.3 |
| 0x16 | Finger 2 touch strength (RO) | See Section 7.2.4 |
| 0x17 | Finger 2 area (RO) | See Section 7.2.5 |
| 0x18 | Touch status ‹0› (RO) | See Table A.4 |
| 0x19 | Touch status ‹1› (RO) | See Table A.4 |
| 0x1A | Touch status ‹2› (RO) | See Table A.4 |
| 0x1B | ALP channel count (RO) | See Section 5.3.2 |
| 0x1C | ALP channel LTA (RO) | See Section 5.4.2 |
| 0x1D | ALP count A (RO) | See Section 5.3.2 |
| 0x1E | ALP count B (RO) | See Section 5.3.2 |
| — | End of Read Only Section |  |
| 0x1F | ALP ATI compensation A | See Section 5.6.3 |
| 0x20 | ALP ATI compensation B | See Section 5.6.3 |
| 0x21 | Trackpad ATI multiplier/dividers (Global) | See Table A.5 |
| 0x22 | Trackpad reference drift limit | See Sections 5.7.2, 5.6.4 |
| 0x23 | Trackpad ATI target | See Section 5.6.3 |
| 0x24 | Trackpad minimum count re-ATI value | See Section 5.7.2 |
| 0x25 | ALP ATI multiplier/dividers | See Table A.5 |
| 0x26 | ALP LTA drift limit | See Section 5.7.2 |
| 0x27 | ALP ATI target | See Section 5.6.3 |
| 0x28 | Active mode report rate (ms) | See Section 6.1 |
| 0x29 | Idle-Touch mode report rate (ms) | See Section 6.1 |
| 0x2A | Idle mode report rate (ms) | See Section 6.1 |
| 0x2B | LP1 mode report rate (ms) | See Section 6.1 |
| 0x2C | LP2 mode report rate (ms) | See Section 6.1 |
| 0x2D | Active mode timeout (s) | See Section 6.2 |
| 0x2E | Idle-Touch mode timeout (s) | See Section 6.2 |
| 0x2F | Idle mode timeout (s) | See Section 6.2 |
| 0x30 | LP1 mode timeout (s) | See Section 6.2 |
| 0x31 | (HIGH byte) Reference update time (s); (LOW byte) Re-ATI retry time (s) | See Sections 5.4.1, 5.7.3 |
| 0x32 | I2C timeout (ms) | See Section 11.6 |
| 0x33 | System control | See Table A.6 |
| 0x34 | Config settings | See Table A.7 |
| 0x35 | Other settings | See Table A.8 |
| 0x36 | ALP setup | See Table A.9 |
| 0x37 | ALP Tx enable | See Table A.10 |
| 0x38 | (HIGH byte) Touch clear multiplier; (LOW byte) Touch set multiplier | See Section 5.5.1 |
| 0x39 | ALP threshold | See Section 5.5.2 |
| 0x3A | (HIGH byte) ALP clear debounce; (LOW byte) ALP set debounce | See Section 5.5.3 |
| 0x3B | (HIGH byte) ALP LTA filter beta – LP1 mode; (LOW byte) ALP count filter beta – LP1 mode | See Sections 5.3.2, 5.4.2 |
| 0x3C | (HIGH byte) ALP LTA filter beta – LP2 mode; (LOW byte) ALP count filter beta – LP2 mode | See Sections 5.3.2, 5.4.2 |
| 0x3D | Trackpad conversion frequency | See Table A.11 |
| 0x3E | ALP conversion frequency | See Table A.11 |
| 0x3F | Trackpad hardware settings | See Table A.12 |
| 0x40 | ALP hardware settings | See Table A.12 |
| 0x41 | Trackpad settings (HIGH byte: Total Rxs; LOW byte: filter/axis/flip bits) | See Table A.13 |
| 0x42 | (HIGH byte) Max multi-touches; (LOW byte) Total Txs | See Sections 7.1.1, 7.3 |
| 0x43 | X resolution | See Section 7.4 |
| 0x44 | Y resolution | See Section 7.4 |
| 0x45 | XY dynamic filter – bottom speed | See Section 7.8.1 |
| 0x46 | XY dynamic filter – top speed | See Section 7.8.1 |
| 0x47 | Static filter beta value | See Section 7.8.2 |
| 0x48 | (HIGH byte) Finger split factor; (LOW byte) Stationary touch movement threshold | See Sections 7.6, 7.5 |
| 0x49 | (HIGH byte) Y trim; (LOW byte) X trim | See Section 7.9 |
| 0x4A | (HIGH byte) Settings major version; (LOW byte) Settings minor version | See Section 10.1.1 |
| 0x4B | Gesture enable | See Table A.14 |
| 0x4C | Tap time (ms) | See Section 8.1 |
| 0x4D | Air time (ms) | See Section 8.1 |
| 0x4E | Tap distance (pixels) | See Section 8.1 |
| 0x4F | Hold time (ms) | See Section 8.2 |
| 0x50 | Swipe time (ms) | See Section 8.3.1 |
| 0x51 | Swipe initial x-distance (pixels) | See Section 8.3.1 |
| 0x52 | Swipe initial y-distance (pixels) | See Section 8.3.1 |
| 0x53 | Swipe consecutive x-distance (pixels) | See Section 8.3.3 |
| 0x54 | Swipe consecutive y-distance (pixels) | See Section 8.3.3 |
| 0x55 | (HIGH byte) Palm threshold; (LOW byte) Swipe angle (64·tan(deg)) | See Sections 8.3, 8.4 |
| 0x56 | RxTx mapping ‹1..0› | See Section 7.1.5 |
| 0x57 | RxTx mapping ‹3..2› | See Section 7.1.5 |
| 0x58 | RxTx mapping ‹5..4› | See Section 7.1.5 |
| 0x59 | RxTx mapping ‹7..6› | See Section 7.1.5 |
| 0x5A | RxTx mapping ‹9..8› | See Section 7.1.5 |
| 0x5B | RxTx mapping ‹11..10› | See Section 7.1.5 |
| 0x5C | (HIGH byte) Reserved (do not use); (LOW byte) RxTx mapping ‹12› | See Section 7.1.5 |
| 0x5D–0x7C | Cycle channel mapping (for cycles 0..20). HIGH byte = ProxA channel idx; LOW byte = ProxB channel idx | See Section 7.1.2 |
| — | End of Read/Write Section |  |
| — | Begin of Read Only Section (extended) |  |
| 0xE0i | Trackpad count values (extended) (RO) | See Section 5.3.1 |
| 0xE1i | Trackpad reference values (extended) (RO) | See Section 5.4.1 |
| 0xE2i | Trackpad delta values (extended) (RO) | See Section 5.3.4 |
| 0xE3i | Trackpad ATI compensation values (extended) (RO) | See Section 5.6.3 |

Notes:

- i: Extended 16-bit memory map page. When using full 16-bit addressing, the base addresses are 0xE000, 0xE100, 0xE200, 0xE300 respectively (little-endian data).
- For 0x5D–0x7C, each address maps a cycle index; the high byte selects the ProxSense engine A channel and the low byte selects engine B for that cycle.
- Datasheet notes that address 0x7C must use high byte `0x01` (not `0x05`) when mapping cycle 20.

---

## Notes for referenced sections

Below are concise, self-contained notes for each “See Section X” reference used above.

### 5.3.1 Trackpad Count Values

- Each mutual-capacitive channel reports an unfiltered count value per scan. Counts are inversely proportional to capacitance. The signed delta is defined as: $\Delta = \text{Count} - \text{Reference}$.

### 5.3.2 ALP Count Values

- If both Rx engines A and B are used for ALP, their counts (A/B) are summed to form the ALP channel count.
- An ALP count filter can be enabled. Filter strength is set by a beta; the damping factor is $(8\cdot \text{Beta} - 7)/2048$. Smaller beta = stronger filtering.

### 5.3.4 Trackpad Delta Value

- Defines the per-channel signed difference between Count and Reference: $\Delta = \text{Count} - \text{Reference}$; used throughout for touch and drift logic.

### 5.4.1 Trackpad References

- Trackpad references are snapshot copies of counts taken with no user activity (equivalent to reseed). In automatic mode they update from LP1/LP2; in manual control, the host must reseed when appropriate.

### 5.4.2 ALP Long-Term Average (LTA)

- ALP uses a filtered LTA instead of snapshots. Separate LTA betas for LP1 and LP2 control tracking speed; smaller beta filters more strongly.

### 5.5.1 Trackpad Touch Output

- Touch sets when Count exceeds Reference by a fractional multiplier: Threshold = Reference × (1 + Multiplier/128). Separate set/clear multipliers provide hysteresis.

### 5.5.2 ALP Output

- ALP output asserts when the absolute delta to the LTA exceeds the configured ALP threshold (proximity/touch per chosen threshold).

### 5.5.3 Output Debounce

- No debounce on trackpad touch to preserve responsiveness. ALP has set/clear debounce counters to stabilize proximity.

### 5.6.3 ATI Compensation (and Target)

- Per-channel 10-bit compensation values are chosen by the ATI routine to achieve the selected ATI target. ALP has separate A/B compensation. Re-ATI is queued by control bits and runs after the comm window, temporarily blocking further I2C until completion.

### 5.6.4 ATI Divider

- Scales the effective size/range of compensation steps. Smaller divider → larger step size (coarser, wider range). Larger divider → finer steps (narrower range).

### 5.7.2 Conditions for Re-ATI to activate

- Reference/LTA drift beyond ATI target ± drift limit triggers re-ATI.
- A persistent abnormal decrease in a trackpad count (beyond the minimum count threshold for 15 cycles) also triggers re-ATI.
- A status flag indicates when a re-ATI just occurred.

### 6.1 Report Rate

- Per-mode report rates (Active, Idle-Touch, Idle, LP1, LP2) define how often the device reports/updates. Lower rates reduce power; LP modes can optionally run auto-prox cycles between reports.

### 6.2 Mode Timeout

- Per-mode timeouts (in seconds) select how long the device remains in a higher-power mode after activity before stepping down to a lower-power mode.

### 7.1.1 Size Selection

- Choose total Rxs and Txs (up to 42 channels total) to match the touch pattern and product size. Resolution is configured independently.

### 7.1.2 Cycle Setup

- Channels are assigned into sensing cycles (timeslots). For each cycle, the high byte selects the Prox engine A channel, and the low byte selects engine B, defining the scan order.

### 7.1.5 Rx/Tx Mapping

- Mapping registers define the logical order/assignment of physical RX/TX pads to trackpad rows/columns, allowing layout flexibility.

### 7.2.2 Relative XY

- Relative X/Y are per-cycle motion deltas. They are cleared during tap detection to avoid jitter (gesture engine behavior).

### 7.2.3 Absolute XY

- High-resolution absolute coordinates for up to two fingers are reported in the configured X/Y resolution space.

### 7.2.4 Touch Strength

- Per-finger strength metric (integrated channel response) indicating press intensity.

### 7.2.5 Area

- Per-finger area estimate (approximate contact patch) to help with gesture robustness and palm detection.

### 7.3 Maximum Number of Multi-touches

- The device tracks up to two fingers. “Too Many Fingers” flag indicates when more touches are present than allowed.

### 7.4 XY Resolution

- X and Y output resolutions are configured separately. Edge trimming can extend the usable range to reach exact 0 and max values.

### 7.5 Stationary Touch

- Movement below a threshold is considered stationary. Used by the engine to suppress motion events and as part of gesture logic.

### 7.6 Multi-touch Finger Split

- Finger split factor controls when a blob is treated as one vs two fingers; affects switching between single- and multi-touch tracking.

### 7.8.1 MAV Filter

- Enables moving-average smoothing on XY outputs. Recommended for stable cursoring with minimal latency impact.

### 7.8.2 IIR Filter

- IIR output filtering can be dynamic (damping varies with speed between bottom/top speed bounds) or static (fixed beta). Static beta is configured separately.

### 7.9 X & Y Trim

- Trims edge dead zones so that the full configured range (0..Xmax, 0..Ymax) is achievable on the sensor surface.

### 8.1 Single/Double/Triple Tap

- Tap gestures require a press-and-release within Tap time and within Tap distance of the start point. Air time gates multi-tap recognition. Relative XY is temporarily zeroed to avoid cursor movement.

### 8.2 Press-and-Hold

- If the touch remains within Tap distance longer than Hold time, a press-and-hold is reported and remains asserted until release. Relative XY is suppressed until the hold is established.

### 8.3.1 Single Swipe

- Swipe triggers when the initial axis distance is exceeded within Swipe time and the path angle is within the Swipe angle threshold. Gesture X/Y report the overall movement vector; Swipe angle uses $64\,\tan(\theta)$ encoding.

### 8.3 Swipe Gesture (overview)

- On-axis motion detection with time and angle constraints; supports single swipe, consecutive swipes (distance-gated), and swipe-and-hold (hold-time-gated) variants.

### 8.3.3 Consecutive Swipe

- After the initial swipe, additional swipe events can emit each time the consecutive distance is exceeded (angle constraint applies). No time limit for consecutive swipes; switching axis generally requires meeting the initial distance for that axis.

### 8.4 Palm Gesture

- Palm is asserted when at least Palm threshold channels simultaneously report touch. Requires a full release (no touches) to clear.

### 10.1.1 Automated Start-up

- Devices can be pre-programmed with customer-specific settings and a settings version. On power-up they auto-run without host setup; otherwise the host should write all settings and trigger re-ATI.

### 11.6 I2C Timeout

- If a comm window isn’t serviced within the I2C timeout, RDY deasserts and processing continues; the missed data is lost but references keep updating.

---

## Appendix A: Memory Map Descriptions

- Version Information: Product number 1112, FW/library versions, commit hashes
- Gestures (0x0E): per-bit gesture occurrence flags
- Info Flags (0x0F): mode, fingers, movement, reset, re-ATI occurred, ATI errors, ALP output
- Touch Status (0x18–0x1A): CH0..CH41 touch flags
- Multipliers/Dividers (0x21/0x25): fine fractional divider, coarse multiplier/divider
- System Control (0x33): SW reset, suspend, Ack Reset, TP/ALP re-ATI/reseed, mode select
- Config Settings (0x34): Event mode, manual control, events enable, WDT, comms end/request, auto re-ATI
- Other Settings (0x35): 14/18 MHz, oscillator adjust
- ALP Setup/Tx Enable (0x36/0x37): count filter, sensing method, Rx/Tx enables
- Conversion Frequency (0x3D/0x3E): period/fraction mapping to Fxfer
- Hardware Settings (0x3F/0x40): RF filters, CS cap select, op-amp bias, max count, auto-prox cycles, init delay
- Trackpad Settings (0x41): Total Rxs, filters (MAV/IIR), axis switch/flip
- Gesture Enable (0x4B): enable bits for gestures

### Table A.1: Version Information (0x00–0x09, RO)

| Field                       | Value      |
| --------------------------- | ---------- |
| Product Number              | 1112       |
| App Major Version           | 1          |
| App Minor Version           | 1          |
| App Patch (commit hash)     | 0x59C2C977 |
| Library Number              | 206        |
| Library Major Version       | 4          |
| Library Minor Version       | 15         |
| Library Patch (commit hash) | 0x16688405 |

### Table A.2: Gestures (0x0E, RO)

Each bit indicates a one-cycle gesture occurrence.

| Bit | Name              | Description |
| --: | ----------------- | ----------- |
|  15 | Swipe and Hold Y− | Occurred    |
|  14 | Swipe and Hold Y+ | Occurred    |
|  13 | Swipe and Hold X− | Occurred    |
|  12 | Swipe and Hold X+ | Occurred    |
|  11 | Swipe Y−          | Occurred    |
|  10 | Swipe Y+          | Occurred    |
|   9 | Swipe X−          | Occurred    |
|   8 | Swipe X+          | Occurred    |
| 7–5 | —                 | Reserved    |
|   4 | Palm Gesture      | Occurred    |
|   3 | Press-and-Hold    | Occurred    |
|   2 | Triple Tap        | Occurred    |
|   1 | Double Tap        | Occurred    |
|   0 | Single Tap        | Occurred    |

### Table A.3: Info Flags (0x0F, RO)

| Bit | Name                | Description / Values                                   |
| --: | ------------------- | ------------------------------------------------------ |
|  15 | —                   | Reserved                                               |
|  14 | ALP Output          | 1 = ALP proximity/touch detected                       |
|  13 | —                   | Reserved                                               |
|  12 | Too Many Fingers    | 1 = More than allowed fingers detected                 |
|  11 | —                   | Reserved                                               |
|  10 | TP Movement         | 1 = Movement detected on trackpad                      |
| 9–8 | No of Fingers       | 00 = 0, 01 = 1, 10 = 2                                 |
|   7 | Show Reset          | 1 = Reset occurred (needs Ack Reset)                   |
|   6 | ALP Re-ATI Occurred | 1 = Re-ATI just completed (ALP)                        |
|   5 | ALP ATI Error       | 1 = Most recent ALP ATI unsuccessful                   |
|   4 | Re-ATI Occurred     | 1 = Re-ATI just completed (TP)                         |
|   3 | ATI Error           | 1 = Most recent TP ATI unsuccessful                    |
| 2–0 | Charging Mode       | 000 Active, 001 Idle-Touch, 010 Idle, 011 LP1, 100 LP2 |

### Table A.4: Touch Status (Z) (0x18–0x1A, RO)

Three 16-bit words covering channels CH0–CH41.

| Address | Bits  | Channels                       |
| ------- | ----- | ------------------------------ |
| 0x18    | 15..0 | CH15..CH0                      |
| 0x19    | 15..0 | CH31..CH16                     |
| 0x1A    | 9..0  | CH41..CH32 (upper bits unused) |

Bit value: 1 = touch detected on channel.

### Table A.5: Trackpad/ALP Multipliers & Dividers (e.g., 0x21 / 0x25)

| Field                   | Bits  | Description  |
| ----------------------- | ----- | ------------ |
| Reserved                | 15–14 | —            |
| Fine Fractional Divider | 13–9  | 5-bit (1–31) |
| Coarse Multiplier       | 8–5   | 4-bit (1–15) |
| Coarse Fractional Divider | 4–0 | 5-bit (1–31) |

### Table A.6: System Control (0x33)

|   Bit | Name        | Effect                                                                    |
| ----: | ----------- | ------------------------------------------------------------------------- |
|    15 | Tx test     | 1 = Enable Tx short test config                                           |
| 14–12 | —           | Reserved                                                                  |
|    11 | Suspend     | 1 = Enter suspend after comm window                                       |
|    10 | —           | Reserved                                                                  |
|     9 | SW Reset    | 1 = Reset after comm window                                               |
|     8 | —           | Reserved                                                                  |
|     7 | Ack Reset   | 1 = Clear Show Reset flag                                                 |
|     6 | ALP Re-ATI  | Queue ALP re-ATI                                                          |
|     5 | TP Re-ATI   | Queue TP re-ATI                                                           |
|     4 | ALP Reseed  | Reseed ALP LTA                                                            |
|     3 | TP Reseed   | Reseed TP references                                                      |
|   2–0 | Mode Select | 000 Active, 001 Idle-Touch, 010 Idle, 011 LP1, 100 LP2 (manual mode only) |

### Table A.7: Config Settings (0x34)

| Bit | Name             | Effect                                        |
| --: | ---------------- | --------------------------------------------- |
|  15 | —                | Reserved                                      |
|  14 | TP Touch Event   | 1 = Touch status change triggers event        |
|  13 | ALP Event        | 1 = ALP prox/touch change triggers event      |
|  12 | —                | Reserved                                      |
|  11 | Re-ATI Event     | 1 = Re-ATI triggers event                     |
|  10 | TP Event         | 1 = Movement or finger up/down triggers event |
|   9 | Gesture Event    | 1 = Gesture triggers event                    |
|   8 | Event Mode       | 1 = Only on events (Show Reset cleared)       |
|   7 | Manual Control   | 1 = Host controls modes                       |
|   6 | Comms End Cmd    | 1 = End comms by write to 0xFF + STOP         |
|   5 | WDT              | 1 = Watchdog enabled                          |
|   4 | Comms Request EN | 1 = Request window (no clock stretching)      |
|   3 | ALP Re-ATI EN    | 1 = Auto re-ATI on ALP                        |
|   2 | TP Re-ATI EN     | 1 = Auto re-ATI on TP                         |
| 1–0 | —                | Reserved                                      |

### Table A.8: Other Settings (0x35)

| Field        | Bits | Description                  |
| ------------ | ---- | ---------------------------- |
| —            | 15–5 | Reserved                     |
| 14MHz/18MHz  | 4    | 0 = 14 MHz, 1 = 18 MHz       |
| Main Osc Adj | 3–0  | 0..15 (0 = no adjustment, 15 = maximum adjustment) |

### Table A.9: ALP Setup (0x36)

|   Bit | Name               | Description                   |
| ----: | ------------------ | ----------------------------- |
| 15–10 | —                  | Reserved                      |
|     9 | ALP Count Filter   | 1 = Enable ALP count filter   |
|     8 | ALP Sensing Method | 0 = Self-cap, 1 = Mutual-cap  |
|   7–0 | RX_EN[7:0]         | 1 = Rx included in ALP sensor |

### Table A.10: ALP Tx Enable (0x37)

|   Bit | Name        | Description                   |
| ----: | ----------- | ----------------------------- |
| 15–13 | —           | Reserved                      |
|  12–0 | TX_EN[12:0] | 1 = Tx included in ALP sensor |

### Table A.11: Conversion Frequency (0x3D / 0x3E)

| Field              | Bits | Description                |
| ------------------ | ---- | -------------------------- |
| Frequency Fraction | 15–8 | 256 × fconv / fclk (0–255) |
| Conversion Period  | 7–0  | 128 / Frequency Fraction − 2 (0–127) |

Notes: With Fraction = 127, Period values map to fxfer: 1→2 MHz, 5→1 MHz, 12→500 kHz, 17→350 kHz, 26→250 kHz, 53→125 kHz.

### Table A.12: Trackpad and ALP Hardware Settings (0x3F / 0x40)

| Field                | Bits  | Values / Description                            |
| -------------------- | ----- | ----------------------------------------------- |
| NM In Static         | 15    | 1 = Enabled (recommended)                       |
| CS_0v5 Discharge     | 14    | 0 = to 0 V (recommended), 1 = to 0.5 V          |
| RF Filter            | 13    | 1 = Enable internal RF filters                  |
| CS Cap Select        | 12    | 0 = 40 pF, 1 = 80 pF (recommended)              |
| Opamp Bias           | 11–10 | 00: 2 µA, 01: 5 µA, 10: 7 µA, 11: 10 µA         |
| Max Count            | 9–8   | 00: 1023, 01: 2047, 10: 4095, 11: 16384         |
| LP2 Auto Prox Cycles | 7–5   | 000: 4, 001: 8, 010: 16, 011: 32, 1xx: Disabled |
| LP1 Auto Prox Cycles | 4–2   | 000: 4, 001: 8, 010: 16, 011: 32, 1xx: Disabled |
| Init Delay           | 1–0   | 00: 4, 01: 16, 10: 32, 11: 64                   |

### Table A.13: Trackpad Settings (0x41)

|  Bit | Name           | Description                                  |
| ---: | -------------- | -------------------------------------------- |
| 15–8 | Total Rxs      | Number of Rxs used                           |
|  7–6 | —              | Reserved                                     |
|    5 | MAV Filter     | 1 = Enable XY moving average                 |
|    4 | IIR Static     | 0 = Dynamic damping (recommended), 1 = Fixed |
|    3 | IIR Filter     | 1 = Enable XY IIR                            |
|    2 | Switch XY Axis | 1 = Swap Rx/Tx axis assignment               |
|    1 | Flip Y         | 1 = Invert Y output                          |
|    0 | Flip X         | 1 = Invert X output                          |

### Table A.14: Gesture Enable (0x4B)

| Bit | Name              | Description |
| --: | ----------------- | ----------- |
|  15 | Swipe and Hold Y− | 1 = Enable  |
|  14 | Swipe and Hold Y+ | 1 = Enable  |
|  13 | Swipe and Hold X− | 1 = Enable  |
|  12 | Swipe and Hold X+ | 1 = Enable  |
|  11 | Swipe Y−          | 1 = Enable  |
|  10 | Swipe Y+          | 1 = Enable  |
|   9 | Swipe X−          | 1 = Enable  |
|   8 | Swipe X+          | 1 = Enable  |
| 7–5 | —                 | Reserved    |
|   4 | Palm Gesture      | 1 = Enable  |
|   3 | Press-and-Hold    | 1 = Enable  |
|   2 | Triple Tap        | 1 = Enable  |
|   1 | Double Tap        | 1 = Enable  |
|   0 | Single Tap        | 1 = Enable  |

---

This document is a driver-focused extract. For full device specifications (electrical, timing, PCB layout, packaging, ordering), see `docs/iqs7211e_datasheet.pdf`.
