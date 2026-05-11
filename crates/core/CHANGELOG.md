# Changelog

## [0.4.0](https://github.com/dluksza/quantedge/compare/quantedge-core-v0.3.0...quantedge-core-v0.4.0) (2026-05-11)


### Features

* **core:** add Indicator associated type to IndicatorConfig ([a6456d6](https://github.com/dluksza/quantedge/commit/a6456d6b5effa174c8c6fca5537b47866c99546e))

## [0.3.0] - 2026-05-05

### Added

- `nz(n: usize) -> NonZero<usize>` `const fn` at the crate root. Indicator config call sites such as `EmaConfig::builder().length(nz(9)).build()` are not test-only, so the previous gating behind the `test-util` feature forced production code to either enable a test feature or repeat `NonZero::new(n).unwrap()` inline. The existing `quantedge_core::test_util::nz` import path keeps working via re-export.

### Changed

- MSRV raised from 1.93 to 1.95. Workspace `rust-toolchain.toml` and CI jobs pinned to 1.95. Enables stabilizations like `core::hint::cold_path` and if-let match guards.
- **Breaking:** `IndicatorConfig` now requires `Clone + Send + Sync + 'static` in addition to its previous bounds. Existing config types in `quantedge-ta` already satisfy these bounds; custom impls that don't will need to add them.
- **Breaking:** `IndicatorConfig::Output` (and therefore `Indicator::Output`) now requires `PartialEq`. Lets callers compare snapshot values directly without workarounds (test assertions, deduplication, change detection). The redundant `Clone` bound was dropped at the same time — `Copy` already implies it. Net bound: `Copy + PartialEq + Display + Debug + Send + Sync + 'static`. Custom output types must derive or implement `PartialEq`; every built-in indicator output already does.
- **Breaking:** `Bar::ohlcv()` returns `Ohlcv` by value instead of `&Ohlcv`. `Ohlcv` is `Copy`, and the by-reference signature forced lifetimes through a non-dyn-compatible trait surface that downstream builders (`MarketSignal`) need to capture.
- **Breaking:** `MarketSnapshot::instrument()` returns `Instrument` by value instead of `&Instrument`. `Instrument` clones are four atomic increments (`Arc<str>` leaves), and the by-value signature lets builders capture it without threading lifetimes.
- **Breaking:** `MarketSnapshot::for_timeframe()` takes `Timeframe` by value instead of `&Timeframe`. `Timeframe` is `Copy`; the by-reference signature forced callers to write `&timeframe` and the implementation to dereference for no benefit.

## [0.2.0] - 2026-04-24

### Added

- `Indicator`, `IndicatorConfig`, `IndicatorConfigBuilder` traits and `PriceSource` enum, relocated from `quantedge-ta` so downstream crates can depend on the trait surface without pulling in the full indicator library. `quantedge-ta` continues to re-export them at their existing paths, so no source changes are required for its consumers.
- `IndicatorConfig::Output` associated type. Pairs with the existing `Indicator::Output` so generic code can resolve an indicator's output from its config alone, without instantiating the indicator. Bound: `'static + Copy + Send + Sync + Display + Debug`.
- `Instrument` module: a typed subscription key composed of `Venue`, `Ticker` (a base/quote `Asset` pair), and `MarketKind`. Each leaf is an ASCII-validated, case-normalized newtype over `Arc<str>`; instrument clones are four atomic increments, cheap enough for log lines and strategy code. Grammar separators (`/`, `:`, `@`) are rejected at the leaf, so `Ticker::from_str` and `Instrument`'s `Display` cannot be broken by pathological input. Exports: `Asset`, `AssetError`, `Instrument`, `MarketKind`, `MarketKindError`, `Ticker`, `TickerError`, `Venue`, `VenueError`.
- Streaming snapshot traits (`Bar`, `TimeframeSnapshot`, `MarketSnapshot`) that define the surface strategy code reads at one tick. Each snapshot is immutable; successive ticks surface as new snapshots. Indexing: `at(0)` / `bars(0..)` = forming bar then closed history newest-first; `closed(0)` = most recent closed bar. Querying an unsubscribed indicator or timeframe panics — subscriptions are fixed at construction, so misses are caller bugs.
- `Timeframe` type and `TimeUnit` enum for mapping Unix-μs timestamps to bar boundaries. Supports seconds through years, with calendar-correct month and year arithmetic using Howard Hinnant's [`civil_from_days`](https://howardhinnant.github.io/date_algorithms.html) (no `chrono` at runtime).
- `Timeframe::open_time(ts)` returns the start of the containing period, `close_time(ts)` the last μs before the next period starts (`close_time(t) + 1 == open_time` of the next period), and `bounds(ts)` returns both at once — sharing computation between the halves for a ~30% speedup on monthly/yearly dispatch.
- Predefined `Timeframe` constants for common bar sizes: `SEC_{1,5,10,15}`, `MIN_{1,3,5,15,30}`, `HOUR_{1,2,4,6,8,12}`, `DAY_{1,3,5}`, `WEEK_1`, `MONTH_{1,2,3,6}`, `YEAR_1`. Multi-month periods are epoch-anchored from January 1970, matching calendar quarters and halves for any N dividing 12.
- `Timeframe::new(count, unit)` constructor with automatic canonicalization (`60s → 1 minute`, `60min → 1 hour`, `24h → 1 day`, `7d → 1 week`, `12M → 1 year`), applied recursively.
- `Timeframe::count()` and `Timeframe::unit()` accessors.
- `Display` impl for `Timeframe` using Binance-style compact notation (`5m`, `1h`, `1d`, `1w`, `3M`, `1Y`); uppercase `M`/`Y` disambiguate month/year from minute. Reads post-canonicalization values, so `Timeframe::new(NonZero::new(120).unwrap(), TimeUnit::Second)` renders as `2m`.
- `Debug`, `Clone`, `Copy`, and `PartialEq` derives on `Ohlcv`.
- `test-util` Cargo feature exposing the `test_util` module: `Ohlcv::new` / `at` / `vol` builder helpers, the `assert_approx!` macro, the `nz` `NonZero` constructor, and the `bar` / `ohlc` / `bar_at` convenience helpers. Gated so the helpers do not leak into the stable public API — production callers build `Ohlcv` with a struct literal or a `From` conversion.

### Changed

- **Breaking:** `Ohlcv` is now a concrete struct with public fields (`open`, `high`, `low`, `close`, `open_time`, `volume`) instead of a trait. Callers build an `Ohlcv` per bar and pass it by reference — no more `impl Ohlcv for MyKline`. Removes the dynamic-dispatch / generic-parameter surface on every indicator signature and makes hot paths direct field reads. Migration: replace trait impls with a conversion that produces an `Ohlcv`.
- **Breaking:** `Indicator::Config` is now constrained as `IndicatorConfig<Output = Self::Output>`, so a single `Output` type flows across the config/indicator pair. Custom `Indicator` impls that previously declared a divergent `Config::Output` no longer compile; set them to the same type.
- **Breaking:** Removed `Ord` and `PartialOrd` derives from `Timeframe`. There is no meaningful ordering between, for example, a 1-month and a 30-day timeframe, and lexicographic order over the `(unit, count, period)` tuple exposed a misleading default.

### Fixed

- `Timeframe::open_time` / `close_time` / `bounds` no longer underflow on `Day` and `Week` units when called with `timestamp < EPOCH_TO_MONDAY_OFFSET` (Jan 5 1970 00:00 UTC). Debug builds now assert the precondition; release builds previously wrapped silently.

## [0.1.0] - 2026-04-21

### Added

- Initial release. Defines the `Ohlcv` bar trait and its `Price` and `Timestamp` aliases, extracted from `quantedge-ta` so downstream crates can share a single bar abstraction without depending on the full indicator library.

[0.3.0]: https://github.com/dluksza/quantedge/releases/tag/quantedge-core-v0.3.0
[0.2.0]: https://github.com/dluksza/quantedge/releases/tag/quantedge-core-v0.2.0
[0.1.0]: https://github.com/dluksza/quantedge/releases/tag/quantedge-core-v0.1.0
