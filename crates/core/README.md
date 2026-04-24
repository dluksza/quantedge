# quantedge-core

[![CI](https://github.com/dluksza/quantedge/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge/branch/main/graph/badge.svg?flag=quantedge-core)](https://codecov.io/gh/dluksza/quantedge?flags[0]=quantedge-core)
[![crates.io](https://img.shields.io/crates/v/quantedge-core.svg)](https://crates.io/crates/quantedge-core)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)
[![wasm](https://img.shields.io/badge/wasm-compatible-green.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)

Core types shared across the [quantedge](https://github.com/dluksza/quantedge) crates.

Defines the `Ohlcv` bar struct, the `Indicator`/`IndicatorConfig` trait surface, the `Instrument` subscription key, streaming snapshot traits (`Bar`, `TimeframeSnapshot`, `MarketSnapshot`), and the `Timeframe` type for bar boundary alignment. Downstream crates share these primitives without pulling in the full indicator library.

## Features

### Plain-struct bar

`Ohlcv` is a `Copy` struct with six public fields (`open`, `high`, `low`, `close`, `open_time`, `volume`). Build one per kline, pass it by reference to consumer crates — no trait to implement, no generic parameters to thread through signatures. Volume-dependent indicators (OBV, VWAP) require a real `volume`; pass `0.0` when feeding indicators that ignore it.

### Shared indicator trait surface

`Indicator`, `IndicatorConfig`, `IndicatorConfigBuilder`, and `PriceSource` live here and are re-exported by `quantedge-ta`. Engine, sim, and snapshot crates can target the traits directly without depending on `quantedge-ta`. `IndicatorConfig::Output` pairs with `Indicator::Output` so generic code can resolve an indicator's output type from its config alone.

### Instrument identifiers

`Instrument` combines a `Venue`, a `Ticker` (base/quote `Asset` pair), and a `MarketKind`. Each leaf is an ASCII-validated, case-normalized newtype over `Arc<str>`; cloning an `Instrument` is four atomic increments, cheap enough to pass through strategies, order paths, and log lines. Grammar separators (`/`, `:`, `@`) are rejected at the leaf so `Ticker::from_str` and `Instrument`'s `Display` cannot be broken by pathological input. `Display` renders `<venue>:<base>/<quote>@<market>`.

### Streaming snapshot traits

`Bar`, `TimeframeSnapshot`, and `MarketSnapshot` define the surface strategy code reads at one tick. Each snapshot is immutable — successive ticks surface as new snapshots rather than mutating prior ones. `at(0)` / `bars(0..)` treat the forming bar as index 0 with closed history at `1..`; `closed(0)` skips the forming bar. Querying an unsubscribed indicator (`Bar::value`) or timeframe (`MarketSnapshot::for_timeframe`) panics — subscriptions are fixed at construction, so a miss is a caller bug.

### Bar timeframes

`Timeframe` maps a Unix-μs timestamp to bar boundaries: `open_time` returns the start of the containing period, `close_time` the last μs before the next period starts. Adjacent bars form a contiguous cover of the timeline (`close_time(t) + 1 == open_time` of the next period). Use `bounds(t)` to get both at once, it shares computation between the two halves, worth ~30% for monthly/yearly dispatch.

Handles second-through-year units with calendar-correct month and year arithmetic, using Howard Hinnant's [`civil_from_days`](https://howardhinnant.github.io/date_algorithms.html) — no `chrono` at runtime. Multi-month periods are epoch-anchored from January 1970, matching calendar quarters (`MONTH_3`) and halves (`MONTH_6`) for any N dividing 12. `Display` uses Binance-style compact notation (`5m`, `1h`, `1d`, `1w`, `3M`, `1Y`).

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

### Instrument

```rust
use quantedge_core::{Asset, Instrument, MarketKind, Ticker, Venue};

let instrument = Instrument::new(
    Venue::new("binance").unwrap(),
    "BTC/USDT".parse::<Ticker>().unwrap(),
    MarketKind::new("perp").unwrap(),
);

assert_eq!(instrument.to_string(), "binance:BTC/USDT@perp");
assert_eq!(instrument.base().as_str(), "BTC");
```

Leaf newtypes (`Venue`, `Asset`, `MarketKind`) trim whitespace, validate the charset (ASCII alphanumeric or `_`), cap length at 32 bytes, and normalize case per type (lowercase for `Venue`/`MarketKind`, uppercase for `Asset`). Prefer `"BTC/USDT".parse()` over `Ticker::new(Asset, Asset)` when the input is textual — it cannot silently swap base and quote.

### Bar boundaries

Consumers detect new bars by comparing `open_time` values: the same timestamp updates (repaints) the current bar, a new timestamp advances the window. `open_time` must be non-decreasing across successive bars.

`Timestamp` is recommended to be microseconds since Unix epoch, monotonically increasing.

## Licence

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
