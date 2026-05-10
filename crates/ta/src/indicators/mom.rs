use std::{
    fmt::{self, Display},
    num::NonZero,
};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price, PriceSource,
    internals::{BarAction, BarState, RingBuffer},
};

/// Configuration for the Momentum ([`Mom`]) indicator.
///
/// # Example
///
/// ```
/// use quantedge_ta::MomConfig;
/// use std::num::NonZero;
///
/// let config = MomConfig::close(NonZero::new(10).unwrap());
/// assert_eq!(config.period(), 10);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MomConfig {
    period: usize,
    source: PriceSource,
}

impl IndicatorConfig for MomConfig {
    type Builder = MomConfigBuilder;
    type Indicator = Mom;
    type Output = Price;

    fn builder() -> Self::Builder {
        MomConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.period + 1
    }

    fn to_builder(&self) -> Self::Builder {
        MomConfigBuilder {
            period: Some(self.period),
            source: self.source,
        }
    }
}

impl MomConfig {
    /// Lookback period for the momentum calculation.
    #[must_use]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Momentum on closing price.
    #[must_use]
    pub fn close(period: NonZero<usize>) -> Self {
        Self::builder().period(period).build()
    }
}

impl Default for MomConfig {
    /// Default: period=10, source=Close (common period, `TradingView` default).
    fn default() -> Self {
        Self {
            period: 10,
            source: PriceSource::Close,
        }
    }
}

impl Display for MomConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MomConfig({}, {})", self.period, self.source)
    }
}

/// Builder for [`MomConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// Period must be set before calling [`build`](IndicatorConfigBuilder::build).
pub struct MomConfigBuilder {
    period: Option<usize>,
    source: PriceSource,
}

impl MomConfigBuilder {
    fn new() -> Self {
        Self {
            period: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the lookback period.
    #[must_use]
    pub fn period(mut self, period: NonZero<usize>) -> Self {
        self.period.replace(period.get());
        self
    }
}

impl IndicatorConfigBuilder<MomConfig> for MomConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> MomConfig {
        MomConfig {
            period: self.period.expect("period is required"),
            source: self.source,
        }
    }
}

/// Momentum (MOM) indicator.
///
/// Measures the rate of change in price by comparing the current price to
/// the price `period` bars ago. A positive value indicates upward momentum;
/// negative, downward momentum.
///
/// ```text
/// MOM = price - price[period_ago]
/// ```
///
/// Uses a ring buffer of size `period` for O(1) updates per bar.
/// Returns `None` until `period` bars have been processed.
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes the momentum without advancing the window.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ohlcv, Mom, MomConfig};
/// use std::num::NonZero;
///
/// fn bar(close: f64, time: u64) -> Ohlcv {
///     Ohlcv { open: close, high: close, low: close, close, volume: 0.0, open_time: time }
/// }
///
/// let mut mom = Mom::new(MomConfig::close(NonZero::new(3).unwrap()));
///
/// assert_eq!(mom.compute(&bar(10.0, 1)), None);
/// assert_eq!(mom.compute(&bar(20.0, 2)), None);
/// assert_eq!(mom.compute(&bar(30.0, 3)), None);
/// // Bar 4: price(4) - price(1) → 40 - 10 = 30
/// assert_eq!(mom.compute(&bar(40.0, 4)), Some(30.0));
/// ```
#[derive(Clone, Debug)]
pub struct Mom {
    config: MomConfig,
    bar_state: BarState,
    buffer: RingBuffer,
    current: Option<Price>,
    /// Cached oldest price in the ring buffer (valid when `current.is_some()`).
    /// Used for repaints so we don't need to query the ring buffer.
    oldest_price: Price,
}

impl Indicator for Mom {
    type Config = MomConfig;
    type Output = Price;

    fn new(config: Self::Config) -> Self {
        Mom {
            config,
            bar_state: BarState::new(config.source),
            buffer: RingBuffer::new(config.period),
            current: None,
            oldest_price: 0.0,
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                if let Some(price_period_ago) = self.buffer.push(price) {
                    self.current = Some(price - price_period_ago);
                    self.oldest_price = price_period_ago;
                }
                self.current
            }
            BarAction::Repaint(price) => {
                self.buffer.replace(price);
                if self.current.is_some() {
                    self.current = Some(price - self.oldest_price);
                }
                self.current
            }
        }
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Mom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MOM({}, {})", self.config.period, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{bar, nz};

    fn mom(period: usize) -> Mom {
        Mom::new(MomConfig::close(nz(period)))
    }

    // ---------------------------------------------------------------------------
    // Reference: momentum(prices, period) returns prices[i] - prices[i-period]
    // for i >= period.
    // ---------------------------------------------------------------------------

    mod reference {
        use super::*;

        /// Reference data:
        /// prices = [44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89]
        /// period = 10
        /// mom[10] = prices[10] - prices[0] = 45.89 - 44.34 = 1.55
        #[test]
        fn matches_reference_basic() {
            let mut mom = mom(10);
            let prices = [
                44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89,
            ];

            for (i, &price) in prices.iter().enumerate() {
                let result = mom.compute(&bar(price, i as u64));
                if i < 10 {
                    assert_eq!(result, None, "bar {i} should be None");
                }
            }

            let val = mom.value().unwrap();
            assert!((val - 1.55).abs() < 1e-10, "MOM should be 1.55, got {val}");
        }

        #[test]
        fn short_input_returns_none() {
            // Reference: prices.len() <= period → all None
            let mut mom = mom(10);
            let prices = [1.0, 2.0, 3.0];

            for (i, &price) in prices.iter().enumerate() {
                assert_eq!(
                    mom.compute(&bar(price, i as u64)),
                    None,
                    "should be None for short input"
                );
            }
            assert_eq!(mom.value(), None);
        }

        #[test]
        fn output_count_matches_reference() {
            // Reference: prices.len() = 11, period = 10 → 1 output (index 10)
            // Our streaming: 11 bars, period 10 → output starts at bar 11 (10 pushes fill, 11th evicts)
            let mut mom = mom(10);
            let prices = [
                44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.42, 45.84, 46.08, 45.89,
            ];

            let mut output_count = 0;
            for (i, &price) in prices.iter().enumerate() {
                if mom.compute(&bar(price, i as u64)).is_some() {
                    output_count += 1;
                }
            }
            assert_eq!(
                output_count, 1,
                "expected 1 output from 11 bars with period=10"
            );
        }
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_period_bars() {
            let mut mom = mom(3);
            assert_eq!(mom.compute(&bar(10.0, 1)), None);
            assert_eq!(mom.compute(&bar(20.0, 2)), None);
        }

        #[test]
        fn first_output_at_period() {
            let mut mom = mom(3);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            mom.compute(&bar(30.0, 3));
            assert!(
                mom.compute(&bar(40.0, 4)).is_some(),
                "expected Some at bar 4 (period=3, needs 4 bars for first output)"
            );
        }

        #[test]
        fn period_one() {
            let mut mom = mom(1);
            let v1 = mom.compute(&bar(10.0, 1));
            // Buffer(1): push(10) fills it. No eviction → None
            assert_eq!(v1, None);

            // push(20) evicts 10 → 20 - 10 = 10
            assert_eq!(mom.compute(&bar(20.0, 2)), Some(10.0));
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn basic_calculation() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            // Buffer(2) full, push(30) evicts 10 → 30-10 = 20
            assert_eq!(mom.compute(&bar(30.0, 3)), Some(20.0));
            // push(40) evicts 20 → 40-20 = 20
            assert_eq!(mom.compute(&bar(40.0, 4)), Some(20.0));
        }

        #[test]
        fn negative_momentum() {
            let mut mom = mom(2);
            mom.compute(&bar(40.0, 1));
            mom.compute(&bar(50.0, 2));
            // push(30) evicts 40 → 30 - 40 = -10
            assert_eq!(mom.compute(&bar(30.0, 3)), Some(-10.0));
        }

        #[test]
        fn zero_momentum() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(10.0, 2));
            // push(10) evicts 10 → 10 - 10 = 0
            assert_eq!(mom.compute(&bar(10.0, 3)), Some(0.0));
        }

        #[test]
        fn multi_step() {
            let mut mom = mom(3);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            mom.compute(&bar(30.0, 3));
            // 40 - 10 = 30 (evicts bar 1)
            assert_eq!(mom.compute(&bar(40.0, 4)), Some(30.0));
            // 35 - 20 = 15 (evicts bar 2)
            assert_eq!(mom.compute(&bar(35.0, 5)), Some(15.0));
            // 50 - 30 = 20 (evicts bar 3)
            assert_eq!(mom.compute(&bar(50.0, 6)), Some(20.0));
        }

        #[test]
        fn ascending_prices_increasing_momentum() {
            // Prices: 10, 20, 30, 40, 50 with period=2
            // MOM: 20, 20, 20 (price[i] - price[i-2] for ascending with step 10)
            let mut mom = mom(2);
            assert_eq!(mom.compute(&bar(10.0, 1)), None);
            assert_eq!(mom.compute(&bar(20.0, 2)), None);
            assert_eq!(mom.compute(&bar(30.0, 3)), Some(20.0));
            assert_eq!(mom.compute(&bar(40.0, 4)), Some(20.0));
            assert_eq!(mom.compute(&bar(50.0, 5)), Some(20.0));
        }

        #[test]
        fn varying_prices() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(15.0, 2));
            // 25 - 10 = 15
            assert_eq!(mom.compute(&bar(25.0, 3)), Some(15.0));
            // 20 - 15 = 5
            assert_eq!(mom.compute(&bar(20.0, 4)), Some(5.0));
            // 30 - 25 = 5
            assert_eq!(mom.compute(&bar(30.0, 5)), Some(5.0));
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn old_price_expires() {
            let mut mom = mom(2);
            // Bar 1: extreme price
            mom.compute(&bar(100.0, 1));
            mom.compute(&bar(10.0, 2));
            // push(20) evicts 100 → 20 - 100 = -80
            let v1 = mom.compute(&bar(20.0, 3)).unwrap();
            assert!((v1 + 80.0).abs() < 1e-10);

            // Advance: bar 2 (price=10) expires from buffer
            // push(30) evicts 10 → 30 - 10 = 20
            let v2 = mom.compute(&bar(30.0, 4)).unwrap();
            assert!((v2 - 20.0).abs() < 1e-10);
        }

        #[test]
        fn slides_across_many_bars() {
            let mut mom = mom(1);
            // period=1: each bar's mom = price[i] - price[i-1]
            mom.compute(&bar(10.0, 1));
            assert_eq!(mom.compute(&bar(12.0, 2)), Some(2.0));
            assert_eq!(mom.compute(&bar(11.0, 3)), Some(-1.0));
            assert_eq!(mom.compute(&bar(15.0, 4)), Some(4.0));
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            // Bar 3: first output, 30 - 10 = 20
            let v1 = mom.compute(&bar(30.0, 3)).unwrap();
            // Repaint bar 3 to 40: 40 - 10 = 30
            let v2 = mom.compute(&bar(40.0, 3)).unwrap();
            assert!((v1 - 20.0).abs() < 1e-10);
            assert!(
                (v2 - 30.0).abs() < 1e-10,
                "repaint with higher price should increase momentum"
            );
        }

        #[test]
        fn repaint_after_convergence() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            mom.compute(&bar(30.0, 3));
            // Bar 4: 40 - 10 = 30
            let v1 = mom.compute(&bar(40.0, 4)).unwrap();
            // Repaint bar 4 to 50: 50 - 10 = 40
            let v2 = mom.compute(&bar(50.0, 4)).unwrap();
            assert!(
                v2 > v1,
                "repaint with higher price should increase momentum: v1={v1} v2={v2}"
            );
        }

        #[test]
        fn repaint_during_filling() {
            let mut mom = mom(3);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 1)); // repaint bar 1: price becomes 20
            mom.compute(&bar(30.0, 2)); // None
            mom.compute(&bar(40.0, 3)); // None (fills buffer)
            // Bar 4: evicts bar 1 (price=20 after repaint) → 50 - 20 = 30
            let v = mom.compute(&bar(50.0, 4)).unwrap();
            assert!(
                (v - 30.0).abs() < 1e-10,
                "repaint during filling should affect future outputs"
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut ind = mom(2);
            ind.compute(&bar(10.0, 1));
            ind.compute(&bar(20.0, 2));
            // Multiple repaints of bar 3
            ind.compute(&bar(30.0, 3));
            ind.compute(&bar(35.0, 3));
            ind.compute(&bar(40.0, 3));
            let v1 = ind.value().unwrap();

            // Clean: bar 3 directly at 40
            let mut clean = mom(2);
            clean.compute(&bar(10.0, 1));
            clean.compute(&bar(20.0, 2));
            clean.compute(&bar(40.0, 3));
            let v2 = clean.value().unwrap();

            assert!(
                (v1 - v2).abs() < 1e-14,
                "repaints should match clean: v1={v1} v2={v2}"
            );
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));

            let mut cloned = mom.clone();

            let orig = mom.compute(&bar(40.0, 3)).unwrap();
            let clone_val = cloned.compute(&bar(5.0, 3)).unwrap();

            assert!(
                (orig - clone_val).abs() > 1e-10,
                "divergent inputs should give different momentum"
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_indicator() {
            let m = mom(10);
            assert_eq!(m.to_string(), "MOM(10, Close)");
        }

        #[test]
        fn formats_config() {
            let config = MomConfig::close(nz(10));
            assert_eq!(config.to_string(), "MomConfig(10, Close)");
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper_uses_close_source() {
            let config = MomConfig::close(nz(10));
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn period_accessor() {
            let config = MomConfig::close(nz(10));
            assert_eq!(config.period(), 10);
        }

        #[test]
        fn convergence_equals_period_plus_one() {
            let config = MomConfig::close(nz(10));
            assert_eq!(config.convergence(), 11);
        }

        #[test]
        fn default_source_is_close() {
            let config = MomConfig::builder().period(nz(10)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn custom_source() {
            let config = MomConfig::builder()
                .period(nz(10))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        #[should_panic(expected = "period is required")]
        fn panics_without_period() {
            let _ = MomConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = MomConfig::close(nz(10));
            let b = MomConfig::close(nz(10));
            let c = MomConfig::close(nz(5));

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = MomConfig::close(nz(10));
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_hl2_source() {
            let mut mom = Mom::new(
                MomConfig::builder()
                    .period(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            // Bar 1: HL2 = (15+15)/2 = 15 (all fields same in bar())
            mom.compute(&bar(15.0, 1));
            mom.compute(&bar(20.0, 2));
            // Bar 3: HL2 = 25 → 25 - 15 = 10
            let val = mom.compute(&bar(25.0, 3));
            assert_eq!(val, Some(10.0));
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            assert_eq!(mom(3).value(), None);
        }

        #[test]
        fn matches_last_compute() {
            let mut mom = mom(2);
            mom.compute(&bar(10.0, 1));
            mom.compute(&bar(20.0, 2));
            let computed = mom.compute(&bar(30.0, 3));
            assert_eq!(mom.value(), computed);
        }
    }
}
