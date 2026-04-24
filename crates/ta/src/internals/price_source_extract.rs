use quantedge_core::{Ohlcv, Price, PriceSource};

pub(crate) fn extract_price(
    price_source: PriceSource,
    ohlcv: &Ohlcv,
    prev_close: Option<Price>,
) -> Price {
    match price_source {
        PriceSource::Close => ohlcv.close,
        PriceSource::HL2 => f64::midpoint(ohlcv.high, ohlcv.low),
        PriceSource::HLC3 => (ohlcv.high + ohlcv.low + ohlcv.close) / 3.0,
        PriceSource::OHLC4 => (ohlcv.open + ohlcv.high + ohlcv.low + ohlcv.close) / 4.0,
        PriceSource::HLCC4 => (ohlcv.high + ohlcv.low + ohlcv.close + ohlcv.close) / 4.0,
        PriceSource::TrueRange => {
            let hl = ohlcv.high - ohlcv.low;

            match prev_close {
                Some(prev_close) => {
                    let hc = (ohlcv.high - prev_close).abs();
                    let lc = (ohlcv.low - prev_close).abs();
                    hl.max(hc).max(lc)
                }
                None => hl,
            }
        }
        PriceSource::Open => ohlcv.open,
        PriceSource::High => ohlcv.high,
        PriceSource::Low => ohlcv.low,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{Bar, assert_approx};

    fn bar() -> Bar {
        Bar::new(10.0, 30.0, 5.0, 20.0)
    }

    #[test]
    fn extract_open() {
        assert_eq!(extract_price(PriceSource::Open, &bar(), None), 10.0);
    }

    #[test]
    fn extract_high() {
        assert_eq!(extract_price(PriceSource::High, &bar(), None), 30.0);
    }

    #[test]
    fn extract_low() {
        assert_eq!(extract_price(PriceSource::Low, &bar(), None), 5.0);
    }

    #[test]
    fn extract_close() {
        assert_eq!(extract_price(PriceSource::Close, &bar(), None), 20.0);
    }

    #[test]
    fn extract_hl2() {
        // (30 + 5) / 2 = 17.5
        assert_eq!(extract_price(PriceSource::HL2, &bar(), None), 17.5);
    }

    #[test]
    fn extract_hlc3() {
        // (30 + 5 + 20) / 3 = 18.333...
        let result = extract_price(PriceSource::HLC3, &bar(), None);
        assert_approx!(result, 55.0 / 3.0);
    }

    #[test]
    fn extract_ohlc4() {
        // (10 + 30 + 5 + 20) / 4 = 16.25
        assert_eq!(extract_price(PriceSource::OHLC4, &bar(), None), 16.25);
    }

    #[test]
    fn extract_hlcc4() {
        // (30 + 5 + 20 + 20) / 4 = 18.75
        assert_eq!(extract_price(PriceSource::HLCC4, &bar(), None), 18.75);
    }

    // TrueRange: max(high - low, |high - prev_close|, |low - prev_close|)

    #[test]
    fn true_range_without_prev_close_falls_back_to_hl() {
        // No previous bar, returns high - low = 25
        assert_eq!(extract_price(PriceSource::TrueRange, &bar(), None), 25.0);
    }

    #[test]
    fn true_range_hl_wins() {
        // prev_close inside the bar range: hl dominates
        // hl = 25, |30 - 15| = 15, |5 - 15| = 10
        let b = bar();
        assert_eq!(extract_price(PriceSource::TrueRange, &b, Some(15.0)), 25.0);
    }

    #[test]
    fn true_range_high_vs_prev_close_wins() {
        // Gap up: prev_close far below low
        // hl = 25, |30 - (-10)| = 40, |5 - (-10)| = 15
        let b = bar();
        assert_eq!(extract_price(PriceSource::TrueRange, &b, Some(-10.0)), 40.0);
    }

    #[test]
    fn true_range_low_vs_prev_close_wins() {
        // Gap down: prev_close far above high
        // hl = 25, |30 - 50| = 20, |5 - 50| = 45
        let b = bar();
        assert_eq!(extract_price(PriceSource::TrueRange, &b, Some(50.0)), 45.0);
    }
}
