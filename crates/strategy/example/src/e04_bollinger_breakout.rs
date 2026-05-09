//! Reference `SignalGenerator` implementation: Bollinger Bands
//! breakout on H4. Bullish when the forming close crosses above the
//! upper band from below; bearish when it crosses below the lower
//! band from above. Demonstrates the composite-output indicator
//! pattern - `Bar::value` returning a struct (`BbValue`) instead of
//! a scalar `Price`.
//!
//! Until now the example crate has only used scalar-output
//! indicators (EMA returns `Price`). Real strategies routinely lean
//! on indicators whose output is a record of related values:
//! Bollinger Bands (`BbValue { upper, middle, lower }`), Donchian
//! Channels, Keltner Channels, MACD, Stochastic, ADX, KDJ. The
//! `Bar::value` signature `value(&cfg) -> Option<C::Output>` is
//! generic over `C::Output`, so composite outputs are first-class.
//! This example shows how the generator and its tests handle them.
//!
//! The test module at the bottom doubles as a worked tour of the
//! `quantedge_strategy::test_util` API for composite-output
//! generators. It covers the same three slices as the earlier
//! examples: declared dependencies, single-tick evaluation, and
//! multi-tick driving via `FakeEngine`.

use quantedge_strategy::{
    Bar, BbConfig, MarketSide, MarketSignal, MarketSignalConfig, MarketSnapshot, SignalGenerator,
    Timeframe, TimeframeSnapshot,
};

// Same field-storage pattern as the earlier examples. `BbConfig`
// bundles length + price source + std-dev multiplier; we use the
// canonical BB(20, Close, 2 std_dev) preset.
pub struct BollingerBreakoutSignalGenerator {
    bb: BbConfig,
}

impl Default for BollingerBreakoutSignalGenerator {
    fn default() -> Self {
        Self {
            bb: BbConfig::default_20(),
        }
    }
}

impl SignalGenerator for BollingerBreakoutSignalGenerator {
    fn id(&self) -> &'static str {
        "bollinger_breakout"
    }

    fn name(&self) -> &'static str {
        "Bollinger Bands breakout"
    }

    // Just one indicator and one closed bar of history - `BbConfig`
    // does the rolling-window math itself.
    fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
        config
            .require_closed_bars(1)
            .require_timeframes(&[Timeframe::HOUR_4])
            .register(&self.bb)
    }

    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
        let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);
        let prev = h4.closed(0)?;
        let forming = h4.forming();

        // `Bar::value(&self.bb)` returns `Option<BbValue>`, the
        // `Output` type associated with `BbConfig`. Field-access on
        // the struct (`bb.upper`, `bb.lower`) reads each band; the
        // shape is identical to scalar reads in the earlier
        // examples, just with a richer return type.
        let prev_bb = prev.value(&self.bb)?;
        let forming_bb = forming.value(&self.bb)?;

        // Close prices come from `bar.ohlcv()`, strategies that
        // compare price-to-band always need both sides.
        let prev_close = prev.ohlcv().close;
        let forming_close = forming.ohlcv().close;

        // Bullish breakout: previous close was at or below the upper
        // band, forming close has pushed above. Symmetric for the
        // lower band on the bearish side.
        if prev_close <= prev_bb.upper && forming_close > forming_bb.upper {
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "upper_breakout")
                    .with_side(MarketSide::Long)
                    .add_reason("upper_breakout", "close broke above BB upper band")
                    .build(),
            );
        } else if prev_close >= prev_bb.lower && forming_close < forming_bb.lower {
            return Some(
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "lower_breakout")
                    .with_side(MarketSide::Short)
                    .add_reason("lower_breakout", "close broke below BB lower band")
                    .build(),
            );
        }

        None
    }
}

// Three slices of the generator's contract, each with its own kind
// of fake. Full API reference for the helpers below lives in
// `quantedge_strategy::test_util`; the tests here are the worked
// tutorial for the composite-output shape.
//
//   - `configure` - assert that H4 is declared, one closed bar is
//     budgeted, and the BB indicator is registered.
//   - `evaluate` (single tick) - hand-build a `FakeMarketSnapshot`
//     where each bar carries a `BbValue` plus a specific close
//     price. The four cases cover bullish breakout, bearish
//     breakout, no-breakout-inside-bands, and the failed-retest
//     case where the previous bar was already outside.
//   - `evaluate` (tick sequence) - drive the generator across a
//     scenario where price walks through the upper band and back.
#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_strategy::{
        BbConfig, BbValue, MarketSide, MarketSignal, SignalGenerator, Timeframe,
        test_util::{FakeEngine, FakeMarketSnapshot, RecordingMarketSignalConfig},
    };

    fn bb() -> BbConfig {
        BbConfig::default_20()
    }

    /// Builds an H4 snapshot whose forming bar and most recent
    /// closed bar each carry a specific close price plus a `BbValue`
    /// for the BB indicator.
    ///
    /// The `with_close` setter on `FakeBar` keeps `open_time` and
    /// indicator values intact while replacing OHLC with a single
    /// close price — the natural shape inside `forming_with` /
    /// `add_closed_with` closures when the generator reads both
    /// `bar.ohlcv().close` and `bar.value(&cfg)`.
    fn evaluate(
        prev_close: f64,
        prev_bb: BbValue,
        forming_close: f64,
        forming_bb: BbValue,
    ) -> Option<MarketSignal> {
        let market = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_4, |t| {
            t.forming_with(|b| b.with_close(forming_close).add_value(&bb(), forming_bb))
                .add_closed_with(|b| b.with_close(prev_close).add_value(&bb(), prev_bb))
        });

        BollingerBreakoutSignalGenerator::default().evaluate(&market)
    }

    /// Convenience: a `BbValue` collapsed around `middle` with
    /// symmetric bands. Keeps tests readable when the test only
    /// cares about the upper/lower breakout boundaries.
    fn band(middle: f64, half_width: f64) -> BbValue {
        BbValue {
            upper: middle + half_width,
            middle,
            lower: middle - half_width,
        }
    }

    #[test]
    fn configure_declares_h4_one_closed_bar_and_bb() {
        let recorder = BollingerBreakoutSignalGenerator::default()
            .configure(RecordingMarketSignalConfig::new());

        assert!(recorder.has_timeframe(&Timeframe::HOUR_4));
        assert_eq!(recorder.required_closed_bars(), 1);
        assert!(recorder.has_indicator(&bb()));
    }

    #[test]
    fn upper_breakout_emits_long_signal() {
        // Prev close (104) at the upper band (105). Forming close
        // (107) breaks above the (106) upper band. Bullish breakout.
        let signal = evaluate(104.0, band(100.0, 5.0), 107.0, band(101.0, 5.0)).unwrap();

        assert_eq!(signal.key, "upper_breakout");
        assert_eq!(signal.market_side, Some(MarketSide::Long));
        assert_eq!(signal.timeframe, Timeframe::HOUR_4);
    }

    #[test]
    fn lower_breakout_emits_short_signal() {
        // Prev close (96) at the lower band (95). Forming close (93)
        // breaks below the (94) lower band. Bearish breakout.
        let signal = evaluate(96.0, band(100.0, 5.0), 93.0, band(99.0, 5.0)).unwrap();

        assert_eq!(signal.key, "lower_breakout");
        assert_eq!(signal.market_side, Some(MarketSide::Short));
    }

    #[test]
    fn close_inside_bands_returns_none() {
        // Forming close (102) is inside the (95, 105) bands -> no
        // breakout in either direction.
        let signal = evaluate(100.0, band(100.0, 5.0), 102.0, band(100.0, 5.0));

        assert!(signal.is_none());
    }

    #[test]
    fn already_outside_does_not_re_fire() {
        // Prev close (107) was already above the upper band (105).
        // Forming close (108) is also above, but the generator's
        // gate (`prev_close <= prev_bb.upper`) suppresses the
        // signal -> the breakout already happened on a prior bar,
        // so this isn't a fresh event.
        let signal = evaluate(107.0, band(100.0, 5.0), 108.0, band(101.0, 5.0));

        assert!(signal.is_none());
    }

    // The `evaluate`-tick-sequence slice. `FakeEngine` drives the
    // generator across multiple ticks where the price walks through
    // the upper band and back: inside, breakout, walking, returned.
    // The breakout signal fires once on entry and is suppressed on
    // subsequent ticks where price stays outside the band.
    mod end_to_end {
        use super::*;

        // Per-tick helper, same pattern as the earlier examples.
        // Carries the (close, BbValue) pair for both the previous
        // closed bar and the forming bar.
        fn tick(
            snapshot: FakeMarketSnapshot,
            prev_close: f64,
            prev_bb: BbValue,
            forming_close: f64,
            forming_bb: BbValue,
        ) -> FakeMarketSnapshot {
            snapshot.with_timeframe(Timeframe::HOUR_4, |t| {
                t.forming_with(|b| b.with_close(forming_close).add_value(&bb(), forming_bb))
                    .add_closed_with(|b| b.with_close(prev_close).add_value(&bb(), prev_bb))
            })
        }

        #[test]
        fn engine_fires_once_on_breakout_and_stays_quiet_outside() {
            // Three ticks. Bands held steady around mid=100, half=5 so
            // upper=105 throughout. Tick 1: price inside (102), no signal.
            // Tick 2: price breaks above (107) from a prev close at the band
            // (104) - bullish breakout fires. Tick 3: price still above the
            // band (108) but prev close was also above (107) — gate suppresses
            // re-fire.
            let signals = FakeEngine::btcusdt()
                //                p_close   p_bb              f_close   f_bb
                .tick(|s| tick(s, 100.0, band(100.0, 5.0), 102.0, band(100.0, 5.0)))
                .tick(|s| tick(s, 104.0, band(100.0, 5.0), 107.0, band(101.0, 5.0)))
                .tick(|s| tick(s, 107.0, band(101.0, 5.0), 108.0, band(102.0, 5.0)))
                .execute::<BollingerBreakoutSignalGenerator>();

            assert_eq!(signals.len(), 3);
            assert!(signals[0].is_none());
            assert_eq!(signals[1].as_ref().unwrap().key, "upper_breakout");
            assert!(signals[2].is_none());
        }
    }
}
