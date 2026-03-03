# Changelog

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

[0.3.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.3.0
[0.2.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.2.0
[0.1.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.1.0
