# Changelog

## [Unreleased]

### Added

- Williams %R (WillR) — a momentum oscillator measuring overbought/oversold levels on a −100 to 0 scale. Compares the current price to the highest high over the lookback window. Returns `f64`. Convenience constructor `WillRConfig::close()` and `length()` accessor.

## [0.8.0] - 2026-03-19

### Added

- Average Directional Index (ADX) — measures trend strength on a 0–100 scale with +DI and −DI directional indicators. Uses Wilder's smoothing (`α = 1/length`). Returns `AdxValue { adx, plus_di, minus_di }`. Reference tests against talipp (717 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

### Changed

- ADX hot path optimized (~5% improvement): precomputed `100/smooth_tr` reciprocal replaces two divisions with one division + two multiplications; reuses push/replace return values to eliminate a redundant branch per tick.

## [0.7.0] - 2026-03-17

### Added

- Donchian Channel (DC) — tracks highest high and lowest low over a rolling window. Returns upper, middle, and lower bands. Default length 20. Reference tests against talipp (725 BTC/USDT bars, 1e-10 tolerance) and Criterion benchmarks.

## [0.6.0] - 2026-03-15

### Added

- Keltner Channel (KC) — EMA-based centre line with ATR-scaled upper/lower bands. Configurable EMA length, ATR length, and band multiplier. Reference tests against talipp (725 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.
- `convergence()` method on `IndicatorConfig` trait — returns the number of bars until `compute()` first returns `Some`. Each indicator implements its own formula.

### Changed

- **Breaking:** Removed `EmaConfig::enforce_convergence()` and `EmaConfigBuilder::enforce_convergence()` — convergence policy is the engine's responsibility, not the indicator's. Use `EmaConfig::full_convergence()` to query how many bars are needed for the seed's influence to decay below 1%.
- **Breaking:** Removed `EmaConfig::required_bars_to_converge()` — use `convergence()` instead.
- **Breaking:** Removed `MacdConfig::convergence_bars()` — use `convergence()` instead.
- **Breaking:** Renamed `MacdConfig::full_convergence_bars()` to `full_convergence()`.
- **Breaking:** Renamed `MacdConfig` period accessors/builders to use `length` terminology: `fast_period()` → `fast_length()`, `slow_period()` → `slow_length()`, `signal_period()` → `signal_length()`.

## [0.5.0] - 2026-03-14

### Added

- Stochastic Oscillator (%K, %D) with reference tests against talipp (729 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

## [0.4.0] - 2026-03-07

### Added

- ATR (Average True Range) with Wilder's smoothing, reference tests against talipp (731 BTC/USDT bars, 1e-6 tolerance), and Criterion benchmarks.

### Changed

- Extracted `BarState` from duplicated bar-boundary detection logic in `PriceWindow`, `Ema`, and `Macd`. Centralizes `open_time` tracking, `prev_close` management, and the non-decreasing timestamp assertion into a single reusable internal type.
- SMA, EMA, and BB performance improved 46–65% (stream), 44–79% (tick), 46–79% (repaint), 42–60% (repaint stream) vs v0.3.0 due to `BarState` extraction enabling better code generation.

## [0.3.0] - 2026-03-03

### Added

- WASM support: library compiles for `wasm32-unknown-unknown` and full test suite runs on `wasm32-wasip1` via wasmtime.
- CI job verifying WASM compatibility on every push and PR.
- MACD (Moving Average Convergence Divergence) with reference tests against talipp (711 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

### Changed

- Removed `length()` from `IndicatorConfig` and `IndicatorConfigBuilder` traits — not all indicators have a single length (e.g. MACD). Each config type still exposes its own length accessor(s) as inherent methods.
- EMA performance improved 26-34% (stream), 43-49% (tick/repaint), 24-34% (repaint stream) vs v0.2.0 by eliminating redundant state in `EmaCore` hot path.

## [0.2.0] - 2026-02-27

### Added

- RSI (Relative Strength Index) with Wilder's smoothing, unit tests, Criterion benchmarks, and reference tests against talipp (730 BTC/USDT bars, 1e-6 tolerance).

### Changed

- `Sma`, `Ema`, and `Bb` now expose `new()`, `compute()`, and `value()` as inherent methods, `use quantedge_ta::Indicator` is no longer required for basic usage.
- Config and builder types (`SmaConfig`, `EmaConfig`, `BbConfig` and their builders) now expose trait methods as inherent methods. `use quantedge_ta::IndicatorConfig` and `use quantedge_ta::IndicatorConfigBuilder` are no longer required for basic usage.
- Benchmark harness: deterministic codegen (`codegen-units = 1`, `lto = "thin"`), lower-overhead batching (`SmallInput`), and tuned tick-group sampling (200 samples, 5s warmup, 10s measurement, 3% noise threshold).
- `PriceWindow` now uses a `const SUM_OF_SQUARES: bool` generic so SMA no longer computes unused sum-of-squares on every tick.
- Replaced `VecDeque` with a custom `RingBuffer` in `PriceWindow`, halves buffer memory (`f64` vs `Option<f64>`), eliminates redundant modulo operations, and improves inlining. SMA and BB stream throughput improved 24-33%, tick latency improved 8-31%.

## [0.1.0] - 2025-02-20

Initial release.

### Added

- SMA (Simple Moving Average)
- EMA (Exponential Moving Average) with configurable convergence enforcement
- Bollinger Bands (upper, middle, lower)
- `Ohlcv` trait for zero-copy integration with user types
- 9 price sources (Close, Open, High, Low, HL2, HLC3, OHLC4, HLCC4, TrueRange)
- Live bar repainting via `open_time` comparison
- Reference tests against 744 BTC/USDT bars
- Criterion benchmarks (stream + tick)

[0.8.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.8.0
[0.7.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.7.0
[0.6.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.6.0
[0.5.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.5.0
[0.4.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.4.0
[0.3.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.3.0
[0.2.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.2.0
[0.1.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.1.0
