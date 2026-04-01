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
For indicators with infinite memory (EMA), `full_convergence()` reports how
many bars are needed for the seed's influence to decay below 1%.

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
`Indicator` trait. SMA, EMA, RSI, and ATR return `f64`. Bollinger Bands returns
`BbValue { upper, middle, lower }`. MACD returns
`MacdValue { macd, signal, histogram }`. Stochastic returns
`StochValue { k, d }`. Keltner Channel returns
`KcValue { upper, middle, lower }`. Donchian Channel returns
`DcValue { upper, middle, lower }`. ADX returns
`AdxValue { adx, plus_di, minus_di }`. Ichimoku Cloud returns
`IchimokuValue { tenkan, kijun, senkou_a, senkou_b, chikou_close }`.
Williams %R, CCI, and CHOP return `f64`.
No downcasting, no enums, full type safety.

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

Derive a new config from an existing one with `to_builder()`:

```rust
use quantedge_ta::{SmaConfig, PriceSource};
use std::num::NonZero;

let sma_close = SmaConfig::close(NonZero::new(20).unwrap());

// Change only the price source, keep the same length
let sma_hl2 = sma_close.to_builder().source(PriceSource::HL2).build();
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

// Sma:   Output = f64
// Ema:   Output = f64
// Rsi:   Output = f64
// Bb:    Output = BbValue { upper: f64, middle: f64, lower: f64 }
// Macd:  Output = MacdValue { macd: f64, signal: Option<f64>, histogram: Option<f64> }
// Stoch: Output = StochValue { k: f64, d: Option<f64> }
// Atr:   Output = f64
// Kc:    Output = KcValue { upper: f64, middle: f64, lower: f64 }
// Dc:    Output = DcValue { upper: f64, middle: f64, lower: f64 }
// Adx:      Output = AdxValue { adx: f64, plus_di: f64, minus_di: f64 }
// Ichimoku: Output = IchimokuValue { tenkan: f64, kijun: f64, senkou_a: f64, senkou_b: f64, chikou_close: f64 }
// WillR:    Output = f64
// Cci:      Output = f64
// Chop:     Output = f64
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

Every indicator config exposes `convergence()` — the number of bars that
`compute()` must process before it starts returning `Some`. During backtesting
this defines the warm-up (seeding) phase: bars where the indicator is
stabilising and should not drive trading decisions.

```rust
use quantedge_ta::{SmaConfig, RsiConfig, MacdConfig};
use std::num::NonZero;

let sma = SmaConfig::close(NonZero::new(20).unwrap());
let rsi = RsiConfig::close(NonZero::new(14).unwrap());
let macd = MacdConfig::default_close(); // MACD(12, 26, 9)

// The slowest indicator determines the warm-up length
let warmup = sma.convergence()   // 20
    .max(rsi.convergence())      // 15
    .max(macd.convergence());    // 26
// → skip the first 26 bars before acting on signals
```

SMA and BB converge as soon as the window fills (`length` bars). EMA and RSI
use exponential smoothing with infinite memory; the SMA seed influences all
subsequent values. RSI output begins at bar `length + 1`. For EMA, `EmaConfig`
provides `full_convergence()` — the number of bars until the seed's
contribution decays below 1% (e.g. `63` for EMA(20) = `3 × (20 + 1)`).

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
| Stoch     | `StochValue`| Stochastic Oscillator (%K, %D)              |
| KC        | `KcValue`  | Keltner Channel (upper, mid, lower)          |
| DC        | `DcValue`  | Donchian Channel (upper, mid, lower)         |
| ADX       | `AdxValue` | Average Directional Index (+DI, −DI, ADX)    |
| WillR     | `f64`      | Williams %R                                  |
| CCI       | `f64`      | Commodity Channel Index                      |
| CHOP      | `f64`      | Choppiness Index                             |
| Ichimoku  | `IchimokuValue`| Ichimoku Cloud (tenkan, kijun, senkou A/B, chikou) |

### Planned

StochRSI, OBV, VWAP, Supertrend, Parabolic SAR.

## Benchmarks

Measured with [Criterion.rs](https://github.com/bheisler/criterion.rs) on 744
BTC/USDT 1-hour bars from Binance.

**Stream** measures end-to-end throughput including window fill.
**Tick** isolates steady-state per-bar cost on a fully converged indicator.
**Repaint** measures single-tick repaint cost (same `open_time`, perturbed close)
on a converged indicator.
**Repaint Stream** measures end-to-end throughput with 3 ticks per bar
(open → mid → final), 2232 total observations.

**Hardware:** Apple M5 Max (18 cores), 128 GB RAM, macOS 26.3.2, rustc 1.93.1,
`--release` profile.

### Stream — process 744 bars from cold start

| Indicator | Period | Time (median) | Throughput     |
|-----------|--------|---------------|----------------|
| SMA       | 20     | 810 ns        | 918 Melem/s    |
| SMA       | 200    | 883 ns        | 842 Melem/s    |
| EMA       | 20     | 986 ns        | 754 Melem/s    |
| EMA       | 200    | 956 ns        | 778 Melem/s    |
| BB        | 20     | 1.01 µs       | 737 Melem/s    |
| BB        | 200    | 959 ns        | 776 Melem/s    |
| RSI       | 14     | 3.04 µs       | 245 Melem/s    |
| RSI       | 140    | 2.94 µs       | 253 Melem/s    |
| MACD      | 12/26/9 | 2.83 µs      | 263 Melem/s    |
| MACD      | 120/260/90 | 2.93 µs   | 254 Melem/s    |
| ATR       | 14     | 1.42 µs       | 524 Melem/s    |
| ATR       | 140    | 1.32 µs       | 564 Melem/s    |
| Stoch     | 14/3/3 | 6.43 µs       | 116 Melem/s    |
| Stoch     | 140/30/30 | 12.4 µs    | 60.1 Melem/s   |
| KC        | 20/10  | 1.53 µs       | 486 Melem/s    |
| KC        | 200/100 | 1.56 µs      | 477 Melem/s    |
| DC        | 20     | 4.36 µs       | 171 Melem/s    |
| DC        | 200    | 10.3 µs       | 72.4 Melem/s   |
| ADX       | 14     | 3.90 µs       | 191 Melem/s    |
| ADX       | 140    | 3.94 µs       | 189 Melem/s    |
| WillR     | 14     | 4.46 µs       | 167 Melem/s    |
| WillR     | 140    | 11.4 µs       | 65.2 Melem/s   |
| CCI       | 20     | 2.61 µs       | 285 Melem/s    |
| CCI       | 200    | 27.8 µs       | 26.7 Melem/s   |
| CHOP      | 14     | 6.52 µs       | 114 Melem/s    |
| CHOP      | 140    | 13.0 µs       | 57.3 Melem/s   |
| Ichimoku  | 9/26/52/26 | 15.4 µs   | 48.3 Melem/s   |
| Ichimoku  | 36/104/208/104 | 25.1 µs | 29.7 Melem/s  |

### Tick — single `compute()` on a converged indicator

| Indicator | Period | Time (median) |
|-----------|--------|---------------|
| SMA       | 20     | 8.31 ns       |
| SMA       | 200    | 17.7 ns       |
| EMA       | 20     | 2.80 ns       |
| EMA       | 200    | 1.80 ns       |
| BB        | 20     | 9.21 ns       |
| BB        | 200    | 18.4 ns       |
| RSI       | 14     | 8.62 ns       |
| RSI       | 140    | 5.72 ns       |
| MACD      | 12/26/9 | 8.80 ns      |
| MACD      | 120/260/90 | 8.81 ns   |
| ATR       | 14     | 1.86 ns       |
| ATR       | 140    | 1.45 ns       |
| Stoch     | 14/3/3 | 40.8 ns       |
| Stoch     | 140/30/30 | 39.9 ns   |
| KC        | 20/10  | 4.32 ns       |
| KC        | 200/100 | 3.84 ns      |
| DC        | 20     | 18.3 ns       |
| DC        | 200    | 24.0 ns       |
| ADX       | 14     | 12.2 ns       |
| ADX       | 140    | 12.1 ns       |
| WillR     | 14     | 19.3 ns       |
| WillR     | 140    | 23.4 ns       |
| CCI       | 20     | 10.7 ns       |
| CCI       | 200    | 63.1 ns       |
| CHOP      | 14     | 30.2 ns       |
| CHOP      | 140    | 38.0 ns       |
| Ichimoku  | 9/26/52/26 | 106 ns   |
| Ichimoku  | 36/104/208/104 | 115 ns |

### Repaint — single `compute()` repaint on a converged indicator

| Indicator | Period | Time (median) |
|-----------|--------|---------------|
| SMA       | 20     | 8.44 ns       |
| SMA       | 200    | 17.5 ns       |
| EMA       | 20     | 2.87 ns       |
| EMA       | 200    | 1.51 ns       |
| BB        | 20     | 9.84 ns       |
| BB        | 200    | 19.6 ns       |
| RSI       | 14     | 7.84 ns       |
| RSI       | 140    | 3.68 ns       |
| MACD      | 12/26/9 | 8.80 ns      |
| MACD      | 120/260/90 | 8.68 ns   |
| ATR       | 14     | 1.95 ns       |
| ATR       | 140    | 1.45 ns       |
| Stoch     | 14/3/3 | 39.7 ns       |
| Stoch     | 140/30/30 | 38.2 ns   |
| KC        | 20/10  | 4.05 ns       |
| KC        | 200/100 | 3.81 ns      |
| DC        | 20     | 16.4 ns       |
| DC        | 200    | 23.2 ns       |
| ADX       | 14     | 11.1 ns       |
| ADX       | 140    | 11.3 ns       |
| WillR     | 14     | 16.6 ns       |
| WillR     | 140    | 21.5 ns       |
| CCI       | 20     | 14.0 ns       |
| CCI       | 200    | 62.7 ns       |
| CHOP      | 14     | 27.9 ns       |
| CHOP      | 140    | 36.9 ns       |
| Ichimoku  | 9/26/52/26 | 74.4 ns  |
| Ichimoku  | 36/104/208/104 | 115 ns |

### Repaint Stream — process 744 bars × 3 ticks from cold start

| Indicator | Period | Time (median) | Throughput     |
|-----------|--------|---------------|----------------|
| SMA       | 20     | 2.49 µs       | 896 Melem/s    |
| SMA       | 200    | 2.60 µs       | 859 Melem/s    |
| EMA       | 20     | 2.65 µs       | 842 Melem/s    |
| EMA       | 200    | 2.67 µs       | 836 Melem/s    |
| BB        | 20     | 2.97 µs       | 751 Melem/s    |
| BB        | 200    | 3.42 µs       | 653 Melem/s    |
| RSI       | 14     | 4.87 µs       | 458 Melem/s    |
| RSI       | 140    | 5.18 µs       | 431 Melem/s    |
| MACD      | 12/26/9 | 7.73 µs      | 289 Melem/s    |
| MACD      | 120/260/90 | 8.07 µs   | 277 Melem/s    |
| ATR       | 14     | 3.41 µs       | 654 Melem/s    |
| ATR       | 140    | 3.43 µs       | 651 Melem/s    |
| Stoch     | 14/3/3 | 13.6 µs       | 164 Melem/s    |
| Stoch     | 140/30/30 | 18.7 µs   | 119 Melem/s    |
| KC        | 20/10  | 5.23 µs       | 427 Melem/s    |
| KC        | 200/100 | 4.62 µs      | 483 Melem/s    |
| DC        | 20     | 7.20 µs       | 310 Melem/s    |
| DC        | 200    | 12.9 µs       | 173 Melem/s    |
| ADX       | 14     | 9.71 µs       | 230 Melem/s    |
| ADX       | 140    | 9.51 µs       | 235 Melem/s    |
| WillR     | 14     | 7.70 µs       | 290 Melem/s    |
| WillR     | 140    | 14.2 µs       | 157 Melem/s    |
| CCI       | 20     | 7.86 µs       | 284 Melem/s    |
| CCI       | 200    | 84.5 µs       | 26.4 Melem/s   |
| CHOP      | 14     | 12.5 µs       | 178 Melem/s    |
| CHOP      | 140    | 18.3 µs       | 122 Melem/s    |
| Ichimoku  | 9/26/52/26 | 27.4 µs   | 81.5 Melem/s   |
| Ichimoku  | 36/104/208/104 | 36.1 µs | 61.9 Melem/s  |

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
