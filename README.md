# Azoteq IQS7211E driver

`iqs7211e` is a `no_std` async driver for the [Azoteq IQS7211E]
capacitive touch and gesture controller. It provides a strongly-typed API
for configuring the sensor over I²C, plus an awaited event API that yields
gestures and one/two-finger snapshots.

## Status

- ✅ Runs on `embedded-hal` and `embedded-hal-async` 1.0
- ✅ End-to-end configuration routine that mirrors the reference design
- ✅ Strongly typed configuration helpers (Rx/Tx pin layout, gesture toggles, ATI tuning)
- ⚠️ Breaking API changes may still occur prior to `1.0`

## Getting started

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
iqs7211e = "0.1.2"
```

Then initialise the device with an async I²C peripheral and a ready (RDY)
GPIO implementing `embedded_hal_async::digital::Wait`.

```rust
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::I2c;
use iqs7211e::{Config, Iqs7211e, Pinout, Pin};

async fn bring_up<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<Iqs7211e<I2C, RDY>, iqs7211e::Error<E>>
where
    I2C: I2c<embedded_hal_async::i2c::SevenBitAddress, Error = E>,
    RDY: Wait,
{
  let pinout = Pinout::new(
    [Pin::RxTx0, Pin::RxTx2, Pin::RxTx4],
    [Pin::Tx8, Pin::Tx9],
    [Pin::RxTx0],
    [Pin::Tx8],
  );
  let config = Config::default().with_pinout(pinout);

    let mut controller = Iqs7211e::new(i2c, rdy, config);
    _ = controller.initialize().await?;

    Ok(controller)
}
```

Refer to the [API documentation](https://docs.rs/iqs7211e) for the full listing of
types and helpers.

## Listening for touch events

Use `Iqs7211e::next_event()` to await gestures, single-touch, or multi-touch updates.
See runnable examples in `examples/`.

## Feature overview

- Full mirror of the Azoteq reference configuration sequence
- Strongly typed bitfield wrappers for gesture, ATI, and hardware settings
- Helpers for deriving valid sensing cycles from Rx/Tx pin layouts
- Manual setup session helper to read the counters documented in Azoteq's GUI workflow
- Convenience helpers to query firmware info, gesture bitfields, and
  per-finger touch snapshots
- No allocation, fits `no_std` targets

## Manual setup workflow

When replicating the "Basic Setup" procedure from Azoteq's reference
documentation you can let the crate act as a runtime setup assistant.

```rust
use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::I2c;
use iqs7211e::{Config, Iqs7211e, SetupSnapshot};

async fn tune<I2C, RDY, E>(i2c: I2C, rdy: RDY) -> Result<(), iqs7211e::Error<E>>
where
    I2C: I2c<embedded_hal_async::i2c::SevenBitAddress, Error = E>,
    RDY: Wait,
{
  let config = Config::default(); // set your pin layout, thresholds, etc. before the session starts

    let mut controller = Iqs7211e::new(i2c, rdy, config);
    let mut session = controller.begin_setup();

    session.initialize().await?;            // mirrors the GUI "Start streaming" + "Write changes"
    session.enter_manual_control().await?;  // enables manual control and forces LP1 charge mode

    let snapshot: SetupSnapshot = session.snapshot().await?;
    let channels = snapshot.rx_count * snapshot.tx_count;
    let deltas = &snapshot.trackpad_deltas[..channels];
    let bases = &snapshot.trackpad_base_targets[..channels];
    // Present the counters however you prefer (defmt logging, rtt, serial, ...)
    // e.g. `defmt::info!("{:?}", bases);`

    session.finish().await?;                // leaves stream/manual modes in a clean state

    // Apply the captured values to `controller.config`, then run `initialize()` again
    // or bake them into your production configuration.
    Ok(())
}
```

The `SetupSnapshot` structure exposes the live counters typically recorded
during bring-up:

- `info` mirrors the GUI indicator block (charge mode, ATI status, etc.)
- `trackpad_deltas` and `trackpad_base_targets` read back addresses `0xE200`
  and `0xE100` flattened to `rx_count * tx_count` entries
- `rx_count` / `tx_count` help you reshape the flattened arrays into your
  physical matrix layout
- `alp_*` fields expose the ALP channel counts and compensation values

You can capture multiple snapshots while manual control is active (for example
after tweaking thresholds) and feed the numbers into your own logging or GUI.

### Advanced setup workflow

The Azoteq documentation splits tuning into intermediate and advanced passes
that refine ATI, thresholds, and power behaviour. The driver mirrors those
steps so you can script the process instead of relying on the GUI.

- For the notes below let `channels = snapshot.rx_count * snapshot.tx_count`.

- **Rx/Tx sanity checks** – While manual control is enabled, lightly touch the
  corners and inspect `snapshot.info.tp_movement` together with
  `&snapshot.trackpad_deltas[..channels]`. If the active channels do not match the board layout,
  revise the pinout configuration and rerun `initialize()`.
- **ATI iteration** – Use `snapshot.trackpad_base_targets` to determine the
  base-target counts (doc section 4.2). Adjust the trackpad ATI settings via
  `config.auto_tune.tune` (adjust `target`, `coarse_divider`, `coarse_multiplier`,
  `fine_divider`) until the reported counts align with the lookup table. After updating
  the config values, call `initialize()` again to write them back and trigger a fresh ATI.
- **Compensation window** – The `alp_comp_*` fields in the snapshot reflect the compensation
  values. Keep them near the centre of the valid range (typically ~512). If the
  snapshot shows values drifting to 0 or 1023, adjust
  `config.auto_tune.tune.compensation_divider` and repeat the ATI step.
- **Threshold tuning** – Record the per-channel deltas while pressing the
  corners (`&snapshot.trackpad_deltas[..channels]`) and compute the touch thresholds suggested in
  doc section 4.3. Apply the resulting multiplier to
  `config.channel_output.touch` (set/clear multipliers) and reinitialise.
- **Mode timing** – Once the touch performance is acceptable, restore event
  mode via `SetupSession::finish()`, then update the report-rate and timeout
  fields in `config.timing` before the final `initialize()`. The defaults match
  the reference design, but you can tailor them to your power budget by following
  section 5 of the guide.

Repeat the capture → adjust → initialise loop until the recorded values match
your targets. Because the configuration structure lives in memory, you can
serialise it (e.g. JSON, TOML, or Rust constants) once the tuning is complete
and feed the same values into production firmware.
