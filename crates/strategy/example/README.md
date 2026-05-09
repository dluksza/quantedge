# quantedge-strategy examples

Reference implementations for `SignalGenerator`. Each file is both a
working generator and a worked tutorial — module docs (`//!`) explain
the pattern; the test block at the bottom doubles as a tour of the
`quantedge_strategy::test_util` API.

## Reading order

Start with **e01**. It establishes the baseline shape — `Default`,
`configure`, `evaluate`, and the three test slices. The other three
can be read in any order; each varies one orthogonal axis from the
baseline.

| Example | Pattern | Axis it varies |
|---|---|---|
| e01 | EMA9/EMA21 cross, intra-bar (forming) trigger | (baseline) |
| e02 | HTF trend filter on LTF cross | Multi-timeframe |
| e03 | EMA9/EMA21 cross, close-of-bar trigger | Closed-bar reads, `MarketSignal::from_closed` |
| e04 | Bollinger Bands breakout | Composite indicator output (`BbValue` struct) |

## Picking an example by problem

- *"How do I read multiple timeframes?"* → **e02**
- *"How do I avoid acting on intra-bar reversals?"* → **e03**
- *"How do I read an indicator that returns a struct (BB, MACD, ADX, KDJ)?"* → **e04**
- *"How do I express a confluence (signal needs A **and** B)?"* → **e02**, which uses one `add_reason` per condition
- *"What does the test scaffolding actually look like?"* → **e01**, then any of the others

## Test patterns demonstrated

Every example covers the same three slices, in their own modules where
helpful:

- **`configure`** — pass a `RecordingMarketSignalConfig` through the
  generator's `configure` and assert the declared dependencies
  (timeframes, closed-bar budget, registered indicators).
- **single-tick `evaluate`** — hand-build one `FakeMarketSnapshot` with
  exactly the values `evaluate` will read, call it, assert the
  produced (or absent) `MarketSignal`. Fast, focused, easy to read.
- **multi-tick `end_to_end`** — drive the generator across a sequence
  via `FakeEngine`. The fake engine wraps each tick in an
  `EnforcingMarketSnapshot`, so a passing run also proves that
  `configure` and `evaluate` agree on what data the generator
  consumes — drift between the two surfaces immediately, instead of
  silently in production.

## Out of scope

- **Writing custom indicators.** Indicators live in `quantedge_ta` and
  have their own contract (rolling state, incremental updates). When
  no shipped indicator fits a strategy's needs, the answer is to
  implement a new one there, not to pull rolling math into `evaluate`
  here.
- **Stateful generators.** The `SignalGenerator` trait is `&self` by
  design — generators are stateless functions of the snapshot. State
  lives in indicators, where the engine can manage it. Reaching for
  `Mutex<…>` inside a generator is a sign the logic should be moved
  into a custom indicator.
- **Window aggregations (rolling max/min/sum/avg) computed in the
  generator.** Same reason as above: the indicator system exists so
  that aggregation cost is paid once per bar, not once per `evaluate`
  call. Reach for an existing indicator (Donchian, Keltner, etc.) or
  add a new one.
