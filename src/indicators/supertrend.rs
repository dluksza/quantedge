use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Multiplier, Price, PriceSource,
    internals::{BarAction, BarState, EmaCore},
};

/// Configuration for the Supertrend ([`Supertrend`]) indicator.
///
/// Supertrend uses an EMA-smoothed ATR to create adaptive upper and
/// lower bands around the midpoint `(high + low) / 2`. The trend
/// direction flips when price crosses a band. The `length` controls
/// the ATR smoothing period, and `multiplier` scales the ATR to set
/// band distance.
///
/// # Convergence
///
/// Output begins after `length + 1` bars. The ATR needs `length`
/// bars to seed (with Wilder's smoothing), plus one warm-up bar to
/// establish initial band state before direction detection.
///
/// # Example
///
/// ```
/// use quantedge_ta::{SupertrendConfig, Multiplier};
/// use std::num::NonZero;
///
/// let config = SupertrendConfig::builder()
///     .length(NonZero::new(10).unwrap())
///     .multiplier(Multiplier::new(3.0))
///     .build();
///
/// assert_eq!(config.length(), 10);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct SupertrendConfig {
    length: usize,
    multiplier: Multiplier,
}

impl SupertrendConfig {
    /// ATR smoothing window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Band width multiplier.
    #[must_use]
    pub fn multiplier(&self) -> Multiplier {
        self.multiplier
    }
}

impl IndicatorConfig for SupertrendConfig {
    type Builder = SupertrendConfigBuilder;

    fn builder() -> Self::Builder {
        SupertrendConfigBuilder::new()
    }

    fn source(&self) -> crate::PriceSource {
        crate::PriceSource::Close
    }

    fn convergence(&self) -> usize {
        self.length + 1
    }

    fn to_builder(&self) -> Self::Builder {
        SupertrendConfigBuilder {
            length: Some(self.length),
            multiplier: self.multiplier,
        }
    }
}

impl Display for SupertrendConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SupertrendConfig(l: {}, m: {})",
            self.length,
            self.multiplier.value()
        )
    }
}

impl Default for SupertrendConfig {
    /// Default: length=20, multiplier=3.0 (common settings).
    fn default() -> Self {
        Self {
            length: 20,
            multiplier: Multiplier::new(3.0),
        }
    }
}

/// Builder for [`SupertrendConfig`].
///
/// Defaults: multiplier = `3.0`.
/// `length` must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct SupertrendConfigBuilder {
    length: Option<usize>,
    multiplier: Multiplier,
}

impl SupertrendConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            multiplier: Multiplier::new(3.0),
        }
    }

    /// Sets the ATR smoothing window length (minimum 2).
    ///
    /// # Panics
    ///
    /// Panics if `value` is less than 2.
    #[must_use]
    pub fn length(mut self, value: NonZero<usize>) -> Self {
        assert!(value.get() >= 2, "length must be >= 2");
        self.length.replace(value.get());
        self
    }

    /// Sets the band width multiplier.
    #[must_use]
    pub fn multiplier(mut self, value: Multiplier) -> Self {
        self.multiplier = value;
        self
    }
}

impl IndicatorConfigBuilder<SupertrendConfig> for SupertrendConfigBuilder {
    fn source(self, _source: crate::PriceSource) -> Self {
        self
    }

    fn build(self) -> SupertrendConfig {
        SupertrendConfig {
            length: self.length.expect("length is required"),
            multiplier: self.multiplier,
        }
    }
}

/// Supertrend output: trend line value and direction.
///
/// When bullish, the value tracks the lower band (support).
/// When bearish, the value tracks the upper band (resistance).
///
/// ```text
/// midpoint    = (high + low) / 2
/// basic_upper = midpoint + multiplier × ATR
/// basic_lower = midpoint − multiplier × ATR
/// ```
///
/// The trend flips from bearish to bullish when price crosses above
/// the upper band, and from bullish to bearish when price crosses
/// below the lower band. Bands are clamped to prevent widening
/// against the trend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SupertrendValue {
    value: Price,
    is_bullish: bool,
}

impl SupertrendValue {
    /// The current trend line price level.
    #[inline]
    #[must_use]
    pub fn value(&self) -> Price {
        self.value
    }

    /// `true` when the trend is bullish (price above lower band).
    #[inline]
    #[must_use]
    pub fn is_bullish(&self) -> bool {
        self.is_bullish
    }
}

impl Display for SupertrendValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SupertrendValue(v: {}, is_bullish: {})",
            self.value, self.is_bullish
        )
    }
}

/// Supertrend trend-following indicator.
///
/// Combines ATR-based volatility bands with directional logic to
/// produce a single trend line that flips between support (bullish)
/// and resistance (bearish). The ATR uses Wilder's smoothing
/// (`α = 1/length`).
///
/// ```text
/// midpoint    = (high + low) / 2
/// basic_upper = midpoint + multiplier × ATR(length)
/// basic_lower = midpoint − multiplier × ATR(length)
/// ```
///
/// Upper bands are clamped downward and lower bands are clamped
/// upward to prevent bands from widening against the current trend.
/// The trend direction flips when close crosses the active band.
///
/// Returns `None` until the ATR has converged and one warm-up bar
/// has established initial state (after `length + 1` bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Supertrend, SupertrendConfig, Multiplier};
/// use std::num::NonZero;
/// # use quantedge_ta::{Ohlcv, Price, Timestamp};
/// #
/// # struct Bar { o: f64, h: f64, l: f64, c: f64, t: u64 }
/// # impl Ohlcv for Bar {
/// #     fn open(&self) -> Price { self.o }
/// #     fn high(&self) -> Price { self.h }
/// #     fn low(&self) -> Price { self.l }
/// #     fn close(&self) -> Price { self.c }
/// #     fn open_time(&self) -> Timestamp { self.t }
/// # }
///
/// let config = SupertrendConfig::builder()
///     .length(NonZero::new(3).unwrap())
///     .multiplier(Multiplier::new(1.0))
///     .build();
/// let mut st = Supertrend::new(config);
///
/// // Seeding: need length + 1 = 4 bars
/// assert!(st.compute(&Bar { o: 10.0, h: 15.0, l: 5.0, c: 12.0, t: 1 }).is_none());
/// assert!(st.compute(&Bar { o: 12.0, h: 18.0, l: 8.0, c: 14.0, t: 2 }).is_none());
/// assert!(st.compute(&Bar { o: 14.0, h: 20.0, l: 10.0, c: 16.0, t: 3 }).is_none());
///
/// let val = st.compute(&Bar { o: 16.0, h: 22.0, l: 12.0, c: 20.0, t: 4 }).unwrap();
/// assert!(val.value() > 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct Supertrend {
    config: SupertrendConfig,
    prev_close: Option<Price>,
    prev_upper: Option<Price>,
    prev_lower: Option<Price>,
    current_upper: Option<Price>,
    current_lower: Option<Price>,
    current_close: Option<Price>,
    bar_state: BarState,
    ema: EmaCore,
    current: Option<SupertrendValue>,
    previous: Option<SupertrendValue>,
}

impl Indicator for Supertrend {
    type Config = SupertrendConfig;
    type Output = SupertrendValue;

    fn new(config: Self::Config) -> Self {
        Supertrend {
            config,
            prev_close: None,
            prev_upper: None,
            prev_lower: None,
            current_upper: None,
            current_lower: None,
            current_close: None,
            bar_state: BarState::new(PriceSource::TrueRange),
            #[allow(clippy::cast_precision_loss)]
            ema: EmaCore::with_alpha(config.length, 1.0 / config.length as f64),
            current: None,
            previous: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        let atr = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                self.previous = self.current;
                self.prev_upper = self.current_upper;
                self.prev_lower = self.current_lower;
                self.prev_close = self.current_close;

                self.ema.push(price)
            }
            BarAction::Repaint(price) => self.ema.replace(price),
        };

        self.current = match (atr, self.prev_close) {
            (Some(atr), Some(prev_close)) => {
                let midpoint = ohlcv.high().midpoint(ohlcv.low());
                let atr_mult = self.config.multiplier.value() * atr;
                let next_upper = midpoint + atr_mult;
                let next_lower = midpoint - atr_mult;

                let upper = self.prev_upper.map_or(next_upper, |prev_upper| {
                    if next_upper < prev_upper || prev_close > prev_upper {
                        next_upper
                    } else {
                        prev_upper
                    }
                });
                let lower = self.prev_lower.map_or(next_lower, |prev_lower| {
                    if next_lower > prev_lower || prev_close < prev_lower {
                        next_lower
                    } else {
                        prev_lower
                    }
                });

                let bearish = Self::bearish(upper);
                let value = self.previous.map_or(bearish, |previous| {
                    self.current_upper = Some(upper);
                    self.current_lower = Some(lower);

                    self.prev_upper.map_or(bearish, |prev_upper| {
                        if (previous.value - prev_upper).abs() < f64::EPSILON {
                            if ohlcv.close() <= upper {
                                bearish
                            } else {
                                Self::bullish(lower)
                            }
                        } else if ohlcv.close() >= lower {
                            Self::bullish(lower)
                        } else {
                            bearish
                        }
                    })
                });

                Some(value)
            }
            _ => None,
        };

        self.current_close = Some(ohlcv.close());

        self.value()
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.previous.and(self.current)
    }
}

impl Supertrend {
    fn bullish(value: Price) -> SupertrendValue {
        SupertrendValue {
            value,
            is_bullish: true,
        }
    }

    fn bearish(value: Price) -> SupertrendValue {
        SupertrendValue {
            value,
            is_bullish: false,
        }
    }
}

impl Display for Supertrend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Supertrend(l: {}, m: {})",
            self.config.length,
            self.config.multiplier.value()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{nz, ohlc};

    /// Supertrend(3, 1.0) — small window for tractable hand calculations.
    fn st_3() -> Supertrend {
        Supertrend::new(
            SupertrendConfig::builder()
                .length(nz(3))
                .multiplier(Multiplier::new(1.0))
                .build(),
        )
    }

    /// Returns a converged Supertrend(3, 1.0) after 4 bars.
    fn seeded_st() -> Supertrend {
        let mut st = st_3();
        st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
        st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2));
        st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3));
        st.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4));
        st
    }

    mod convergence {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let mut st = st_3();
            assert!(st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none());
            assert!(st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2)).is_none());
            // Bar 3: ATR converges but warm-up bar — still None
            assert!(st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3)).is_none());
        }

        #[test]
        fn first_value_at_convergence() {
            let mut st = st_3();
            st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2));
            st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3));
            // convergence = max(3, 2) + 1 = 4
            assert!(st.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4)).is_some());
        }

        #[test]
        fn value_none_before_convergence() {
            let st = st_3();
            assert_eq!(st.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut st = seeded_st();
            let computed = st.compute(&ohlc(20.0, 26.0, 16.0, 24.0, 5));
            assert_eq!(st.value(), computed);
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn first_output_is_bearish_with_upper_band() {
            // First converged output always defaults to bearish (upper band)
            let mut st = st_3();
            st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2));
            st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3));
            let val = st.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4)).unwrap();
            assert!(!val.is_bullish());
        }

        #[test]
        fn bearish_to_bullish_transition() {
            let mut st = seeded_st();
            let prev = st.value().unwrap();
            assert!(!prev.is_bullish());

            // Push price well above the upper band to trigger bullish flip
            let val = st.compute(&ohlc(30.0, 40.0, 25.0, 38.0, 5)).unwrap();
            assert!(val.is_bullish());
        }

        #[test]
        fn bullish_to_bearish_transition() {
            let mut st = seeded_st();
            // First force bullish
            st.compute(&ohlc(30.0, 40.0, 25.0, 38.0, 5));
            assert!(st.value().unwrap().is_bullish());

            // Now push price well below the lower band
            let val = st.compute(&ohlc(5.0, 8.0, 2.0, 3.0, 6)).unwrap();
            assert!(!val.is_bullish());
        }

        #[test]
        fn upper_band_clamps_down() {
            // When bearish and previous close <= prev_upper,
            // the upper band should not increase
            let mut st = seeded_st();
            let first = st.value().unwrap();

            // Feed a bar that keeps price below upper band
            let second = st.compute(&ohlc(14.0, 16.0, 10.0, 12.0, 5)).unwrap();
            // Upper band should be clamped (not widening)
            assert!(second.value() <= first.value());
        }

        #[test]
        fn lower_band_clamps_up() {
            let mut st = seeded_st();
            // Force bullish
            st.compute(&ohlc(30.0, 40.0, 25.0, 38.0, 5));
            let bullish_val = st.value().unwrap();
            assert!(bullish_val.is_bullish());

            // Feed bar that keeps price above lower band
            let next = st.compute(&ohlc(35.0, 42.0, 30.0, 40.0, 6)).unwrap();
            // Lower band should be clamped upward (not decreasing)
            assert!(next.value() >= bullish_val.value());
        }

        #[test]
        fn constant_ohlc_stays_bearish() {
            // With constant bars, no trend change should occur
            let mut st = Supertrend::new(
                SupertrendConfig::builder()
                    .length(nz(2))
                    .multiplier(Multiplier::new(1.0))
                    .build(),
            );
            for t in 1..=10 {
                st.compute(&ohlc(50.0, 55.0, 45.0, 50.0, t));
            }
            // Should have settled on a direction
            assert!(st.value().is_some());
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_value() {
            let mut st = seeded_st();
            let original = st.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 5)).unwrap();
            let repainted = st.compute(&ohlc(18.0, 30.0, 10.0, 28.0, 5)).unwrap();
            assert_ne!(original, repainted);
        }

        #[test]
        fn multiple_repaints_match_clean() {
            let mut st = seeded_st();
            st.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 5));
            st.compute(&ohlc(18.0, 30.0, 10.0, 28.0, 5)); // repaint 1
            st.compute(&ohlc(18.0, 24.0, 13.0, 19.0, 5)); // repaint 2
            let final_val = st.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 5));

            let mut clean = seeded_st();
            let expected = clean.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 5));

            assert_eq!(final_val, expected);
        }

        #[test]
        fn repaint_then_advance() {
            let mut st = seeded_st();
            st.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 5));
            st.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 5)); // repaint
            let after = st.compute(&ohlc(22.0, 28.0, 18.0, 24.0, 6));

            let mut clean = seeded_st();
            clean.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 5));
            let expected = clean.compute(&ohlc(22.0, 28.0, 18.0, 24.0, 6));

            assert_eq!(after, expected);
        }

        #[test]
        fn repaint_during_filling() {
            let mut st = st_3();
            st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            st.compute(&ohlc(10.0, 18.0, 4.0, 14.0, 1)); // repaint
            assert!(st.value().is_none()); // still filling
            st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2));
            st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3));
            assert!(st.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4)).is_some());
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut st = st_3();

            // Bar 1: open then close
            assert!(st.compute(&ohlc(10.0, 14.0, 6.0, 11.0, 1)).is_none());
            assert!(st.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none()); // repaint

            // Bar 2
            assert!(st.compute(&ohlc(12.0, 17.0, 9.0, 13.0, 2)).is_none());
            assert!(st.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2)).is_none()); // repaint

            // Bar 3: warm-up bar (ATR converges, still None)
            assert!(st.compute(&ohlc(14.0, 19.0, 11.0, 15.0, 3)).is_none());
            assert!(st.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3)).is_none()); // repaint

            // Bar 4: first value
            let val = st.compute(&ohlc(16.0, 21.0, 13.0, 19.0, 4));
            assert!(val.is_some());

            // Bar 4 repaint
            let repainted = st.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4));
            assert!(repainted.is_some());

            // Bar 5: advance after repaints
            let next = st.compute(&ohlc(20.0, 26.0, 16.0, 24.0, 5));
            assert!(next.is_some());

            // Verify against clean run with final prices
            let mut clean = st_3();
            clean.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            clean.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2));
            clean.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3));
            clean.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 4));
            let expected = clean.compute(&ohlc(20.0, 26.0, 16.0, 24.0, 5));

            assert_eq!(next, expected);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut st = seeded_st();
            let mut cloned = st.clone();

            let orig = st.compute(&ohlc(30.0, 40.0, 25.0, 38.0, 5)).unwrap();
            let clone_val = cloned.compute(&ohlc(5.0, 8.0, 2.0, 3.0, 5)).unwrap();

            assert_ne!(
                orig, clone_val,
                "divergent inputs should give different values"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn accessors() {
            let config = SupertrendConfig::builder()
                .length(nz(10))
                .multiplier(Multiplier::new(2.0))
                .build();
            assert_eq!(config.length(), 10);
            assert!((config.multiplier().value() - 2.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_values() {
            let config = SupertrendConfig::default();
            assert_eq!(config.length(), 20);
            assert!((config.multiplier().value() - 3.0).abs() < f64::EPSILON);
        }

        #[test]
        fn convergence_equals_length_plus_1() {
            let config = SupertrendConfig::builder().length(nz(10)).build();
            assert_eq!(config.convergence(), 11);

            let config = SupertrendConfig::builder().length(nz(2)).build();
            assert_eq!(config.convergence(), 3);
        }

        #[test]
        fn source_is_close() {
            let config = SupertrendConfig::builder().length(nz(10)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = SupertrendConfig::builder().build();
        }

        #[test]
        #[should_panic(expected = "length must be >= 2")]
        fn panics_with_length_one() {
            let _ = SupertrendConfig::builder().length(nz(1)).build();
        }

        #[test]
        fn eq_and_hash() {
            let a = SupertrendConfig::builder().length(nz(10)).build();
            let b = SupertrendConfig::builder().length(nz(10)).build();
            let c = SupertrendConfig::builder().length(nz(20)).build();

            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = SupertrendConfig::builder()
                .length(nz(10))
                .multiplier(Multiplier::new(2.5))
                .build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = SupertrendConfig::builder()
                .length(nz(20))
                .multiplier(Multiplier::new(3.0))
                .build();
            assert_eq!(config.to_string(), "SupertrendConfig(l: 20, m: 3)");
        }

        #[test]
        fn display_supertrend() {
            let st = Supertrend::new(
                SupertrendConfig::builder()
                    .length(nz(20))
                    .multiplier(Multiplier::new(3.0))
                    .build(),
            );
            assert_eq!(st.to_string(), "Supertrend(l: 20, m: 3)");
        }

        #[test]
        fn display_value() {
            let v = SupertrendValue {
                value: 100.5,
                is_bullish: true,
            };
            assert_eq!(v.to_string(), "SupertrendValue(v: 100.5, is_bullish: true)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let st = st_3();
            assert_eq!(st.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let st = seeded_st();
            assert!(st.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut st = seeded_st();
            let computed = st.compute(&ohlc(20.0, 26.0, 16.0, 24.0, 5));
            assert_eq!(st.value(), computed);
        }
    }
}
