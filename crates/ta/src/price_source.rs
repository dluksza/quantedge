use crate::{Ohlcv, Price};

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

impl PriceSource {
    pub(crate) fn extract(self, ohlcv: &impl Ohlcv, prev_close: Option<Price>) -> Price {
        match self {
            Self::Open => ohlcv.open(),
            Self::High => ohlcv.high(),
            Self::Close => ohlcv.close(),
            Self::Low => ohlcv.low(),
            Self::HL2 => f64::midpoint(ohlcv.high(), ohlcv.low()),
            Self::HLC3 => (ohlcv.high() + ohlcv.low() + ohlcv.close()) / 3.0,
            Self::OHLC4 => (ohlcv.open() + ohlcv.high() + ohlcv.low() + ohlcv.close()) / 4.0,
            Self::HLCC4 => (ohlcv.high() + ohlcv.low() + ohlcv.close() + ohlcv.close()) / 4.0,
            Self::TrueRange => {
                let hl = ohlcv.high() - ohlcv.low();

                match prev_close {
                    Some(prev_close) => {
                        let hc = (ohlcv.high() - prev_close).abs();
                        let lc = (ohlcv.low() - prev_close).abs();
                        hl.max(hc).max(lc)
                    }
                    None => hl,
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::test_util::{Bar, assert_approx};

    fn bar() -> Bar {
        Bar::new(10.0, 30.0, 5.0, 20.0)
    }

    #[test]
    fn extract_open() {
        assert_eq!(PriceSource::Open.extract(&bar(), None), 10.0);
    }

    #[test]
    fn extract_high() {
        assert_eq!(PriceSource::High.extract(&bar(), None), 30.0);
    }

    #[test]
    fn extract_low() {
        assert_eq!(PriceSource::Low.extract(&bar(), None), 5.0);
    }

    #[test]
    fn extract_close() {
        assert_eq!(PriceSource::Close.extract(&bar(), None), 20.0);
    }

    #[test]
    fn extract_hl2() {
        // (30 + 5) / 2 = 17.5
        assert_eq!(PriceSource::HL2.extract(&bar(), None), 17.5);
    }

    #[test]
    fn extract_hlc3() {
        // (30 + 5 + 20) / 3 = 18.333...
        let result = PriceSource::HLC3.extract(&bar(), None);
        assert_approx!(result, 55.0 / 3.0);
    }

    #[test]
    fn extract_ohlc4() {
        // (10 + 30 + 5 + 20) / 4 = 16.25
        assert_eq!(PriceSource::OHLC4.extract(&bar(), None), 16.25);
    }

    #[test]
    fn extract_hlcc4() {
        // (30 + 5 + 20 + 20) / 4 = 18.75
        assert_eq!(PriceSource::HLCC4.extract(&bar(), None), 18.75);
    }

    // TrueRange: max(high - low, |high - prev_close|, |low - prev_close|)

    #[test]
    fn true_range_without_prev_close_falls_back_to_hl() {
        // No previous bar, returns high - low = 25
        assert_eq!(PriceSource::TrueRange.extract(&bar(), None), 25.0);
    }

    #[test]
    fn true_range_hl_wins() {
        // prev_close inside the bar range: hl dominates
        // hl = 25, |30 - 15| = 15, |5 - 15| = 10
        let b = bar();
        assert_eq!(PriceSource::TrueRange.extract(&b, Some(15.0)), 25.0);
    }

    #[test]
    fn true_range_high_vs_prev_close_wins() {
        // Gap up: prev_close far below low
        // hl = 25, |30 - (-10)| = 40, |5 - (-10)| = 15
        let b = bar();
        assert_eq!(PriceSource::TrueRange.extract(&b, Some(-10.0)), 40.0);
    }

    #[test]
    fn true_range_low_vs_prev_close_wins() {
        // Gap down: prev_close far above high
        // hl = 25, |30 - 50| = 20, |5 - 50| = 45
        let b = bar();
        assert_eq!(PriceSource::TrueRange.extract(&b, Some(50.0)), 45.0);
    }
}
