use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, PriceSource,
    internals::{BarAction, BarState, RollingExtremes, RollingSum},
};

/// Configuration for the Choppiness Index ([`Chop`]) indicator.
///
/// The Choppiness Index measures whether the market is trending or
/// ranging on a 0–100 scale. Values near 100 indicate a choppy,
/// sideways market; values near 0 indicate a strong trend. The
/// indicator uses the ratio of the sum of True Range over a window
/// to the highest-high minus lowest-low range.
///
/// Output begins after `length` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::ChopConfig;
/// use std::num::NonZero;
///
/// let config = ChopConfig::builder().length(NonZero::new(14).unwrap()).build();
/// assert_eq!(config.length(), 14);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ChopConfig {
    length: usize,
}

impl IndicatorConfig for ChopConfig {
    type Builder = ChopConfigBuilder;

    fn builder() -> Self::Builder {
        ChopConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        PriceSource::TrueRange
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        ChopConfigBuilder {
            length: Some(self.length),
        }
    }
}

impl ChopConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }
}

impl Default for ChopConfig {
    /// Default: length=14 (`TradingView` default).
    fn default() -> Self {
        Self { length: 14 }
    }
}

impl Display for ChopConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChopConfig({})", self.length)
    }
}

/// Builder for [`ChopConfig`].
///
/// Defaults: source = [`PriceSource::TrueRange`].
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct ChopConfigBuilder {
    length: Option<usize>,
}

impl ChopConfigBuilder {
    fn new() -> Self {
        Self { length: None }
    }

    /// Sets the indicator window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }
}

impl IndicatorConfigBuilder<ChopConfig> for ChopConfigBuilder {
    fn source(self, _source: PriceSource) -> Self {
        self
    }

    fn build(self) -> ChopConfig {
        ChopConfig {
            length: self.length.expect("length is required"),
        }
    }
}

/// Choppiness Index (CHOP).
///
/// Measures whether the market is trending or consolidating on a
/// 0–100 scale. Higher values indicate a choppy, range-bound market;
/// lower values indicate a strong directional trend.
///
/// The formula is:
///
/// ```text
/// CHOP = 100 × log10(sum(TR, length) / (highest_high − lowest_low)) / log10(length)
/// ```
///
/// When the highest high equals the lowest low (flat market), returns
/// 100.
///
/// Returns `None` until the lookback window is full (after `length`
/// bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Chop, ChopConfig};
/// use std::num::NonZero;
/// # use quantedge_ta::{Ohlcv, Price, Timestamp};
/// #
/// # struct Bar { h: f64, l: f64, c: f64, t: u64 }
/// # impl Ohlcv for Bar {
/// #     fn open(&self) -> Price { self.c }
/// #     fn high(&self) -> Price { self.h }
/// #     fn low(&self) -> Price { self.l }
/// #     fn close(&self) -> Price { self.c }
/// #     fn open_time(&self) -> Timestamp { self.t }
/// # }
///
/// let mut chop = Chop::new(ChopConfig::builder().length(NonZero::new(2).unwrap()).build());
///
/// assert_eq!(chop.compute(&Bar { h: 20.0, l: 10.0, c: 15.0, t: 1 }), None);
/// // Window full — returns a value between 0 and 100
/// let val = chop.compute(&Bar { h: 22.0, l: 12.0, c: 18.0, t: 2 }).unwrap();
/// assert!((0.0..=100.0).contains(&val));
/// ```
#[derive(Clone, Debug)]
pub struct Chop {
    config: ChopConfig,
    bar_state: BarState,
    extremes: RollingExtremes,
    tr_sum: RollingSum,
    current: Option<f64>,
    log_length_reciprocal: f64,
}

impl Indicator for Chop {
    type Config = ChopConfig;
    type Output = f64;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            bar_state: BarState::new(PriceSource::TrueRange),
            extremes: RollingExtremes::new(config.length),
            tr_sum: RollingSum::new(config.length),
            current: None,
            #[allow(clippy::cast_precision_loss)]
            log_length_reciprocal: 100.0 / f64::log10(config.length as f64),
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        let extremes = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                self.tr_sum.push(price);
                self.extremes.push(ohlcv)
            }
            BarAction::Repaint(price) => {
                self.tr_sum.replace(price);
                self.extremes.replace(ohlcv)
            }
        };

        self.current = extremes.and_then(|(highest_high, lowest_low)| {
            let extreme_diff = highest_high - lowest_low;

            if extreme_diff < f64::EPSILON {
                Some(100.0)
            } else {
                self.tr_sum
                    .sum()
                    .map(|sum| (sum / extreme_diff).log10() * self.log_length_reciprocal)
            }
        });

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Chop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CHOP({})", self.config.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{assert_approx, nz, ohlc};

    fn chop(length: usize) -> Chop {
        Chop::new(ChopConfig::builder().length(nz(length)).build())
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            let mut chop = chop(3);
            assert_eq!(chop.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)), None);
            assert_eq!(chop.compute(&ohlc(12.0, 22.0, 8.0, 18.0, 2)), None);
        }

        #[test]
        fn returns_value_when_full() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            assert!(chop.compute(&ohlc(12.0, 22.0, 8.0, 18.0, 2)).is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn flat_market_gives_100() {
            // All bars identical → extreme_diff < EPSILON → 100
            let mut chop = chop(3);
            for t in 1..=3 {
                chop.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
            }
            assert_eq!(chop.value(), Some(100.0));
        }

        #[test]
        fn value_between_0_and_100() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            let val = chop.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)).unwrap();
            assert!(
                (0.0..=100.0).contains(&val),
                "CHOP should be between 0 and 100, got {val}"
            );
        }

        #[test]
        fn manual_computation() {
            // length=2
            // Bar 1: no prev_close → TR = high - low = 20 - 5 = 15
            // Bar 2: TR = max(22-12, |22-15|, |12-15|) = max(10, 7, 3) = 10
            // tr_sum = 15 + 10 = 25
            // HH = 22, LL = 5, extreme_diff = 17
            // CHOP = 100 * log10(25/17) / log10(2)
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            let val = chop.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)).unwrap();
            let expected = 100.0 * f64::log10(25.0 / 17.0) * (1.0 / f64::log10(2.0));
            assert_approx!(val, expected);
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn old_bar_expires() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 50.0, 1.0, 15.0, 1)); // extreme bar
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 2));
            let v1 = chop.value().unwrap();

            // Advance: bar 1 expires
            chop.compute(&ohlc(10.0, 22.0, 12.0, 17.0, 3));
            let v2 = chop.value().unwrap();

            assert!(
                (v1 - v2).abs() > 1e-10,
                "value should change when extreme ages out"
            );
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let v1 = chop.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2)).unwrap();
            let v2 = chop.compute(&ohlc(12.0, 25.0, 6.0, 16.0, 2)).unwrap();
            assert!(
                (v1 - v2).abs() > 1e-10,
                "repaint with different extremes should change value"
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut ind = chop(2);
            ind.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            ind.compute(&ohlc(12.0, 22.0, 8.0, 16.0, 2));
            ind.compute(&ohlc(12.0, 25.0, 7.0, 18.0, 2)); // repaint
            let final_val = ind.compute(&ohlc(12.0, 26.0, 6.0, 17.0, 2)).unwrap();

            // Clean computation with final values only
            let mut clean = chop(2);
            clean.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let expected = clean.compute(&ohlc(12.0, 26.0, 6.0, 17.0, 2)).unwrap();

            assert!((final_val - expected).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling() {
            let mut chop = chop(3);
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            chop.compute(&ohlc(12.0, 22.0, 8.0, 16.0, 1)); // repaint bar 1 (h up, l down)
            assert_eq!(chop.compute(&ohlc(14.0, 22.0, 14.0, 18.0, 2)), None);
            // Bar 3: window full
            assert!(chop.compute(&ohlc(16.0, 24.0, 16.0, 20.0, 3)).is_some());
        }

        #[test]
        fn advance_after_repaint() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            chop.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            chop.compute(&ohlc(12.0, 22.0, 8.0, 19.0, 2)); // repaint
            let after = chop.compute(&ohlc(14.0, 24.0, 14.0, 20.0, 3)).unwrap();
            assert!((0.0..=100.0).contains(&after));
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut chop = chop(2);

            // Bar 1: open then close
            assert_eq!(chop.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)), None);
            assert_eq!(chop.compute(&ohlc(10.0, 18.0, 4.0, 10.0, 1)), None); // repaint

            // Bar 2: open
            let v1 = chop.compute(&ohlc(11.0, 16.0, 8.0, 14.0, 2)).unwrap();
            // Bar 2: close (repaint)
            let v2 = chop.compute(&ohlc(11.0, 20.0, 6.0, 11.0, 2)).unwrap();

            assert!((v1 - v2).abs() > 1e-10, "repaint should change value");

            // Bar 3
            let v3 = chop.compute(&ohlc(12.0, 22.0, 10.0, 18.0, 3)).unwrap();
            assert!((0.0..=100.0).contains(&v3));
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn always_between_0_and_100() {
            let mut chop = chop(3);
            let bars = [
                ohlc(100.0, 110.0, 90.0, 105.0, 1),
                ohlc(102.0, 115.0, 88.0, 98.0, 2),
                ohlc(99.0, 108.0, 92.0, 103.0, 3),
                ohlc(101.0, 120.0, 85.0, 95.0, 4),
                ohlc(96.0, 105.0, 80.0, 100.0, 5),
                ohlc(98.0, 130.0, 75.0, 110.0, 6),
            ];
            for b in &bars {
                if let Some(value) = chop.compute(b) {
                    assert!(
                        (0.0..=100.0).contains(&value),
                        "CHOP out of bounds: {value}"
                    );
                }
            }
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            chop.compute(&ohlc(12.0, 22.0, 8.0, 18.0, 2));

            let mut cloned = chop.clone();

            // Advance original with wide-range bar
            let orig_val = chop.compute(&ohlc(14.0, 40.0, 2.0, 20.0, 3)).unwrap();

            // Clone advances with narrow-range bar
            let clone_val = cloned.compute(&ohlc(14.0, 16.0, 14.0, 15.0, 3)).unwrap();
            assert!(
                (orig_val - clone_val).abs() > 1e-10,
                "divergent inputs should give different CHOP"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper() {
            let config = ChopConfig::builder().length(nz(14)).build();
            assert_eq!(config.length(), 14);
        }

        #[test]
        fn convergence_equals_length() {
            let config = ChopConfig::builder().length(nz(14)).build();
            assert_eq!(config.convergence(), 14);

            let config = ChopConfig::builder().length(nz(3)).build();
            assert_eq!(config.convergence(), 3);
        }

        #[test]
        fn source_is_true_range() {
            let config = ChopConfig::builder().length(nz(14)).build();
            assert_eq!(config.source(), PriceSource::TrueRange);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = ChopConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = ChopConfig::builder().length(nz(14)).build();
            let b = ChopConfig::builder().length(nz(14)).build();
            let c = ChopConfig::builder().length(nz(7)).build();

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = ChopConfig::builder().length(nz(14)).build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let chop = chop(14);
            assert_eq!(chop.to_string(), "CHOP(14)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = ChopConfig::builder().length(nz(14)).build();
            assert_eq!(config.to_string(), "ChopConfig(14)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let chop = chop(3);
            assert_eq!(chop.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            chop.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            assert!(chop.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut chop = chop(2);
            chop.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let computed = chop.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            assert_eq!(chop.value(), computed);
        }
    }
}
