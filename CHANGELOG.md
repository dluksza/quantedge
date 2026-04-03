# Changelog

## [Unreleased]

### Added

- Volume Weighted Average Price (VWAP) — cumulative price-volume average that resets on a configurable anchor period (1h, 2h, 4h, 8h, 12h, Day, or manual User reset). Up to three standard-deviation bands. Default source HLC3, anchor Day. Returns `VwapValue { vwap, band_1, band_2, band_3 }`. Unit tests covering convergence, computation, bands, anchor reset, repaints, live data, clone, config, display, and value accessor.

### Changed

- `Timestamp` docs now recommend microseconds since Unix epoch (required for VWAP session boundary detection).
- `Ohlcv::volume()` docs clarify that overriding the default is **required** when using OBV or VWAP; the default `0.0` produces meaningless results.

## [0.14.0] - 2026-04-02

### Added

- On-Balance Volume (OBV) — cumulative volume indicator that adds volume on up-price bars and subtracts on down-price bars. Configurable price source (default Close). Returns `f64`. Convergence after 1 bar. Reference tests against talipp (744 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks. Unit tests covering convergence, computation, repaints, live data, clone, config, display, and value accessor.

## [0.13.0] - 2026-04-01

### Added

- Stochastic RSI (StochRSI) — applies the Stochastic Oscillator formula to RSI values, producing a 0–100 scale that measures whether RSI is near its recent high or low. Configurable RSI length, stochastic lookback, %K smoothing, and %D smoothing. Standard settings via `StochRsiConfig::default()` (14/14/3/3). Returns `StochRsiValue { k, d }`. Reference tests against talipp (713 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks. Unit tests covering convergence, computation, repaints, bounds, clone, config, display, and value accessor.
- `to_builder()` on `IndicatorConfig` trait — returns a builder pre-filled with the config's current values, allowing single-field overrides without reconstructing from scratch. Implemented for all 15 indicators.
- `Default` impl for all 15 indicator configs with industry-standard parameters (matching TradingView defaults and original author specifications). `Default` is now a supertrait bound on `IndicatorConfig`.

### Changed

- **Breaking:** `StochConfig::default()` now produces the **Slow Stochastic** (14/3/3) instead of the Fast Stochastic (14/1/3), matching TradingView's default "Stoch" indicator. Set `k_smooth` to 1 for the previous Fast Stochastic behavior.
- `ChopConfigBuilder::source()` no longer stores the argument — CHOP always derives from ATR internally, so the source field was dead state. Calling `source()` on the builder is now a no-op.
- `RollingExtremes` optimized: removed `forming_high`/`forming_low` fields by folding the forming bar into the tracked extreme directly, leveraging OHLCV monotonicity (high only increases, low only decreases during bar formation). `highest_high()`/`lowest_low()` are now pure field reads. Stream throughput improved 2–12% and repaint stream throughput improved 1–12% for all indicators using `RollingExtremes` (Stoch, DC, WillR, CHOP, Ichimoku). Internal-only change, no public API affected.

### Fixed

- Stoch: repainting a bar while `k_sum` was still filling (when `k_smooth > 1`) skipped `k_sum.replace()` because it was gated behind `current.map()`. Stale intermediate values persisted in `k_sum`, corrupting smoothed %K after convergence. Moved `k_sum.replace()` outside the convergence gate so it fires whenever the extremes window is ready. Also removed unused `RollingExtremes::extremes()`. Stoch stream throughput improved 4–8%, tick/repaint latency improved 8–10%.

## [0.12.0] - 2026-03-28

### Added

- Ichimoku Cloud (Ichimoku Kinko Hyo) — comprehensive trend indicator producing five lines: Tenkan-sen (conversion), Kijun-sen (base), Senkou Span A/B (cloud boundaries), and Chikou close (lagging span input). Configurable lookback windows and displacement. Standard settings via `IchimokuBuilder::default()` (9/26/52/26). Returns `IchimokuValue { tenkan, kijun, senkou_a, senkou_b, chikou_close }`. Reference tests against talipp (666 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks. Unit tests covering filling, computation, sliding, repaint, live data, clone, config, display, and value accessor.

### Changed

- Encoded convergence in `RollingExtremes` and `RollingSum` return types — `push()`/`replace()` now return `Option` so callers use `?`-propagation instead of separate `is_ready()` guards. Simplifies convergence logic in CHOP, DC, Stoch, WillR, and Ichimoku. Internal-only change, no public API affected.

## [0.11.0] - 2026-03-26

### Added

- Choppiness Index (CHOP) — measures whether the market is trending or ranging on a 0–100 scale. Uses the ratio of the sum of True Range over a window to the highest-high minus lowest-low range, scaled by log10(length). Higher values indicate a choppy, sideways market; lower values indicate a strong trend. Returns `f64`. Convenience constructor `ChopConfig::close()` and `length()` accessor. Reference tests against talipp (731 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

### Changed

- Removed `#[inline]` annotations from generic, private, and cold-path methods across all indicators and internals. Kept only on non-generic pub `value()` impls and output struct accessors where it enables cross-crate inlining. No effect on hot-path performance.

## [0.10.0] - 2026-03-22

### Added

- Commodity Channel Index (CCI) — measures deviation of price from its statistical mean, scaled by mean absolute deviation. Uses the traditional 0.015 constant so ~70–80% of values fall between −100 and +100. Default source is HLC3 (typical price). Convenience constructors `CciConfig::close()` and `CciConfig::hlc3()`. Returns `f64`. Reference tests against talipp (725 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

## [0.9.0] - 2026-03-21

### Added

- Williams %R (WillR) — a momentum oscillator measuring overbought/oversold levels on a −100 to 0 scale. Compares the current price to the highest high over the lookback window. Returns `f64`. Convenience constructor `WillRConfig::close()` and `length()` accessor. Reference tests against talipp (731 BTC/USDT bars, 1e-6 tolerance) and Criterion benchmarks.

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

[0.14.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.14.0
[0.13.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.13.0
[0.12.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.12.0
[0.11.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.11.0
[0.10.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.10.0
[0.9.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.9.0
[0.8.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.8.0
[0.7.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.7.0
[0.6.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.6.0
[0.5.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.5.0
[0.4.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.4.0
[0.3.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.3.0
[0.2.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.2.0
[0.1.0]: https://github.com/dluksza/quantedge-ta/releases/tag/v0.1.0
