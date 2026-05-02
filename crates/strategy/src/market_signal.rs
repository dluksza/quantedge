use std::{
    cmp::Ordering,
    collections::BTreeSet,
    hash::{Hash, Hasher},
};

use quantedge_core::{Bar, Instrument, MarketSnapshot, Timeframe, TimeframeSnapshot};
use quantedge_ta::Ohlcv;

use crate::SignalGenerator;

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

impl MarketSignal {
    /// Starts a [`MarketSignalBuilder`] anchored on the forming bar
    /// at `timeframe`.
    ///
    /// Captures the snapshot's [`instrument`](MarketSnapshot::instrument),
    /// the generator's [`id`](SignalGenerator::id) and
    /// [`name`](SignalGenerator::name), and the forming bar's
    /// [`Ohlcv`] up front. The forming bar is always present, so
    /// this constructor is infallible. Use it for signals that
    /// fire intra-bar.
    ///
    /// `key` discriminates among the signal types this generator
    /// emits; `(generator_id, key)` is globally unique.
    ///
    /// # Panics
    ///
    /// Panics if `timeframe` was not subscribed for this snapshot
    /// (see [`MarketSnapshot::for_timeframe`]).
    #[must_use]
    pub fn from_forming(
        signal_generator: &impl SignalGenerator,
        snapshot: &impl MarketSnapshot,
        timeframe: Timeframe,
        key: &'static str,
    ) -> MarketSignalBuilder {
        MarketSignalBuilder::from_forming(signal_generator, snapshot, timeframe, key)
    }

    /// Starts a [`MarketSignalBuilder`] anchored on the closed bar at
    /// `idx` of `timeframe`, where `0` is the most recent closed bar.
    ///
    /// Returns `None` when fewer than `idx + 1` closed bars are
    /// retained on the snapshot — propagate with `?` from
    /// [`SignalGenerator::evaluate`]. Use this constructor for
    /// signals that should only fire on bar close.
    ///
    /// Captures the same generator and instrument metadata as
    /// [`from_forming`](Self::from_forming); the bar at `idx`
    /// supplies the [`Ohlcv`].
    ///
    /// `key` discriminates among the signal types this generator
    /// emits; `(generator_id, key)` is globally unique.
    ///
    /// # Panics
    ///
    /// Panics if `timeframe` was not subscribed for this snapshot
    /// (see [`MarketSnapshot::for_timeframe`]).
    #[must_use]
    pub fn from_closed(
        signal_generator: &impl SignalGenerator,
        snapshot: &impl MarketSnapshot,
        timeframe: Timeframe,
        idx: usize,
        key: &'static str,
    ) -> Option<MarketSignalBuilder> {
        MarketSignalBuilder::from_closed(signal_generator, snapshot, timeframe, idx, key)
    }
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

/// Builder for [`MarketSignal`].
///
/// Obtained from [`MarketSignal::from_forming`] or
/// [`MarketSignal::from_closed`], which capture the triggering bar's
/// OHLCV, instrument, and generator metadata up front. Layer on
/// directional bias via [`with_side`](Self::with_side) and supporting
/// reasons via [`add_reason`](Self::add_reason), then finalize with
/// [`build`](Self::build).
///
/// Methods chain by value; the builder is single-use.
pub struct MarketSignalBuilder {
    key: &'static str,
    generator_id: &'static str,
    generator_name: &'static str,
    timeframe: Timeframe,
    instrument: Instrument,
    ohlcv: Ohlcv,
    side: Option<MarketSide>,
    reasons: Vec<SignalReason>,
}

impl MarketSignalBuilder {
    fn from_forming(
        generator: &impl SignalGenerator,
        snapshot: &impl MarketSnapshot,
        timeframe: Timeframe,
        key: &'static str,
    ) -> Self {
        let ohlcv = snapshot.for_timeframe(&timeframe).forming().ohlcv();

        Self {
            key,
            generator_id: generator.id(),
            generator_name: generator.name(),
            timeframe,
            instrument: snapshot.instrument(),
            ohlcv,
            side: None,
            reasons: vec![],
        }
    }

    fn from_closed(
        generator: &impl SignalGenerator,
        snapshot: &impl MarketSnapshot,
        timeframe: Timeframe,
        idx: usize,
        key: &'static str,
    ) -> Option<Self> {
        let ohlcv = snapshot.for_timeframe(&timeframe).closed(idx)?.ohlcv();

        Some(Self {
            key,
            generator_id: generator.id(),
            generator_name: generator.name(),
            timeframe,
            instrument: snapshot.instrument(),
            ohlcv,
            side: None,
            reasons: vec![],
        })
    }

    /// Sets the directional bias on
    /// [`MarketSignal::market_side`].
    ///
    /// Omit for non-directional signals — filters, regime detectors,
    /// volatility events — which leave `market_side` as `None`.
    #[must_use]
    pub fn with_side(mut self, side: MarketSide) -> Self {
        self.side = Some(side);
        self
    }

    /// Appends a supporting [`SignalReason`].
    ///
    /// Reasons form a set keyed by [`SignalReason::id`]; a later
    /// `add_reason` with the same `id` is a no-op.
    /// [`description`](SignalReason::description) is presentation
    /// only and not part of identity.
    #[must_use]
    pub fn add_reason(mut self, id: &'static str, description: &'static str) -> Self {
        self.reasons.push(SignalReason { id, description });
        self
    }

    /// Consumes the builder and returns the [`MarketSignal`].
    pub fn build(self) -> MarketSignal {
        MarketSignal {
            key: self.key,
            generator_id: self.generator_id,
            generator_name: self.generator_name,
            timeframe: self.timeframe,
            instrument: self.instrument,
            market_side: self.side,
            reasons: BTreeSet::from_iter(self.reasons),
            ohlcv: self.ohlcv,
        }
    }
}
