# quantedge-core

[![CI](https://github.com/dluksza/quantedge/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge/branch/main/graph/badge.svg?flag=quantedge-core)](https://codecov.io/gh/dluksza/quantedge?flags[0]=quantedge-core)
[![crates.io](https://img.shields.io/crates/v/quantedge-core.svg)](https://crates.io/crates/quantedge-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)
[![wasm](https://img.shields.io/badge/wasm-compatible-green.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)

Core types shared across the [quantedge](https://github.com/dluksza/quantedge) crates.

Defines the `Ohlcv` bar struct, its `Price` and `Timestamp` aliases, and the `Timeframe` type for bar boundary alignment, so downstream crates can share a single bar abstraction without depending on the full indicator library.

## Features

### Plain-struct bar

`Ohlcv` is a `Copy` struct with six public fields (`open`, `high`, `low`, `close`, `open_time`, `volume`). Build one per kline, pass it by reference to consumer crates — no trait to implement, no generic parameters to thread through signatures. Volume-dependent indicators (OBV, VWAP) require a real `volume`; pass `0.0` when feeding indicators that ignore it.

### Bar timeframes

`Timeframe` maps a Unix-μs timestamp to bar boundaries: `open_time` returns the start of the containing period, `close_time` the last μs before the next period starts. Adjacent bars form a contiguous cover of the timeline (`close_time(t) + 1 == open_time` of the next period). Use `bounds(t)` to get both at once, it shares computation between the two halves, worth ~30% for monthly/yearly dispatch.

Handles second-through-year units with calendar-correct month and year arithmetic, using Howard Hinnant's [`civil_from_days`](https://howardhinnant.github.io/date_algorithms.html) — no `chrono` at runtime. Multi-month periods are epoch-anchored from January 1970, matching calendar quarters (`MONTH_3`) and halves (`MONTH_6`) for any N dividing 12.

### Zero runtime dependencies

Pure Rust, no runtime deps.

### WASM compatible

Compiles for `wasm32-unknown-unknown` and `wasm32-wasip1`. No filesystem or OS calls.

## Usage

### Ohlcv

```rust
use quantedge_core::Ohlcv;

let bar = Ohlcv {
    open: 10.0,
    high: 12.0,
    low: 9.0,
    close: 11.0,
    volume: 100.0,
    open_time: 1_745_798_400_000_000,
};
```

Converting from your own kline type is a field-wise copy:

```rust
use quantedge_core::Ohlcv;

struct MyKline { o: f64, h: f64, l: f64, c: f64, v: f64, t: u64 }

impl From<&MyKline> for Ohlcv {
    fn from(k: &MyKline) -> Self {
        Ohlcv {
            open: k.o,
            high: k.h,
            low: k.l,
            close: k.c,
            volume: k.v,
            open_time: k.t,
        }
    }
}
```

### Timeframe

```rust
use quantedge_core::Timeframe;

let ts = 1_745_798_730_123_000; // Mon Apr 28 2025 00:05:30.123 UTC

let (open, close) = Timeframe::HOUR_1.bounds(ts);
// open  == 1_745_798_400_000_000  (00:00:00)
// close == 1_745_801_999_999_999  (00:59:59.999999)

// Constants for common bar sizes: SEC_*, MIN_*, HOUR_*, DAY_*,
// WEEK_1, MONTH_{1,2,3,6}, YEAR_1. Build any (count, unit) pair
// via `Timeframe::new(NonZero::new(n).unwrap(), TimeUnit::X)`;
// canonicalizes `60s → 1 minute`, `7d → 1 week`, etc.
```

### Bar boundaries

Consumers detect new bars by comparing `open_time` values: the same timestamp updates (repaints) the current bar, a new timestamp advances the window. `open_time` must be non-decreasing across successive bars.

`Timestamp` is recommended to be microseconds since Unix epoch, monotonically increasing.

## Licence

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
