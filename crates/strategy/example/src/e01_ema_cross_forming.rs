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

// Two slices of the generator's contract, each covered with the
// minimum machinery:
//   - `configure` - pass in a `RecordingMarketSignalConfig` and assert
//     it captured the declared dependencies.
//   - `evaluate` - hand-build a `FakeMarketSnapshot` with the EMA
//     values the generator will read, call `evaluate`, assert the
//     produced (or absent) `MarketSignal`.
//
// End-to-end coverage (driving the generator across a tick sequence
// under `EnforcingMarketSnapshot`) belongs in a higher-level harness
// once `FakeEngine` lands; these tests stay focused on the generator's
// own input → output contract.
#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_strategy::{
        EmaConfig, MarketSide, MarketSignal, SignalGenerator, Timeframe, nz,
        test_util::{FakeMarketSnapshot, RecordingMarketSignalConfig},
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
    /// nothing else. Times default to midnight 2026-01-01 with
    /// timeframe-aligned bars; we don't care about the exact values
    /// because the generator doesn't either.
    fn evaluate(
        prev_ema9: f64,
        prev_ema21: f64,
        forming_ema9: f64,
        forming_ema21: f64,
    ) -> Option<MarketSignal> {
        let market = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_4, |t| {
            t.customize_forming(|b| {
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
}
