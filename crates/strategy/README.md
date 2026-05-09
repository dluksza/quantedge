# quantedge-strategy

[![CI](https://github.com/dluksza/quantedge/actions/workflows/ci.yml/badge.svg)](https://github.com/dluksza/quantedge/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/dluksza/quantedge/branch/main/graph/badge.svg?flag=quantedge-strategy)](https://codecov.io/gh/dluksza/quantedge?flags[0]=quantedge-strategy)
[![crates.io](https://img.shields.io/crates/v/quantedge-strategy.svg)](https://crates.io/crates/quantedge-strategy)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#licence)

Trait surface and test harness for writing market-signal generators in the
[quantedge](https://github.com/dluksza/quantedge) ecosystem.

A generator is a stateless function from a market snapshot to an optional
signal. This crate defines the `SignalGenerator` trait, the `MarketSignal`
output type, the declarative `MarketSignalConfig` DSL for declaring data
dependencies, and a complete `test_util` module for unit-testing generators
without a live engine.

## Status: preview

The engine that consumes `SignalGenerator` is not yet implemented. The trait
shape is informed by the test harness (`FakeEngine` + `EnforcingMarketSnapshot`)
but has not been validated end-to-end by a real engine. Expect breaking
changes once engine implementation surfaces friction.

If you depend on this crate during the preview period, **pin to an exact
version** (`= "0.0.1"`, not `"0.0"` or `"^0.0.1"`). Each `0.0.x` bump should
be assumed breaking.

The `0.1.0` release will be cut once the engine has consumed the trait
end-to-end and the contract has earned the right to claim stability.

## Features

### One trait to implement

`SignalGenerator` is the entire customer-facing contract. Three required
methods (`id`, `name`, `configure`, `evaluate`), one supertrait bound
(`Default + Sync + Send`). `configure` declares what data the generator will
read; `evaluate` reads it and returns `Option<MarketSignal>`. Stateless by
design - no `&mut self`, no internal state to manage.

### Declarative dependency declaration

`configure` takes a `MarketSignalConfig` builder and returns it transformed.
Three primitives:

- `require_timeframes(&[..])` - which bar resolutions the generator reads.
- `require_closed_bars(N)` - how far back into closed-bar history the
  generator looks (per timeframe).
- `register(&IndicatorConfig)` — which indicators the engine should compute
  on every required timeframe and surface via `Bar::value`.

The engine guarantees the declared data is present when `evaluate` runs.
Anything `evaluate` reads beyond what `configure` declared is a contract
violation - caught loudly by the test harness, would silently misbehave in
production.

### Multi-timeframe and composite outputs

Generators routinely span multiple timeframes (HTF trend filter on LTF
trigger) and consume indicators with structured outputs (Bollinger Bands
returns `BbValue { upper, middle, lower }`, MACD returns `MacdValue`, etc.).
Both are first-class - the trait surface doesn't change shape, the same
`Bar::value(&cfg)` pattern works for scalar and struct outputs alike.

### Test harness included

The `test-util` feature exposes a complete kit for unit-testing generators:

- `RecordingMarketSignalConfig` - spy that captures every `configure` call so
  tests can assert the declared dependencies.
- `FakeMarketSnapshot` / `FakeTimeframeSnapshot` / `FakeBar` - hand-build any
  market snapshot shape with a fluent, closure-based builder. Indicator
  values are injected directly (no real TA pipeline runs); tests exercise
  generator branching logic in isolation from indicator math.
- `EnforcingMarketSnapshot` - wraps a `FakeMarketSnapshot` with a recorder
  and panics on any read the generator didn't declare in `configure`.
  Catches `configure ↔ evaluate` drift at test time.
- `FakeEngine` - multi-tick driver. Constructs the generator via `Default`,
  runs `configure` once, runs `evaluate` per tick under
  `EnforcingMarketSnapshot`, returns one `Option<MarketSignal>` per tick.
  A passing run also proves the contract.

### Re-exports core + ta

Consumers import `quantedge_strategy::{...}` for everything: bar types,
timeframe enum, indicator configs, signal types. No need to also depend on
`quantedge-core` or `quantedge-ta` directly.

## Quickstart

A minimal EMA-cross generator:

```rust
use quantedge_strategy::{
    Bar, EmaConfig, MarketSide, MarketSignal, MarketSignalConfig, MarketSnapshot,
    SignalGenerator, Timeframe, TimeframeSnapshot, nz,
};

#[derive(Default)]
pub struct MyEmaCross {
    ema9: EmaConfig,
    ema21: EmaConfig,
}

impl SignalGenerator for MyEmaCross {
    fn id(&self) -> &'static str { "my_ema_cross" }
    fn name(&self) -> &'static str { "EMA9/21 cross" }

    fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
        config
            .require_closed_bars(1)
            .require_timeframes(&[Timeframe::HOUR_4])
            .register(&self.ema9)
            .register(&self.ema21)
    }

    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
        let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);
        let prev = h4.closed(0)?;
        let forming = h4.forming();

        let prev_fast = prev.value(&self.ema9)?;
        let prev_slow = prev.value(&self.ema21)?;
        let cur_fast = forming.value(&self.ema9)?;
        let cur_slow = forming.value(&self.ema21)?;

        if prev_fast < prev_slow && cur_fast > cur_slow {
            Some(MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "bull")
                .with_side(MarketSide::Long)
                .build())
        } else {
            None
        }
    }
}

# fn main() {
let _ = MyEmaCross { ema9: EmaConfig::close(nz(9)), ema21: EmaConfig::close(nz(21)) };
# }
```

Default field values are baked into `Default`; the engine constructs each
generator with `T::default()` before calling `configure`.

## Examples

Four worked examples live in the [`example/`](example/) sub-crate, each one
a complete generator plus its tutorial-style test block:

| Example | Pattern | Axis it teaches |
|---|---|---|
| e01 | EMA cross, intra-bar trigger | Baseline shape |
| e02 | HTF trend filter on LTF cross | Multi-timeframe |
| e03 | EMA cross, close-of-bar trigger | `MarketSignal::from_closed`, two adjacent closed bars |
| e04 | Bollinger Bands breakout | Composite indicator output (`BbValue` struct) |

Start with e01. The other three are independent variations and can be read
in any order. See [`example/README.md`](example/README.md) for the full
reading guide.

## Test utilities

To use the test harness in your own crate, enable the `test-util` feature:

```toml
[dev-dependencies]
quantedge-strategy = { version = "0.0.1", features = ["test-util"] }
```

The harness lives at `quantedge_strategy::test_util` and is the canonical
way to test a generator. The example crate's tests (linked above) double as
a worked tour of every helper.

## Out of scope

- **Implementing custom indicators.** Indicators belong in
  [`quantedge-ta`](https://crates.io/crates/quantedge-ta), where the engine
  can manage their state via the `Indicator` trait. When no shipped
  indicator fits a strategy's needs, add a new one there — don't pull
  rolling math into `evaluate`.
- **Stateful generators.** The `SignalGenerator` trait is `&self` by design.
  State lives in indicators. Reaching for `Mutex<…>` inside a generator is a
  sign the logic should be extracted into a custom indicator.
- **Engine implementation.** The engine that actually drives generators in
  production — wasmtime registration, scheduling, instrument multiplexing,
  bar feeding — is not yet shipped. This crate defines only the contract.

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

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this crate by you, as defined in the Apache-2.0
licence, shall be dual-licensed as above, without any additional terms or
conditions.
