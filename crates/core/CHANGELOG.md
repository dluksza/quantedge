# Changelog

## [Unreleased]

### Added

- `Timeframe` type and `TimeUnit` enum for mapping Unix-μs timestamps to bar boundaries. Supports seconds through years, with calendar-correct month and year arithmetic using Howard Hinnant's [`civil_from_days`](https://howardhinnant.github.io/date_algorithms.html) (no `chrono` at runtime).
- `Timeframe::open_time(ts)` returns the start of the containing period, `close_time(ts)` the last μs before the next period starts (`close_time(t) + 1 == open_time` of the next period), and `bounds(ts)` returns both at once — sharing computation between the halves for a ~30% speedup on monthly/yearly dispatch.
- Predefined constants for common bar sizes: `SEC_{1,5,10,15}`, `MIN_{1,3,5,15,30}`, `HOUR_{1,2,4,6,8,12}`, `DAY_{1,3,5}`, `WEEK_1`, `MONTH_{1,2,3,6}`, `YEAR_1`. Multi-month periods are epoch-anchored from January 1970, matching calendar quarters and halves for any N dividing 12.
- `Timeframe::new(count, unit)` constructor with automatic canonicalization (`60s -> 1 minute`, `60min -> 1 hour`, `24h -> 1 day`, `7d → -> week`, applied recursively).
- `Timeframe::count()` and `Timeframe::unit()` accessors.

## [0.1.0] - 2026-04-21

### Added

- Initial release. Defines the `Ohlcv` bar trait and its `Price` and `Timestamp` aliases, extracted from `quantedge-ta` so downstream crates can share a single bar abstraction without depending on the full indicator library.

[0.1.0]: https://github.com/dluksza/quantedge/releases/tag/quantedge-core-v0.1.0
