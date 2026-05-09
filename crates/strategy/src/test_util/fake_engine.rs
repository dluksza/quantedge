//! Multi-tick driver for [`SignalGenerator`] tests on a single
//! instrument across one or more timeframes.
//!
//! [`FakeEngine`] mirrors the production engine contract: it owns the
//! generator's lifecycle (constructed via [`Default`], `configure`d
//! once, `evaluate`d per tick) and wraps every snapshot in an
//! [`EnforcingMarketSnapshot`].
//!
//! # Enforces `configure` ↔ `evaluate` consistency
//!
//! `configure` declares what the generator *will* read; `evaluate` is
//! what it *actually* reads. The two halves are easy to drift apart —
//! a developer adds an indicator read in `evaluate` and forgets to
//! `register` it, or extends the closed-bar window in `evaluate` past
//! what `require_closed_bars(..)` budgeted for. The production engine
//! never feeds undeclared data, so the bug stays silent until prod.
//!
//! `FakeEngine` makes the gap loud by panicking on every undeclared
//! read:
//!
//! - timeframe not in [`require_timeframes`](crate::MarketSignalConfig::require_timeframes)
//! - closed-bar index past [`require_closed_bars`](crate::MarketSignalConfig::require_closed_bars)
//! - indicator not in [`register`](crate::MarketSignalConfig::register)
//!
//! Each panic names the offending input and points at the missing
//! `configure(..)` call. So a passing `FakeEngine` test is also proof
//! that `configure` and `evaluate` agree on what data the generator
//! consumes.
//!
//! # Recommended pattern: extract a per-tick helper
//!
//! Inline `.tick(|s| ...)` closures get noisy when several ticks share
//! the same shape and only the values differ. Extract a function that
//! builds one tick from a fresh snapshot plus the per-tick parameters:
//!
//! ```ignore
//! fn tick(s: FakeMarketSnapshot, h4_forming: f64, h4_prev: f64) -> FakeMarketSnapshot {
//!     s.with_timeframe(Timeframe::HOUR_4, |t| {
//!         t.forming_value(&ema9(), h4_forming)
//!             .add_closed_value(&ema9(), h4_prev)
//!     })
//! }
//!
//! FakeEngine::btcusdt()
//!     .tick(|s| tick(s, 100.0, 100.0))
//!     .tick(|s| tick(s, 105.0, 100.0))
//!     .tick(|s| tick(s, 95.0, 100.0))
//!     .execute::<MyGen>();
//! ```
//!
//! [`SignalGenerator`]: crate::SignalGenerator

use std::sync::Arc;

use crate::{
    MarketSignal, SignalGenerator,
    test_util::{EnforcingMarketSnapshot, FakeMarketSnapshot, RecordingMarketSignalConfig},
};

/// Driver that runs a [`SignalGenerator`] across a sequence of ticks
/// for a single instrument.
///
/// Build with one of the per-instrument constructors (e.g.
/// [`FakeEngine::btcusdt`]), append ticks with [`tick`](Self::tick),
/// then run with [`execute`](Self::execute).
///
/// Each tick is a closure that customises a fresh
/// [`FakeMarketSnapshot`] for the chosen instrument — typically by
/// declaring one or more timeframes via
/// [`with_timeframe`](FakeMarketSnapshot::with_timeframe).
pub struct FakeEngine {
    instrument_factory: fn() -> FakeMarketSnapshot,
    ticks: Vec<FakeMarketSnapshot>,
}

impl FakeEngine {
    /// Drives ticks against `BTCUSDT` perpetuals.
    #[must_use]
    pub fn btcusdt() -> Self {
        Self {
            instrument_factory: FakeMarketSnapshot::btcusdt,
            ticks: Vec::new(),
        }
    }

    /// Appends a tick. The closure receives a fresh
    /// [`FakeMarketSnapshot`] for the engine's instrument; populate
    /// it with the timeframes the generator under test will read.
    #[must_use]
    pub fn tick(mut self, f: impl FnOnce(FakeMarketSnapshot) -> FakeMarketSnapshot) -> Self {
        let snapshot = f((self.instrument_factory)());
        self.ticks.push(snapshot);
        self
    }

    /// Runs `G` across each pushed tick and returns the per-tick
    /// signals in order.
    ///
    /// `configure` runs exactly once on a freshly-defaulted generator;
    /// the resulting recorder is shared (via `Arc`) across every
    /// tick's [`EnforcingMarketSnapshot`].
    #[must_use]
    pub fn execute<G: SignalGenerator>(self) -> Vec<Option<MarketSignal>> {
        let generator = G::default();
        let config = Arc::new(generator.configure(RecordingMarketSignalConfig::new()));

        self.ticks
            .into_iter()
            .map(|snapshot| {
                let ems = EnforcingMarketSnapshot::new(snapshot, Arc::clone(&config));
                generator.evaluate(&ems)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use quantedge_core::{Bar, MarketSnapshot, Timeframe, TimeframeSnapshot, nz};
    use quantedge_ta::EmaConfig;

    use crate::{
        MarketSide, MarketSignal, MarketSignalConfig, SignalGenerator,
        test_util::{FakeEngine, FakeMarketSnapshot},
    };

    // Minimal generator: declares H4 + D1 and EMA(9), reads ema9 on
    // H4's prev/forming, fires `up` when ema9 rose. D1 is declared
    // (and its EMA21 read) purely to exercise the multi-timeframe
    // path through EMS.
    #[derive(Default)]
    struct EmaUpGen;

    impl SignalGenerator for EmaUpGen {
        fn id(&self) -> &'static str {
            "test_ema_up"
        }
        fn name(&self) -> &'static str {
            "test ema up"
        }
        fn configure<C: MarketSignalConfig>(&self, config: C) -> C {
            config
                .require_closed_bars(1)
                .require_timeframes(&[Timeframe::HOUR_4, Timeframe::DAY_1])
                .register(&ema9())
                .register(&ema21())
        }
        fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal> {
            let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);
            let d1 = snapshot.for_timeframe(Timeframe::DAY_1);

            let _ = d1.forming().value(&ema21())?;
            let prev = h4.closed(0)?.value(&ema9())?;
            let cur = h4.forming().value(&ema9())?;

            (cur > prev).then(|| {
                MarketSignal::from_forming(self, snapshot, Timeframe::HOUR_4, "up")
                    .with_side(MarketSide::Long)
                    .build()
            })
        }
    }

    fn ema9() -> EmaConfig {
        EmaConfig::close(nz(9))
    }

    fn ema21() -> EmaConfig {
        EmaConfig::close(nz(21))
    }

    // Per-tick helper: `extract a tick helper` pattern from the module
    // docs. Each test parameterises the values that change tick-to-tick.
    fn tick(s: FakeMarketSnapshot, h4_forming: f64, h4_prev: f64) -> FakeMarketSnapshot {
        s.with_timeframe(Timeframe::HOUR_4, |t| {
            t.forming_value(&ema9(), h4_forming)
                .add_closed_value(&ema9(), h4_prev)
        })
        .with_timeframe(Timeframe::DAY_1, |t| t.forming_value(&ema21(), 1.0))
    }

    /// Three ticks across H4 + D1: signal only fires on tick 2.
    #[test]
    fn execute_runs_generator_per_tick_across_multiple_timeframes() {
        let signals = FakeEngine::btcusdt()
            .tick(|s| tick(s, 100.0, 100.0))
            .tick(|s| tick(s, 105.0, 100.0))
            .tick(|s| tick(s, 95.0, 100.0))
            .execute::<EmaUpGen>();

        assert_eq!(signals.len(), 3);
        assert!(signals[0].is_none());
        assert_eq!(signals[1].as_ref().unwrap().key, "up");
        assert!(signals[2].is_none());
    }
}
