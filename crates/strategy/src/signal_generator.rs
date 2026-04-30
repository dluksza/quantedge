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

use std::{
    cmp::Ordering,
    collections::BTreeSet,
    fmt::Debug,
    hash::{Hash, Hasher},
};

use quantedge_core::{IndicatorConfig, Instrument, MarketSnapshot, Ohlcv, Timeframe, Timestamp};

/// Directional bias of a signal.
///
/// Carried as `Option<MarketSide>` on [`MarketSignal::market_side`]:
/// `Some` for directional signals (trend, momentum, reversal),
/// `None` for non-directional ones used as filters or context
/// (volatility regime, range break, liquidity event).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketSide {
    /// Bullish bias.
    Long,

    /// Bearish bias.
    Short,
}

/// A reason supporting a [`MarketSignal`].
///
/// Identity is determined by [`id`](Self::id) alone;
/// [`description`](Self::description) is presentation. Two reasons
/// with the same `id` are considered the same reason.
#[derive(Debug, Clone, Eq)]
pub struct SignalReason {
    /// Stable identifier. Defines reason identity.
    pub id: &'static str,

    /// Human-readable explanation. Not part of identity.
    pub description: &'static str,
}

impl PartialEq for SignalReason {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for SignalReason {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for SignalReason {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SignalReason {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(other.id)
    }
}

/// A market signal emitted by a [`SignalGenerator`].
///
/// Two signals are equal when their underlying state matches.
/// [`ohlcv`](Self::ohlcv) and [`generator_name`](Self::generator_name)
/// are metadata and do not affect equality.
#[derive(Debug, Clone)]
pub struct MarketSignal {
    /// Discriminator across the signal types this generator can emit
    /// (e.g. `"trend_cross"`, `"momentum_reversal"`).
    ///
    /// Unique within a generator; `(generator_id, key)` is globally
    /// unique.
    pub key: &'static str,

    /// Identifier of the emitting generator. Matches
    /// [`SignalGenerator::id`].
    pub generator_id: &'static str,

    /// Human-readable name of the emitting generator. Metadata.
    pub generator_name: &'static str,

    /// Primary timeframe the signal was triggered on.
    /// [`ohlcv`](Self::ohlcv) is taken from this timeframe.
    pub timeframe: Timeframe,

    /// Instrument the signal applies to.
    pub instrument: Instrument,

    /// Directional bias when the signal carries one.
    ///
    /// `None` for non-directional signals — filters, regime detectors,
    /// volatility events.
    pub market_side: Option<MarketSide>,

    /// Reasons supporting the signal. Duplicates by
    /// [`SignalReason::id`] collapse on insert.
    pub reasons: BTreeSet<SignalReason>,

    /// Triggering bar at [`timeframe`](Self::timeframe).
    ///
    /// May be the forming bar or the most recently closed bar
    /// depending on the generator's logic.
    pub ohlcv: Ohlcv,
}

impl PartialEq for MarketSignal {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.generator_id == other.generator_id
            && self.timeframe == other.timeframe
            && self.instrument == other.instrument
            && self.market_side == other.market_side
            && self.reasons == other.reasons
    }
}

impl Eq for MarketSignal {}

impl Hash for MarketSignal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.generator_id.hash(state);
        self.timeframe.hash(state);
        self.instrument.hash(state);
        self.market_side.hash(state);
        self.reasons.hash(state);
    }
}

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
    fn require_timeframes(self, timeframes: &[Timeframe]) -> Self;

    /// Declares the minimum number of closed bars to retain at each
    /// required timeframe.
    ///
    /// Repeated calls take the maximum.
    fn require_closed_bars(self, bars: usize) -> Self;

    /// Declares an indicator dependency.
    ///
    /// The indicator is made available at every required timeframe;
    /// query its value via [`Bar::value`] on the snapshot with the
    /// same `config`.
    ///
    /// [`Bar::value`]: quantedge_core::Bar::value
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
