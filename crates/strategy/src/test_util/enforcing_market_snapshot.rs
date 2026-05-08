//! Enforcing decorators that turn a [`FakeMarketSnapshot`] +
//! [`RecordingMarketSignalConfig`] pair into a strict
//! [`MarketSnapshot`] for testing [`SignalGenerator`]s.
//!
//! [`EnforcingMarketSnapshot`] is the public entry point. Its
//! `for_timeframe`/`closed`/`at`/`bars`/`value` methods **panic**
//! when the generator under test reads data it didn't declare in its
//! [`configure`](crate::SignalGenerator::configure):
//!
//! - timeframe not in `require_timeframes(&[..])`
//! - closed-bar index past `require_closed_bars(N)`
//! - indicator not in `register(&cfg)`
//!
//! [`SignalGenerator`]: crate::SignalGenerator

use std::{collections::HashMap, fmt::Display, sync::Arc};

use quantedge_core::{Bar, Instrument, MarketSnapshot, Timeframe, TimeframeSnapshot, Timestamp};
use quantedge_ta::{IndicatorConfig, Ohlcv};

use crate::test_util::{
    FakeBar, FakeMarketSnapshot, FakeTimeframeSnapshot, RecordingMarketSignalConfig,
};

#[derive(Debug)]
struct EnforcingBar {
    bar: FakeBar,
    signal_generator_config: Arc<RecordingMarketSignalConfig>,
}

impl EnforcingBar {
    #[must_use]
    fn new(bar: FakeBar, signal_generator_config: Arc<RecordingMarketSignalConfig>) -> Self {
        Self {
            bar,
            signal_generator_config,
        }
    }
}

impl Bar for EnforcingBar {
    fn is_closed(&self) -> bool {
        self.bar.is_closed()
    }

    fn ohlcv(&self) -> Ohlcv {
        self.bar.ohlcv()
    }

    #[track_caller]
    fn value<C: IndicatorConfig>(&self, config: &C) -> Option<C::Output> {
        assert!(
            self.signal_generator_config.has_indicator(config),
            "EnforcingBar: indicator {config} accessed but not registered in `configure()`. \
             Add `.register(&{config})` to your generator's `configure()`."
        );

        self.bar.value(config)
    }
}

impl Display for EnforcingBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EnforcingBar({})", self.bar)
    }
}

#[derive(Debug)]
struct EnforcingTimeframeSnapshot {
    timeframe: Timeframe,
    tick_time: Timestamp,
    max_bars: usize,
    forming: EnforcingBar,
    closed: Vec<EnforcingBar>,
    signal_generator_config: Arc<RecordingMarketSignalConfig>,
}

impl EnforcingTimeframeSnapshot {
    #[must_use]
    fn new(
        timeframe_snapshot: FakeTimeframeSnapshot,
        signal_generator_config: Arc<RecordingMarketSignalConfig>,
    ) -> Self {
        let timeframe = timeframe_snapshot.timeframe();
        let tick_time = timeframe_snapshot.tick_time();
        let max_bars = timeframe_snapshot.max_bars();
        let forming = EnforcingBar::new(
            timeframe_snapshot.forming,
            Arc::clone(&signal_generator_config),
        );
        let closed = timeframe_snapshot
            .closed
            .into_iter()
            .map(|b| EnforcingBar::new(b, Arc::clone(&signal_generator_config)))
            .collect();

        Self {
            timeframe,
            tick_time,
            max_bars,
            forming,
            closed,
            signal_generator_config,
        }
    }
}

impl TimeframeSnapshot for EnforcingTimeframeSnapshot {
    fn max_bars(&self) -> usize {
        self.max_bars
    }

    fn closed_count(&self) -> usize {
        self.closed.len()
    }

    fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    fn tick_time(&self) -> Timestamp {
        self.tick_time
    }

    #[track_caller]
    fn at(&self, idx: usize) -> Option<&impl Bar> {
        let cap = self.signal_generator_config.required_closed_bars();

        assert!(
            idx <= cap,
            "EnforcingTimeframeSnapshot: at({idx}) exceeds budget - only the forming bar plus {cap} closed bars are reachable. \
             Increase via `.require_closed_bars({idx})` in `configure()`.",
        );
        if idx == 0 {
            return Some(&self.forming);
        }

        self.closed.get(idx - 1)
    }

    #[track_caller]
    fn bars(&self, range: std::ops::Range<usize>) -> impl Iterator<Item = &impl Bar> {
        let cap = self.signal_generator_config.required_closed_bars();
        let needed = range.end.saturating_sub(1);

        assert!(
            range.end <= cap + 1,
            "EnforcingTimeframeSnapshot: bars({start}..{end}) exceeds budget - only the forming bar plus {cap} closed bars are reachable. \
             Increase via `.require_closed_bars({needed})` in `configure()`.",
            start = range.start,
            end = range.end,
        );

        range.map_while(|idx| self.at(idx))
    }

    fn forming(&self) -> &impl Bar {
        &self.forming
    }

    #[track_caller]
    fn closed(&self, idx: usize) -> Option<&impl Bar> {
        let cap = self.signal_generator_config.required_closed_bars();
        let needed = idx + 1;

        assert!(
            idx < cap,
            "EnforcingTimeframeSnapshot: closed({idx}) exceeds required_closed_bars={cap}. \
             Increase via `.require_closed_bars({needed})` in `configure()`.",
        );

        self.closed.get(idx)
    }
}

impl Display for EnforcingTimeframeSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EnforcingTimeframeSnapshot(tf={}, closed={}, tick={})",
            self.timeframe,
            self.closed.len(),
            self.tick_time,
        )
    }
}

/// Strict [`MarketSnapshot`] for testing [`SignalGenerator`]s.
///
/// Wraps a [`FakeMarketSnapshot`] with a
/// [`RecordingMarketSignalConfig`] (typically captured by passing the
/// recorder through [`SignalGenerator::configure`]) so accesses can be
/// checked against what the generator actually declared. See the
/// [module docs][self] for the three checks.
///
/// [`SignalGenerator`]: crate::SignalGenerator
/// [`SignalGenerator::configure`]: crate::SignalGenerator::configure
#[derive(Debug)]
pub struct EnforcingMarketSnapshot {
    instrument: Instrument,
    tick_time: Timestamp,
    signal_generator_config: Arc<RecordingMarketSignalConfig>,
    wrappers: HashMap<Timeframe, EnforcingTimeframeSnapshot>,
}

impl EnforcingMarketSnapshot {
    /// Wraps `market_snapshot` so accesses are checked against
    /// `signal_generator_config`.
    ///
    /// Build the recorder by passing it through your generator's
    /// [`configure`](crate::SignalGenerator::configure):
    /// `let recorder = gen.configure(RecordingMarketSignalConfig::new());`.
    #[must_use]
    pub fn new(
        market_snapshot: FakeMarketSnapshot,
        signal_generator_config: Arc<RecordingMarketSignalConfig>,
    ) -> Self {
        let instrument = market_snapshot.instrument();
        let tick_time = market_snapshot.tick_time();
        let wrappers = market_snapshot
            .timeframes
            .into_iter()
            .map(|(tf, snap)| {
                (
                    tf,
                    EnforcingTimeframeSnapshot::new(snap, Arc::clone(&signal_generator_config)),
                )
            })
            .collect();

        Self {
            instrument,
            tick_time,
            signal_generator_config,
            wrappers,
        }
    }
}

impl MarketSnapshot for EnforcingMarketSnapshot {
    fn instrument(&self) -> Instrument {
        self.instrument.clone()
    }

    fn tick_time(&self) -> Timestamp {
        self.tick_time
    }

    #[track_caller]
    fn for_timeframe(&self, timeframe: Timeframe) -> &impl TimeframeSnapshot {
        assert!(
            self.signal_generator_config.has_timeframe(&timeframe),
            "EnforcingMarketSnapshot: timeframe {timeframe} accessed but not registered in `configure()`. \
             Add `.require_timeframes(&[{timeframe}])` to your generator's `configure()`."
        );

        self.wrappers
            .get(&timeframe)
            .expect("internal: wrappers/config out of sync")
    }
}

impl Display for EnforcingMarketSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "EnforcingMarketSnapshot(instrument={}, tick={}, timeframes={})",
            self.instrument,
            self.tick_time,
            self.wrappers.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use quantedge_core::{Bar, MarketSnapshot, Timeframe, TimeframeSnapshot, nz};
    use quantedge_ta::EmaConfig;

    use crate::{
        MarketSignalConfig,
        test_util::{EnforcingMarketSnapshot, FakeMarketSnapshot, RecordingMarketSignalConfig},
    };

    fn ema9() -> EmaConfig {
        EmaConfig::close(nz(9))
    }

    fn ema21() -> EmaConfig {
        EmaConfig::close(nz(21))
    }

    /// Builds an `EMS` over `HOUR_1` with `closed_bars` closed bars and a
    /// recorder configured for `cap` closed bars, the `HOUR_1` timeframe,
    /// and the EMA(9) indicator. The forming bar carries `ema9` = 1.0.
    fn fixture(closed_bars: usize, cap: usize) -> EnforcingMarketSnapshot {
        let market = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_1, |t| {
            t.forming_value(&ema9(), 1.0)
                .add_closed_prices(&vec![1.0; closed_bars])
        });

        let recorder = RecordingMarketSignalConfig::new()
            .require_timeframes(&[Timeframe::HOUR_1])
            .require_closed_bars(cap)
            .register(&ema9());

        EnforcingMarketSnapshot::new(market, Arc::new(recorder))
    }

    mod enforcing_bar {
        use super::*;

        #[test]
        fn value_returns_for_registered_indicator() {
            let ems = fixture(0, 0);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            assert_eq!(tf.forming().value(&ema9()), Some(1.0));
        }

        #[test]
        #[should_panic(expected = "not registered in `configure()`")]
        fn value_panics_on_unregistered_indicator() {
            let ems = fixture(0, 0);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            let _ = tf.forming().value(&ema21());
        }
    }

    mod enforcing_timeframe_snapshot {
        use super::*;

        #[test]
        fn at_zero_returns_forming_within_cap() {
            let ems = fixture(2, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            assert!(tf.at(0).is_some());
            assert!(tf.at(1).is_some());
            assert!(tf.at(2).is_some());
        }

        #[test]
        #[should_panic(expected = "exceeds budget")]
        fn at_panics_past_cap() {
            let ems = fixture(5, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            let _ = tf.at(3);
        }

        #[test]
        fn closed_returns_within_cap() {
            let ems = fixture(2, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            assert!(tf.closed(0).is_some());
            assert!(tf.closed(1).is_some());
        }

        #[test]
        #[should_panic(expected = "exceeds required_closed_bars=2")]
        fn closed_panics_past_cap() {
            let ems = fixture(5, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            let _ = tf.closed(2);
        }

        #[test]
        fn bars_iterates_within_cap() {
            let ems = fixture(2, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            let count = tf.bars(0..3).count();
            assert_eq!(count, 3);
        }

        #[test]
        #[should_panic(expected = "exceeds budget")]
        fn bars_panics_past_cap() {
            let ems = fixture(5, 2);
            let tf = ems.for_timeframe(Timeframe::HOUR_1);

            let _ = tf.bars(0..4).count();
        }
    }

    mod enforcing_market_snapshot {
        use super::*;

        #[test]
        fn for_timeframe_returns_registered() {
            let ems = fixture(0, 0);

            let tf = ems.for_timeframe(Timeframe::HOUR_1);
            assert_eq!(tf.timeframe(), Timeframe::HOUR_1);
        }

        #[test]
        #[should_panic(expected = "not registered in `configure()`")]
        fn for_timeframe_panics_on_unregistered() {
            let ems = fixture(0, 0);

            let _ = ems.for_timeframe(Timeframe::HOUR_4);
        }
    }
}
