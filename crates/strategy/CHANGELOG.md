# Changelog

## [Unreleased]

## [0.0.1] - 2026-05-09

Initial preview release. The engine that consumes `SignalGenerator` is not
yet implemented; the trait shape is informed by the in-crate test harness
but has not been validated end-to-end by a real engine. Each `0.0.x` bump
during the preview period should be assumed breaking.

### Added

- `SignalGenerator` trait — the customer-facing contract. Required methods: `id`, `name`, `configure`, `evaluate`. Supertrait bound: `Default + Sync + Send`. Stateless by design (`evaluate(&self, snapshot)` — no `&mut self`). The engine constructs each generator with `T::default()` before calling `configure`.
- `MarketSignalConfig` trait — the declarative dependency-declaration DSL passed into `configure`. Three primitives: `require_timeframes(&[..])`, `require_closed_bars(N)`, `register(&IndicatorConfig)`. The engine guarantees declared data is present when `evaluate` runs.
- `MarketSignal` output type with `MarketSide`, `SignalReason`, and the `MarketSignalBuilder` produced by `MarketSignal::from_forming(...)` / `MarketSignal::from_closed(...)`. Reasons form a deduplicated set keyed by `SignalReason::id`.
- `test-util` Cargo feature exposing the `test_util` module for unit-testing generators without a live engine:
  - `RecordingMarketSignalConfig` — `MarketSignalConfig` spy that captures every call so tests can assert declared timeframes, closed-bar budget, and registered indicators.
  - `FakeMarketSnapshot` / `FakeTimeframeSnapshot` / `FakeBar` — fluent, closure-based builders for hand-built market snapshots. Indicator values are injected directly (no real TA pipeline runs); timestamps default to midnight 2026-01-01 UTC, snapped to each timeframe boundary; closed bars chain backwards via `add_closed_with` / `add_closed_value` / `add_closed_prices` / `add_closed_ohlc` / `add_closed_ohlcv`. `forming_with` / `forming_value` cover the forming-bar shape; `with_close` is the in-closure setter for tests that read both `bar.ohlcv().close` and `bar.value(&cfg)`.
  - `EnforcingMarketSnapshot` — wraps a `FakeMarketSnapshot` with a `RecordingMarketSignalConfig` and panics on every read the generator didn't declare in `configure` (unregistered timeframe, closed-bar index past `require_closed_bars(N)`, unregistered indicator). Catches `configure ↔ evaluate` drift at test time, with actionable panic messages naming the offending input and pointing at the missing `configure(..)` call.
  - `FakeEngine` — multi-tick driver. `FakeEngine::btcusdt().tick(|s| ...).execute::<G>()` constructs the generator via `Default`, runs `configure` once, runs `evaluate` per tick under `EnforcingMarketSnapshot`, returns one `Option<MarketSignal>` per tick. A passing run also proves the contract.
- Re-exports from `quantedge-core` (bar/snapshot/timeframe/instrument types, `Indicator` trait surface) and `quantedge-ta` (every shipped indicator config + value type) so consumers import everything from `quantedge_strategy::{...}` without depending on the lower crates directly.
- Four worked examples in the [`example/`](example/) sub-crate, each a complete generator plus a tutorial-style test block: e01 (EMA cross, intra-bar trigger), e02 (HTF trend filter on LTF cross), e03 (EMA cross, close-of-bar trigger), e04 (Bollinger Bands breakout — composite indicator output).
