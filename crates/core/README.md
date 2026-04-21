# quantedge-core

[![CI](https://github.com/dluksza/quantedge/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge/branch/main/graph/badge.svg?flag=quantedge-core)](https://codecov.io/gh/dluksza/quantedge?flags[0]=quantedge-core)
[![crates.io](https://img.shields.io/crates/v/quantedge-core.svg)](https://crates.io/crates/quantedge-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)
[![wasm](https://img.shields.io/badge/wasm-compatible-green.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)

Core types shared across the [quantedge](https://github.com/dluksza/quantedge) crates.

Defines the `Ohlcv` bar trait and its `Price` and `Timestamp` aliases so downstream crates can share a single bar abstraction without depending on the full indicator library.

## Features

### Bring your own data

Implement `Ohlcv` on your existing kline/candle type to feed it into any consumer crate without per-tick conversion. `volume()` has a default implementation for data sources that don't provide it.

### Zero dependencies

No runtime dependencies. Pure type definitions.

### WASM compatible

Compiles for `wasm32-unknown-unknown` and `wasm32-wasip1`. No filesystem or OS calls.

## Usage

```rust
use quantedge_core::{Ohlcv, Price, Timestamp};

struct MyKline {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    open_time: u64,
}

impl Ohlcv for MyKline {
    fn open(&self) -> Price { self.open }
    fn high(&self) -> Price { self.high }
    fn low(&self) -> Price { self.low }
    fn close(&self) -> Price { self.close }
    fn open_time(&self) -> Timestamp { self.open_time }
    // fn volume(&self) -> f64 { 0.0 }  -- default, override when volume is required
}
```

### Bar boundaries

Consumers detect new bars by comparing `open_time` values: the same timestamp updates (repaints) the current bar, a new timestamp advances the window. `open_time` must be non-decreasing across successive bars.

`Timestamp` is recommended to be microseconds since Unix epoch, monotonically increasing.

## Licence

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
