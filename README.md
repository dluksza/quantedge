# quantedge-ta

[![CI](https://github.com/dluksza/quantedge-ta/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge-ta/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge-ta/branch/main/graph/badge.svg)](https://codecov.io/gh/dluksza/quantedge-ta)
[![crates.io](https://img.shields.io/crates/v/quantedge-ta.svg)](https://crates.io/crates/quantedge-ta)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)
[![wasm](https://img.shields.io/badge/wasm-compatible-green.svg)](https://github.com/dluksza/quantedge-ta/actions/workflows/ci.yml)

A streaming technical analysis library for Rust. Correct, tested, documented.

## Features

### Type-safe convergence

Indicators return `Option<Self::Output>`. No value until there's enough data.
No silent NaN, no garbage early values. The type system enforces correctness.
For indicators with infinite memory (EMA), convergence enforcement is
configurable: opt in to suppress values until the seed's influence has decayed
below 1%.

### Bring your own data

Indicators accept any type implementing the `Ohlcv` trait. No forced conversion
to a library-specific struct. Implement five required methods on your existing
type and you're done. Volume has a default implementation for data sources that
don't provide it.

### O(1) incremental updates

Indicators maintain running state and update in constant time per tick. No
re-scanning the window.

### WASM compatible

Works in WebAssembly environments. The library compiles for
`wasm32-unknown-unknown` (browser) and `wasm32-wasip1` (WASI runtimes). Zero
dependencies, no filesystem or OS calls in the library itself. CI verifies
WASM compatibility on every commit.

### Live repainting

Indicators track bar boundaries using `open_time`. A kline with a new
`open_time` advances the window; same `open_time` replaces the current value.
Useful for trading terminals and real-time systems that need indicator values
on forming bars.

### Typed outputs

Each indicator defines its own output type via an associated type on the
`Indicator` trait. SMA, EMA, and RSI return `f64`. Bollinger Bands returns
`BbValue { upper, middle, lower }`. MACD returns
`MacdValue { macd, signal, histogram }`. No downcasting, no enums, full type
safety.

## Usage

```rust
use quantedge_ta::{Sma, SmaConfig};
use std::num::NonZero;

let mut sma = Sma::new(SmaConfig::close(NonZero::new(20).unwrap()));

for kline in stream {
    if let Some(value) = sma.compute(&kline) {
        println!("SMA(20): {value}");
    }
    // None = not enough data yet
}
```

Bollinger Bands returns a struct:

```rust
use quantedge_ta::{Bb, BbConfig};
use std::num::NonZero;

let config = BbConfig::builder()
    .length(NonZero::new(20).unwrap())
    .build();
let mut bb = Bb::new(config);

for kline in stream {
    if let Some(value) = bb.compute(&kline) {
        println!("BB upper: {}, middle: {}, lower: {}",
            value.upper(), value.middle(), value.lower());
    }
}
```

Custom standard deviation multiplier:

```rust
use quantedge_ta::{BbConfig, StdDev};
use std::num::NonZero;

let config = BbConfig::builder()
    .length(NonZero::new(20).unwrap())
    .std_dev(StdDev::new(1.5))
    .build();
```

Live data with repainting:

```rust
// Open kline arrives (open_time = 1000)
sma.compute(&open_kline);    // computes with current bar

// Same bar, new trade (open_time = 1000, updated close)
sma.compute(&updated_kline); // replaces current bar value

// Next bar (open_time = 2000)
sma.compute(&next_kline);    // advances the window
```

The caller controls bar boundaries. The library handles the rest.

### Indicator Trait

Each indicator defines its output type. No downcasting needed:

```rust
trait Indicator: Sized + Clone + Display + Debug {
    type Config: IndicatorConfig;
    type Output: Send + Sync + Display + Debug;

    fn new(config: Self::Config) -> Self;
    fn compute(&mut self, kline: &impl Ohlcv) -> Option<Self::Output>;
    fn value(&self) -> Option<Self::Output>;
}

// Sma:  Output = f64
// Ema:  Output = f64
// Rsi:  Output = f64
// Bb:   Output = BbValue { upper: f64, middle: f64, lower: f64 }
// Macd: Output = MacdValue { macd: f64, signal: Option<f64>, histogram: Option<f64> }
```

### Ohlcv Trait

Implement the `Ohlcv` trait on your own data type:

```rust
use quantedge_ta::{Ohlcv, Price, Timestamp};

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
    // fn volume(&self) -> f64 { 0.0 }  -- default, override if needed
}
```

### Convergence

SMA and BB converge as soon as the window fills (`length` bars). EMA and RSI
use exponential smoothing with infinite memory; the SMA seed influences all
subsequent values. RSI output begins at bar `length + 1`. For EMA, `EmaConfig`
provides methods to control convergence:

- `enforce_convergence()` -- when `true`, `compute()` returns `None` until
  the seed's contribution decays below 1%.
- `required_bars_to_converge()` -- returns the number of bars needed.

```rust
use quantedge_ta::EmaConfig;
use std::num::NonZero;

let config = EmaConfig::builder()
    .length(NonZero::new(20).unwrap())
    .enforce_convergence(true) // None until ~63 bars
    .build();
config.required_bars_to_converge(); // 63 = 3 * (20 + 1)
```

Use `required_bars_to_converge()` to determine how much history to fetch before
going live.

### Price Sources

Each indicator is configured with a `PriceSource` that determines which value
to extract from the Ohlcv input:

| Source    | Formula                                                   |
|-----------|-----------------------------------------------------------|
| Close     | close                                                     |
| Open      | open                                                      |
| High      | high                                                      |
| Low       | low                                                       |
| HL2       | (high + low) / 2                                          |
| HLC3      | (high + low + close) / 3                                  |
| OHLC4     | (open + high + low + close) / 4                           |
| HLCC4     | (high + low + close + close) / 4                          |
| TrueRange | max(high - low, \|high - prev_close\|, \|low - prev_close\|) |

## Indicators

| Indicator | Output     | Description                                  |
|-----------|------------|----------------------------------------------|
| SMA       | `f64`      | Simple Moving Average                        |
| EMA       | `f64`      | Exponential Moving Average                   |
| RSI       | `f64`      | Relative Strength Index (Wilder's smoothing) |
| BB        | `BbValue`  | Bollinger Bands (upper, mid, lower)          |
| MACD      | `MacdValue`| Moving Average Convergence Divergence        |
| ATR       | `f64`      | Average True Range                           |

### Planned

CHOP and more.

## Benchmarks

Measured with [Criterion.rs](https://github.com/bheisler/criterion.rs) on 744
BTC/USDT 1-hour bars from Binance.

**Stream** measures end-to-end throughput including window fill.
**Tick** isolates steady-state per-bar cost on a fully converged indicator.
**Repaint** measures single-tick repaint cost (same `open_time`, perturbed close)
on a converged indicator.
**Repaint Stream** measures end-to-end throughput with 3 ticks per bar
(open → mid → final), 2232 total observations.

**Hardware:** Apple M3 Max (16 cores), 48 GB RAM, macOS 26.3, rustc 1.93.1,
`--release` profile.

### Stream — process 744 bars from cold start

| Indicator | Period | Time (median) | Throughput     |
|-----------|--------|---------------|----------------|
| SMA       | 20     | 2.93 µs       | 254 Melem/s    |
| SMA       | 200    | 2.82 µs       | 264 Melem/s    |
| EMA       | 20     | 1.38 µs       | 540 Melem/s    |
| EMA       | 200    | 1.33 µs       | 560 Melem/s    |
| BB        | 20     | 3.62 µs       | 206 Melem/s    |
| BB        | 200    | 3.54 µs       | 210 Melem/s    |
| RSI       | 14     | 4.02 µs       | 185 Melem/s    |
| RSI       | 140    | 3.83 µs       | 194 Melem/s    |
| MACD      | 12/26/9 | 3.78 µs      | 197 Melem/s    |
| MACD      | 120/260/90 | 3.68 µs   | 202 Melem/s    |
| ATR       | 14     | 1.76 µs       | 422 Melem/s    |
| ATR       | 140    | 1.70 µs       | 438 Melem/s    |

### Tick — single `compute()` on a converged indicator

| Indicator | Period | Time (median) |
|-----------|--------|---------------|
| SMA       | 20     | 18.1 ns       |
| SMA       | 200    | 70.7 ns       |
| EMA       | 20     | 2.55 ns       |
| EMA       | 200    | 2.58 ns       |
| BB        | 20     | 24.3 ns       |
| BB        | 200    | 77.5 ns       |
| RSI       | 14     | 7.62 ns       |
| RSI       | 140    | 7.53 ns       |
| MACD      | 12/26/9 | 12.1 ns      |
| MACD      | 120/260/90 | 11.9 ns   |
| ATR       | 14     | 1.86 ns       |
| ATR       | 140    | 1.82 ns       |

### Repaint — single `compute()` repaint on a converged indicator

| Indicator | Period | Time (median) |
|-----------|--------|---------------|
| SMA       | 20     | 18.3 ns       |
| SMA       | 200    | 69.6 ns       |
| EMA       | 20     | 2.60 ns       |
| EMA       | 200    | 2.51 ns       |
| BB        | 20     | 21.6 ns       |
| BB        | 200    | 74.8 ns       |
| RSI       | 14     | 8.52 ns       |
| RSI       | 140    | 8.30 ns       |
| MACD      | 12/26/9 | 12.1 ns      |
| MACD      | 120/260/90 | 11.9 ns   |
| ATR       | 14     | 2.00 ns       |
| ATR       | 140    | 1.96 ns       |

### Repaint Stream — process 744 bars × 3 ticks from cold start

| Indicator | Period | Time (median) | Throughput     |
|-----------|--------|---------------|----------------|
| SMA       | 20     | 8.67 µs       | 257 Melem/s    |
| SMA       | 200    | 8.89 µs       | 251 Melem/s    |
| EMA       | 20     | 3.40 µs       | 656 Melem/s    |
| EMA       | 200    | 3.31 µs       | 673 Melem/s    |
| BB        | 20     | 10.9 µs       | 204 Melem/s    |
| BB        | 200    | 10.9 µs       | 206 Melem/s    |
| RSI       | 14     | 6.62 µs       | 337 Melem/s    |
| RSI       | 140    | 6.82 µs       | 327 Melem/s    |
| MACD      | 12/26/9 | 8.45 µs      | 264 Melem/s    |
| MACD      | 120/260/90 | 9.35 µs   | 239 Melem/s    |
| ATR       | 14     | 5.16 µs       | 433 Melem/s    |
| ATR       | 140    | 5.03 µs       | 443 Melem/s    |

Run locally:

```bash
cargo bench                    # all benchmarks
cargo bench -- stream          # stream only
cargo bench -- tick            # single-tick only
cargo bench -- repaint$        # single-repaint only
cargo bench -- repaint_stream  # repaint stream only
```

## Minimum Supported Rust Version

1.93

## Licence

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

Contributions welcome. Please open an issue before submitting large changes.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 licence, shall
be dual-licensed as above, without any additional terms or conditions.
