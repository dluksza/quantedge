use std::fmt::Display;

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, PriceSource, Timestamp,
    internals::extract_price,
};

/// Configuration for the On-Balance Volume ([`Obv`]) indicator.
///
/// On-Balance Volume is a momentum indicator that relates
/// volume to price change. The cumulative sum of volume
/// depends on whether the current close is higher or lower
/// than the previous close.
///
/// The [`PriceSource`](crate::PriceSource) selects which OHLCV field
/// drives the volume change calculation (typically [`PriceSource::Close`]).
///
/// # Example
///
/// ```
/// use quantedge_ta::{Obv, ObvConfig, Ohlcv};
///
/// fn bar(close: f64, volume: f64, time: u64) -> Ohlcv {
///     Ohlcv { open: close, high: close, low: close, close, volume, open_time: time }
/// }
///
/// let mut obv = Obv::new(ObvConfig::default());
///
/// assert_eq!(obv.compute(&bar(10.0, 100.0, 1)), Some(100.0));
/// assert_eq!(obv.compute(&bar(12.0, 150.0, 2)), Some(250.0));
/// assert_eq!(obv.compute(&bar(11.0, 200.0, 3)), Some(50.0));
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ObvConfig {
    source: PriceSource,
}

impl IndicatorConfig for ObvConfig {
    type Builder = ObvConfigBuilder;
    type Output = f64;

    fn builder() -> Self::Builder {
        ObvConfigBuilder::new()
    }

    fn source(&self) -> crate::PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        1
    }

    fn to_builder(&self) -> Self::Builder {
        ObvConfigBuilder {
            source: self.source,
        }
    }
}

impl Default for ObvConfig {
    /// Default: source=Close.
    fn default() -> Self {
        ObvConfig {
            source: PriceSource::Close,
        }
    }
}

impl Display for ObvConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObvConfig()")
    }
}

pub struct ObvConfigBuilder {
    source: PriceSource,
}

impl ObvConfigBuilder {
    fn new() -> Self {
        Self {
            source: PriceSource::Close,
        }
    }
}

impl IndicatorConfigBuilder<ObvConfig> for ObvConfigBuilder {
    fn source(mut self, value: crate::PriceSource) -> Self {
        self.source = value;
        self
    }

    fn build(self) -> ObvConfig {
        ObvConfig {
            source: self.source,
        }
    }
}

/// On-Balance Volume (OBV).
///
/// A cumulative volume indicator that adds volume on up-price
/// bars and subtracts volume on down-price bars. When the
/// current price equals the previous price, OBV is unchanged.
///
/// ```text
/// first bar:                OBV = volume
/// if price > prev_price:    OBV = prev_OBV + volume
/// if price < prev_price:    OBV = prev_OBV − volume
/// if price = prev_price:    OBV = prev_OBV
/// ```
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous OBV without
/// advancing state.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Obv, ObvConfig, Ohlcv};
///
/// fn bar(close: f64, volume: f64, time: u64) -> Ohlcv {
///     Ohlcv { open: close, high: close, low: close, close, volume, open_time: time }
/// }
///
/// let mut obv = Obv::new(ObvConfig::default());
///
/// // First bar: OBV = volume
/// assert_eq!(obv.compute(&bar(10.0, 100.0, 1)), Some(100.0));
///
/// // Price up: OBV = 100 + 150 = 250
/// assert_eq!(obv.compute(&bar(12.0, 150.0, 2)), Some(250.0));
///
/// // Price down: OBV = 250 − 200 = 50
/// assert_eq!(obv.compute(&bar(11.0, 200.0, 3)), Some(50.0));
/// ```
#[derive(Clone, Debug)]
pub struct Obv {
    config: ObvConfig,
    current_price: Option<f64>,
    prev_price: Option<f64>,
    previous: f64,
    current: Option<f64>,
    last_open_time: Option<Timestamp>,
}

impl Indicator for Obv {
    type Config = ObvConfig;
    type Output = f64;

    fn new(config: Self::Config) -> Self {
        Obv {
            config,
            current_price: None,
            prev_price: None,
            previous: 0.0,
            current: None,
            last_open_time: None,
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time);

        if is_next_bar {
            self.last_open_time = Some(ohlcv.open_time);
            self.prev_price = self.current_price;
            self.previous = self.current.unwrap_or(0.0);
        }

        let price = extract_price(self.config.source, ohlcv, self.prev_price);
        self.current_price = Some(price);

        self.current = match self.prev_price {
            Some(prev_price) => {
                let value = if price > prev_price {
                    self.previous + ohlcv.volume
                } else if price < prev_price {
                    self.previous - ohlcv.volume
                } else {
                    self.previous
                };

                Some(value)
            }
            None => Some(ohlcv.volume),
        };

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Obv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Obv()")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::bar;

    fn vbar(close: f64, volume: f64, time: u64) -> quantedge_core::test_util::Bar {
        bar(close, time).vol(volume)
    }

    fn obv() -> Obv {
        Obv::new(ObvConfig::default())
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_volume_on_first_bar() {
            let mut obv = obv();
            assert_eq!(obv.compute(&vbar(10.0, 100.0, 1)), Some(100.0));
        }

        #[test]
        fn convergence_is_one() {
            assert_eq!(ObvConfig::default().convergence(), 1);
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn price_up_adds_volume() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            assert_eq!(obv.compute(&vbar(12.0, 150.0, 2)), Some(250.0));
        }

        #[test]
        fn price_down_subtracts_volume() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            assert_eq!(obv.compute(&vbar(8.0, 200.0, 2)), Some(-100.0));
        }

        #[test]
        fn price_unchanged_keeps_obv() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            assert_eq!(obv.compute(&vbar(10.0, 200.0, 2)), Some(100.0));
        }

        #[test]
        fn cumulative_across_bars() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(12.0, 100.0, 2)); // up: 100+100=200
            obv.compute(&vbar(14.0, 200.0, 3)); // up: 200+200=400
            obv.compute(&vbar(13.0, 50.0, 4)); // down: 400-50=350
            assert_eq!(obv.compute(&vbar(13.0, 300.0, 5)), Some(350.0)); // unchanged
        }

        #[test]
        fn zero_volume_bars() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 0.0, 1)); // seed: 0
            assert_eq!(obv.compute(&vbar(12.0, 0.0, 2)), Some(0.0));
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(12.0, 150.0, 2)); // up: 250
            // Repaint bar 2 with lower price → down
            assert_eq!(obv.compute(&vbar(8.0, 150.0, 2)), Some(-50.0));
        }

        #[test]
        fn multiple_repaints() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(12.0, 150.0, 2)); // up: 250
            obv.compute(&vbar(8.0, 150.0, 2)); // repaint down: -50
            obv.compute(&vbar(10.0, 150.0, 2)); // repaint unchanged: 100
            assert_eq!(obv.value(), Some(100.0));
        }

        #[test]
        fn repaint_then_advance() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(12.0, 100.0, 2)); // up: 200
            obv.compute(&vbar(15.0, 100.0, 2)); // repaint: still up: 200
            // Advance: previous = 200, price up → 200+200=400
            assert_eq!(obv.compute(&vbar(20.0, 200.0, 3)), Some(400.0));
        }

        #[test]
        fn repaint_during_first_bar() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(15.0, 200.0, 1)); // repaint: seed: 200
            assert_eq!(obv.value(), Some(200.0));
            // Advance: compares against repainted price (15)
            assert_eq!(obv.compute(&vbar(20.0, 300.0, 2)), Some(500.0));
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut obv = obv();

            // Bar 1: open then close
            assert_eq!(obv.compute(&vbar(10.0, 100.0, 1)), Some(100.0));
            assert_eq!(obv.compute(&vbar(12.0, 150.0, 1)), Some(150.0)); // repaint

            // Bar 2: open (up from 12) → 150+200=350
            assert_eq!(obv.compute(&vbar(15.0, 200.0, 2)), Some(350.0));
            // Bar 2: close (down from 12) → 150-300=-150
            assert_eq!(obv.compute(&vbar(10.0, 300.0, 2)), Some(-150.0));

            // Bar 3: price up from 10 → -150+100=-50
            assert_eq!(obv.compute(&vbar(11.0, 100.0, 3)), Some(-50.0));
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1)); // seed: 100
            obv.compute(&vbar(12.0, 100.0, 2)); // up: 200

            let mut cloned = obv.clone();

            // Advance original: up → 200+200=400
            assert_eq!(obv.compute(&vbar(14.0, 200.0, 3)), Some(400.0));

            // Clone still at 200
            assert_eq!(cloned.value(), Some(200.0));

            // Clone advances independently: down → 200-50=150
            assert_eq!(cloned.compute(&vbar(10.0, 50.0, 3)), Some(150.0));
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_source_is_close() {
            let config = ObvConfig::default();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn eq_and_hash() {
            let a = ObvConfig::default();
            let b = ObvConfig::builder().build();

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = ObvConfig::default();
            assert_eq!(config.to_builder().build(), config);
        }

        #[test]
        fn display_config() {
            let config = ObvConfig::default();
            assert_eq!(config.to_string(), "ObvConfig()");
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let obv = obv();
            assert_eq!(obv.to_string(), "Obv()");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_any_bar() {
            let obv = obv();
            assert_eq!(obv.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1));
            obv.compute(&vbar(12.0, 150.0, 2));
            assert_eq!(obv.value(), Some(250.0));
        }

        #[test]
        fn matches_last_compute() {
            let mut obv = obv();
            obv.compute(&vbar(10.0, 100.0, 1));
            let computed = obv.compute(&vbar(12.0, 150.0, 2));
            assert_eq!(obv.value(), computed);
        }
    }

    mod no_volume {
        use super::*;

        #[test]
        fn works_with_default_zero_volume() {
            let mut obv = obv();
            assert_eq!(obv.compute(&bar(10.0, 1)), Some(0.0));
            assert_eq!(obv.compute(&bar(12.0, 2)), Some(0.0));
        }
    }
}
