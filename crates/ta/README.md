# quantedge-ta

[![CI](https://github.com/dluksza/quantedge/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge/branch/main/graph/badge.svg?flag=quantedge-ta)](https://codecov.io/gh/dluksza/quantedge?flags[0]=quantedge-ta)
[![crates.io](https://img.shields.io/crates/v/quantedge-ta.svg)](https://crates.io/crates/quantedge-ta)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)
[![wasm](https://img.shields.io/badge/wasm-compatible-green.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)

A streaming technical analysis library for Rust. Correct, tested, documented.

## Features

### Type-safe convergence

Indicators return `Option<Self::Output>`. No value until there's enough data.
No silent NaN, no garbage early values. The type system enforces correctness.
For indicators with infinite memory (EMA), `full_convergence()` reports how
many bars are needed for the seed's influence to decay below 1%.

### Plain-struct bar input

Indicators take `&Ohlcv`, a `Copy` struct with six public fields (`open`,
`high`, `low`, `close`, `open_time`, `volume`). Build one per kline with a
struct literal or a `From` conversion from your own kline type — no trait
impl required, no generic parameters on `compute`.

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
`StochValue { k, d }`. Stochastic RSI returns
`StochRsiValue { k, d }`. Keltner Channel returns
`KcValue { upper, middle, lower }`. Donchian Channel returns
`DcValue { upper, middle, lower }`. ADX returns
`AdxValue { adx, plus_di, minus_di }`. Ichimoku Cloud returns
`IchimokuValue { tenkan, kijun, senkou_a, senkou_b, chikou_close }`.
KDJ returns `KdjValue { k, d, j }`. Momentum returns `f64`.
VWAP returns `VwapValue { vwap, band_1, band_2, band_3 }`.
Supertrend returns `SupertrendValue { value, is_bullish }`.
Parabolic SAR returns `ParabolicSarValue { sar, is_long }`.
Williams %R, CCI, CHOP, and OBV return `f64`.
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

Bollinger Bands returns a struct with public fields:

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
            value.upper, value.middle, value.lower);
    }
}
```

Custom standard deviation multiplier:

```rust
use quantedge_ta::{BbConfig, Multiplier};
use std::num::NonZero;

let config = BbConfig::builder()
    .length(NonZero::new(20).unwrap())
    .std_dev(Multiplier::new(1.5))
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
    type Config: IndicatorConfig<Output = Self::Output>;
    type Output: 'static + Copy + Send + Sync + Display + Debug;

    fn new(config: Self::Config) -> Self;
    fn compute(&mut self, kline: &Ohlcv) -> Option<Self::Output>;
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
// StochRsi:  Output = StochRsiValue { k: f64, d: Option<f64> }
// Kdj:       Output = KdjValue { k: f64, d: f64, j: f64 }
// Mom:       Output = f64
// Obv:       Output = f64
// Vwap:       Output = VwapValue { vwap: f64, band_1: Option<VwapBand>, band_2: Option<VwapBand>, band_3: Option<VwapBand> }
// Supertrend:    Output = SupertrendValue { value: f64, is_bullish: bool }
// ParabolicSar:  Output = ParabolicSarValue { sar: f64, is_long: bool }
```

### Ohlcv Struct

`Ohlcv` is a plain `Copy` struct — build one per bar and pass it by reference
to `compute()`. Convert from your own kline type with a field-wise copy:

```rust
use quantedge_ta::Ohlcv;

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

// let bar: Ohlcv = (&my_kline).into();
// indicator.compute(&bar);
```

`Timestamp` is recommended to be microseconds since Unix epoch, monotonically
increasing. This is **required** for the VWAP indicator, which uses timestamps
to detect session boundaries. `volume` has no implicit default: set it to a
real figure for OBV and VWAP, or `0.0` when feeding indicators that ignore it.

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

## Indicators (21)

| Indicator | Output     | Description                                  |
|-----------|------------|----------------------------------------------|
| SMA        | `f64`      | Simple Moving Average                       |
| EMA        | `f64`      | Exponential Moving Average                  |
| RSI        | `f64`      | Relative Strength Index (Wilder's smoothing)|
| BB         | `BbValue`  | Bollinger Bands (upper, mid, lower)         |
| MACD       | `MacdValue`| Moving Average Convergence Divergence       |
| ATR        | `f64`      | Average True Range                          |
| Stoch      | `StochValue`| Stochastic Oscillator (%K, %D)             |
| KC         | `KcValue`  | Keltner Channel (upper, mid, lower)         |
| DC         | `DcValue`  | Donchian Channel (upper, mid, lower)        |
| ADX        | `AdxValue` | Average Directional Index (+DI, −DI, ADX)   |
| WillR      | `f64`      | Williams %R                                 |
| CCI        | `f64`      | Commodity Channel Index                     |
| CHOP       | `f64`      | Choppiness Index                            |
| Ichimoku   | `IchimokuValue`| Ichimoku Cloud (tenkan, kijun, senkou A/B, chikou) |
| StochRSI   | `StochRsiValue`| Stochastic RSI (%K, %D)                 |
| KDJ        | `KdjValue`  | KDJ Oscillator (K, D, J)                |
| MOM        | `f64`      | Momentum (price change over period)      |
| OBV        | `f64`      | On-Balance Volume                           |
| VWAP       | `VwapValue`| Volume Weighted Average Price               |
| Supertrend | `SupertrendValue` | Supertrend (trend line + direction)  |
| Parabolic SAR | `ParabolicSarValue` | Parabolic Stop and Reverse (SAR + direction) |

## Benchmarks

Measured with [Criterion.rs](https://github.com/bheisler/criterion.rs) on 744
BTC/USDT 1-hour bars from Binance, split into a 349-bar warm-up seed and 395
measured bars so every group times steady-state work.

**Stream** measures end-to-end throughput over the 395 post-warmup bars from a
pre-converged seed.
**Tick** isolates steady-state per-bar cost on a fully converged indicator.
**Repaint** measures single-tick repaint cost (same `open_time`, perturbed close)
on a converged indicator.
**Repaint Stream** measures end-to-end throughput with 3 ticks per bar
(open → mid → final), 1185 total observations on a pre-converged seed.

**Hardware:** Apple M5 Max (18 cores), 128 GB RAM, macOS 26.4.1, rustc 1.95.0,
`--release` profile.

### Stream — process 395 post-warmup bars

| Indicator     | Period         | Time (median) | Throughput    |
|---------------|----------------|---------------|---------------|
| SMA           | 20             | 437 ns        | 905 Melem/s   |
| SMA           | 200            | 454 ns        | 869 Melem/s   |
| EMA           | 20             | 868 ns        | 455 Melem/s   |
| EMA           | 200            | 863 ns        | 458 Melem/s   |
| BB            | 20             | 480 ns        | 824 Melem/s   |
| BB            | 200            | 497 ns        | 794 Melem/s   |
| RSI           | 14             | 746 ns        | 529 Melem/s   |
| RSI           | 140            | 743 ns        | 532 Melem/s   |
| MACD          | 12/26/9        | 914 ns        | 432 Melem/s   |
| MACD          | 120/260/90     | 915 ns        | 432 Melem/s   |
| ATR           | 14             | 511 ns        | 773 Melem/s   |
| ATR           | 140            | 511 ns        | 773 Melem/s   |
| Stoch         | 14/3/3         | 3.44 µs       | 115 Melem/s   |
| Stoch         | 140/30/30      | 6.75 µs       | 58.5 Melem/s  |
| KC            | 20/10          | 1.00 µs       | 395 Melem/s   |
| KC            | 200/100        | 998 ns        | 396 Melem/s   |
| DC            | 20             | 2.37 µs       | 167 Melem/s   |
| DC            | 200            | 8.52 µs       | 46.4 Melem/s  |
| ADX           | 14             | 2.06 µs       | 192 Melem/s   |
| ADX           | 140            | 2.06 µs       | 192 Melem/s   |
| WillR         | 14             | 2.42 µs       | 163 Melem/s   |
| WillR         | 140            | 6.00 µs       | 65.8 Melem/s  |
| CCI           | 20             | 1.40 µs       | 283 Melem/s   |
| CCI           | 200            | 19.34 µs      | 20.4 Melem/s  |
| CHOP          | 14             | 3.57 µs       | 111 Melem/s   |
| CHOP          | 140            | 7.01 µs       | 56.4 Melem/s  |
| Ichimoku      | 9/26/52/26     | 8.78 µs       | 45.0 Melem/s  |
| Ichimoku      | 36/104/208/104 | 16.94 µs      | 23.3 Melem/s  |
| StochRSI      | 14/14/3/3      | 4.06 µs       | 97.3 Melem/s  |
| StochRSI      | 140/140/30/30  | 6.24 µs       | 63.3 Melem/s  |
| KDJ           | 9/3/3          | 3.48 µs       | 114 Melem/s   |
| KDJ           | 90/30/30       | 6.38 µs       | 61.9 Melem/s  |
| MOM           | 10             | 384 ns        | 1.03 Gelem/s  |
| MOM           | 100            | 429 ns        | 921 Melem/s   |
| Supertrend    | 20             | 1.04 µs       | 380 Melem/s   |
| Supertrend    | 200            | 1.04 µs       | 380 Melem/s   |
| OBV           | —              | 450 ns        | 877 Melem/s   |
| VWAP          | Day            | 648 ns        | 609 Melem/s   |
| Parabolic SAR | 0.02/0.2       | 3.05 µs       | 130 Melem/s   |
| Parabolic SAR | 0.01/0.4       | 3.03 µs       | 130 Melem/s   |

### Tick — single `compute()` on a converged indicator

| Indicator     | Period         | Time (median) |
|---------------|----------------|---------------|
| SMA           | 20             | 11.37 ns      |
| SMA           | 200            | 24.22 ns      |
| EMA           | 20             | 1.96 ns       |
| EMA           | 200            | 2.02 ns       |
| BB            | 20             | 14.46 ns      |
| BB            | 200            | 28.97 ns      |
| RSI           | 14             | 5.15 ns       |
| RSI           | 140            | 5.15 ns       |
| MACD          | 12/26/9        | 8.45 ns       |
| MACD          | 120/260/90     | 8.43 ns       |
| ATR           | 14             | 2.15 ns       |
| ATR           | 140            | 1.95 ns       |
| Stoch         | 14/3/3         | 40.11 ns      |
| Stoch         | 140/30/30      | 119 ns        |
| KC            | 20/10          | 4.82 ns       |
| KC            | 200/100        | 4.31 ns       |
| DC            | 20             | 27.18 ns      |
| DC            | 200            | 59.37 ns      |
| ADX           | 14             | 11.24 ns      |
| ADX           | 140            | 11.24 ns      |
| WillR         | 14             | 19.28 ns      |
| WillR         | 140            | 63.06 ns      |
| CCI           | 20             | 13.46 ns      |
| CCI           | 200            | 65.78 ns      |
| CHOP          | 14             | 31.32 ns      |
| CHOP          | 140            | 75.99 ns      |
| Ichimoku      | 9/26/52/26     | 85.42 ns      |
| Ichimoku      | 36/104/208/104 | 233 ns        |
| StochRSI      | 14/14/3/3      | 40.80 ns      |
| StochRSI      | 140/140/30/30  | 124 ns        |
| KDJ           | 9/3/3          | 49.94 ns      |
| KDJ           | 90/30/30       | 142 ns        |
| MOM           | 10             | 12.88 ns      |
| MOM           | 100            | 48.41 ns      |
| Supertrend    | 20             | 9.39 ns       |
| Supertrend    | 200            | 9.08 ns       |
| OBV           | —              | 1.36 ns       |
| VWAP          | Day            | 7.06 ns       |
| Parabolic SAR | 0.02/0.2       | 8.81 ns       |
| Parabolic SAR | 0.01/0.4       | 8.59 ns       |

### Repaint — single `compute()` repaint on a converged indicator

| Indicator     | Period         | Time (median) |
|---------------|----------------|---------------|
| SMA           | 20             | 11.28 ns      |
| SMA           | 200            | 23.60 ns      |
| EMA           | 20             | 1.84 ns       |
| EMA           | 200            | 2.10 ns       |
| BB            | 20             | 14.05 ns      |
| BB            | 200            | 27.76 ns      |
| RSI           | 14             | 3.94 ns       |
| RSI           | 140            | 3.90 ns       |
| MACD          | 12/26/9        | 7.75 ns       |
| MACD          | 120/260/90     | 8.39 ns       |
| ATR           | 14             | 2.03 ns       |
| ATR           | 140            | 1.99 ns       |
| Stoch         | 14/3/3         | 40.44 ns      |
| Stoch         | 140/30/30      | 119 ns        |
| KC            | 20/10          | 4.48 ns       |
| KC            | 200/100        | 4.97 ns       |
| DC            | 20             | 18.32 ns      |
| DC            | 200            | 55.31 ns      |
| ADX           | 14             | 10.71 ns      |
| ADX           | 140            | 10.01 ns      |
| WillR         | 14             | 17.99 ns      |
| WillR         | 140            | 54.33 ns      |
| CCI           | 20             | 13.17 ns      |
| CCI           | 200            | 67.34 ns      |
| CHOP          | 14             | 30.20 ns      |
| CHOP          | 140            | 75.03 ns      |
| Ichimoku      | 9/26/52/26     | 83.00 ns      |
| Ichimoku      | 36/104/208/104 | 223 ns        |
| StochRSI      | 14/14/3/3      | 40.79 ns      |
| StochRSI      | 140/140/30/30  | 120 ns        |
| KDJ           | 9/3/3          | 50.58 ns      |
| KDJ           | 90/30/30       | 143 ns        |
| MOM           | 10             | 12.62 ns      |
| MOM           | 100            | 46.65 ns      |
| Supertrend    | 20             | 7.52 ns       |
| Supertrend    | 200            | 7.75 ns       |
| OBV           | —              | 1.40 ns       |
| VWAP          | Day            | 6.69 ns       |
| Parabolic SAR | 0.02/0.2       | 6.27 ns       |
| Parabolic SAR | 0.01/0.4       | 6.47 ns       |

### Repaint Stream — process 395 bars × 3 ticks post-warmup

| Indicator     | Period         | Time (median) | Throughput    |
|---------------|----------------|---------------|---------------|
| SMA           | 20             | 1.33 µs       | 891 Melem/s   |
| SMA           | 200            | 1.31 µs       | 908 Melem/s   |
| EMA           | 20             | 1.91 µs       | 622 Melem/s   |
| EMA           | 200            | 1.90 µs       | 623 Melem/s   |
| BB            | 20             | 1.51 µs       | 785 Melem/s   |
| BB            | 200            | 1.49 µs       | 797 Melem/s   |
| RSI           | 14             | 2.25 µs       | 527 Melem/s   |
| RSI           | 140            | 2.25 µs       | 528 Melem/s   |
| MACD          | 12/26/9        | 1.73 µs       | 684 Melem/s   |
| MACD          | 120/260/90     | 1.73 µs       | 686 Melem/s   |
| ATR           | 14             | 1.09 µs       | 1.09 Gelem/s  |
| ATR           | 140            | 1.09 µs       | 1.09 Gelem/s  |
| Stoch         | 14/3/3         | 7.09 µs       | 167 Melem/s   |
| Stoch         | 140/30/30      | 10.44 µs      | 114 Melem/s   |
| KC            | 20/10          | 2.70 µs       | 439 Melem/s   |
| KC            | 200/100        | 2.71 µs       | 437 Melem/s   |
| DC            | 20             | 4.02 µs       | 295 Melem/s   |
| DC            | 200            | 9.83 µs       | 121 Melem/s   |
| ADX           | 14             | 5.23 µs       | 227 Melem/s   |
| ADX           | 140            | 5.23 µs       | 227 Melem/s   |
| WillR         | 14             | 4.12 µs       | 288 Melem/s   |
| WillR         | 140            | 7.40 µs       | 160 Melem/s   |
| CCI           | 20             | 4.26 µs       | 278 Melem/s   |
| CCI           | 200            | 58.43 µs      | 20.3 Melem/s  |
| CHOP          | 14             | 7.41 µs       | 160 Melem/s   |
| CHOP          | 140            | 10.59 µs      | 112 Melem/s   |
| Ichimoku      | 9/26/52/26     | 14.50 µs      | 81.7 Melem/s  |
| Ichimoku      | 36/104/208/104 | 22.04 µs      | 53.8 Melem/s  |
| StochRSI      | 14/14/3/3      | 9.58 µs       | 124 Melem/s   |
| StochRSI      | 140/140/30/30  | 11.46 µs      | 103 Melem/s   |
| KDJ           | 9/3/3          | 11.59 µs      | 102 Melem/s   |
| KDJ           | 90/30/30       | 13.75 µs      | 86.2 Melem/s  |
| MOM           | 10             | 1.15 µs       | 1.03 Gelem/s  |
| MOM           | 100            | 1.20 µs       | 990 Melem/s   |
| Supertrend    | 20             | 2.67 µs       | 444 Melem/s   |
| Supertrend    | 200            | 2.69 µs       | 440 Melem/s   |
| OBV           | —              | 1.42 µs       | 833 Melem/s   |
| VWAP          | Day            | 1.65 µs       | 719 Melem/s   |
| Parabolic SAR | 0.02/0.2       | 5.24 µs       | 226 Melem/s   |
| Parabolic SAR | 0.01/0.4       | 5.24 µs       | 226 Melem/s   |

Run locally:

```bash
cargo bench                    # all benchmarks
cargo bench -- stream          # stream only
cargo bench -- tick            # single-tick only
cargo bench -- repaint$        # single-repaint only
cargo bench -- repaint_stream  # repaint stream only
```

## Minimum Supported Rust Version

1.95

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
