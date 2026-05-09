//! Reference `SignalGenerator` implementation: H1 EMA9 vs EMA21 cross,
//! gated by a D1 EMA50 vs EMA200 trend filter. Bullish crosses only
//! fire when the daily trend is up; bearish crosses only fire when it
//! is down. Demonstrates the multi-timeframe pattern - declaring
//! several timeframes in `configure`, reading each via `for_timeframe`
//! in `evaluate`, and seeding both timeframes in tests.
//!
//! The test module at the bottom doubles as a worked tour of the
//! `quantedge_strategy::test_util` API for multi-timeframe generators.
//! It covers the same three slices as `e01`: declared dependencies,
//! single-tick evaluation across all four trend x cross combinations,
//! and multi-tick driving via `FakeEngine` where the trend regime
//! flips mid-stream.

use quantedge_strategy::{
    Bar, EmaConfig, MarketSide, MarketSignal, MarketSignalConfig, MarketSnapshot, SignalGenerator,
    Timeframe, TimeframeSnapshot, nz,
};

// Same field-storage pattern as e01, scaled to two timeframes. The
// HTF (higher timeframe) configs are reused only for trend bias; the
// LTF configs drive the entry trigger. Storing them as fields keeps
// the same instances reachable from both `configure` (for `register`)
// and `evaluate` (for `Bar::value`).
pub struct HtfFilteredEmaCrossSignalGenerator {
    htf_fast: EmaConfig,
    htf_slow: EmaConfig,
    ltf_fast: EmaConfig,
    ltf_slow: EmaConfig,
}

impl Default for HtfFilteredEmaCrossSignalGenerator {
    fn default() -> Self {
        Self {
            htf_fast: EmaConfig::close(nz(50)),
            htf_slow: EmaConfig::close(nz(200)),
            ltf_fast: EmaConfig::close(nz(9)),
            ltf_slow: EmaConfig::close(nz(21)),
        }
    }
}

impl SignalGenerator for HtfFilteredEmaCrossSignalGenerator {
    fn id(&self) -> &'static str {
        "htf_filtered_ema_cross"
    }

    fn name(&self) -> &'static str {
        "HTF-filtered EMA9/EMA21 cross"
    }

    // Two timeframes are declared in a single `require_timeframes`
    // call. `register(&cfg)` is *global* across all required
    // timeframes, one registration here means the engine computes
    // each EMA on both D1 and H1, and the value is reachable via
    // `Bar::value(&cfg)` on bars from either timeframe. There is no
    // per-timeframe registration in this trait.
    //
    // `require_closed_bars(1)` applies per timeframe: we get the
    // most-recent closed bar on D1 (for the trend read) and on H1
    // (for the prior side of the cross).
    fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
        config
            .require_closed_bars(1)
            .require_timeframes(&[Timeframe::DAY_1, Timeframe::HOUR_1])
            .register(&self.htf_fast)
            .register(&self.htf_slow)
            .register(&self.ltf_fast)
            .register(&self.ltf_slow)
    }

    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
        // Read both timeframes. Each `for_timeframe` call returns the
        // snapshot for one declared timeframe; reading an undeclared
        // one panics under `EnforcingMarketSnapshot`.
        let d1 = snapshot.for_timeframe(Timeframe::DAY_1);
        let h1 = snapshot.for_timeframe(Timeframe::HOUR_1);

        // HTF bias from D1's most recent closed bar. Closed (not
        // forming) so the regime is stable across the H1 ticks that
        // happen inside the same D1 candle.
        let d1_closed = d1.closed(0)?;
        let htf_fast = d1_closed.value(&self.htf_fast)?;
        let htf_slow = d1_closed.value(&self.htf_slow)?;
        let htf_bullish = htf_fast > htf_slow;

        // LTF cross between H1 closed(0) and H1 forming, same shape
        // as e01.
        let h1_prev = h1.closed(0)?;
        let h1_forming = h1.forming();
        let prev_fast = h1_prev.value(&self.ltf_fast)?;
        let prev_slow = h1_prev.value(&self.ltf_slow)?;
        let forming_fast = h1_forming.value(&self.ltf_fast)?;
        let forming_slow = h1_forming.value(&self.ltf_slow)?;

        // Gate: a bullish cross only emits when the HTF agrees, and
        // vice versa for bearish. Crosses against the trend are
        // suppressed silently — they are still detected, they just
        // don't graduate to a signal.
        if prev_fast < prev_slow && forming_fast > forming_slow && htf_bullish {
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_1, "bull_cross")
                    .with_side(MarketSide::Long)
                    // Multi-reason signals: each `add_reason` records
                    // an independent justification. Useful when a
                    // signal is the conjunction of several conditions
                    // and you want each visible downstream.
                    .add_reason("ltf_bull_cross", "H1 EMA9 crossed above EMA21")
                    .add_reason("htf_uptrend", "D1 EMA50 above EMA200")
                    .build(),
            );
        } else if prev_fast > prev_slow && forming_fast < forming_slow && !htf_bullish {
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_1, "bear_cross")
                    .with_side(MarketSide::Short)
                    .add_reason("ltf_bear_cross", "H1 EMA9 crossed below EMA21")
                    .add_reason("htf_downtrend", "D1 EMA50 below EMA200")
                    .build(),
            );
        }

        None
    }
}

// Three slices of the generator's contract, each with its own kind
// of fake. Full API reference for the helpers below lives in
// `quantedge_strategy::test_util`; the tests here are the worked
// tutorial for the multi-timeframe shape.
//
//   - `configure` - assert that both timeframes and all four
//     indicators are declared via `RecordingMarketSignalConfig`.
//   - `evaluate` (single tick) - hand-build a `FakeMarketSnapshot`
//     carrying both D1 and H1 timeframes. Four cases cover the
//     trend x cross matrix: aligned crosses fire, opposed crosses
//     are suppressed.
//   - `evaluate` (tick sequence) - drive the generator across a
//     scenario where the HTF regime flips mid-stream. The same
//     LTF cross emits a signal in one regime and is silently
//     suppressed in the other.
#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_strategy::{
        EmaConfig, MarketSide, MarketSignal, SignalGenerator, Timeframe, nz,
        test_util::{FakeEngine, FakeMarketSnapshot, RecordingMarketSignalConfig},
    };

    fn htf_fast() -> EmaConfig {
        EmaConfig::close(nz(50))
    }

    fn htf_slow() -> EmaConfig {
        EmaConfig::close(nz(200))
    }

    fn ltf_fast() -> EmaConfig {
        EmaConfig::close(nz(9))
    }

    fn ltf_slow() -> EmaConfig {
        EmaConfig::close(nz(21))
    }

    /// Builds a snapshot carrying both required timeframes:
    /// - D1 with `htf_fast` / `htf_slow` on the most recent closed
    ///   bar (driving the trend bias).
    /// - H1 with `ltf_fast` / `ltf_slow` on the previous closed
    ///   bar and the forming bar (driving the cross detection).
    ///
    /// Both bars on each timeframe carry two indicators, so the
    /// closure forms `forming_with` / `add_closed_with` are the
    /// natural fit.
    fn evaluate(
        d1_htf_fast: f64,
        d1_htf_slow: f64,
        h1_prev_fast: f64,
        h1_prev_slow: f64,
        h1_forming_fast: f64,
        h1_forming_slow: f64,
    ) -> Option<MarketSignal> {
        let market = FakeMarketSnapshot::btcusdt()
            .with_timeframe(Timeframe::DAY_1, |t| {
                t.add_closed_with(|b| {
                    b.add_value(&htf_fast(), d1_htf_fast)
                        .add_value(&htf_slow(), d1_htf_slow)
                })
            })
            .with_timeframe(Timeframe::HOUR_1, |t| {
                t.forming_with(|b| {
                    b.add_value(&ltf_fast(), h1_forming_fast)
                        .add_value(&ltf_slow(), h1_forming_slow)
                })
                .add_closed_with(|b| {
                    b.add_value(&ltf_fast(), h1_prev_fast)
                        .add_value(&ltf_slow(), h1_prev_slow)
                })
            });

        HtfFilteredEmaCrossSignalGenerator::default().evaluate(&market)
    }

    #[test]
    fn configure_declares_both_timeframes_and_all_four_emas() {
        let recorder = HtfFilteredEmaCrossSignalGenerator::default()
            .configure(RecordingMarketSignalConfig::new());

        assert!(recorder.has_timeframe(&Timeframe::DAY_1));
        assert!(recorder.has_timeframe(&Timeframe::HOUR_1));
        assert_eq!(recorder.required_closed_bars(), 1);
        assert!(recorder.has_indicator(&htf_fast()));
        assert!(recorder.has_indicator(&htf_slow()));
        assert!(recorder.has_indicator(&ltf_fast()));
        assert!(recorder.has_indicator(&ltf_slow()));
    }

    #[test]
    fn bull_cross_in_uptrend_emits_long_signal() {
        // D1 uptrend (50 > 200). H1 bull cross (prev 9 < 21, forming 9 > 21).
        let signal = evaluate(110.0, 100.0, 99.0, 100.0, 101.0, 100.5).unwrap();

        assert_eq!(signal.key, "bull_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Long));
        assert_eq!(signal.timeframe, Timeframe::HOUR_1);
    }

    #[test]
    fn bull_cross_in_downtrend_is_suppressed() {
        // D1 downtrend (50 < 200) but H1 still bull-crosses -> no signal.
        let signal = evaluate(90.0, 100.0, 99.0, 100.0, 101.0, 100.5);

        assert!(signal.is_none());
    }

    #[test]
    fn bear_cross_in_downtrend_emits_short_signal() {
        // D1 downtrend (50 < 200). H1 bear cross (prev 9 > 21, forming 9 < 21).
        let signal = evaluate(90.0, 100.0, 101.0, 100.0, 99.0, 100.5).unwrap();

        assert_eq!(signal.key, "bear_cross");
        assert_eq!(signal.market_side, Some(MarketSide::Short));
    }

    #[test]
    fn bear_cross_in_uptrend_is_suppressed() {
        // D1 uptrend but H1 bear-crosses against it -> no signal.
        let signal = evaluate(110.0, 100.0, 101.0, 100.0, 99.0, 100.5);

        assert!(signal.is_none());
    }

    #[test]
    fn no_cross_returns_none_regardless_of_trend() {
        // H1 doesn't cross at all -> no signal in either regime.
        assert!(evaluate(110.0, 100.0, 102.0, 100.0, 103.0, 100.5).is_none());
        assert!(evaluate(90.0, 100.0, 102.0, 100.0, 103.0, 100.5).is_none());
    }

    // The `evaluate`-tick-sequence slice. `FakeEngine` drives the
    // generator across multiple ticks the way the production engine
    // would. `Default::default()` to construct, `configure` once,
    // `evaluate` per tick under an `EnforcingMarketSnapshot`.
    //
    // Multi-timeframe scenarios make the regime-change story
    // expressible: the same LTF cross emits a signal in one tick
    // and is silently suppressed in another, depending only on the
    // HTF state at that moment.
    mod end_to_end {
        use super::*;

        // Per-tick helper, same pattern as e01's `end_to_end::tick`.
        // Carries one extra parameter pair for the HTF state, since
        // multi-timeframe tests need to specify both bars.
        fn tick(
            snapshot: FakeMarketSnapshot,
            d1_htf_fast: f64,
            d1_htf_slow: f64,
            h1_prev_fast: f64,
            h1_prev_slow: f64,
            h1_forming_fast: f64,
            h1_forming_slow: f64,
        ) -> FakeMarketSnapshot {
            snapshot
                .with_timeframe(Timeframe::DAY_1, |t| {
                    t.add_closed_with(|b| {
                        b.add_value(&htf_fast(), d1_htf_fast)
                            .add_value(&htf_slow(), d1_htf_slow)
                    })
                })
                .with_timeframe(Timeframe::HOUR_1, |t| {
                    t.forming_with(|b| {
                        b.add_value(&ltf_fast(), h1_forming_fast)
                            .add_value(&ltf_slow(), h1_forming_slow)
                    })
                    .add_closed_with(|b| {
                        b.add_value(&ltf_fast(), h1_prev_fast)
                            .add_value(&ltf_slow(), h1_prev_slow)
                    })
                })
        }

        #[test]
        fn htf_regime_flip_gates_the_same_ltf_cross() {
            // Three ticks, same H1 bull cross each time. The D1 trend starts
            // bullish (signal fires), flips bearish (same cross, no signal),
            // then flips back bullish (signal fires again). Reads
            // top-to-bottom as a regime-change story.
            let signals = FakeEngine::btcusdt()
                //                d1_fast  d1_slow  prev9   prev21  fwd9    fwd21
                .tick(|s| tick(s, 110.0, 100.0, 99.0, 100.0, 101.0, 100.5))
                .tick(|s| tick(s, 90.0, 100.0, 99.0, 100.0, 101.0, 100.5))
                .tick(|s| tick(s, 110.0, 100.0, 99.0, 100.0, 101.0, 100.5))
                .execute::<HtfFilteredEmaCrossSignalGenerator>();

            assert_eq!(signals.len(), 3);
            assert_eq!(signals[0].as_ref().unwrap().key, "bull_cross");
            assert!(signals[1].is_none());
            assert_eq!(signals[2].as_ref().unwrap().key, "bull_cross");
        }
    }
}
