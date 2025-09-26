# Copilot instructions for this repo

Purpose: no_std async Rust driver for the Azoteq IQS7211E capacitive touch/gesture controller. Targets embedded-hal-async 1.0 and provides a typed API for config, tuning, and events.

## Quick facts

- Crate entry: `src/lib.rs` defines `Iqs7211e<I2C, RDY>` and re-exports modules.
- I²C addr: 0x56. RDY pin uses `embedded_hal_async::digital::Wait` to gate comm windows.
- Async only; `no_std`.
- Optional feature `touchpad` exposes a high-level event façade.

## Key modules and roles

- `src/defs.rs` – Register map (`Reg`), constants (`PRODUCT_NUMBER`), enums (`ChargeMode`, `InterruptMode`), and packed bitfields (`InfoFlags`, `SysControl`, `ConfigSettings`). Uses `packbits` derive.
- `src/config.rs` – Staged configuration model written during init: `PinMapping` (Rx/Tx/ALP pins + cycle generation), `GestureMask`, `TrackpadAutoTuning`, `LowPowerAutoTuning`, `LowPowerCompensation`, `ReportRates`, `ModeTimeouts`, `MaintenanceTimers`.
- `src/control.rs` – Typed getters/setters around `Reg` (e.g., `app_version`, `info_flags`, `set_charge_mode`, `set_manual_control`, `set_interrupt_mode`).
- `src/event.rs` – Raw event layer: `Report { gesture, info, fingers }`, `Finger` packed layout, `Gesture` decoding; `Iqs7211e::read_report()` that also manages init/reset state.
- `src/setup.rs` – Bring-up assistant: `SetupSession` flips to stream mode, disables auto-prox cycles, forces LP1, snapshots counters from 0xE100/0xE200 (extended reads).
- `src/touchpad.rs` (feature `touchpad`) – High-level façade: `Frame`, `Changes`, `State`, `Touch`, `SwipeDirection`, utilities and a small `stream()` helper.

## I/O and protocol patterns (important!)

- **RDY window management**: Call `wait_for_comm_window()` once before a **sequence** of register operations, not before each individual I²C transaction. Multiple reads/writes can occur within a single RDY window. The low-level `read_bytes`/`write_bytes` helpers do NOT wait for RDY—that's the caller's responsibility. See Arduino code for reference: RDY is checked once at the start of higher-level operations.
- Addressing: regular regs are 8-bit; diagnostics use 16-bit "extended" reads (`read_u16_ext`, `read_ext_bytes`).
- Write size limit: `write_bytes` allows max 31 data bytes (+1 reg) → otherwise `Error::BufferOverflow`.
- Typed I/O: prefer `read<const N, T: TryFrom<[u8; N]>>()` and `write<const N, T: TryInto<[u8; N]>>()` with `packbits` types over manual buffers.

## Initialization flow (mirror this ordering)

1. Validate product ID (`app_version().number == PRODUCT_NUMBER`).
2. If needed, `software_reset()` and handle reset indicator (`InfoFlags.show_reset`).
3. Push staged config (`write_config(self.config)` inside `initialize()`), `acknowledge_reset()`.
4. Trigger trackpad retune and poll until `InfoFlags.re_auto_tuning_occurred`.
5. Set final interrupt mode from config.

Manual tuning: `begin_setup() -> SetupSession` then `initialize().await`, `enter_manual_control().await`, `snapshot().await` (reads 0xE100/0xE200 and ALP counters), `finish().await` to restore modes.

## Developer workflows

- Build: `cargo build` (MSRV 1.75).
- Tests: `cargo test` (core), `cargo test --features touchpad` (include façade tests). No hardware required for unit tests.
- Docs: `cargo doc --no-deps --all-features`.

## Adding a new register/field (recipe)

1. Add address to `Reg` in `defs.rs` (keep group/comments by datasheet section).
2. Define/extend a `#[packbits::pack(...)]` type matching on-wire layout.
3. Add typed accessors in `control.rs` or on `Iqs7211e` using generic `read`/`write` helpers and the RDY gating.
4. If staged config, thread through `config.rs` and ensure `initialize()` writes it at the correct step.

## Integration and references

- Traits: `embedded_hal_async::i2c::I2c<SevenBitAddress>` and `embedded_hal_async::digital::Wait`.
- Datasheets and vendor example live under `docs/` (PDFs and Arduino code). Use to verify bitfields and tuning steps.
- README shows usage; if names drift (e.g., mapping types), treat code as source of truth and adjust README in the same change.
