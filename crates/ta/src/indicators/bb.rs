use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Multiplier, Ohlcv, Price, PriceSource,
    internals::{PriceWindow, PriceWindowWithSumOfSquares},
};

/// Configuration for the Bollinger Bands ([`Bb`]) indicator.
///
/// # Convergence
///
/// Bollinger Bands use an SMA for the middle band. Like SMA, values are exact
/// once the window is full, there is no warm-up bias to suppress.
///
/// # Example
///
/// ```
/// use quantedge_ta::BbConfig;
/// use std::num::NonZero;
///
/// // Default: length 20, close, 2.0 std devs
/// let config = BbConfig::builder()
///     .length(NonZero::new(20).unwrap())
///     .build();
///
/// assert_eq!(config.length(), 20);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct BbConfig {
    length: usize,
    source: PriceSource,
    std_dev: Multiplier,
}

impl IndicatorConfig for BbConfig {
    type Builder = BbConfigBuilder;
    type Output = BbValue;

    fn builder() -> Self::Builder {
        BbConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        BbConfigBuilder {
            length: Some(self.length),
            source: self.source,
            std_dev: self.std_dev,
        }
    }
}

impl BbConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Standard deviation multiplier for the upper and lower bands.
    #[must_use]
    pub fn std_dev(&self) -> Multiplier {
        self.std_dev
    }

    /// BB(20, Close, 2σ) — the standard Bollinger Bands setting.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn default_20() -> Self {
        Self::builder().length(NonZero::new(20).unwrap()).build()
    }

    /// BB with custom length, close price, 2σ.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }
}

impl Default for BbConfig {
    /// Default: length=20, source=Close, `std_dev`=2.0 (Bollinger's original, `TradingView` default).
    fn default() -> Self {
        Self {
            length: 20,
            source: PriceSource::Close,
            std_dev: Multiplier::new(2.0),
        }
    }
}

impl Display for BbConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BbConfig({}, {}, {})",
            self.length,
            self.source,
            self.std_dev.value()
        )
    }
}

/// Builder for [`BbConfig`].
///
/// Defaults: source = [`PriceSource::Close`],
/// `std_dev` = `2.0`.
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct BbConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
    std_dev: Multiplier,
}

impl BbConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            source: PriceSource::Close,
            std_dev: Multiplier::new(2.0),
        }
    }

    /// Sets the indicator window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }

    /// Sets the standard deviation multiplier for the upper and lower bands.
    #[must_use]
    pub fn std_dev(mut self, std_dev: Multiplier) -> Self {
        self.std_dev = std_dev;
        self
    }
}

impl IndicatorConfigBuilder<BbConfig> for BbConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> BbConfig {
        BbConfig {
            length: self.length.expect("length is required"),
            source: self.source,
            std_dev: self.std_dev,
        }
    }
}

/// Bollinger Bands output: upper, middle, and lower bands.
///
/// The middle band is the SMA. Upper and lower bands are offset by
/// `std_dev × σ`, where `σ` is the population standard deviation of the window.
///
/// ```text
/// upper  = SMA + k × σ
/// middle = SMA
/// lower  = SMA − k × σ
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BbValue {
    upper: Price,
    middle: Price,
    lower: Price,
}

impl BbValue {
    /// Upper band: `SMA + k × σ`.
    #[inline]
    #[must_use]
    pub fn upper(&self) -> Price {
        self.upper
    }

    /// Middle band: SMA of the window.
    #[inline]
    #[must_use]
    pub fn middle(&self) -> Price {
        self.middle
    }

    /// Lower band: `SMA − k × σ`.
    #[inline]
    #[must_use]
    pub fn lower(&self) -> Price {
        self.lower
    }

    /// Band width: `upper − lower`.
    ///
    /// Useful for measuring volatility. Narrow width indicates
    /// consolidation (Bollinger squeeze); wide width indicates
    /// high volatility.
    #[inline]
    #[must_use]
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }
}

impl Display for BbValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BB(u: {}, m: {}, l: {})",
            self.upper, self.middle, self.lower
        )
    }
}

/// Bollinger Bands (BB).
///
/// A volatility indicator consisting of three bands: a simple moving average
/// (middle) with upper and lower bands offset by a configurable number of
/// standard deviations.
///
/// Uses a running sum and sum of squares for O(1) updates per tick. The only
/// non-constant operation is `sqrt` for the standard deviation, which is
/// unavoidable.
///
/// Supports live repainting: feeding a bar with the same `open_time` replaces
/// the current value without advancing the window.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Bb, BbConfig};
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
/// let config = BbConfig::builder()
///     .length(NonZero::new(20).unwrap())
///     .build();
/// let mut bb = Bb::new(config);
///
/// // Feed bars...
/// # for i in 1..=19 { bb.compute(&Bar(100.0, i)); }
///
/// if let Some(value) = bb.compute(&Bar(100.0, 20)) {
///     println!("upper: {}, middle: {}, lower: {}",
///         value.upper(), value.middle(), value.lower());
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Bb {
    config: BbConfig,
    length_reciprocal: f64,
    std_dev_multiplier: f64,
    window: PriceWindowWithSumOfSquares,
    current: Option<BbValue>,
}

impl Indicator for Bb {
    type Config = BbConfig;
    type Output = BbValue;

    fn new(config: Self::Config) -> Self {
        let window = PriceWindow::with_sum_of_squares(config.length, config.source);

        Self {
            config,
            #[allow(clippy::cast_precision_loss)]
            length_reciprocal: 1.0 / config.length as f64,
            std_dev_multiplier: config.std_dev.value(),
            window,
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl Ohlcv) -> Option<Self::Output> {
        self.window.add(ohlcv);

        self.current = match (self.window.sum(), self.window.sum_of_squares()) {
            (Some(sum), Some(sum_of_squares)) => {
                let mean = sum * self.length_reciprocal;

                // Variance = E[X^2] - (E[X])^2 = (sum_of_squares / n) - mean^2
                let variance = sum_of_squares.mul_add(self.length_reciprocal, -(mean * mean));
                let std_dev = variance.max(0.0).sqrt() * self.std_dev_multiplier;

                Some(Self::Output {
                    upper: mean + std_dev,
                    middle: mean,
                    lower: mean - std_dev,
                })
            }
            _ => None,
        };

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Bb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BB({}, {}, {})",
            self.config.length, self.config.source, self.std_dev_multiplier,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{Bar, nz};

    fn bb(length: usize) -> Bb {
        Bb::new(BbConfig::builder().length(nz(length)).build())
    }

    fn bb_with_std_dev(length: usize, std_dev: f64) -> Bb {
        Bb::new(
            BbConfig::builder()
                .length(nz(length))
                .std_dev(Multiplier::new(std_dev))
                .build(),
        )
    }

    fn bar(close: f64, time: u64) -> Bar {
        Bar::new(0.0, 0.0, 0.0, close).at(time)
    }

    fn assert_bb(value: Option<BbValue>, upper: f64, middle: f64, lower: f64) {
        let v = value.expect("expected Some(BbValue)");
        assert!(
            (v.upper() - upper).abs() < 1e-10,
            "upper: expected {upper}, got {}",
            v.upper()
        );
        assert!(
            (v.middle() - middle).abs() < 1e-10,
            "middle: expected {middle}, got {}",
            v.middle()
        );
        assert!(
            (v.lower() - lower).abs() < 1e-10,
            "lower: expected {lower}, got {}",
            v.lower()
        );
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            let mut bb = bb(3);
            assert!(bb.compute(&bar(10.0, 1)).is_none());
            assert!(bb.compute(&bar(20.0, 2)).is_none());
        }

        #[test]
        fn returns_value_when_full() {
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            assert!(bb.compute(&bar(5.0, 2)).is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn basic_bands() {
            // window [3, 5], std_dev=2
            // mean=4, variance=1, σ=1
            // upper=6, middle=4, lower=2
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            assert_bb(bb.compute(&bar(5.0, 2)), 6.0, 4.0, 2.0);
        }

        #[test]
        fn constant_input_zero_width() {
            // All values equal → variance=0 → bands collapse
            let mut bb = bb(3);
            bb.compute(&bar(10.0, 1));
            bb.compute(&bar(10.0, 2));
            assert_bb(bb.compute(&bar(10.0, 3)), 10.0, 10.0, 10.0);
        }

        #[test]
        fn bands_are_symmetric() {
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            let v = bb.compute(&bar(5.0, 2)).unwrap();
            let upper_dist = v.upper() - v.middle();
            let lower_dist = v.middle() - v.lower();
            assert!((upper_dist - lower_dist).abs() < 1e-10);
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn updates_on_advance() {
            // [3, 5] → [5, 7]
            // mean=6, variance=1, σ=1
            // upper=8, middle=6, lower=4
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            bb.compute(&bar(5.0, 2));
            assert_bb(bb.compute(&bar(7.0, 3)), 8.0, 6.0, 4.0);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn replaces_current_bar() {
            // [3, 5] then repaint → [3, 7]
            // mean=5, variance=4, σ=2
            // upper=9, middle=5, lower=1
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            bb.compute(&bar(5.0, 2));
            assert_bb(bb.compute(&bar(7.0, 2)), 9.0, 5.0, 1.0);
        }

        #[test]
        fn repaint_during_filling() {
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            bb.compute(&bar(4.0, 1)); // repaint
            assert!(bb.compute(&bar(4.0, 1)).is_none()); // still filling
            // Advance to convergence after repaint
            // [4, 6], mean=5, var=1, σ=1, k=2 → (7, 5, 3)
            assert_bb(bb.compute(&bar(6.0, 2)), 7.0, 5.0, 3.0);
        }
    }

    mod std_dev_multiplier {
        use super::*;

        #[test]
        fn multiplier_of_one() {
            // [3, 5], std_dev=1 → σ=1
            // upper=5, middle=4, lower=3
            let mut bb = bb_with_std_dev(2, 1.0);
            bb.compute(&bar(3.0, 1));
            assert_bb(bb.compute(&bar(5.0, 2)), 5.0, 4.0, 3.0);
        }

        #[test]
        fn fractional_multiplier() {
            // [3, 5], std_dev=1.5 → σ=1
            // upper=4+1.5=5.5, middle=4, lower=4-1.5=2.5
            let mut bb = bb_with_std_dev(2, 1.5);
            bb.compute(&bar(3.0, 1));
            assert_bb(bb.compute(&bar(5.0, 2)), 5.5, 4.0, 2.5);
        }

        #[test]
        fn wider_multiplier_wider_bands() {
            let mut bb1 = bb_with_std_dev(2, 1.0);
            let mut bb2 = bb_with_std_dev(2, 3.0);

            bb1.compute(&bar(3.0, 1));
            bb2.compute(&bar(3.0, 1));

            let v1 = bb1.compute(&bar(5.0, 2)).unwrap();
            let v2 = bb2.compute(&bar(5.0, 2)).unwrap();

            assert!(v2.width() > v1.width());
        }
    }

    mod width {
        use super::*;

        #[test]
        fn equals_upper_minus_lower() {
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            let v = bb.compute(&bar(5.0, 2)).unwrap();
            assert!((v.width() - (v.upper() - v.lower())).abs() < 1e-10);
        }

        #[test]
        fn zero_for_constant_input() {
            let mut bb = bb(2);
            bb.compute(&bar(10.0, 1));
            let v = bb.compute(&bar(10.0, 2)).unwrap();
            assert!((v.width()).abs() < 1e-10);
        }
    }

    mod value {
        use super::*;

        #[test]
        fn returns_last_computed() {
            let mut bb = bb(2);
            bb.compute(&bar(3.0, 1));
            bb.compute(&bar(5.0, 2));
            assert_eq!(bb.value(), bb.compute(&bar(5.0, 2)));
        }

        #[test]
        fn none_before_first_value() {
            let bb = bb(2);
            assert!(bb.value().is_none());
        }
    }

    mod config {
        use super::*;

        #[test]
        fn convergence_equals_length() {
            let config = BbConfig::close(nz(20));
            assert_eq!(config.convergence(), 20);

            let config = BbConfig::close(nz(200));
            assert_eq!(config.convergence(), 200);
        }

        #[test]
        fn default_std_dev_is_two() {
            let config = BbConfig::builder().length(nz(20)).build();
            assert!((config.std_dev().value() - 2.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_source_is_close() {
            let config = BbConfig::builder().length(nz(20)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = BbConfig::builder().build();
        }

        #[test]
        #[should_panic(expected = "multiplier must be positive")]
        fn std_dev_rejects_zero() {
            let _ = Multiplier::new(0.0);
        }

        #[test]
        #[should_panic(expected = "multiplier must be positive")]
        fn std_dev_rejects_negative() {
            let _ = Multiplier::new(-1.0);
        }

        #[test]
        #[should_panic(expected = "multiplier must not be NaN")]
        fn std_dev_rejects_nan() {
            let _ = Multiplier::new(f64::NAN);
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = BbConfig::builder()
                .length(nz(15))
                .std_dev(Multiplier::new(2.5))
                .source(PriceSource::HLC3)
                .build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut bb = bb(3);
            bb.compute(&bar(10.0, 1));
            bb.compute(&bar(20.0, 2));

            let mut cloned = bb.clone();

            // Advance original to convergence
            // [10, 20, 30], mean=20, var=200/3, σ=√(200/3), k=2
            assert!(bb.compute(&bar(30.0, 3)).is_some());

            // Clone still has no value (only saw 2 bars)
            assert_eq!(cloned.value(), None);

            // Clone converges independently with different data
            assert!(cloned.compute(&bar(90.0, 3)).is_some());
            assert!(
                (bb.value().unwrap().middle() - cloned.value().unwrap().middle()).abs() > 1e-10
            );
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn hl2_source() {
            let mut bb = Bb::new(
                BbConfig::builder()
                    .length(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            // HL2 = (high + low) / 2
            bb.compute(&Bar::new(0.0, 20.0, 10.0, 0.0).at(1)); // HL2 = 15
            let v = bb.compute(&Bar::new(0.0, 30.0, 20.0, 0.0).at(2)).unwrap(); // HL2 = 25
            // [15, 25], mean=20
            assert!((v.middle() - 20.0).abs() < 1e-10);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn bb_formats_correctly() {
            let bb = bb(20);
            assert_eq!(bb.to_string(), "BB(20, Close, 2)");
        }

        #[test]
        fn bb_value_formats_correctly() {
            let v = BbValue {
                upper: 6.0,
                middle: 4.0,
                lower: 2.0,
            };
            assert_eq!(v.to_string(), "BB(u: 6, m: 4, l: 2)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = BbConfig::builder().length(nz(20)).build();
            assert_eq!(config.to_string(), "BbConfig(20, Close, 2)");
        }
    }

    mod eq_and_hash {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn identical_configs_match() {
            let a = BbConfig::builder().length(nz(20)).build();
            let b = BbConfig::builder().length(nz(20)).build();
            let c = BbConfig::builder().length(nz(10)).build();

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }
    }
}
