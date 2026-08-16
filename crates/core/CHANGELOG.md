# Changelog

## [0.4.0](https://github.com/dluksza/tickglide/compare/tickglide-core-v0.3.0...tickglide-core-v0.4.0) (2026-08-16)


### Features

* add ErasedIndicatorConfig trait ([0009ba8](https://github.com/dluksza/tickglide/commit/0009ba88717b79fc4e1fa3a3e3bf9cd066b3bb6c))
* add ErasedIndicatorOutput, hide erased plumbing ([d7d6709](https://github.com/dluksza/tickglide/commit/d7d670974c68e3ff87464b0cca7564fd0fced435))
* add Instrument identifier types ([81c1142](https://github.com/dluksza/tickglide/commit/81c11421f8797452f5fd2966a41ae60c22e63a21))
* add MarketSignal builder API ([7afe15f](https://github.com/dluksza/tickglide/commit/7afe15fcaf250aee5342a10ae2e8dcbd715c187a))
* add quantedge-core crate for shared types ([ec4c153](https://github.com/dluksza/tickglide/commit/ec4c15328da443e78c04903ef47612a2d026df0f))
* add streaming snapshot traits ([76ff747](https://github.com/dluksza/tickglide/commit/76ff747da521b9267f302ffce44218bfecf71388))
* add Timeframe for bar boundary alignment ([25e5a4e](https://github.com/dluksza/tickglide/commit/25e5a4eebcefed33ba432b1f84a0f617f9cd0df0))
* canonicalize 12 months to 1 year in Timeframe::new ([77c1fd2](https://github.com/dluksza/tickglide/commit/77c1fd21661c44f001ba4bd57ae5f0a024d6e303))
* cap ascii_ident length at 32 bytes ([c1d3487](https://github.com/dluksza/tickglide/commit/c1d3487f58de47f2410a1b5a9346d48264bc600d))
* **core:** add ErasedIndicator object-safe trait ([627a539](https://github.com/dluksza/tickglide/commit/627a5397b1457ca26885cf3a7201d59c05746af8))
* **core:** add Indicator associated type to IndicatorConfig ([a6456d6](https://github.com/dluksza/tickglide/commit/a6456d6b5effa174c8c6fca5537b47866c99546e))
* implement Display for Timeframe ([9891cb7](https://github.com/dluksza/tickglide/commit/9891cb7cca4e97947e100dc87c00be0b20c054e6))
* promote nz helper to public quantedge_core API ([77cc686](https://github.com/dluksza/tickglide/commit/77cc6868e69c773197b7e789cbb3c0f394caba9e))
* require PartialEq on indicator outputs and bars ([58eff5f](https://github.com/dluksza/tickglide/commit/58eff5f1ee3c53115503494fc1724dd7318bc633))


### Bug Fixes

* debug-assert period_months fits in u32 ([14bb3ed](https://github.com/dluksza/tickglide/commit/14bb3ed792e23d5bcf481d7276efa1fdf0285447))
* drop Ord/PartialOrd derive from Timeframe ([d40d067](https://github.com/dluksza/tickglide/commit/d40d0678a68601fa88eea0d39df7a862183c7b53))
* guard Day/Week open/close against pre-epoch underflow ([68bfbe8](https://github.com/dluksza/tickglide/commit/68bfbe85bf62e26d5881f1b1ae1b432d478334f9))


### Performance Improvements

* cache period and add bounds() to Timeframe ([10d1c47](https://github.com/dluksza/tickglide/commit/10d1c47ad8c5aa19763dde4a28d2c8709671b9bc))
* relocate price extraction to ta internals ([a69a4f6](https://github.com/dluksza/tickglide/commit/a69a4f6e2679c974d7bb01382e9598b1db2a1644))

## [0.3.0] - 2026-05-05

### Added

- `nz(n: usize) -> NonZero<usize>` `const fn` at the crate root. Indicator config call sites such as `EmaConfig::builder().length(nz(9)).build()` are not test-only, so the previous gating behind the `test-util` feature forced production code to either enable a test feature or repeat `NonZero::new(n).unwrap()` inline. The existing `tickglide_core::test_util::nz` import path keeps working via re-export.

### Changed

- MSRV raised from 1.93 to 1.95. Workspace `rust-toolchain.toml` and CI jobs pinned to 1.95. Enables stabilizations like `core::hint::cold_path` and if-let match guards.
- **Breaking:** `IndicatorConfig` now requires `Clone + Send + Sync + 'static` in addition to its previous bounds. Existing config types in `tickglide-ta` already satisfy these bounds; custom impls that don't will need to add them.
- **Breaking:** `IndicatorConfig::Output` (and therefore `Indicator::Output`) now requires `PartialEq`. Lets callers compare snapshot values directly without workarounds (test assertions, deduplication, change detection). The redundant `Clone` bound was dropped at the same time — `Copy` already implies it. Net bound: `Copy + PartialEq + Display + Debug + Send + Sync + 'static`. Custom output types must derive or implement `PartialEq`; every built-in indicator output already does.
- **Breaking:** `Bar::ohlcv()` returns `Ohlcv` by value instead of `&Ohlcv`. `Ohlcv` is `Copy`, and the by-reference signature forced lifetimes through a non-dyn-compatible trait surface that downstream builders (`MarketSignal`) need to capture.
- **Breaking:** `MarketSnapshot::instrument()` returns `Instrument` by value instead of `&Instrument`. `Instrument` clones are four atomic increments (`Arc<str>` leaves), and the by-value signature lets builders capture it without threading lifetimes.
- **Breaking:** `MarketSnapshot::for_timeframe()` takes `Timeframe` by value instead of `&Timeframe`. `Timeframe` is `Copy`; the by-reference signature forced callers to write `&timeframe` and the implementation to dereference for no benefit.

## [0.2.0] - 2026-04-24

### Added

- `Indicator`, `IndicatorConfig`, `IndicatorConfigBuilder` traits and `PriceSource` enum, relocated from `tickglide-ta` so downstream crates can depend on the trait surface without pulling in the full indicator library. `tickglide-ta` continues to re-export them at their existing paths, so no source changes are required for its consumers.
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

- Initial release. Defines the `Ohlcv` bar trait and its `Price` and `Timestamp` aliases, extracted from `tickglide-ta` so downstream crates can share a single bar abstraction without depending on the full indicator library.

[0.3.0]: https://github.com/dluksza/tickglide/releases/tag/quantedge-core-v0.3.0
[0.2.0]: https://github.com/dluksza/tickglide/releases/tag/quantedge-core-v0.2.0
[0.1.0]: https://github.com/dluksza/tickglide/releases/tag/quantedge-core-v0.1.0
