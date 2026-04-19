/// A price value.
///
/// Semantic alias for [`f64`]. Documents intent in function signatures
/// without introducing newtype construction overhead.
pub type Price = f64;

/// Bar open timestamp or sequence number.
///
/// Used for bar boundary detection. Must be non-decreasing
/// between consecutive calls to [`Indicator::compute`].
///
/// Recommended: microseconds since Unix epoch, monotonically increasing.
/// This is **required** for the VWAP indicator, which uses timestamps
/// to detect session boundaries.
pub type Timestamp = u64;

/// OHLCV bar data used as input to all indicators.
///
/// Implement this on your own kline/candle type to avoid per-tick
/// conversion. Indicators accept `&impl Ohlcv` and extract the
/// configured [`PriceSource`] internally.
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
/// use quantedge_ta::{Ohlcv, Price, Timestamp};
///
/// struct MyKline {
///     o: f64, h: f64, l: f64, c: f64,
///     ts: u64,
/// }
///
/// impl Ohlcv for MyKline {
///     fn open(&self) -> Price { self.o }
///     fn high(&self) -> Price { self.h }
///     fn low(&self) -> Price { self.l }
///     fn close(&self) -> Price { self.c }
///     fn open_time(&self) -> Timestamp { self.ts }
/// }
/// ```
pub trait Ohlcv {
    /// Opening price of the bar.
    fn open(&self) -> Price;

    /// Highest price during the bar.
    fn high(&self) -> Price;

    /// Lowest price during the bar.
    fn low(&self) -> Price;

    /// Closing (or latest) price of the bar.
    fn close(&self) -> Price;

    /// Bar open timestamp or sequence number.
    ///
    /// Used for bar boundary detection: consecutive calls with the same value
    /// repaint the current bar; a new value advances the indicator window.
    ///
    /// Values must be non-decreasing between calls. Behaviour is undefined if
    /// `open_time` decreases.
    fn open_time(&self) -> Timestamp;

    /// Trade volume during the bar. Defaults to `0.0`.
    ///
    /// **Required** for volume-dependent indicators (OBV, VWAP).
    /// You must override this method when using these indicators;
    /// the default `0.0` will produce meaningless results.
    /// Indicators that don't use volume ignore this value.
    fn volume(&self) -> f64 {
        0.0
    }
}
