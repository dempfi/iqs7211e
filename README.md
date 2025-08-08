# Rust [Azoteq Iqs7211e](https://www.google.com/search?client=safari&rls=en&q=Iqs7211e&ie=UTF-8&oe=UTF-8) crate

A `no_std` driver for the [Azoteq IQS7211E](https://www.azoteq.com/products/proximity-sensors/iqs7211a-iqs7211b-iqs7211c-iqs7211d-iqs7211e/) touch and trackpad controller.

The library is built on top of [`embedded-hal`](https://docs.rs/embedded-hal) and
[`embedded-hal-async`](https://docs.rs/embedded-hal-async) traits and therefore
should work with any hardware abstraction layer that implements these traits.

## Features

- Async I²C communication using `embedded-hal-async`
- Configurable sensor settings via the `Config` structure
- Helpers for managing the device's ready pin and communication window

## Usage

Add the crate to your `Cargo.toml` and create an instance of `Iqs7211e`:

```rust
use iqs7211e::{Iqs7211e, config::Config};

// `i2c` and `rdy` should be provided by your HAL implementation
# async fn example<I2C, RdyPin>(i2c: I2C, rdy: RdyPin)
# where
#     I2C: embedded_hal::i2c::I2c<embedded_hal::i2c::SevenBitAddress>,
#     RdyPin: embedded_hal_async::digital::Wait,
# {
    let config = Config::default();
    let mut device = Iqs7211e::new(i2c, rdy, config);
    // initialise the controller and start using it...
# }
```

## Documentation

Additional resources such as application notes and example code can be found in
the [`docs/`](docs) directory.

## License

Licensed under either the MIT or Apache-2.0 license at your option.
