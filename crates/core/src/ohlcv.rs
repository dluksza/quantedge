/// A price value.
///
/// Semantic alias for [`f64`]. Documents intent in function signatures
/// without introducing newtype construction overhead.
pub type Price = f64;

/// Bar open timestamp or sequence number.
///
/// Used for bar boundary detection. Must be non-decreasing
/// across successive bars.
///
/// Recommended: microseconds since Unix epoch, monotonically increasing.
/// This is **required** for the VWAP indicator, which uses timestamps
/// to detect session boundaries.
pub type Timestamp = u64;

/// OHLCV bar data used as input to all indicators.
///
/// Construct one per bar and pass it by reference to each indicator's
/// `compute` call.
///
/// # Bar boundaries
///
/// Indicators detect new bars by comparing [`open_time`](Ohlcv::open_time)
/// values: same timestamp updates (repaints) the current bar, a new timestamp
/// advances the window.
///
/// # Example
///
/// ```
/// use quantedge_core::Ohlcv;
///
/// let bar = Ohlcv {
///     open: 10.0,
///     high: 12.0,
///     low: 9.0,
///     close: 11.0,
///     volume: 100.0,
///     open_time: 1,
/// };
/// assert_eq!(bar.close, 11.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ohlcv {
    /// Opening price of the bar.
    pub open: Price,

    /// Highest price during the bar.
    pub high: Price,

    /// Lowest price during the bar.
    pub low: Price,

    /// Closing (or latest) price of the bar.
    pub close: Price,

    /// Bar open timestamp or sequence number.
    ///
    /// Used for bar boundary detection: consecutive calls with the same value
    /// repaint the current bar; a new value advances the indicator window.
    ///
    /// Values must be non-decreasing between calls. Behaviour is undefined if
    /// `open_time` decreases.
    pub open_time: Timestamp,

    /// Trade volume during the bar.
    ///
    /// Required by volume-dependent indicators (OBV, VWAP). Set to `0.0`
    /// when feeding indicators that ignore volume.
    pub volume: f64,
}
