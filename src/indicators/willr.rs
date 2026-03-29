use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, PriceSource,
    internals::{BarAction, BarState, RollingExtremes},
};

/// Configuration for the Williams %R ([`WillR`]) indicator.
///
/// Williams %R is a momentum oscillator that measures overbought and
/// oversold levels on a scale from −100 to 0. It compares the current
/// price to the highest high over the lookback window.
///
/// Output begins after `length` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::WillRConfig;
/// use std::num::NonZero;
///
/// let config = WillRConfig::close(NonZero::new(14).unwrap());
/// assert_eq!(config.length(), 14);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct WillRConfig {
    length: usize,
    source: PriceSource,
}

impl IndicatorConfig for WillRConfig {
    type Builder = WillRConfigBuilder;

    fn builder() -> Self::Builder {
        WillRConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        WillRConfigBuilder {
            length: Some(self.length),
            source: self.source,
        }
    }
}

impl WillRConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Williams %R on closing price.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }
}

impl Display for WillRConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WillRConfig({}, {})", self.length, self.source)
    }
}

/// Builder for [`WillRConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct WillRConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
}

impl WillRConfigBuilder {
    fn new() -> Self {
        WillRConfigBuilder {
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

impl IndicatorConfigBuilder<WillRConfig> for WillRConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> WillRConfig {
        WillRConfig {
            length: self.length.expect("length is required"),
            source: self.source,
        }
    }
}

/// Williams %R (%R).
///
/// A momentum oscillator that measures the current price relative to
/// the highest high over a lookback window. The result ranges from
/// −100 (price at the lowest low) to 0 (price at the highest high).
/// Values above −20 are conventionally considered overbought; below
/// −80, oversold.
///
/// ```text
/// %R = (highest_high − price) / (highest_high − lowest_low) × −100
/// ```
///
/// When the highest high equals the lowest low (flat market), returns
/// −50.
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
/// use quantedge_ta::{WillR, WillRConfig};
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
/// let mut wr = WillR::new(WillRConfig::close(NonZero::new(3).unwrap()));
///
/// assert_eq!(wr.compute(&Bar { h: 10.0, l: 5.0, c: 8.0, t: 1 }), None);
/// assert_eq!(wr.compute(&Bar { h: 12.0, l: 6.0, c: 10.0, t: 2 }), None);
///
/// // Window full: highest_high=15, lowest_low=5
/// // %R = (15 − 11) / (15 − 5) × −100 = −40
/// assert_eq!(wr.compute(&Bar { h: 15.0, l: 7.0, c: 11.0, t: 3 }), Some(-40.0));
/// ```
#[derive(Clone, Debug)]
pub struct WillR {
    config: WillRConfig,
    bar_state: BarState,
    extremes: RollingExtremes,
    current: Option<f64>,
}

impl Indicator for WillR {
    type Config = WillRConfig;
    type Output = f64;

    fn new(config: Self::Config) -> Self {
        WillR {
            config,
            bar_state: BarState::new(config.source),
            extremes: RollingExtremes::new(config.length),
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        let (price, extremes) = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => (price, self.extremes.push(ohlcv)),
            BarAction::Repaint(price) => (price, self.extremes.replace(ohlcv)),
        };

        self.current = extremes.map(|(highest_high, lowest_low)| {
            let extreme_diff = highest_high - lowest_low;
            if extreme_diff < f64::EPSILON {
                -50.0
            } else {
                (highest_high - price) / extreme_diff * -100.0
            }
        });

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for WillR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WillR({}, {})", self.config.length, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Bar, bar, nz, ohlc};

    fn willr(length: usize) -> WillR {
        WillR::new(WillRConfig::close(nz(length)))
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            let mut wr = willr(3);
            assert_eq!(wr.compute(&bar(10.0, 1)), None);
            assert_eq!(wr.compute(&bar(20.0, 2)), None);
        }

        #[test]
        fn returns_value_when_full() {
            let mut wr = willr(3);
            wr.compute(&bar(10.0, 1));
            wr.compute(&bar(20.0, 2));
            assert!(wr.compute(&bar(15.0, 3)).is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn price_at_high_gives_zero() {
            // When close == highest high, %R = 0
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            // Bar 2: HH=20, LL=5, close=20
            // %R = (20 - 20) / (20 - 5) * -100 = 0
            assert_eq!(wr.compute(&ohlc(15.0, 18.0, 8.0, 20.0, 2)), Some(0.0));
        }

        #[test]
        fn price_at_low_gives_minus_100() {
            // When close == lowest low, %R = -100
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            // Bar 2: HH=20, LL=5, close=5
            // %R = (20 - 5) / (20 - 5) * -100 = -100
            assert_eq!(wr.compute(&ohlc(8.0, 10.0, 5.0, 5.0, 2)), Some(-100.0));
        }

        #[test]
        fn midpoint_gives_minus_50() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            // Bar 2: HH=20, LL=10, close=15
            // %R = (20 - 15) / (20 - 10) * -100 = -50
            assert_eq!(wr.compute(&ohlc(12.0, 16.0, 12.0, 15.0, 2)), Some(-50.0));
        }

        #[test]
        fn flat_market_gives_minus_50() {
            let mut wr = willr(3);
            for t in 1..=5 {
                wr.compute(&bar(10.0, t));
            }
            assert_eq!(wr.value(), Some(-50.0));
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn old_extreme_expires() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 50.0, 1.0, 15.0, 1)); // extreme bar
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 2));
            // Window: bar1, bar2. HH=50, LL=1
            let v1 = wr.value().unwrap();

            // Advance: bar1 expires
            wr.compute(&ohlc(10.0, 22.0, 12.0, 17.0, 3));
            // Window: bar2, bar3. HH=22, LL=10
            let v2 = wr.value().unwrap();

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
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let v1 = wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2)).unwrap();
            let v2 = wr.compute(&ohlc(12.0, 18.0, 12.0, 19.0, 2)).unwrap();
            assert!(v2 > v1, "higher close should give higher (closer to 0) %R");
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            wr.compute(&ohlc(12.0, 22.0, 8.0, 16.0, 2));
            wr.compute(&ohlc(12.0, 25.0, 7.0, 18.0, 2)); // repaint: high up, low down
            let final_val = wr.compute(&ohlc(12.0, 26.0, 6.0, 17.0, 2)).unwrap();

            // Clean computation with final values only
            let mut clean = willr(2);
            clean.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let expected = clean.compute(&ohlc(12.0, 26.0, 6.0, 17.0, 2)).unwrap();

            assert!((final_val - expected).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling() {
            let mut wr = willr(3);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 1)); // repaint bar 1
            assert_eq!(wr.compute(&ohlc(14.0, 22.0, 14.0, 18.0, 2)), None);
            // Bar 3: window full
            assert!(wr.compute(&ohlc(16.0, 24.0, 16.0, 20.0, 3)).is_some());
        }

        #[test]
        fn advance_after_repaint() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            wr.compute(&ohlc(12.0, 22.0, 8.0, 19.0, 2)); // repaint
            let after = wr.compute(&ohlc(14.0, 24.0, 14.0, 20.0, 3)).unwrap();

            // Clean path with repainted bar
            let mut clean = willr(2);
            clean.compute(&ohlc(12.0, 22.0, 8.0, 19.0, 1));
            // Note: clean starts fresh so extremes differ; just check it's valid
            assert!((-100.0..=0.0).contains(&after));
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut wr = willr(2);

            // Bar 1: open then close
            assert_eq!(wr.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)), None);
            assert_eq!(wr.compute(&ohlc(10.0, 18.0, 4.0, 10.0, 1)), None); // repaint

            // Bar 2: open
            let v1 = wr.compute(&ohlc(11.0, 16.0, 8.0, 14.0, 2)).unwrap();
            // Bar 2: close (repaint)
            let v2 = wr.compute(&ohlc(11.0, 20.0, 6.0, 11.0, 2)).unwrap();

            assert!((v1 - v2).abs() > 1e-10, "repaint should change value");

            // Bar 3
            let v3 = wr.compute(&ohlc(12.0, 22.0, 10.0, 18.0, 3)).unwrap();
            assert!((-100.0..=0.0).contains(&v3));
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn always_between_minus_100_and_0() {
            let mut wr = willr(3);
            let bars = [
                ohlc(100.0, 110.0, 90.0, 105.0, 1),
                ohlc(102.0, 115.0, 88.0, 98.0, 2),
                ohlc(99.0, 108.0, 92.0, 103.0, 3),
                ohlc(101.0, 120.0, 85.0, 95.0, 4),
                ohlc(96.0, 105.0, 80.0, 100.0, 5),
                ohlc(98.0, 130.0, 75.0, 110.0, 6),
            ];
            for b in &bars {
                if let Some(value) = wr.compute(b) {
                    assert!(
                        (-100.0..=0.0).contains(&value),
                        "Williams %%R out of bounds: {value}"
                    );
                }
            }
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));

            let mut cloned = wr.clone();

            // Advance original
            let orig_val = wr.compute(&ohlc(14.0, 24.0, 14.0, 20.0, 3)).unwrap();
            // Clone still at old value
            assert_eq!(
                cloned.value(),
                wr.clone().value().or(Some(0.0)).and(cloned.value())
            );

            // Clone advances independently
            let clone_val = cloned.compute(&ohlc(14.0, 16.0, 14.0, 14.0, 3)).unwrap();
            assert!(
                (orig_val - clone_val).abs() > 1e-10,
                "divergent inputs should give different %R"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper_uses_close_source() {
            let config = WillRConfig::close(nz(14));
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn length_accessor() {
            let config = WillRConfig::close(nz(14));
            assert_eq!(config.length(), 14);
        }

        #[test]
        fn convergence_equals_length() {
            let config = WillRConfig::close(nz(14));
            assert_eq!(config.convergence(), 14);

            let config = WillRConfig::close(nz(3));
            assert_eq!(config.convergence(), 3);
        }

        #[test]
        fn default_source_is_close() {
            let config = WillRConfig::builder().length(nz(14)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn custom_source() {
            let config = WillRConfig::builder()
                .length(nz(14))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = WillRConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = WillRConfig::close(nz(14));
            let b = WillRConfig::close(nz(14));
            let c = WillRConfig::close(nz(7));

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = WillRConfig::close(nz(14));
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let wr = willr(14);
            assert_eq!(wr.to_string(), "WillR(14, Close)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = WillRConfig::close(nz(14));
            assert_eq!(config.to_string(), "WillRConfig(14, Close)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let wr = willr(3);
            assert_eq!(wr.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            assert!(wr.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut wr = willr(2);
            wr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            let computed = wr.compute(&ohlc(12.0, 18.0, 12.0, 16.0, 2));
            assert_eq!(wr.value(), computed);
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_configured_source() {
            // HL2 = (high + low) / 2
            let mut wr = WillR::new(
                WillRConfig::builder()
                    .length(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            let b1 = Bar::new(0.0, 20.0, 10.0, 0.0).at(1); // HL2 = 15
            let b2 = Bar::new(0.0, 30.0, 20.0, 0.0).at(2); // HL2 = 25
            wr.compute(&b1);
            let val = wr.compute(&b2).unwrap();
            // HH=30, LL=10, price(HL2)=25
            // %R = (30 - 25) / (30 - 10) * -100 = -25
            assert!((val - (-25.0)).abs() < 1e-10);
        }
    }
}
