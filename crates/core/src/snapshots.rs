//! Bar history and market snapshots for streaming strategy code.
//!
//! A [`TimeframeSnapshot`] is an immutable view at one tick: a rolling
//! window of bars at one [`Timeframe`] with the currently-forming bar
//! alongside closed history. A [`MarketSnapshot`] groups the
//! [`TimeframeSnapshot`]s subscribed for one [`Instrument`], keyed by
//! [`Timeframe`]. Successive ticks produce new snapshots; earlier ones
//! do not mutate.
//!
//! See [`TimeframeSnapshot`] for the indexing convention shared across
//! [`at`](TimeframeSnapshot::at), [`bars`](TimeframeSnapshot::bars),
//! and [`closed`](TimeframeSnapshot::closed).

use std::{
    fmt::{Debug, Display},
    ops::Range,
};

use crate::{IndicatorConfig, Instrument, Ohlcv, Timeframe, Timestamp};

/// A single bar within a [`TimeframeSnapshot`].
///
/// Wraps the underlying [`Ohlcv`] data and exposes any indicator values
/// registered on the owning snapshot. In a given snapshot the forming
/// bar shows the in-progress state for the current period; closed bars
/// show the final state of periods that have ended. Across successive
/// snapshots the forming bar advances while closed bars stay fixed.
pub trait Bar: PartialEq + Eq + Send + Sync + Display + Debug {
    /// Whether this bar's period has ended.
    ///
    /// Always `false` for the forming bar and always `true` for any
    /// bar returned by [`TimeframeSnapshot::closed`] — implies both
    /// that the period has elapsed and that this is not the
    /// currently-forming bar.
    fn is_closed(&self) -> bool;

    /// Underlying OHLCV data for this bar.
    fn ohlcv(&self) -> Ohlcv;

    /// Indicator output for `config` at this bar.
    ///
    /// For the forming bar, returns the current in-progress value.
    /// For a closed bar, returns the value as of when the bar closed.
    /// `None` until the indicator has converged.
    ///
    /// # Panics
    ///
    /// Panics if `config` was not subscribed on the owning snapshot.
    fn value<C: IndicatorConfig>(&self, config: &C) -> Option<C::Output>;

    /// Open timestamp of this bar, from the underlying [`Ohlcv`].
    fn open_time(&self) -> Timestamp {
        self.ohlcv().open_time
    }
}

/// A rolling view of bars at one [`Timeframe`].
///
/// The forming bar is always present; closed bars are retained up to
/// [`max_bars`](Self::max_bars). This view is immutable — subsequent
/// ticks surface as new snapshots.
///
/// # Indexing
///
/// [`at`](Self::at) and [`bars`](Self::bars) treat index `0` as the
/// forming bar, with closed bars at `1..` from most recent to oldest.
/// [`closed`](Self::closed) skips the forming bar: `0` is the most
/// recent closed bar. So `at(0)` and `closed(0)` differ by one bar,
/// and `bars(0..3)` includes the forming bar while `bars(1..3)` is
/// closed-only.
pub trait TimeframeSnapshot: Send + Sync + Display + Debug {
    /// Maximum number of bars retained, including the forming bar.
    fn max_bars(&self) -> usize;

    /// Number of closed bars currently retained.
    fn closed_count(&self) -> usize;

    /// The timeframe this snapshot tracks.
    fn timeframe(&self) -> Timeframe;

    /// Timestamp of the latest event observed.
    ///
    /// Advances with each intra-bar update on the forming bar; at the
    /// moment a bar closes, equals that bar's close timestamp.
    fn tick_time(&self) -> Timestamp;

    /// Bar at `idx`, where `0` is the forming bar and `1..` are closed
    /// bars from most recent to oldest.
    ///
    /// `None` when `idx` exceeds what is currently retained.
    fn at(&self, idx: usize) -> Option<&impl Bar>;

    /// Iterator over bars in `range`, using the same indexing as
    /// [`at`](Self::at). `0..3` yields the forming bar and the two
    /// most recent closed bars; `1..3` yields the two most recent
    /// closed bars.
    fn bars(&self, range: Range<usize>) -> impl Iterator<Item = &impl Bar>;

    /// The currently-forming bar. Always present.
    fn forming(&self) -> &impl Bar;

    /// Closed bar at `idx`, where `0` is the most recent closed bar.
    /// Skips the forming bar.
    ///
    /// `None` when `idx >= closed_count()`.
    fn closed(&self, idx: usize) -> Option<&impl Bar>;
}

/// A snapshot of one [`Instrument`] across its subscribed timeframes.
///
/// Strategies query [`for_timeframe`](Self::for_timeframe) for any
/// [`Timeframe`] they subscribed to; querying an unsubscribed
/// timeframe panics.
pub trait MarketSnapshot: Send + Sync + Display + Debug {
    /// The instrument this snapshot describes.
    fn instrument(&self) -> Instrument;

    /// Timestamp of the latest event observed across all subscribed
    /// timeframes.
    fn tick_time(&self) -> Timestamp;

    /// [`TimeframeSnapshot`] for `timeframe`.
    ///
    /// # Panics
    ///
    /// Panics if `timeframe` was not subscribed for this market.
    fn for_timeframe(&self, timeframe: Timeframe) -> &impl TimeframeSnapshot;
}
