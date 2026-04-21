# quantedge

Streaming-first Rust primitives for systematic trading. Zero-alloc hot paths.

Today: 19 streaming indicators in `quantedge-ta`, verified against talipp, TA-Lib, and pandas-ta, benchmarked with Criterion. MSRV 1.93.

## Status

| Crate              | Version       | Status        | Purpose                                              |
|--------------------|---------------|---------------|------------------------------------------------------|
| `quantedge-ta`     | `0.18.1`      | pre-publish   | Streaming technical analysis indicators              |
| `quantedge-core`   | `0.1.0`       | published     | Shared structs, traits, and public interfaces        |
| `quantedge-engine` | —             | planned       | Streaming runtime — event loop, multi-TF state, execution |
| `quantedge-sim`    | —             | planned       | Backtester and forward-tester with honest fill models |
| `quantedge-ob`     | —             | planned       | Order book reconstruction and L2/L3 event handling   |

`quantedge-ta` is feature-complete for v1.0. Publish to crates.io is gated on validation under real streaming workloads.

## Design principles

- **Streaming-first.** Every indicator processes one tick at a time with O(1) update cost. Batch APIs are built on top of the streaming primitives, not the other way around.
- **Zero-alloc hot paths.** No hidden heap work inside the update loop. Runs in the browser via `wasm32-unknown-unknown` (verified in CI).
- **Numerical correctness.** NaN, Inf, and warmup semantics are documented per-indicator, not glossed over. `quantedge-ta` is verified against talipp, TA-Lib, and pandas-ta.
- **Benchmark-driven.** Every performance claim has a `criterion` bench behind it. No hand-waving.

## Quick start

See [`crates/ta/README.md`](crates/ta/README.md) for indicator usage and [`crates/ta/example/`](crates/ta/example/) for a runnable WebSocket streaming demo (Binance BTC/USDT 5m klines with SMA(20) and intra-bar repaints).

## Writeups

Design notes and per-indicator writeups at [luksza.org/category/quantedge](https://luksza.org/category/quantedge/):

- [Why the Forming Bar Makes It Hard](https://luksza.org/2026/why-the-forming-bar-makes-it-hard/) — 2026-04-13
- [Donchian Channels: When the Infrastructure Does All the Work](https://luksza.org/2026/donchian-channels-when-the-infrastructure-does-all-the-work/) — 2026-04-05
- [quantedge-ta: Real-Time Technical Analysis for Rust](https://luksza.org/2026/quantedge-ta-real-time-technical-analysis-for-rust/) — 2026-03-08
- [5 Years, 3 Languages, One Trading System](https://luksza.org/2026/5-years-3-languages-one-trading-system/) — 2026-02-28

## Repository layout

```
crates/
  core/         # shared types (Ohlcv, Price, Timestamp)
  ta/           # technical analysis indicators
    example/    # runnable WebSocket demo
```

More crates (`engine`, `sim`, `ob`) will land here as they're built.

## Roadmap

1. `quantedge-ta` v1.0 — publish to crates.io once validated under streaming workloads
2. `quantedge-core` — shared structs, traits, and public interfaces
3. `quantedge-engine` — streaming runtime with multi-timeframe state and deterministic event loop
4. `quantedge-sim` — backtester with realistic fills, slippage, and drawdown reporting
5. `quantedge-ob` — streaming order book with deterministic event replay

## Contributing

Contributions welcome. Please open an issue to discuss before submitting large changes.

Contributions are accepted under the licence of the crate being modified. Unless you explicitly state otherwise, your contribution shall be licensed under the same terms as that crate, without any additional terms or conditions.

## License

Each crate in this workspace carries its own licence. See the `LICENSE-*` files in each crate directory for the full text.

Today, every published crate is dual-licensed under the Apache License 2.0 and the MIT License, at your option.
