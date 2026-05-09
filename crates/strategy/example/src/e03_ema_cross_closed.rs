//! Reference `SignalGenerator` implementation: EMA9 vs EMA21 cross on
//! H4, fired only after a bar closes. Compares the most recent two
//! closed bars (`closed(0)` against `closed(1)`) - the forming bar
//! is never read. Demonstrates the close-of-bar trigger pattern,
//! which is the natural counterpart to e01's intra-bar trigger.
//!
//! When to choose this shape over e01's: when intra-bar reversals
//! are a concern. e01 fires on the forming bar's current state, so a
//! cross can un-fire if the forming bar reverts before close. e03
//! waits for the bar to close, trading latency (one full bar of
//! delay) for confirmation (the cross is in the historical record).
//!
//! The test module at the bottom doubles as a worked tour of the
//! `quantedge_strategy::test_util` API for closed-bar generators.
//! It covers the same three slices as e01: declared dependencies,
//! single-tick evaluation, and multi-tick driving via `FakeEngine`.

use quantedge_strategy::{
    Bar, EmaConfig, MarketSide, MarketSignal, MarketSignalConfig, MarketSnapshot, SignalGenerator,
    Timeframe, TimeframeSnapshot, nz,
};

// Same field-storage pattern as e01.
pub struct EmaCrossingClosedSignalGenerator {
    ema9: EmaConfig,
    ema21: EmaConfig,
}

impl Default for EmaCrossingClosedSignalGenerator {
    fn default() -> Self {
        Self {
            ema9: EmaConfig::close(nz(9)),
            ema21: EmaConfig::close(nz(21)),
        }
    }
}

impl SignalGenerator for EmaCrossingClosedSignalGenerator {
    fn id(&self) -> &'static str {
        "ema_9_21_crossing_closed"
    }

    fn name(&self) -> &'static str {
        "EMA9 and EMA21 close-of-bar cross"
    }

    // `require_closed_bars(2)` - we compare two adjacent closed bars,
    // so we need both reachable. e01 only needed one closed bar
    // because its second comparand was the forming bar.
    fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
        config
            .require_closed_bars(2)
            .require_timeframes(&[Timeframe::HOUR_4])
            .register(&self.ema9)
            .register(&self.ema21)
    }

    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
        let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);

        // `closed(0)` is the most recent closed bar; `closed(1)` is
        // the one before it. The forming bar is intentionally never
        // read - that is what makes this a close-of-bar trigger.
        let newer = h4.closed(0)?;
        let older = h4.closed(1)?;

        let older_ema9 = older.value(&self.ema9)?;
        let older_ema21 = older.value(&self.ema21)?;
        let newer_ema9 = newer.value(&self.ema9)?;
        let newer_ema21 = newer.value(&self.ema21)?;

        // Cross detected between the two most recent closed bars.
        // Once detected, the cross is permanent - closed bars don't
        // change retroactively, so there is no un-fire concern.
        if older_ema9 < older_ema21 && newer_ema9 > newer_ema21 {
            // `from_closed(..., idx, ...)` anchors the signal to a
            // specific closed bar's OHLCV. We use `idx = 0` (the
            // newer bar, the one where the cross was confirmed).
            // Returns `Option<MarketSignalBuilder>`; `?` propagates
            // the warm-up case where the bar isn't available.
            return Some(
                MarketSignal::from_closed(self, snapshot, Timeframe::HOUR_4, 0, "bull_cross")?
                    .with_side(MarketSide::Long)
                    .add_reason("bull_cross", "EMA9 closed above EMA21 on H4")
                    .build(),
            );
        } else if older_ema9 > older_ema21 && newer_ema9 < newer_ema21 {
            return Some(
                MarketSignal::from_closed(self, snapshot, Timeframe::HOUR_4, 0, "bear_cross")?
                    .with_side(MarketSide::Short)
                    .add_reason("bear_cross", "EMA9 closed below EMA21 on H4")
                    .build(),
            );
        }

        None
    }
}

// Three slices of the generator's contract, each with its own kind
// of fake. Full API reference for the helpers below lives in
// `quantedge_strategy::test_util`; the tests here are the worked
// tutorial for the closed-bar trigger shape.
//
//   - `configure` - assert that H4 is declared, two closed bars are
//     budgeted, and both EMAs are registered.
//   - `evaluate` (single tick) - hand-build a `FakeMarketSnapshot`
//     with the EMA values on the two most recent closed bars. The
//     forming bar's state is irrelevant and we leave it bare.
//   - `evaluate` (tick sequence) - drive the generator across three
//     ticks (pre-cross / cross / post-cross), each tick representing
//     a fresh bar-close event in the H4 history.
#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_strategy::{
        EmaConfig, MarketSide, MarketSignal, SignalGenerator, Timeframe, nz,
        test_util::{FakeEngine, FakeMarketSnapshot, RecordingMarketSignalConfig},
    };

    fn ema9() -> EmaConfig {
        EmaConfig::close(nz(9))
    }

    fn ema21() -> EmaConfig {
        EmaConfig::close(nz(21))
    }

    /// Builds an H4 snapshot whose most recent two closed bars carry
    /// the given EMA values. The forming bar is left bare (no
    /// indicator values, default OHLC) since the generator never
    /// reads it.
    ///
    /// `add_closed_with` extends history backwards: the first call
    /// becomes `closed(0)` (the newer bar), the second becomes
    /// `closed(1)` (the older bar).
    fn evaluate(
        older_ema9: f64,
        older_ema21: f64,
        newer_ema9: f64,
        newer_ema21: f64,
    ) -> Option<MarketSignal> {
        let market = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_4, |t| {
            t.add_closed_with(|b| {
                b.add_value(&ema9(), newer_ema9)
                    .add_value(&ema21(), newer_ema21)
            })
            .add_closed_with(|b| {
                b.add_value(&ema9(), older_ema9)
                    .add_value(&ema21(), older_ema21)
            })
        });

        EmaCrossingClosedSignalGenerator::default().evaluate(&market)
    }

    #[test]
    fn configure_declares_h4_two_closed_bars_and_both_emas() {
        let recorder = EmaCrossingClosedSignalGenerator::default()
            .configure(RecordingMarketSignalConfig::new());

        assert!(recorder.has_timeframe(&Timeframe::HOUR_4));
        assert_eq!(recorder.required_closed_bars(), 2);
        assert!(recorder.has_indicator(&ema9()));
        assert!(recorder.has_indicator(&ema21()));
    }

    #[test]
    fn bull_cross_emits_long_signal() {
        // Older bar: EMA9 below EMA21. Newer bar: EMA9 above. Bull cross.
        let signal = evaluate(99.0, 100.0, 101.0, 100.5).unwrap();

        assert_eq!(signal.key, "bull_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Long));
        assert_eq!(signal.timeframe, Timeframe::HOUR_4);
        assert_eq!(signal.generator_id, "ema_9_21_crossing_closed");
    }

    #[test]
    fn bear_cross_emits_short_signal() {
        // Older bar: EMA9 above EMA21. Newer bar: EMA9 below. Bear cross.
        let signal = evaluate(101.0, 100.0, 99.0, 100.5).unwrap();

        assert_eq!(signal.key, "bear_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Short));
    }

    #[test]
    fn no_cross_returns_none() {
        // EMA9 stays above EMA21 across both closed bars -> no cross.
        let signal = evaluate(102.0, 100.0, 103.0, 100.5);

        assert!(signal.is_none());
    }

    // The `evaluate`-tick-sequence slice. Each tick here corresponds
    // to a fresh bar-close event in the H4 history: the closed-bar
    // window slides forward by one. A close-of-bar generator only
    // produces signals on these tick boundaries - the production
    // engine just won't call `evaluate` between bar closes for this
    // shape of generator.
    mod end_to_end {
        use super::*;

        // Per-tick helper. Same pattern as e01/e02 - one function
        // that builds a single tick's H4 snapshot from the two
        // closed-bar EMA pairs, parameterised by the values that
        // change tick-to-tick.
        fn tick(
            snapshot: FakeMarketSnapshot,
            older_ema9: f64,
            older_ema21: f64,
            newer_ema9: f64,
            newer_ema21: f64,
        ) -> FakeMarketSnapshot {
            snapshot.with_timeframe(Timeframe::HOUR_4, |t| {
                t.add_closed_with(|b| {
                    b.add_value(&ema9(), newer_ema9)
                        .add_value(&ema21(), newer_ema21)
                })
                .add_closed_with(|b| {
                    b.add_value(&ema9(), older_ema9)
                        .add_value(&ema21(), older_ema21)
                })
            })
        }

        #[test]
        fn engine_drives_generator_across_a_three_bar_close_cross() {
            // Three bar-close events. On tick 1 both closed bars are
            // bearish-stacked, no signal. On tick 2 the older bar is bearish,
            // the newer bar is bullish - bull cross confirmed. On tick 3 both
            //  bars are bullish-stacked, no further signal (the cross is in the past).
            let signals = FakeEngine::btcusdt()
                //                older9 older21 newer9 newer21
                .tick(|s| tick(s, 99.0, 100.0, 99.5, 100.5))
                .tick(|s| tick(s, 99.5, 100.5, 101.0, 100.5))
                .tick(|s| tick(s, 101.0, 100.5, 102.0, 101.0))
                .execute::<EmaCrossingClosedSignalGenerator>();

            assert_eq!(signals.len(), 3);
            assert!(signals[0].is_none());
            assert_eq!(signals[1].as_ref().unwrap().key, "bull_cross");
            assert!(signals[2].is_none());
        }
    }
}
