//! Fakes for unit-testing [`SignalGenerator::evaluate`] without a real
//! TA or bar-history pipeline.
//!
//! [`FakeBar`] holds an [`Ohlcv`], a closed/forming flag, and a map of
//! indicator values keyed by [`IndicatorConfig`]. The values are
//! injected directly via [`FakeBar::add_value`] — no indicator math
//! runs — so tests exercise a generator's branching logic in isolation
//! from indicator behaviour.
//!
//! [`FakeTimeframeSnapshot`] holds a [`Timeframe`], a forming
//! [`FakeBar`], and a stack of closed [`FakeBar`]s ordered most-recent
//! first. All `add_closed*` methods auto-align bar timestamps to the
//! timeframe boundary, walking backwards from the current oldest bar
//! (or the forming bar's `open_time` when none exist) — caller-supplied
//! `open_time` values on input bars are silently overwritten so
//! sequencing stays consistent regardless of how the API is composed.
//!
//! [`FakeMarketSnapshot`] holds an [`Instrument`] and a per-[`Timeframe`]
//! map of [`FakeTimeframeSnapshot`]s. Common [`Instrument`] components
//! are exposed as [`tickers`](super::tickers),
//! [`market_kinds`](super::market_kinds), and [`venues`](super::venues).
//!
//! [`SignalGenerator::evaluate`]: crate::SignalGenerator::evaluate

use std::{
    any::Any,
    collections::HashMap,
    fmt::{self, Debug, Display, Formatter},
    ops::Range,
};

use quantedge_core::{
    Bar, ErasedIndicatorConfig, IndicatorConfig, Instrument, MarketSnapshot, Ohlcv, Price,
    Timeframe, TimeframeSnapshot, Timestamp, test_util::bar,
};

/// Default forming-bar `open_time` used by [`FakeTimeframeSnapshot::new`]:
/// midnight 2026-01-01 UTC in microseconds since epoch. Divisible by
/// `HOUR_1` / `HOUR_4` / `DAY_1` periods so the snap inside `forming_at`
/// is a no-op for those common timeframes. Far enough from epoch that
/// closed-bar history of any reasonable depth fits without underflow.
pub const DEFAULT_FORMING_TIME: Timestamp = 1_767_225_600_000_000;

/// Fake [`Bar`] with an explicit closed/forming state and indicator
/// values supplied directly by the test.
///
/// Construct with [`forming`](Self::forming) or [`closed`](Self::closed),
/// then layer on indicator values with [`add_value`](Self::add_value).
/// [`Bar::value`] returns whatever was injected for a given config and
/// `None` for any config that was not — letting tests model the
/// warm-up window by simply omitting `add_value`. The [`Bar::value`]
/// trait contract panics on *unsubscribed* configs, but subscription
/// is a snapshot-level concept; that enforcement belongs in a wrapper
/// that has access to the generator's declared indicators.
#[derive(Debug)]
pub struct FakeBar {
    ohlcv: Ohlcv,
    is_closed: bool,
    values: HashMap<Box<dyn ErasedIndicatorConfig>, Box<dyn Any + Send + Sync>>,
}

impl FakeBar {
    /// Creates a forming (in-progress) bar with no indicator values.
    ///
    /// [`Bar::is_closed`] returns `false`. Add indicator values with
    /// [`add_value`](Self::add_value). For close-only forming bars use
    /// `FakeBar::forming(bar(close, time))`; for richer shapes see
    /// [`forming_with_volume`](Self::forming_with_volume),
    /// [`forming_ohlc`](Self::forming_ohlc), and
    /// [`forming_ohlcv`](Self::forming_ohlcv).
    #[must_use]
    pub fn forming(ohlcv: Ohlcv) -> Self {
        Self {
            ohlcv,
            is_closed: false,
            values: HashMap::new(),
        }
    }

    /// Forming bar with OHLC collapsed to `close`, given `volume` and
    /// `open_time`.
    #[must_use]
    pub fn forming_with_volume(close: Price, volume: f64, open_time: Timestamp) -> Self {
        Self::forming(bar(close, open_time).vol(volume))
    }

    /// Forming bar with full `(open, high, low, close)` OHLC at
    /// `open_time`, `volume = 0.0`.
    #[must_use]
    pub fn forming_ohlc(
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        open_time: Timestamp,
    ) -> Self {
        Self::forming(Ohlcv::new(open, high, low, close).at(open_time))
    }

    /// Forming bar with full `(open, high, low, close, volume)` OHLCV
    /// at `open_time`.
    #[must_use]
    pub fn forming_ohlcv(
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        volume: f64,
        open_time: Timestamp,
    ) -> Self {
        Self::forming(Ohlcv::new(open, high, low, close).at(open_time).vol(volume))
    }

    /// Creates a closed bar with no indicator values.
    ///
    /// [`Bar::is_closed`] returns `true`. Add indicator values with
    /// [`add_value`](Self::add_value).
    ///
    /// `open_time` on the supplied [`Ohlcv`] is honored when the bar is
    /// used standalone, but is overwritten when the bar is appended via
    /// [`FakeTimeframeSnapshot::add_closed`] / friends — those methods
    /// derive the open time from the snapshot's anchor.
    #[must_use]
    pub fn closed(ohlcv: Ohlcv) -> Self {
        Self {
            ohlcv,
            is_closed: true,
            values: HashMap::new(),
        }
    }

    /// Closed bar with OHLC collapsed to `close` and given `volume`.
    /// `open_time` defaults to `0` and is overwritten when the bar is
    /// appended via [`FakeTimeframeSnapshot::add_closed`] / friends.
    #[must_use]
    pub fn closed_with_volume(close: Price, volume: f64) -> Self {
        Self::closed(bar(close, 0).vol(volume))
    }

    /// Closed bar with full `(open, high, low, close)` OHLC at
    /// `open_time`, `volume = 0.0`.
    #[must_use]
    pub fn closed_ohlc(
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        open_time: Timestamp,
    ) -> Self {
        Self::closed(Ohlcv::new(open, high, low, close).at(open_time))
    }

    /// Closed bar with full `(open, high, low, close, volume)` OHLCV
    /// at `open_time`.
    #[must_use]
    pub fn closed_ohlcv(
        open: Price,
        high: Price,
        low: Price,
        close: Price,
        volume: f64,
        open_time: Timestamp,
    ) -> Self {
        Self::closed(Ohlcv::new(open, high, low, close).at(open_time).vol(volume))
    }

    /// Injects `value` as the result [`Bar::value`] returns when
    /// queried with `config`.
    ///
    /// Identity is by [`IndicatorConfig`] equality — an indicator with
    /// different parameters (e.g. a different length) is a different
    /// indicator. Calling `add_value` again with the same `config`
    /// overwrites the previous value.
    #[must_use]
    pub fn add_value<C: IndicatorConfig>(mut self, config: &C, value: C::Output) -> Self {
        self.values.insert(config.clone_erased(), Box::new(value));
        self
    }
}

impl Bar for FakeBar {
    fn is_closed(&self) -> bool {
        self.is_closed
    }

    fn ohlcv(&self) -> Ohlcv {
        self.ohlcv
    }

    fn value<C: IndicatorConfig>(&self, config: &C) -> Option<C::Output> {
        let key: &dyn ErasedIndicatorConfig = config;
        self.values.get(key)?.downcast_ref::<C::Output>().copied()
    }
}

impl Display for FakeBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FakeBar(closed={}, open_time={}, close={}, indicators={})",
            self.is_closed,
            self.ohlcv.open_time,
            self.ohlcv.close,
            self.values.len(),
        )
    }
}

/// Fake [`TimeframeSnapshot`] populated bar-by-bar by the test.
///
/// Construct with [`new`](Self::new), then push closed bars with
/// [`add_closed`](Self::add_closed) (single) or
/// [`add_closed_iter`](Self::add_closed_iter) (iter). Each call extends
/// history backwards: the first call becomes the most recent closed
/// bar (returned by [`closed(0)`](TimeframeSnapshot::closed)); each
/// subsequent call becomes one period older. Bars are stored
/// most-recent-first internally so [`closed`](Self::closed) and
/// [`at`](Self::at) match the [`TimeframeSnapshot`] indexing contract:
/// `closed(0)` is the most recent closed bar.
///
/// All `add_closed*` methods auto-align bar `open_time`s to
/// [`Self::timeframe`], walking backwards from the current oldest
/// closed bar (or from the forming bar when none exist). Caller-supplied
/// `open_time` values on input bars are overwritten — composition is
/// always consistent regardless of which `add_closed*` variant is used
/// or in what order.
///
/// `tick_time` and `max_bars` are inferred by default — `tick_time`
/// from the forming bar's `open_time` (the bar-boundary value at which
/// the snapshot is observed), `max_bars` from `closed_count() + 1`.
/// Tests that assert on tick-time semantics (age, latency, intra-bar
/// ordering) should override via [`with_tick_time`](Self::with_tick_time);
/// the default is a bar boundary, not an intra-bar tick.
#[derive(Debug)]
pub struct FakeTimeframeSnapshot {
    timeframe: Timeframe,
    pub(crate) forming: FakeBar,
    pub(crate) closed: Vec<FakeBar>,
    tick_time: Option<Timestamp>,
    max_bars: Option<usize>,
}

impl FakeTimeframeSnapshot {
    /// Creates a snapshot with a default forming bar (OHLC = 0.0,
    /// volume = 0.0) at midnight 2026-01-01 UTC
    /// ([`DEFAULT_FORMING_TIME`]), snapped to the `timeframe` boundary.
    ///
    /// The forming bar's `open_time` is the single source of truth
    /// for the snapshot's position in time — closed bars added via
    /// [`add_closed`](Self::add_closed) and friends count backwards
    /// from it. Override the anchor via [`forming_at`](Self::forming_at).
    ///
    /// Customize the forming bar's *content* (OHLCV, indicators) with
    /// [`forming_with`](Self::forming_with) (closure),
    /// [`forming_value`](Self::forming_value) (single-indicator
    /// shorthand), or [`replace_forming`](Self::replace_forming)
    /// (explicit replacement).
    ///
    /// `tick_time` defaults to the forming bar's `open_time` and
    /// `max_bars` defaults to `closed_count() + 1`; override via
    /// [`with_tick_time`](Self::with_tick_time) or
    /// [`with_max_bars`](Self::with_max_bars).
    #[must_use]
    pub fn new(timeframe: Timeframe) -> Self {
        let anchor = timeframe.open_time(DEFAULT_FORMING_TIME);
        Self {
            timeframe,
            forming: FakeBar::forming(bar(0.0, anchor)),
            closed: vec![],
            tick_time: None,
            max_bars: None,
        }
    }

    /// Sets the forming bar's `open_time`, snapped to the `timeframe`
    /// boundary. This is the only knob for the snapshot's position in
    /// time — closed bars added afterwards count backwards from it.
    ///
    /// Call before [`add_closed`](Self::add_closed) and friends to keep
    /// closed-bar timestamps consistent with the new anchor.
    #[must_use]
    pub fn forming_at(mut self, time: Timestamp) -> Self {
        self.forming.ohlcv.open_time = self.timeframe.open_time(time);
        self
    }

    /// Customizes the forming bar's content via a closure. The closure
    /// receives the current forming bar; the returned bar's OHLCV
    /// (excluding `open_time`) and indicator values are kept.
    ///
    /// Typical use: `.forming_with(|b| b.add_value(&ema9, 1.0).add_value(&ema21, 2.0))`.
    /// For the single-indicator case, prefer the
    /// [`forming_value`](Self::forming_value) shorthand.
    ///
    /// # Panics
    ///
    /// Panics if the closure returns a bar whose `open_time` differs
    /// from the snapshot's anchor. Use [`forming_at`](Self::forming_at)
    /// to change the anchor; this method is for content only.
    #[must_use]
    #[track_caller]
    pub fn forming_with(mut self, f: impl FnOnce(FakeBar) -> FakeBar) -> Self {
        let anchor = self.forming.ohlcv.open_time;
        let new_forming = f(self.forming);
        assert_eq!(
            new_forming.ohlcv.open_time, anchor,
            "FakeTimeframeSnapshot::forming_with: closure returned a bar with open_time={} but snapshot anchor is {}. \
             Use `forming_at(time)` to change the anchor; `forming_with` is for content only.",
            new_forming.ohlcv.open_time, anchor
        );
        self.forming = new_forming;
        self
    }

    /// Attaches a single indicator value to the forming bar.
    ///
    /// Shorthand for `.forming_with(|b| b.add_value(cfg, value))` —
    /// covers the common single-indicator case without the closure
    /// noise. For multi-indicator bars, use
    /// [`forming_with`](Self::forming_with).
    #[must_use]
    pub fn forming_value<C: IndicatorConfig>(self, config: &C, value: C::Output) -> Self {
        self.forming_with(|b| b.add_value(config, value))
    }

    /// Replaces the forming bar's content with `forming`.
    ///
    /// # Panics
    ///
    /// Panics if `forming.open_time` differs from the snapshot's
    /// anchor. Use [`forming_at`](Self::forming_at) to change the
    /// anchor first if needed.
    #[must_use]
    #[track_caller]
    pub fn replace_forming(mut self, forming: FakeBar) -> Self {
        let anchor = self.forming.ohlcv.open_time;
        assert_eq!(
            forming.ohlcv.open_time, anchor,
            "FakeTimeframeSnapshot::replace_forming: input bar has open_time={} but snapshot anchor is {}. \
             Use `forming_at(time)` to change the anchor first, or build the bar at the snapshot's anchor.",
            forming.ohlcv.open_time, anchor
        );
        self.forming = forming;
        self
    }

    /// Pushes one closed bar onto the snapshot's history. The bar's
    /// `open_time` is overwritten with the next backward-aligned
    /// timeframe boundary — one period older than the current oldest
    /// closed bar (or the forming bar when none exist).
    ///
    /// First call becomes the most recent closed bar
    /// ([`closed(0)`](TimeframeSnapshot::closed)); each subsequent call
    /// becomes one period older.
    ///
    /// # Panics
    ///
    /// Panics if the snapshot's oldest `open_time` is too small to
    /// step backwards by one period. Set the forming bar's `open_time`
    /// to a sufficiently large, timeframe-aligned timestamp (the
    /// default [`DEFAULT_FORMING_TIME`] fits any reasonable history).
    #[must_use]
    #[track_caller]
    pub fn add_closed(mut self, mut closed: FakeBar) -> Self {
        closed.ohlcv.open_time = self.next_aligned_open_time();
        self.closed.push(closed);
        self
    }

    /// Bulk variant of [`add_closed`](Self::add_closed). Items are
    /// consumed in chronological order (oldest first) within the
    /// batch; the last item abuts the current oldest closed bar (or
    /// the forming bar when none exist). Each item's `open_time` is
    /// overwritten with a backward-aligned timeframe boundary.
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_iter(mut self, closeds: impl IntoIterator<Item = FakeBar>) -> Self {
        let bars: Vec<FakeBar> = closeds.into_iter().collect();
        let times = self.aligned_closed_times(bars.len());
        self.closed
            .extend(bars.into_iter().rev().zip(times).map(|(mut b, t)| {
                b.ohlcv.open_time = t;
                b
            }));
        self
    }

    /// Pushes one closed bar via a closure that receives a default
    /// `FakeBar::closed(...)` seeded with the next backward-aligned
    /// `open_time`. The returned bar's `open_time` is overwritten with
    /// the snapshot's computed value regardless of what the closure
    /// returns.
    ///
    /// Multiple calls extend history backwards: the first call becomes
    /// the most recent closed bar (`closed(0)`); each subsequent call
    /// becomes one period older.
    ///
    /// Typical use is to attach indicator values to closed bars:
    /// `.add_closed_with(|b| b.add_value(&ema9, prev_value))`.
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_with(mut self, f: impl FnOnce(FakeBar) -> FakeBar) -> Self {
        let new_open_time = self.next_aligned_open_time();
        let mut new_closed = f(FakeBar::closed(bar(0.0, new_open_time)));
        new_closed.ohlcv.open_time = new_open_time;
        self.closed.push(new_closed);
        self
    }

    /// Pushes one closed bar carrying a single indicator value.
    ///
    /// Shorthand for `.add_closed_with(|b| b.add_value(cfg, value))` —
    /// covers the common single-indicator case without the closure
    /// noise. For multi-indicator closed bars, use
    /// [`add_closed_with`](Self::add_closed_with).
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_value<C: IndicatorConfig>(self, config: &C, value: C::Output) -> Self {
        self.add_closed_with(|b| b.add_value(config, value))
    }

    /// Pushes closed bars whose OHLC collapses to each `prices` entry,
    /// with `open_time`s aligned to [`Self::timeframe`] by stepping
    /// backwards from the current oldest closed bar (or the forming
    /// bar when none exist).
    ///
    /// Within a batch, prices are interpreted chronologically:
    /// `prices[0]` is the oldest, `prices[len - 1]` is the most recent
    /// (abutting the current oldest closed bar or the forming bar).
    /// Across calls, each batch extends further backwards.
    ///
    /// Companion variants:
    /// [`add_closed_with_volume`](Self::add_closed_with_volume),
    /// [`add_closed_ohlc`](Self::add_closed_ohlc),
    /// [`add_closed_ohlcv`](Self::add_closed_ohlcv).
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_prices(mut self, prices: &[Price]) -> Self {
        let times = self.aligned_closed_times(prices.len());
        self.closed.extend(
            prices
                .iter()
                .rev()
                .zip(times)
                .map(|(&price, time)| FakeBar::closed(bar(price, time))),
        );
        self
    }

    /// Pushes closed bars from `(close, volume)` pairs, with OHLC
    /// collapsed to `close` and `open_time`s aligned to
    /// [`Self::timeframe`]. Pairs interpreted chronologically (oldest
    /// first); see [`add_closed_prices`](Self::add_closed_prices) for
    /// the timing convention.
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_with_volume(mut self, rows: &[(Price, f64)]) -> Self {
        let times = self.aligned_closed_times(rows.len());
        self.closed.extend(
            rows.iter()
                .rev()
                .zip(times)
                .map(|(&(close, volume), time)| FakeBar::closed(bar(close, time).vol(volume))),
        );
        self
    }

    /// Pushes closed bars from `(open, high, low, close)` tuples, with
    /// `volume = 0.0` and `open_time`s aligned to [`Self::timeframe`].
    /// Tuples interpreted chronologically (oldest first); see
    /// [`add_closed_prices`](Self::add_closed_prices) for the timing
    /// convention.
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_ohlc(mut self, rows: &[(Price, Price, Price, Price)]) -> Self {
        let times = self.aligned_closed_times(rows.len());
        self.closed.extend(
            rows.iter()
                .rev()
                .zip(times)
                .map(|(&(o, h, l, c), time)| FakeBar::closed(Ohlcv::new(o, h, l, c).at(time))),
        );
        self
    }

    /// Pushes closed bars from `(open, high, low, close, volume)`
    /// tuples, with `open_time`s aligned to [`Self::timeframe`].
    /// Tuples interpreted chronologically (oldest first); see
    /// [`add_closed_prices`](Self::add_closed_prices) for the timing
    /// convention.
    ///
    /// # Panics
    ///
    /// Same as [`add_closed`](Self::add_closed).
    #[must_use]
    #[track_caller]
    pub fn add_closed_ohlcv(mut self, rows: &[(Price, Price, Price, Price, f64)]) -> Self {
        let times = self.aligned_closed_times(rows.len());
        self.closed.extend(
            rows.iter()
                .rev()
                .zip(times)
                .map(|(&(o, h, l, c, v), time)| {
                    FakeBar::closed(Ohlcv::new(o, h, l, c).at(time).vol(v))
                }),
        );
        self
    }

    /// Returns the next backward-aligned `open_time`: one timeframe
    /// period before the current oldest closed bar (or the forming
    /// bar when none exist).
    #[track_caller]
    fn next_aligned_open_time(&self) -> Timestamp {
        self.aligned_closed_times(1)[0]
    }

    /// Computes `count` `open_time`s walking backwards from the current
    /// oldest closed bar (or the forming bar when none exist), snapped
    /// to timeframe boundaries.
    ///
    /// Returned in newest-first order (`times[0]` is one period before
    /// the current oldest; `times[count - 1]` is `count` periods back).
    #[track_caller]
    fn aligned_closed_times(&self, count: usize) -> Vec<Timestamp> {
        let mut current = self
            .closed
            .last()
            .map_or(self.forming.ohlcv.open_time, |b| b.ohlcv.open_time);
        let mut times = Vec::with_capacity(count);
        for _ in 0..count {
            let prev = current.checked_sub(1).expect(
                "FakeTimeframeSnapshot: oldest open_time too small to step backwards; \
                 set forming.open_time to a sufficiently large, timeframe-aligned timestamp",
            );
            current = self.timeframe.open_time(prev);
            times.push(current);
        }
        times
    }

    /// Overrides the [`TimeframeSnapshot::tick_time`] value.
    ///
    /// Defaults to the forming bar's `open_time` (a bar-boundary
    /// value, not an intra-bar tick) when not set. Tests asserting on
    /// tick-time semantics (age, latency, intra-bar ordering) should
    /// set this explicitly.
    #[must_use]
    pub fn with_tick_time(mut self, tick_time: Timestamp) -> Self {
        self.tick_time = Some(tick_time);
        self
    }

    /// Overrides the [`TimeframeSnapshot::max_bars`] value.
    ///
    /// Defaults to `closed_count() + 1` when not set — i.e. exactly
    /// the bars currently held.
    #[must_use]
    pub fn with_max_bars(mut self, max_bars: usize) -> Self {
        self.max_bars = Some(max_bars);
        self
    }
}

impl TimeframeSnapshot for FakeTimeframeSnapshot {
    fn max_bars(&self) -> usize {
        self.max_bars.unwrap_or(self.closed.len() + 1)
    }

    fn closed_count(&self) -> usize {
        self.closed.len()
    }

    fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    fn tick_time(&self) -> Timestamp {
        self.tick_time.unwrap_or(self.forming.ohlcv.open_time)
    }

    fn at(&self, idx: usize) -> Option<&impl Bar> {
        if idx == 0 {
            return Some(&self.forming);
        }

        self.closed.get(idx - 1)
    }

    fn bars(&self, range: Range<usize>) -> impl Iterator<Item = &impl Bar> {
        range.map_while(|idx| self.at(idx))
    }

    fn forming(&self) -> &impl Bar {
        &self.forming
    }

    fn closed(&self, idx: usize) -> Option<&impl Bar> {
        self.closed.get(idx)
    }
}

impl Display for FakeTimeframeSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FakeTimeframeSnapshot(tf={}, closed={}, tick={})",
            self.timeframe,
            self.closed.len(),
            self.tick_time(),
        )
    }
}

/// Fake [`MarketSnapshot`] populated by the test with a known
/// [`Instrument`] and a set of per-[`Timeframe`] [`FakeTimeframeSnapshot`]s.
///
/// Construct with [`btcusdt`](Self::btcusdt) for the common BTC/USDT
/// fixture or [`for_instrument`](Self::for_instrument) for a
/// caller-built [`Instrument`], then build per-timeframe state with
/// [`with_timeframe`](Self::with_timeframe) (closure builder, anchors
/// each timeframe to the market's [`now`](Self::now)) or
/// [`add_timeframe`](Self::add_timeframe) (registers a pre-built
/// snapshot as-is).
///
/// `tick_time` defaults to the maximum `tick_time` across registered
/// timeframes. Calling [`tick_time`](MarketSnapshot::tick_time) when
/// no timeframes are registered and no override has been set panics
/// (the value would be meaningless). Override with
/// [`with_tick_time`](Self::with_tick_time) to assert a specific value
/// or to query an empty snapshot.
#[derive(Debug)]
pub struct FakeMarketSnapshot {
    instrument: Instrument,
    tick_time: Option<Timestamp>,
    now: Option<Timestamp>,
    pub(crate) timeframes: HashMap<Timeframe, FakeTimeframeSnapshot>,
}

impl FakeMarketSnapshot {
    /// Creates a snapshot for the BTC/USDT spot instrument on the
    /// synthetic `test` venue, with no timeframes.
    ///
    /// Convenience for the most common test setup. For other
    /// instruments use [`for_instrument`](Self::for_instrument).
    #[must_use]
    pub fn btcusdt() -> Self {
        use super::fixtures::{market_kinds, tickers, venues};
        Self::for_instrument(Instrument::new(
            venues::test(),
            tickers::btcusdt(),
            market_kinds::spot(),
        ))
    }

    /// Creates a snapshot for the given `instrument`, with no timeframes.
    #[must_use]
    pub fn for_instrument(instrument: Instrument) -> Self {
        Self {
            tick_time: None,
            now: None,
            instrument,
            timeframes: HashMap::new(),
        }
    }

    /// Sets the market-wide "now" timestamp used to anchor every
    /// timeframe added afterwards via
    /// [`with_timeframe`](Self::with_timeframe). Each timeframe snaps
    /// `now` to its own boundary, so `HOUR_4` and `DAY_1` ticking at
    /// "now = 5am" land their forming bars at 4am and midnight
    /// respectively.
    ///
    /// # Panics
    ///
    /// Panics if called more than once, or after any timeframe has
    /// been registered via [`with_timeframe`](Self::with_timeframe) or
    /// [`add_timeframe`](Self::add_timeframe). `now` would silently
    /// fail to propagate to already-registered timeframes; the panic
    /// makes the misuse loud.
    #[must_use]
    #[track_caller]
    pub fn with_now(mut self, now: Timestamp) -> Self {
        assert!(
            self.now.is_none(),
            "FakeMarketSnapshot::with_now: already set; call with_now() exactly once before any with_timeframe()/add_timeframe() call",
        );
        assert!(
            self.timeframes.is_empty(),
            "FakeMarketSnapshot::with_now: cannot be called after with_timeframe()/add_timeframe(); call with_now() first so it propagates to every registered timeframe",
        );
        self.now = Some(now);
        self
    }

    /// Builds and registers a [`FakeTimeframeSnapshot`] for `timeframe`
    /// via a closure. The closure receives a default snapshot anchored
    /// to the market's [`with_now`](Self::with_now) timestamp (or each
    /// timeframe's own default when `with_now` was not called),
    /// customizes it, and returns it.
    ///
    /// A repeated call with the same `timeframe` overwrites the
    /// previous registration.
    #[must_use]
    pub fn with_timeframe(
        mut self,
        timeframe: Timeframe,
        f: impl FnOnce(FakeTimeframeSnapshot) -> FakeTimeframeSnapshot,
    ) -> Self {
        let mut snap = FakeTimeframeSnapshot::new(timeframe);
        if let Some(now) = self.now {
            snap = snap.forming_at(now);
        }
        self.timeframes.insert(timeframe, f(snap));
        self
    }

    /// Registers `snapshot` under `timeframe` as-is. The market's
    /// [`with_now`](Self::with_now) is **not** applied; the snapshot
    /// keeps its own anchor. Use [`with_timeframe`](Self::with_timeframe)
    /// when you want the market's `now` to propagate.
    ///
    /// A repeated `add_timeframe` call with the same `timeframe`
    /// overwrites the previous registration.
    #[must_use]
    pub fn add_timeframe(mut self, timeframe: Timeframe, snapshot: FakeTimeframeSnapshot) -> Self {
        self.timeframes.insert(timeframe, snapshot);
        self
    }

    /// Overrides the [`MarketSnapshot::tick_time`] value.
    ///
    /// Defaults to the max `tick_time` across registered timeframes
    /// when not set. Required if you intend to call `tick_time()` on a
    /// snapshot with no timeframes — the default would be meaningless
    /// and panics instead.
    #[must_use]
    pub fn with_tick_time(mut self, tick_time: Timestamp) -> Self {
        self.tick_time = Some(tick_time);
        self
    }
}

impl MarketSnapshot for FakeMarketSnapshot {
    fn instrument(&self) -> Instrument {
        self.instrument.clone()
    }

    #[track_caller]
    fn tick_time(&self) -> Timestamp {
        if let Some(t) = self.tick_time {
            return t;
        }
        self.timeframes
            .values()
            .map(TimeframeSnapshot::tick_time)
            .max()
            .expect(
                "FakeMarketSnapshot::tick_time: no timeframes registered and no override set. \
                 Register at least one timeframe or call `with_tick_time(...)`.",
            )
    }

    #[track_caller]
    fn for_timeframe(&self, timeframe: Timeframe) -> &impl TimeframeSnapshot {
        match self.timeframes.get(&timeframe) {
            Some(snapshot) => snapshot,
            None => panic!(
                "FakeMarketSnapshot: timeframe {timeframe} was not registered. \
                 Add it via add_timeframe()."
            ),
        }
    }
}

impl Display for FakeMarketSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "FakeMarketSnapshot(instrument={}, timeframes=[",
            self.instrument,
        )?;
        for (i, tf) in self.timeframes.keys().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{tf}")?;
        }
        write!(f, "])")
    }
}

#[cfg(test)]
mod tests {
    mod fake_bar {
        use quantedge_core::{Bar, test_util::bar};
        use quantedge_ta::{BbConfig, BbValue, EmaConfig};

        use crate::test_util::FakeBar;

        #[test]
        fn create_forming() {
            let ohlcv = bar(22.2, 100);
            let bar = FakeBar::forming(ohlcv);

            assert!(!bar.is_closed());
            assert_eq!(bar.ohlcv(), ohlcv);
        }

        #[test]
        fn create_closed() {
            let ohlcv = bar(55.5, 200);
            let bar = FakeBar::closed(ohlcv);

            assert!(bar.is_closed());
            assert_eq!(bar.ohlcv(), ohlcv);
        }

        #[test]
        fn add_indicator_value() {
            let ohlcv = bar(77.7, 300);
            let config = EmaConfig::default();
            let value = 100.88;
            let bar = FakeBar::forming(ohlcv).add_value(&config, value);

            assert_eq!(bar.value(&config), Some(value));
        }

        #[test]
        fn value_returns_none_for_missing_config() {
            let bar = FakeBar::forming(bar(1.0, 0));

            assert_eq!(bar.value(&EmaConfig::default()), None);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn forming_constructors_capture_full_ohlcv() {
            // Spot-check the three forming_* helpers in one go — they
            // all delegate to `Ohlcv::new(...).at(...).vol(...)` so a
            // single ohlcv-coverage assertion per helper is enough.
            let with_volume = FakeBar::forming_with_volume(50.0, 1000.0, 999);
            assert!(!with_volume.is_closed());
            let o = with_volume.ohlcv();
            assert_eq!((o.close, o.volume, o.open_time), (50.0, 1000.0, 999));

            let ohlc = FakeBar::forming_ohlc(10.0, 12.0, 9.0, 11.0, 555);
            let o = ohlc.ohlcv();
            assert_eq!(
                (o.open, o.high, o.low, o.close, o.volume, o.open_time),
                (10.0, 12.0, 9.0, 11.0, 0.0, 555)
            );

            let ohlcv = FakeBar::forming_ohlcv(10.0, 12.0, 9.0, 11.0, 500.0, 777);
            let o = ohlcv.ohlcv();
            assert_eq!(
                (o.open, o.high, o.low, o.close, o.volume, o.open_time),
                (10.0, 12.0, 9.0, 11.0, 500.0, 777)
            );
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn closed_constructors_capture_full_ohlcv() {
            let with_volume = FakeBar::closed_with_volume(50.0, 1000.0);
            assert!(with_volume.is_closed());
            let o = with_volume.ohlcv();
            assert_eq!((o.close, o.volume), (50.0, 1000.0));

            let ohlc = FakeBar::closed_ohlc(10.0, 12.0, 9.0, 11.0, 555);
            let o = ohlc.ohlcv();
            assert_eq!(
                (o.open, o.high, o.low, o.close, o.volume, o.open_time),
                (10.0, 12.0, 9.0, 11.0, 0.0, 555)
            );

            let ohlcv = FakeBar::closed_ohlcv(10.0, 12.0, 9.0, 11.0, 500.0, 777);
            let o = ohlcv.ohlcv();
            assert_eq!(
                (o.open, o.high, o.low, o.close, o.volume, o.open_time),
                (10.0, 12.0, 9.0, 11.0, 500.0, 777)
            );
        }

        #[test]
        fn add_composite_indicator_value() {
            let ohlcv = bar(11.1, 400);
            let config = BbConfig::default();
            let value = BbValue {
                lower: 10.0,
                middle: 12.0,
                upper: 14.0,
            };
            let bar = FakeBar::forming(ohlcv).add_value(&config, value);

            assert_eq!(bar.value(&config), Some(value));
        }
    }

    mod fake_timeframe_snapshot {
        use quantedge_core::{Bar, Timeframe, TimeframeSnapshot, Timestamp, test_util::bar};

        use crate::test_util::{DEFAULT_FORMING_TIME, FakeBar, FakeTimeframeSnapshot};

        // 1000 hours from epoch in microseconds - aligned to HOUR_1.
        const H1_ANCHOR: Timestamp = 3_600_000_000_000_000;
        const H1: Timestamp = 3_600_000_000;

        #[test]
        fn forming_at_snaps_time_to_timeframe_boundary() {
            let raw = H1_ANCHOR + 1;
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).forming_at(raw);

            assert_eq!(snapshot.forming().open_time(), H1_ANCHOR);
            assert_eq!(snapshot.tick_time(), H1_ANCHOR);
        }

        #[test]
        fn forming_at_repositions_anchor_for_subsequent_closed_bars() {
            let raw = H1_ANCHOR + 1;
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(raw)
                .add_closed_prices(&[1.0, 2.0, 3.0]);

            let newest_closed = snapshot.closed(0).unwrap();
            assert_eq!(newest_closed.open_time() + H1, H1_ANCHOR);
        }

        #[test]
        fn default_forming_open_time_is_default_const() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1);

            assert_eq!(snapshot.forming().open_time(), DEFAULT_FORMING_TIME);
            assert_eq!(snapshot.tick_time(), DEFAULT_FORMING_TIME);
        }

        #[test]
        fn forming_with_preserves_open_time() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_with(|b| b.add_value(&quantedge_ta::EmaConfig::default(), 1.0));

            assert_eq!(snapshot.forming().open_time(), DEFAULT_FORMING_TIME);
        }

        #[test]
        #[should_panic(expected = "anchor")]
        fn forming_with_panics_on_time_mutation() {
            let _ = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_with(|_| FakeBar::forming(bar(42.0, 999)));
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn replace_forming_keeps_anchor() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .replace_forming(FakeBar::forming(bar(42.0, DEFAULT_FORMING_TIME)));

            assert_eq!(snapshot.forming().ohlcv().close, 42.0);
            assert_eq!(snapshot.forming().open_time(), DEFAULT_FORMING_TIME);
        }

        #[test]
        #[should_panic(expected = "anchor")]
        fn replace_forming_panics_on_time_mismatch() {
            let _ = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .replace_forming(FakeBar::forming(bar(42.0, 999)));
        }

        #[test]
        fn no_closed_bars() {
            let ohlcv = bar(99.88, H1_ANCHOR);
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(H1_ANCHOR)
                .replace_forming(FakeBar::forming(ohlcv));

            assert_eq!(snapshot.timeframe(), Timeframe::HOUR_1);
            assert_eq!(snapshot.closed_count(), 0);
            assert_eq!(snapshot.forming().ohlcv(), ohlcv);
            assert!(snapshot.closed(0).is_none());
        }

        #[test]
        fn tick_time_defaults_to_forming_open_time() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).forming_at(H1_ANCHOR);

            assert_eq!(snapshot.tick_time(), H1_ANCHOR);
        }

        #[test]
        fn tick_time_setter_overrides_default() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).with_tick_time(9999);

            assert_eq!(snapshot.tick_time(), 9999);
        }

        #[test]
        fn max_bars_defaults_to_closed_count_plus_one() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed(FakeBar::closed(bar(1.0, 0)))
                .add_closed(FakeBar::closed(bar(1.0, 0)));

            assert_eq!(snapshot.max_bars(), 3);
        }

        #[test]
        fn max_bars_setter_overrides_default() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed(FakeBar::closed(bar(1.0, 0)))
                .with_max_bars(50);

            assert_eq!(snapshot.max_bars(), 50);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn add_closed_extends_history_backwards() {
            // First call -> newest closed; each subsequent call -> one
            // period older. Caller-supplied open_time is overwritten.
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed(FakeBar::closed(bar(3.0, 0)))
                .add_closed(FakeBar::closed(bar(2.0, 0)))
                .add_closed(FakeBar::closed(bar(1.0, 0)));
            let anchor = snapshot.forming().open_time();

            assert_eq!(snapshot.closed_count(), 3);
            let newest = snapshot.closed(0).unwrap();
            let middle = snapshot.closed(1).unwrap();
            let oldest = snapshot.closed(2).unwrap();
            assert_eq!(newest.ohlcv().close, 3.0);
            assert_eq!(newest.open_time(), anchor - H1);
            assert_eq!(middle.ohlcv().close, 2.0);
            assert_eq!(middle.open_time(), anchor - 2 * H1);
            assert_eq!(oldest.ohlcv().close, 1.0);
            assert_eq!(oldest.open_time(), anchor - 3 * H1);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn add_closed_prices_aligns_open_times_to_timeframe() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed_prices(&[10.0, 20.0, 30.0]);
            let anchor = snapshot.forming().open_time();

            assert_eq!(snapshot.closed_count(), 3);
            let newest = snapshot.closed(0).unwrap();
            let middle = snapshot.closed(1).unwrap();
            let oldest = snapshot.closed(2).unwrap();
            assert_eq!(newest.ohlcv().close, 30.0);
            assert_eq!(newest.open_time(), anchor - H1);
            assert_eq!(middle.ohlcv().close, 20.0);
            assert_eq!(middle.open_time(), anchor - 2 * H1);
            assert_eq!(oldest.ohlcv().close, 10.0);
            assert_eq!(oldest.open_time(), anchor - 3 * H1);
        }

        #[test]
        fn add_closed_prices_empty_is_noop() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).add_closed_prices(&[]);

            assert_eq!(snapshot.closed_count(), 0);
        }

        #[test]
        #[should_panic(expected = "step backwards")]
        fn add_closed_prices_panics_on_underflow() {
            let _ = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(0)
                .add_closed_prices(&[1.0]);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn add_closed_prices_multi_call_extends_backwards() {
            // Two batches -> open_times monotonically decrease, no
            // duplicates: each batch picks up where the prior left off.
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed_prices(&[10.0, 20.0])
                .add_closed_prices(&[30.0, 40.0]);
            let anchor = snapshot.forming().open_time();

            assert_eq!(snapshot.closed_count(), 4);
            // closed[0] is newest from first batch; closed[3] is oldest from second.
            assert_eq!(snapshot.closed(0).unwrap().open_time(), anchor - H1);
            assert_eq!(snapshot.closed(1).unwrap().open_time(), anchor - 2 * H1);
            assert_eq!(snapshot.closed(2).unwrap().open_time(), anchor - 3 * H1);
            assert_eq!(snapshot.closed(3).unwrap().open_time(), anchor - 4 * H1);
            // Close prices follow chronology: first batch's newest abuts forming.
            assert_eq!(snapshot.closed(0).unwrap().ohlcv().close, 20.0);
            assert_eq!(snapshot.closed(1).unwrap().ohlcv().close, 10.0);
            assert_eq!(snapshot.closed(2).unwrap().ohlcv().close, 40.0);
            assert_eq!(snapshot.closed(3).unwrap().ohlcv().close, 30.0);
        }

        #[test]
        fn mixed_add_closed_methods_keep_timestamps_aligned() {
            // Compose every variant in arbitrary order; assert
            // open_times are strictly decreasing and one period apart.
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed_prices(&[1.0])
                .add_closed_with(|b| b)
                .add_closed_iter([FakeBar::closed(bar(2.0, 0))])
                .add_closed_with_volume(&[(3.0, 100.0)])
                .add_closed_ohlc(&[(1.0, 2.0, 0.5, 1.5)])
                .add_closed_ohlcv(&[(1.0, 2.0, 0.5, 1.5, 50.0)])
                .add_closed(FakeBar::closed(bar(4.0, 0)));
            let anchor = snapshot.forming().open_time();

            assert_eq!(snapshot.closed_count(), 7);
            for i in 0..snapshot.closed_count() {
                let expected = anchor - (i as Timestamp + 1) * H1;
                assert_eq!(
                    snapshot.closed(i).unwrap().open_time(),
                    expected,
                    "closed[{i}] open_time mismatch",
                );
            }
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn add_closed_with_attaches_indicator_and_extends_history() {
            let cfg = quantedge_ta::EmaConfig::default();
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed_with(|b| b.add_value(&cfg, 10.0))
                .add_closed_with(|b| b.add_value(&cfg, 20.0));

            assert_eq!(snapshot.closed_count(), 2);
            let newest = snapshot.closed(0).unwrap();
            let older = snapshot.closed(1).unwrap();
            assert_eq!(newest.value(&cfg), Some(10.0));
            assert_eq!(older.value(&cfg), Some(20.0));
            assert_eq!(newest.open_time() + H1, snapshot.forming().open_time());
            assert_eq!(older.open_time() + H1, newest.open_time());
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn add_closed_iter_chronological_within_batch() {
            // Within a batch: oldest first, last item abuts forming.
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).add_closed_iter([
                FakeBar::closed(bar(1.0, 0)),
                FakeBar::closed(bar(2.0, 0)),
                FakeBar::closed(bar(3.0, 0)),
            ]);

            assert_eq!(snapshot.closed_count(), 3);
            assert_eq!(snapshot.closed(0).unwrap().ohlcv().close, 3.0);
            assert_eq!(snapshot.closed(1).unwrap().ohlcv().close, 2.0);
            assert_eq!(snapshot.closed(2).unwrap().ohlcv().close, 1.0);
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn at_zero_is_forming_then_closed_most_recent_first() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(H1_ANCHOR)
                .replace_forming(FakeBar::forming(bar(10.0, H1_ANCHOR)))
                .add_closed(FakeBar::closed(bar(2.0, 0)))
                .add_closed(FakeBar::closed(bar(1.0, 0)));

            assert_eq!(snapshot.at(0).unwrap().ohlcv().close, 10.0);
            assert_eq!(snapshot.at(1).unwrap().ohlcv().close, 2.0);
            assert_eq!(snapshot.at(2).unwrap().ohlcv().close, 1.0);
            assert!(snapshot.at(3).is_none());
        }

        #[test]
        #[allow(clippy::float_cmp)]
        fn bars_iterates_in_indexing_order() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(H1_ANCHOR)
                .replace_forming(FakeBar::forming(bar(10.0, H1_ANCHOR)))
                .add_closed(FakeBar::closed(bar(2.0, 0)))
                .add_closed(FakeBar::closed(bar(1.0, 0)));

            let closes: Vec<f64> = snapshot.bars(0..3).map(|b| b.ohlcv().close).collect();
            assert_eq!(closes, vec![10.0, 2.0, 1.0]);
        }

        #[test]
        fn bars_stops_at_end() {
            let snapshot = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .add_closed(FakeBar::closed(bar(1.0, 0)));

            let count = snapshot.bars(0..100).count();
            assert_eq!(count, 2);
        }
    }

    mod fake_market_snapshot {
        use quantedge_core::{Bar, MarketSnapshot, Timeframe, TimeframeSnapshot, test_util::bar};

        use crate::test_util::{FakeBar, FakeMarketSnapshot, FakeTimeframeSnapshot};

        #[test]
        fn instrument_round_trips() {
            let snapshot = FakeMarketSnapshot::btcusdt();

            assert_eq!(snapshot.instrument().ticker().to_string(), "BTC/USDT");
        }

        #[test]
        #[should_panic(expected = "no timeframes registered")]
        fn tick_time_panics_when_no_timeframes_and_no_override() {
            let snapshot = FakeMarketSnapshot::btcusdt();

            let _ = snapshot.tick_time();
        }

        #[test]
        fn tick_time_defaults_to_max_across_timeframes() {
            // HOUR_4 anchor is later than HOUR_1 anchor.
            let h1_anchor = 3_600_000_000_000_000_u64; // 1000h
            let h4_anchor = 14_400_000_000_000_000_u64; // 4000h, aligned to HOUR_4
            let h1 = FakeTimeframeSnapshot::new(Timeframe::HOUR_1).forming_at(h1_anchor);
            let h4 = FakeTimeframeSnapshot::new(Timeframe::HOUR_4).forming_at(h4_anchor);
            let snapshot = FakeMarketSnapshot::btcusdt()
                .add_timeframe(Timeframe::HOUR_1, h1)
                .add_timeframe(Timeframe::HOUR_4, h4);

            assert_eq!(snapshot.tick_time(), h4_anchor);
        }

        #[test]
        fn tick_time_setter_overrides_default() {
            let snapshot = FakeMarketSnapshot::btcusdt().with_tick_time(7777);

            assert_eq!(snapshot.tick_time(), 7777);
        }

        #[test]
        fn for_timeframe_returns_registered_snapshot() {
            let anchor = 3_600_000_000_000_000; // HOUR_1-aligned
            let ohlcv = bar(100.0, anchor);
            let h1 = FakeTimeframeSnapshot::new(Timeframe::HOUR_1)
                .forming_at(anchor)
                .replace_forming(FakeBar::forming(ohlcv));
            let snapshot = FakeMarketSnapshot::btcusdt().add_timeframe(Timeframe::HOUR_1, h1);

            let returned = snapshot.for_timeframe(Timeframe::HOUR_1);
            assert_eq!(returned.timeframe(), Timeframe::HOUR_1);
            assert_eq!(returned.forming().ohlcv(), ohlcv);
        }

        #[test]
        #[should_panic(expected = "was not registered")]
        fn for_timeframe_panics_on_unregistered() {
            let snapshot = FakeMarketSnapshot::btcusdt();
            let _ = snapshot.for_timeframe(Timeframe::HOUR_4);
        }

        #[test]
        fn with_timeframe_anchors_to_market_now() {
            // 5am Jan 1 2026 — HOUR_4 snaps to 4am, DAY_1 snaps to midnight.
            let now = 1_767_225_600_000_000 + 5 * 3_600_000_000;
            let snapshot = FakeMarketSnapshot::btcusdt()
                .with_now(now)
                .with_timeframe(Timeframe::HOUR_4, |t| t)
                .with_timeframe(Timeframe::DAY_1, |t| t);

            let h4 = snapshot.for_timeframe(Timeframe::HOUR_4);
            let d1 = snapshot.for_timeframe(Timeframe::DAY_1);
            assert_eq!(h4.forming().open_time(), Timeframe::HOUR_4.open_time(now));
            assert_eq!(d1.forming().open_time(), Timeframe::DAY_1.open_time(now));
        }

        #[test]
        fn with_timeframe_uses_default_anchor_when_now_unset() {
            let snapshot = FakeMarketSnapshot::btcusdt().with_timeframe(Timeframe::HOUR_1, |t| t);

            let h1 = snapshot.for_timeframe(Timeframe::HOUR_1);
            assert_eq!(h1.forming().open_time(), 1_767_225_600_000_000);
        }

        #[test]
        fn with_timeframe_closure_can_chain_helpers() {
            let snapshot = FakeMarketSnapshot::btcusdt()
                .with_timeframe(Timeframe::HOUR_1, |t| t.add_closed_prices(&[1.0, 2.0, 3.0]));

            let h1 = snapshot.for_timeframe(Timeframe::HOUR_1);
            assert_eq!(h1.closed_count(), 3);
        }

        #[test]
        fn add_timeframe_does_not_apply_market_now() {
            let now = 1_767_225_600_000_000 + 100 * 3_600_000_000;
            let h1 = FakeTimeframeSnapshot::new(Timeframe::HOUR_1);
            let h1_anchor = h1.forming().open_time();
            let snapshot = FakeMarketSnapshot::btcusdt()
                .with_now(now)
                .add_timeframe(Timeframe::HOUR_1, h1);

            assert_eq!(
                snapshot
                    .for_timeframe(Timeframe::HOUR_1)
                    .forming()
                    .open_time(),
                h1_anchor
            );
        }

        #[test]
        #[should_panic(expected = "already set")]
        fn with_now_panics_when_called_twice() {
            let _ = FakeMarketSnapshot::btcusdt().with_now(1).with_now(2);
        }

        #[test]
        #[should_panic(expected = "cannot be called after")]
        fn with_now_panics_when_called_after_with_timeframe() {
            let _ = FakeMarketSnapshot::btcusdt()
                .with_timeframe(Timeframe::HOUR_1, |t| t)
                .with_now(1);
        }

        #[test]
        #[should_panic(expected = "cannot be called after")]
        fn with_now_panics_when_called_after_add_timeframe() {
            let h1 = FakeTimeframeSnapshot::new(Timeframe::HOUR_1);
            let _ = FakeMarketSnapshot::btcusdt()
                .add_timeframe(Timeframe::HOUR_1, h1)
                .with_now(1);
        }
    }
}
