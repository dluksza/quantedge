use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, PriceSource, internals::PriceWindow,
};

/// Configuration for the Commodity Channel Index ([`Cci`]) indicator.
///
/// CCI measures the deviation of a price from its statistical mean.
/// Uses the typical price (HLC3) by default. Output begins after
/// `length` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::CciConfig;
/// use std::num::NonZero;
///
/// let config = CciConfig::close(NonZero::new(20).unwrap());
/// assert_eq!(config.length(), 20);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct CciConfig {
    length: usize,
    source: PriceSource,
}

impl CciConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// CCI on closing price.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder()
            .length(length)
            .source(PriceSource::Close)
            .build()
    }

    /// CCI on typical price: `(high + low + close) / 3`.
    #[must_use]
    pub fn hlc3(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }
}

impl IndicatorConfig for CciConfig {
    type Builder = CciConfigBuilder;
    type Indicator = Cci;
    type Output = f64;

    fn builder() -> Self::Builder {
        CciConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        CciConfigBuilder {
            length: Some(self.length),
            source: self.source,
        }
    }
}

impl Default for CciConfig {
    /// Default: length=20, source=HLC3 (Lambert's original Typical Price, `TradingView` default).
    fn default() -> Self {
        Self {
            length: 20,
            source: PriceSource::HLC3,
        }
    }
}

impl Display for CciConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CciConfig({}, {})", self.length, self.source)
    }
}

/// Builder for [`CciConfig`].
///
/// Defaults: source = [`PriceSource::HLC3`].
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct CciConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
}

impl CciConfigBuilder {
    fn new() -> Self {
        CciConfigBuilder {
            length: None,
            source: PriceSource::HLC3,
        }
    }

    /// Sets the indicator window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }
}

impl IndicatorConfigBuilder<CciConfig> for CciConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> CciConfig {
        CciConfig {
            length: self.length.expect("length is required"),
            source: self.source,
        }
    }
}

/// Commodity Channel Index (CCI).
///
/// Measures the deviation of the current price from its simple
/// moving average, scaled by the mean absolute deviation. Positive
/// values indicate the price is above its average; negative values
/// indicate it is below.
///
/// ```text
/// CCI = (price − SMA) / (0.015 × mean_deviation)
/// ```
///
/// The constant 0.015 scales CCI so that roughly 70–80% of values
/// fall between −100 and +100. Values outside that range suggest
/// strong trend or overbought/oversold conditions.
///
/// When the mean deviation is near zero (flat market), returns 0.0.
///
/// Returns `None` until the window is full (after `length` bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Cci, CciConfig, Ohlcv};
/// use std::num::NonZero;
///
/// fn bar(close: f64, time: u64) -> Ohlcv {
///     Ohlcv { open: close, high: close, low: close, close, volume: 0.0, open_time: time }
/// }
///
/// let mut cci = Cci::new(CciConfig::close(NonZero::new(3).unwrap()));
///
/// assert_eq!(cci.compute(&bar(10.0, 1)), None);
/// assert_eq!(cci.compute(&bar(20.0, 2)), None);
///
/// // SMA = 20, mean_dev = 20/3, CCI = (30 − 20) / (0.015 × 20/3) = 100
/// let value = cci.compute(&bar(30.0, 3)).unwrap();
/// assert!((value - 100.0).abs() < 1e-6);
/// ```
#[derive(Clone, Debug)]
pub struct Cci {
    config: CciConfig,
    window: PriceWindow,
    current: Option<f64>,
    length_reciprocal: f64,
}

impl Indicator for Cci {
    type Config = CciConfig;
    type Output = f64;

    fn new(config: Self::Config) -> Self {
        Cci {
            config,
            window: PriceWindow::new(config.length, config.source),
            current: None,
            #[allow(clippy::cast_precision_loss)]
            length_reciprocal: 1.0 / config.length as f64,
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        let price = self.window.add(ohlcv);

        self.current = self.window.sum().map(|sum| {
            let sma = sum * self.length_reciprocal;
            let mean_dev = self
                .window
                .fold(0.0, |acc, price| acc + (price - sma).abs())
                * self.length_reciprocal;

            if mean_dev < f64::EPSILON {
                0.0
            } else {
                (price - sma) / (0.015 * mean_dev)
            }
        });

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Cci {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CCI({}, {})", self.config.length, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{Bar, assert_approx, bar, nz};

    fn make_cci(length: usize) -> Cci {
        Cci::new(CciConfig::close(nz(length)))
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            let mut cci = make_cci(3);
            assert_eq!(cci.compute(&bar(10.0, 1)), None);
            assert_eq!(cci.compute(&bar(20.0, 2)), None);
        }

        #[test]
        fn returns_value_when_full() {
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            assert!(cci.compute(&bar(30.0, 3)).is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn basic_cci_value() {
            // prices: 10, 20, 30 → SMA = 20
            // deviations: |10-20|=10, |20-20|=0, |30-20|=10
            // mean_dev = 20/3
            // CCI = (30 - 20) / (0.015 * 20/3) = 10 / 0.1 = 100
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            let value = cci.compute(&bar(30.0, 3)).unwrap();
            assert_approx!(value, 100.0);
        }

        #[test]
        fn negative_cci() {
            // prices: 30, 20, 10 → SMA = 20
            // deviations: |30-20|=10, |20-20|=0, |10-20|=10
            // mean_dev = 20/3
            // CCI = (10 - 20) / (0.015 * 20/3) = -10 / 0.1 = -100
            let mut cci = make_cci(3);
            cci.compute(&bar(30.0, 1));
            cci.compute(&bar(20.0, 2));
            let value = cci.compute(&bar(10.0, 3)).unwrap();
            assert_approx!(value, -100.0);
        }

        #[test]
        fn price_at_mean_gives_zero() {
            // prices: 10, 30, 20 → SMA = 20
            // CCI = (20 - 20) / ... = 0
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(30.0, 2));
            assert_eq!(cci.compute(&bar(20.0, 3)), Some(0.0));
        }

        #[test]
        fn flat_market_gives_zero() {
            let mut cci = make_cci(3);
            for t in 1..=5 {
                cci.compute(&bar(10.0, t));
            }
            assert_eq!(cci.value(), Some(0.0));
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn drops_oldest_on_advance() {
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            cci.compute(&bar(15.0, 3)); // window [10,20,15], SMA=15
            let v1 = cci.value().unwrap();
            cci.compute(&bar(25.0, 4)); // window [20,15,25], SMA=20
            let v2 = cci.value().unwrap();
            // Different windows produce different CCI values
            assert!((v1 - v2).abs() > 1e-10);
        }

        #[test]
        fn slides_across_many_bars() {
            let mut cci = make_cci(2);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            cci.compute(&bar(30.0, 3));
            cci.compute(&bar(40.0, 4));
            // Window = [40, 50], SMA = 45
            // mean_dev = (|40-45| + |50-45|) / 2 = 5
            // CCI = (50 - 45) / (0.015 * 5) = 5 / 0.075 = 66.666...
            let value = cci.compute(&bar(50.0, 5)).unwrap();
            assert_approx!(value, 5.0 / 0.075);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            let v1 = cci.compute(&bar(25.0, 3)).unwrap();
            let v2 = cci.compute(&bar(30.0, 3)).unwrap();
            assert!(v2 > v1, "higher price should give higher CCI");
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut cci = make_cci(2);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            cci.compute(&bar(25.0, 2)); // repaint
            let final_val = cci.compute(&bar(30.0, 2)).unwrap();

            let mut clean = make_cci(2);
            clean.compute(&bar(10.0, 1));
            let expected = clean.compute(&bar(30.0, 2)).unwrap();

            assert!((final_val - expected).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling() {
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(15.0, 1)); // repaint
            assert_eq!(cci.compute(&bar(20.0, 2)), None); // still filling
            assert!(cci.compute(&bar(30.0, 3)).is_some());
        }

        #[test]
        fn advance_after_repaint() {
            let mut cci = make_cci(2);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            cci.compute(&bar(25.0, 2)); // repaint
            let after = cci.compute(&bar(30.0, 3)).unwrap();

            // Clean path with repainted bar
            let mut clean = make_cci(2);
            clean.compute(&bar(25.0, 1));
            let expected = clean.compute(&bar(30.0, 2)).unwrap();

            assert!((after - expected).abs() < 1e-10);
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut cci = make_cci(3);

            // Bar 1: open then close
            assert_eq!(cci.compute(&bar(5.0, 1)), None);
            assert_eq!(cci.compute(&bar(3.0, 1)), None); // repaint

            // Bar 2: open then close
            assert_eq!(cci.compute(&bar(6.0, 2)), None);
            assert_eq!(cci.compute(&bar(8.0, 2)), None); // repaint

            // Bar 3: open
            let v1 = cci.compute(&bar(4.0, 3)).unwrap();

            // Bar 3: close (repaint)
            let v2 = cci.compute(&bar(7.0, 3)).unwrap();
            assert!((v1 - v2).abs() > 1e-10, "repaint should change value");

            // Bar 4
            let v3 = cci.compute(&bar(9.0, 4)).unwrap();
            assert!(v3.is_finite());
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut cci = make_cci(3);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            cci.compute(&bar(30.0, 3)); // converged

            let mut cloned = cci.clone();

            // Advance original
            let orig_val = cci.compute(&bar(25.0, 4)).unwrap();

            // Clone still at previous value
            assert_eq!(
                cloned.value(),
                Some(orig_val).map(|_| cloned.value().unwrap())
            );

            // Clone advances independently with different price
            let clone_val = cloned.compute(&bar(50.0, 4)).unwrap();
            assert!(
                (orig_val - clone_val).abs() > 1e-10,
                "divergent inputs should give different CCI"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper_uses_close_source() {
            let config = CciConfig::close(nz(20));
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn hlc3_helper_uses_hlc3_source() {
            let config = CciConfig::hlc3(nz(20));
            assert_eq!(config.source(), PriceSource::HLC3);
        }

        #[test]
        fn length_accessor() {
            let config = CciConfig::close(nz(20));
            assert_eq!(config.length(), 20);
        }

        #[test]
        fn convergence_equals_length() {
            let config = CciConfig::close(nz(20));
            assert_eq!(config.convergence(), 20);

            let config = CciConfig::close(nz(200));
            assert_eq!(config.convergence(), 200);
        }

        #[test]
        fn default_source_is_hlc3() {
            let config = CciConfig::builder().length(nz(20)).build();
            assert_eq!(config.source(), PriceSource::HLC3);
        }

        #[test]
        fn custom_source() {
            let config = CciConfig::builder()
                .length(nz(20))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = CciConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = CciConfig::close(nz(20));
            let b = CciConfig::close(nz(20));
            let c = CciConfig::close(nz(10));

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = CciConfig::hlc3(nz(20));
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let cci = make_cci(20);
            assert_eq!(cci.to_string(), "CCI(20, Close)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = CciConfig::close(nz(20));
            assert_eq!(config.to_string(), "CciConfig(20, Close)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let cci = make_cci(3);
            assert_eq!(cci.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut cci = make_cci(2);
            cci.compute(&bar(10.0, 1));
            cci.compute(&bar(20.0, 2));
            assert!(cci.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut cci = make_cci(2);
            cci.compute(&bar(10.0, 1));
            let computed = cci.compute(&bar(20.0, 2));
            assert_eq!(cci.value(), computed);
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_configured_source() {
            // HL2 = (high + low) / 2
            let mut cci = Cci::new(
                CciConfig::builder()
                    .length(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            // HL2 bar 1: (20 + 10) / 2 = 15
            // HL2 bar 2: (30 + 20) / 2 = 25
            // SMA = 20, mean_dev = 5
            // CCI = (25 - 20) / (0.015 * 5)
            let b1 = Bar::new(0.0, 20.0, 10.0, 0.0).at(1);
            let b2 = Bar::new(0.0, 30.0, 20.0, 0.0).at(2);
            cci.compute(&b1);
            let value = cci.compute(&b2).unwrap();
            let expected = 5.0 / (0.015 * 5.0);
            assert_approx!(value, expected);
        }
    }
}
