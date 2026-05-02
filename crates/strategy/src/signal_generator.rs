//! Stateless signal generators that emit [`MarketSignal`]s from
//! [`MarketSnapshot`]s.
//!
//! A [`SignalGenerator`] declares its data dependencies through a
//! [`MarketSignalConfig`] and is then asked to produce an
//! `Option<MarketSignal>` from a snapshot. Signals carry an optional
//! [`MarketSide`], a set of supporting [`SignalReason`]s, and the bar
//! that triggered them.
//!
//! [`SignalEvent`] pairs an emitted signal with its wall-clock
//! emission time for downstream age and latency tracking.

use std::fmt::Debug;

use quantedge_core::{IndicatorConfig, MarketSnapshot, Timeframe, Timestamp};

use crate::MarketSignal;

/// A [`MarketSignal`] paired with the wall-clock time it was emitted.
///
/// Use [`timestamp`](Self::timestamp) to compute signal age. To compare
/// signal state, compare the inner [`MarketSignal`] directly.
#[derive(Debug, Clone)]
pub struct SignalEvent {
    /// The signal payload.
    pub signal: MarketSignal,

    /// Wall-clock time of emission, in microseconds since the Unix epoch.
    ///
    /// Distinct from the bar's `open_time` and from any exchange
    /// transaction time.
    pub timestamp: Timestamp,
}

/// Declares a [`SignalGenerator`]'s data dependencies.
///
/// A generator only observes data it has declared here.
pub trait MarketSignalConfig: Sync + Send {
    /// Declares timeframes the generator needs bar history at.
    ///
    /// Additive across calls.
    #[must_use]
    fn require_timeframes(self, timeframes: &[Timeframe]) -> Self;

    /// Declares the minimum number of closed bars to retain at each
    /// required timeframe.
    ///
    /// Repeated calls take the maximum.
    #[must_use]
    fn require_closed_bars(self, bars: usize) -> Self;

    /// Declares an indicator dependency.
    ///
    /// The indicator is made available at every required timeframe;
    /// query its value via [`Bar::value`] on the snapshot with the
    /// same `config`.
    ///
    /// [`Bar::value`]: quantedge_core::Bar::value
    #[must_use]
    fn register(self, config: &impl IndicatorConfig) -> Self;
}

/// A stateless detector of market conditions.
///
/// Repeated calls to [`evaluate`](Self::evaluate) with the same
/// [`MarketSnapshot`] produce the same result. Cross-bar state lives
/// in the snapshot's bar history; request retention via
/// [`MarketSignalConfig::require_closed_bars`].
///
/// [`configure`](Self::configure) is called once to declare
/// dependencies. [`evaluate`](Self::evaluate) is called on every tick
/// and on each required-timeframe close.
///
/// `evaluate` may emit the same logical signal across consecutive
/// ticks; the engine dedupes via [`MarketSignal`] equality.
pub trait SignalGenerator: Sync + Send + Default {
    /// Stable identifier. Used as [`MarketSignal::generator_id`].
    fn id(&self) -> &'static str;

    /// Human-readable name. Used as [`MarketSignal::generator_name`].
    fn name(&self) -> &'static str;

    /// Declares the generator's data dependencies on `config`.
    ///
    /// Called once at registration.
    fn configure<C: MarketSignalConfig>(&self, config: C) -> C;

    /// Returns a signal if the current snapshot matches the
    /// generator's condition, or `None` otherwise.
    ///
    /// Must be deterministic on `snapshot`. The snapshot exposes only
    /// the timeframes and indicators declared via
    /// [`configure`](Self::configure).
    fn evaluate(&self, snapshot: &impl MarketSnapshot) -> Option<MarketSignal>;
}
