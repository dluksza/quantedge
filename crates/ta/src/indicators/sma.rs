use std::{
    fmt::{Debug, Display},
    num::NonZero,
};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price, PriceSource,
    internals::PriceWindow,
};

/// Configuration for the Simple Moving Average ([`Sma`]) indicator.
///
/// # Example
///
/// ```rust
/// use quantedge_ta::SmaConfig;
/// use std::num::NonZero;
///
/// let config = SmaConfig::close(NonZero::new(20).unwrap());
/// assert_eq!(config.length(), 20);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct SmaConfig {
    length: usize,
    source: PriceSource,
}

impl IndicatorConfig for SmaConfig {
    type Builder = SmaConfigBuilder;

    fn builder() -> Self::Builder {
        SmaConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        SmaConfigBuilder {
            length: Some(self.length),
            source: self.source,
        }
    }
}

impl SmaConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// SMA on closing price.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }

    /// SMA on median price: `(high + low) / 2`.
    #[must_use]
    pub fn hl2(length: NonZero<usize>) -> Self {
        Self::builder()
            .length(length)
            .source(PriceSource::HL2)
            .build()
    }

    /// SMA on average price: `(open + high + low + close) / 4`.
    #[must_use]
    pub fn ohlc4(length: NonZero<usize>) -> Self {
        Self::builder()
            .length(length)
            .source(PriceSource::OHLC4)
            .build()
    }
}

impl Default for SmaConfig {
    /// Default: length=20, source=Close (common medium-term period, `TradingView` default).
    fn default() -> Self {
        Self {
            length: 20,
            source: PriceSource::Close,
        }
    }
}

impl Display for SmaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SmaConfig({}, {})", self.length, self.source)
    }
}

/// Builder for [`SmaConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// Length must be set before calling [`build`](IndicatorConfigBuilder::build).
pub struct SmaConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
}

impl SmaConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the indicator window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }
}

impl IndicatorConfigBuilder<SmaConfig> for SmaConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> SmaConfig {
        SmaConfig {
            length: self.length.expect("length is required"),
            source: self.source,
        }
    }
}

/// Simple Moving Average (SMA).
///
/// Computes the unweighted mean of the last *n* values, where *n* is the
/// configured window length. Returns `None` until the window is full.
///
/// Uses a running sum for O(1) updates per bar. Supports live repainting:
/// feeding a bar with the same `open_time` replaces the current value without
/// advancing the window.
///
/// # Example
///
/// ```rust
/// use quantedge_ta::{Sma, SmaConfig};
/// use std::num::NonZero;
/// # use quantedge_ta::{Ohlcv, Price, Timestamp};
/// #
/// # struct Bar(f64, u64);
/// # impl Ohlcv for Bar {
/// #     fn open(&self) -> Price { self.0 }
/// #     fn high(&self) -> Price { self.0 }
/// #     fn low(&self) -> Price { self.0 }
/// #     fn close(&self) -> Price { self.0 }
/// #     fn open_time(&self) -> Timestamp { self.1 }
/// # }
///
/// let mut sma = Sma::new(SmaConfig::close(NonZero::new(3).unwrap()));
///
/// assert_eq!(sma.compute(&Bar(10.0, 1)), None);
/// assert_eq!(sma.compute(&Bar(20.0, 2)), None);
/// assert_eq!(sma.compute(&Bar(30.0, 3)), Some(20.0));
/// ```
#[derive(Clone, Debug)]
pub struct Sma {
    config: SmaConfig,
    window: PriceWindow,
    length_reciprocal: f64,
    current: Option<Price>,
}

impl Indicator for Sma {
    type Config = SmaConfig;
    type Output = Price;

    fn new(config: Self::Config) -> Self {
        let window = PriceWindow::new(config.length, config.source);

        Self {
            config,
            window,
            #[allow(clippy::cast_precision_loss)]
            length_reciprocal: 1.0 / config.length as f64,
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl Ohlcv) -> Option<Self::Output> {
        self.window.add(ohlcv);

        self.current = self.window.sum().map(|sum| sum * self.length_reciprocal);

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Sma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SMA({}, {})", self.config.length, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{assert_approx, bar, nz};

    fn sma(length: usize) -> Sma {
        Sma::new(SmaConfig::close(nz(length)))
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            let mut sma = sma(3);
            assert_eq!(sma.compute(&bar(10.0, 1)), None);
            assert_eq!(sma.compute(&bar(20.0, 2)), None);
        }

        #[test]
        fn returns_average_when_full() {
            let mut sma = sma(3);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            assert_eq!(sma.compute(&bar(30.0, 3)), Some(20.0));
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn drops_oldest_on_advance() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            // (20 + 30) / 2 = 25
            assert_eq!(sma.compute(&bar(30.0, 3)), Some(25.0));
        }

        #[test]
        fn slides_across_many_bars() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            sma.compute(&bar(30.0, 3));
            sma.compute(&bar(40.0, 4));
            // (40 + 50) / 2 = 45
            assert_eq!(sma.compute(&bar(50.0, 5)), Some(45.0));
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            assert_eq!(sma.compute(&bar(30.0, 2)), Some(20.0));
            // (10 + 30) / 2 = 20
        }

        #[test]
        fn multiple_repaints() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            sma.compute(&bar(25.0, 2));
            sma.compute(&bar(30.0, 2));
            // (10 + 30) / 2 = 20
            assert_eq!(sma.compute(&bar(30.0, 2)), Some(20.0));
        }

        #[test]
        fn repaint_during_filling() {
            let mut sma = sma(3);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(15.0, 1)); // repaint
            assert_eq!(sma.compute(&bar(20.0, 2)), None); // still filling
            // (15 + 20 + 30) / 3 = 21.666...
            let result = sma.compute(&bar(30.0, 3));
            assert_approx!(result.unwrap(), 65.0 / 3.0);
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut sma = sma(3);

            // Bar 1: open then close
            assert_eq!(sma.compute(&bar(5.0, 1)), None);
            assert_eq!(sma.compute(&bar(3.0, 1)), None); // repaint

            // Bar 2: open then close
            assert_eq!(sma.compute(&bar(6.0, 2)), None);
            assert_eq!(sma.compute(&bar(8.0, 2)), None); // repaint

            // Bar 3: open
            let result = sma.compute(&bar(4.0, 3));
            // (3 + 8 + 4) / 3 = 5
            assert_eq!(result, Some(5.0));

            // Bar 3: close (repaint)
            let result = sma.compute(&bar(7.0, 3));
            // (3 + 8 + 7) / 3 = 6
            assert_eq!(result, Some(6.0));

            // Bar 4
            let result = sma.compute(&bar(9.0, 4));
            // (8 + 7 + 9) / 3 = 8
            assert_eq!(result, Some(8.0));
        }
    }

    mod price_source {
        use super::*;
        use crate::test_util::Bar;

        #[test]
        fn hl2_source() {
            let mut sma = Sma::new(SmaConfig::hl2(nz(2)));
            // HL2 = (high + low) / 2
            sma.compute(&Bar::new(0.0, 20.0, 10.0, 0.0).at(1)); // HL2 = 15
            let result = sma.compute(&Bar::new(0.0, 30.0, 20.0, 0.0).at(2)); // HL2 = 25
            // (15 + 25) / 2 = 20
            assert_eq!(result, Some(20.0));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let sma = sma(20);
            assert_eq!(sma.to_string(), "SMA(20, Close)");
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut sma = sma(3);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));

            let mut cloned = sma.clone();

            // Advance original to convergence
            assert_eq!(sma.compute(&bar(30.0, 3)), Some(20.0));

            // Clone still has no value (only saw 2 bars)
            assert_eq!(cloned.value(), None);

            // Clone converges independently
            assert_eq!(cloned.compute(&bar(90.0, 3)), Some(40.0));
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper_uses_close_source() {
            let config = SmaConfig::close(nz(10));
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn hl2_helper_uses_hl2_source() {
            let config = SmaConfig::hl2(nz(10));
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        fn ohlc4_helper_uses_ohlc4_source() {
            let config = SmaConfig::ohlc4(nz(10));
            assert_eq!(config.source(), PriceSource::OHLC4);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = SmaConfig::builder().build();
        }

        #[test]
        fn convergence_equals_length() {
            let config = SmaConfig::close(nz(20));
            assert_eq!(config.convergence(), 20);

            let config = SmaConfig::close(nz(200));
            assert_eq!(config.convergence(), 200);
        }

        #[test]
        fn display_config() {
            let config = SmaConfig::close(nz(20));
            assert_eq!(config.to_string(), "SmaConfig(20, Close)");
        }

        #[test]
        fn eq_and_hash() {
            let a = SmaConfig::close(nz(20));
            let b = SmaConfig::close(nz(20));
            let c = SmaConfig::close(nz(10));

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = SmaConfig::hl2(nz(10));
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod true_range {
        use super::*;
        use crate::test_util::ohlc;

        fn tr_sma(length: usize) -> Sma {
            Sma::new(
                SmaConfig::builder()
                    .length(nz(length))
                    .source(PriceSource::TrueRange)
                    .build(),
            )
        }

        #[test]
        fn first_bar_uses_high_minus_low() {
            let mut sma = tr_sma(1);
            // No prev_close → TR = high - low = 25
            assert_eq!(sma.compute(&ohlc(10.0, 30.0, 5.0, 20.0, 1)), Some(25.0));
        }

        #[test]
        fn averages_true_range_over_window() {
            let mut sma = tr_sma(2);
            sma.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)); // TR=15
            // TR2: hl=10, |22-15|=7, |12-15|=3 → 10
            // SMA = (15 + 10) / 2 = 12.5
            assert_eq!(sma.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)), Some(12.5),);
        }

        #[test]
        fn gap_up_uses_prev_close() {
            let mut sma = tr_sma(1);
            sma.compute(&ohlc(10.0, 15.0, 5.0, 10.0, 1)); // close=10
            // Gap up: hl=10, |30-10|=20, |20-10|=10 → 20
            assert_eq!(sma.compute(&ohlc(25.0, 30.0, 20.0, 28.0, 2)), Some(20.0),);
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let sma = sma(3);
            assert_eq!(sma.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            sma.compute(&bar(20.0, 2));
            assert_eq!(sma.value(), Some(15.0));
        }

        #[test]
        fn matches_last_compute() {
            let mut sma = sma(2);
            sma.compute(&bar(10.0, 1));
            let computed = sma.compute(&bar(20.0, 2));
            assert_eq!(sma.value(), computed);
        }
    }
}
