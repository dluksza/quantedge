# quantedge

Streaming-first Rust primitives for systematic trading. Zero-alloc hot paths.

Today: 21 streaming indicators in `quantedge-ta`, verified against talipp, TA-Lib, and pandas-ta, benchmarked with Criterion; the `quantedge-strategy` trait surface for writing signal generators is now published as a `0.0.1` preview alongside. MSRV 1.95.

## Status

| Crate              | Version       | Status        | Purpose                                              |
|--------------------|---------------|---------------|------------------------------------------------------|
| `quantedge-core`     | [![crates.io](https://img.shields.io/crates/v/quantedge-core.svg)](https://crates.io/crates/quantedge-core) | published     | Shared structs, traits, and public interfaces        |
| `quantedge-ta`       | [![crates.io](https://img.shields.io/crates/v/quantedge-ta.svg)](https://crates.io/crates/quantedge-ta) | published     | Streaming technical analysis indicators              |
| `quantedge-strategy` | [![crates.io](https://img.shields.io/crates/v/quantedge-strategy.svg)](https://crates.io/crates/quantedge-strategy) | preview       | User-facing API for authoring strategies executed by the engine |
| `quantedge-engine`   | —             | planned       | Streaming runtime — event loop, multi-TF state, execution |
| `quantedge-sim`      | —             | planned       | Backtester and forward-tester with honest fill models |
| `quantedge-ob`       | —             | planned       | Order book reconstruction and L2/L3 event handling   |

`quantedge-ta` (`0.21.0`) and `quantedge-core` (`0.3.0`) are published on crates.io. Both are feature-complete for v1.0; the v1.0 bump is gated on validation under real streaming workloads.

`quantedge-strategy` (`0.0.1`) ships the `SignalGenerator` trait surface plus a complete `test_util` kit (`FakeMarketSnapshot`, `RecordingMarketSignalConfig`, `EnforcingMarketSnapshot`, `FakeEngine`) for unit-testing generators without a live engine. Released as a **preview**: the engine that consumes the trait is not yet implemented, so the contract has not been validated end-to-end. Pin to an exact `0.0.x` version during the preview period — each bump should be assumed breaking. The `0.1.0` cut is gated on engine consumption.

## Design principles

- **Streaming-first.** Every indicator processes one tick at a time with O(1) update cost. Batch APIs are built on top of the streaming primitives, not the other way around.
- **Zero-alloc hot paths.** No hidden heap work inside the update loop. Runs in the browser via `wasm32-unknown-unknown` (verified in CI).
- **Numerical correctness.** NaN, Inf, and warmup semantics are documented per-indicator, not glossed over. `quantedge-ta` is verified against talipp, TA-Lib, and pandas-ta.
- **Benchmark-driven.** Every performance claim has a `criterion` bench behind it. No hand-waving.

## Quick start

- Indicators: [`crates/ta/README.md`](crates/ta/README.md), with a runnable WebSocket streaming demo at [`crates/ta/example/`](crates/ta/example/) (Binance BTC/USDT 5m klines with SMA(20) and intra-bar repaints).
- Signal generators: [`crates/strategy/README.md`](crates/strategy/README.md), with four worked examples at [`crates/strategy/example/`](crates/strategy/example/) covering single-timeframe forming triggers, multi-timeframe filtering, close-of-bar triggers, and composite indicator outputs.

## Writeups

Design notes and per-indicator writeups at [luksza.org/category/quantedge](https://luksza.org/category/quantedge/):

- [Why the Forming Bar Makes It Hard](https://luksza.org/2026/why-the-forming-bar-makes-it-hard/) — 2026-04-13
- [Donchian Channels: When the Infrastructure Does All the Work](https://luksza.org/2026/donchian-channels-when-the-infrastructure-does-all-the-work/) — 2026-04-05
- [quantedge-ta: Real-Time Technical Analysis for Rust](https://luksza.org/2026/quantedge-ta-real-time-technical-analysis-for-rust/) — 2026-03-08
- [5 Years, 3 Languages, One Trading System](https://luksza.org/2026/5-years-3-languages-one-trading-system/) — 2026-02-28

## Repository layout

```
crates/
  core/         # shared types (Ohlcv struct, Price, Timestamp, Timeframe)
  ta/           # technical analysis indicators
    example/    # runnable WebSocket demo
  strategy/     # SignalGenerator trait + test harness (re-exports core + ta surface)
    example/    # four worked example generators (e01-e04) doubling as test_util tutorial
```

More crates (`engine`, `sim`, `ob`) will land here as they're built.

## Roadmap

1. `quantedge-core` v1.0 - currently at `0.3.0`; v1.0 cut once validated under real streaming workloads
2. `quantedge-ta` v1.0 - currently at `0.21.0`; v1.0 cut on the same gate
3. `quantedge-strategy` v0.1.0 - currently at `0.0.1` preview; `0.1.0` cut once the engine has consumed the trait end-to-end
4. `quantedge-engine` - streaming runtime with multi-timeframe state, deterministic event loop, and wasmtime-based generator registration
5. `quantedge-sim` - backtester with realistic fills, slippage, and drawdown reporting
6. `quantedge-ob` - streaming order book with deterministic event replay

## Contributing

Contributions welcome. Please open an issue to discuss before submitting large changes.

Contributions are accepted under the licence of the crate being modified. Unless you explicitly state otherwise, your contribution shall be licensed under the same terms as that crate, without any additional terms or conditions.

## License

Each crate in this workspace carries its own licence. See the `LICENSE-*` files in each crate directory for the full text.

Today, every published crate is dual-licensed under the Apache License 2.0 and the MIT License, at your option.
