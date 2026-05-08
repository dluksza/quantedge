//! Reference `SignalGenerator` implementation: EMA9 vs EMA21 cross on
//! the H4 timeframe. Demonstrates the three pieces every generator
//! needs — indicator configs as fields, `configure` to declare
//! dependencies, `evaluate` to detect and emit signals.

use quantedge_strategy::{
    Bar, EmaConfig, MarketSide, MarketSignal, MarketSignalConfig, MarketSnapshot, SignalGenerator,
    Timeframe, TimeframeSnapshot, nz,
};

// Recommended pattern: store indicator configs as struct fields so
// the same instance is reused in `register` (in `configure`) and in
// `Bar::value` (in `evaluate`). Configs are immutable and equatable,
// so reconstructing them per tick also works. Field storage is just
// shorter and harder to get wrong.
pub struct EmaCrossingFormingSignalGenerator {
    ema9: EmaConfig,
    ema21: EmaConfig,
}

// `SignalGenerator: Default` - the engine constructs each generator
// type with `T::default()` before calling `configure`. Parameters
// (here: the EMA lengths) are baked into `Default`.
impl Default for EmaCrossingFormingSignalGenerator {
    fn default() -> Self {
        // `EmaConfig::close(len)` is shorthand for the close-priced
        // EMA; reach for `EmaConfig::builder().source(...).build()`
        // when you need a different `PriceSource`.
        Self {
            ema9: EmaConfig::close(nz(9)),
            ema21: EmaConfig::close(nz(21)),
        }
    }
}

impl SignalGenerator for EmaCrossingFormingSignalGenerator {
    // Stable per generator type. Appears on every emitted signal as
    // `MarketSignal::generator_id`.
    fn id(&self) -> &'static str {
        "ema_9_21_crossing"
    }

    fn name(&self) -> &'static str {
        "EMA9 and EMA21 crossing"
    }

    // Declares this generator's data dependencies:
    //   `require_closed_bars(1)` keeps the most recent closed bar
    //     so we can compare it against the forming bar.
    //   `require_timeframes(...)` selects which timeframes the
    //     snapshot will carry.
    //   `register(&config)` subscribes an indicator — the engine
    //     computes it on every required timeframe and surfaces its
    //     value via `Bar::value(&config)`.
    fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
        config
            .require_closed_bars(1)
            .require_timeframes(&[Timeframe::HOUR_4])
            .register(&self.ema9)
            .register(&self.ema21)
    }

    // Stateless: same `snapshot` in → same result out. The engine
    // calls this on every tick and on each required-timeframe close.
    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
        let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);

        // `closed(0)` is the most recent closed bar; `forming()` is
        // the currently-building bar.
        let prev_closed = h4.closed(0)?;
        let forming = h4.forming();

        // `Bar::value` returns `Option` to cover the warm-up window.
        // The engine seeds indicators with history at startup, so
        // values are usually present - `?` guards the edge case
        // (a brand-new instrument or a very long indicator window).
        let prev_ema9 = prev_closed.value(&self.ema9)?;
        let prev_ema21 = prev_closed.value(&self.ema21)?;
        let forming_ema9 = forming.value(&self.ema9)?;
        let forming_ema21 = forming.value(&self.ema21)?;

        // Cross detected between the last closed bar and the forming
        // bar - fires intra-bar. The cross can un-fire if the forming
        // bar reverts before close; downstream dedup handles that.
        if prev_ema9 < prev_ema21 && forming_ema9 > forming_ema21 {
            // `from_forming` captures the forming bar's OHLCV at the
            // given timeframe and the snapshot's instrument. `key`
            // discriminates among the signal types this generator
            // emits (bull vs bear here); `(generator_id, key)` is
            // globally unique across all generators.
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "bull_cross")
                    // `Some(side)` for directional signals; `None`
                    // for filters or non-directional context.
                    .with_side(MarketSide::Long)
                    // Reasons form a set keyed by `SignalReason::id`
                    // — duplicates by id collapse on insert.
                    .add_reason("bull_cross", "BULL EMA9 cross EMA21")
                    .build(),
            );
        } else if prev_ema9 > prev_ema21 && forming_ema9 < forming_ema21 {
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "bear_cross")
                    .with_side(MarketSide::Short)
                    .add_reason("bear_cross", "BEAR EMA9 cross EMA21")
                    .build(),
            );
        }

        None
    }
}

// Three slices of the generator's contract, each with its own kind
// of fake:
//   - `configure` - pass in a `RecordingMarketSignalConfig` and
//     assert it captured the declared dependencies.
//   - `evaluate` (single tick) - hand-build a `FakeMarketSnapshot`
//     with the EMA values the generator will read, call `evaluate`,
//     assert the produced (or absent) `MarketSignal`.
//   - `evaluate` (tick sequence) - drive the generator with
//     `FakeEngine` across multiple ticks. Each tick reuses a
//     per-tick helper so the test reads as a table of inputs,
//     not a wall of nested closures.
#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_strategy::{
        EmaConfig, MarketSide, MarketSignal, SignalGenerator, Timeframe, nz,
        test_util::{FakeEngine, FakeMarketSnapshot, RecordingMarketSignalConfig},
    };

    // Same configs the generator stores on itself; reused across tests
    // so the recorder identifies them by `IndicatorConfig` equality.
    fn ema9() -> EmaConfig {
        EmaConfig::close(nz(9))
    }

    fn ema21() -> EmaConfig {
        EmaConfig::close(nz(21))
    }

    /// Builds an H4 snapshot with the given EMA values on the previous
    /// closed bar and the forming bar - exactly what `evaluate` reads,
    /// nothing else. Both bars carry two indicators (EMA9 + EMA21), so
    /// the closure forms `forming_with` / `add_closed_with` are a
    /// better fit than the single-indicator shorthands.
    fn evaluate(
        prev_ema9: f64,
        prev_ema21: f64,
        forming_ema9: f64,
        forming_ema21: f64,
    ) -> Option<MarketSignal> {
        let market = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_4, |t| {
            t.forming_with(|b| {
                b.add_value(&ema9(), forming_ema9)
                    .add_value(&ema21(), forming_ema21)
            })
            .add_closed_with(|b| {
                b.add_value(&ema9(), prev_ema9)
                    .add_value(&ema21(), prev_ema21)
            })
        });

        EmaCrossingFormingSignalGenerator::default().evaluate(&market)
    }

    #[test]
    fn configure_declares_h4_one_closed_bar_and_both_emas() {
        // `RecordingMarketSignalConfig` is itself a `MarketSignalConfig`,
        // so it can be passed straight into `configure` to record what
        // the generator asks for.
        let recorder = EmaCrossingFormingSignalGenerator::default()
            .configure(RecordingMarketSignalConfig::new());

        assert!(recorder.has_timeframe(&Timeframe::HOUR_4));
        assert_eq!(recorder.required_closed_bars(), 1);
        assert!(recorder.has_indicator(&ema9()));
        assert!(recorder.has_indicator(&ema21()));
    }

    #[test]
    fn bull_cross_emits_long_signal() {
        // Prev: EMA9 below EMA21. Forming: EMA9 above. Bullish cross.
        let signal = evaluate(99.0, 100.0, 101.0, 100.5).unwrap();

        assert_eq!(signal.key, "bull_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Long));
        assert_eq!(signal.timeframe, Timeframe::HOUR_4);
        assert_eq!(signal.generator_id, "ema_9_21_crossing");
    }

    #[test]
    fn bear_cross_emits_short_signal() {
        // Prev: EMA9 above EMA21. Forming: EMA9 below. Bearish cross.
        let signal = evaluate(101.0, 100.0, 99.0, 100.5).unwrap();

        assert_eq!(signal.key, "bear_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Short));
    }

    #[test]
    fn no_cross_returns_none() {
        // EMA9 stays above EMA21 across both bars — no crossing.
        let signal = evaluate(102.0, 100.0, 103.0, 100.5);

        assert!(signal.is_none());
    }

    // The `evaluate`-tick-sequence slice. `FakeEngine` drives the
    // generator across multiple ticks the way the production engine
    // would. `Default::default()` to construct, `configure` once,
    // `evaluate` per tick under an `EnforcingMarketSnapshot`.
    //
    // The enforcement is the point: every `evaluate` call panics if
    // it reads a timeframe, closed-bar index, or indicator the
    // generator didn't declare in `configure`. So a passing run here
    // doubles as a contract check between `configure` and `evaluate`,
    // drift between the two surfaces immediately, instead of in
    // prod when the engine quietly serves an empty bar.
    //
    // That guarantee is qualitatively different from the per-tick
    // unit tests above, so this slice lives in its own module to make
    // the intent obvious at a glance.
    mod end_to_end {
        use super::*;

        // Per-tick helper. This is the pattern recommended in the
        // `FakeEngine` module docs: one function that builds a single
        // tick, parameterised by the values that change tick-to-tick.
        // Inline closures explode visually when several ticks share
        // the same shape; the helper keeps the test body shaped like
        // a table of inputs.
        fn tick(
            snapshot: FakeMarketSnapshot,
            prev_ema9: f64,
            prev_ema21: f64,
            forming_ema9: f64,
            forming_ema21: f64,
        ) -> FakeMarketSnapshot {
            snapshot.with_timeframe(Timeframe::HOUR_4, |t| {
                t.forming_with(|b| {
                    b.add_value(&ema9(), forming_ema9)
                        .add_value(&ema21(), forming_ema21)
                })
                .add_closed_with(|b| {
                    b.add_value(&ema9(), prev_ema9)
                        .add_value(&ema21(), prev_ema21)
                })
            })
        }

        #[test]
        fn engine_drives_generator_across_a_three_tick_cross() {
            // Three ticks: pre-cross (no signal), the cross itself
            // (bullish), then post-cross drift (no signal). Reads
            // top-to-bottom as the price path the generator sees.
            let signals = FakeEngine::btcusdt()
                //                prev9  prev21  forming9  forming21
                .tick(|s| tick(s, 99.0, 100.0, 99.5, 100.5))
                .tick(|s| tick(s, 99.0, 100.0, 101.0, 100.5))
                .tick(|s| tick(s, 101.0, 100.5, 102.0, 101.0))
                .execute::<EmaCrossingFormingSignalGenerator>();

            assert_eq!(signals.len(), 3);
            assert!(signals[0].is_none());
            assert_eq!(signals[1].as_ref().unwrap().key, "bull_cross");
            assert!(signals[2].is_none());
        }
    }
}
