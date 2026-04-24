use std::fmt::{Debug, Display};

/// Price source extracted from an [`Ohlcv`] bar before feeding into an
/// indicator.
///
/// Each indicator is configured with a `PriceSource` that determines which
/// value (or derived value) to compute on.
#[derive(PartialEq, Eq, Hash, Clone, Copy, Default, Debug)]
pub enum PriceSource {
    /// Opening price.
    Open,
    /// Highest price.
    High,
    /// Closing price.
    #[default]
    Close,
    /// Lowest price.
    Low,
    /// Median price: `(high + low) / 2`.
    HL2,
    /// Typical price: `(high + low + close) / 3`.
    HLC3,
    /// Average price: `(open + high + low + close) / 4`.
    OHLC4,
    /// Weighted close: `(high + low + close + close) / 4`.
    HLCC4,
    /// True range: `max(high - low, |high - prev_close|, |low - prev_close|)`.
    ///
    /// On the first bar (no previous close), falls back to `high - low`.
    TrueRange,
}

impl Display for PriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
