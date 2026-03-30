use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Price, Timestamp,
    internals::RollingExtremes,
};

/// Configuration for the Donchian Channel ([`Dc`]) indicator.
///
/// The Donchian Channel tracks the highest high and lowest low over a
/// rolling window of `length` bars. The source is always the OHLC
/// high/low extremes — the `source` field is fixed to
/// [`PriceSource::Close`] and has no effect on computation.
///
/// # Example
///
/// ```
/// use quantedge_ta::DcConfig;
///
/// let config = DcConfig::builder().build();
/// assert_eq!(config.convergence(), 20);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct DcConfig {
    length: usize,
}

impl IndicatorConfig for DcConfig {
    type Builder = DcConfigBuilder;

    fn builder() -> Self::Builder {
        DcConfigBuilder::new()
    }

    fn source(&self) -> crate::PriceSource {
        crate::PriceSource::Close
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        DcConfigBuilder {
            length: self.length,
        }
    }
}

impl DcConfig {
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }
}

impl Default for DcConfig {
    /// Default: length=20 (Donchian's original, `TradingView` default).
    fn default() -> Self {
        Self { length: 20 }
    }
}

impl Display for DcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DcConfig(l: {})", self.length)
    }
}

/// Builder for [`DcConfig`].
///
/// Defaults: length = 20.
pub struct DcConfigBuilder {
    length: usize,
}

impl DcConfigBuilder {
    fn new() -> Self {
        DcConfigBuilder { length: 20 }
    }

    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length = length.get();
        self
    }
}

impl IndicatorConfigBuilder<DcConfig> for DcConfigBuilder {
    fn source(self, _source: crate::PriceSource) -> Self {
        self
    }

    fn build(self) -> DcConfig {
        DcConfig {
            length: self.length,
        }
    }
}

/// Donchian Channel output: upper, middle, and lower bands.
///
/// ```text
/// upper  = highest high over the lookback window
/// lower  = lowest low over the lookback window
/// middle = (upper + lower) / 2
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcValue {
    upper: Price,
    middle: Price,
    lower: Price,
}

impl DcValue {
    /// Upper band (highest high).
    #[inline]
    #[must_use]
    pub fn upper(&self) -> Price {
        self.upper
    }

    /// Middle band: `(upper + lower) / 2`.
    #[inline]
    #[must_use]
    pub fn middle(&self) -> Price {
        self.middle
    }

    /// Lower band (lowest low).
    #[inline]
    #[must_use]
    pub fn lower(&self) -> Price {
        self.lower
    }
}

impl Display for DcValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DcValue(u: {}, m: {}, l: {})",
            self.upper, self.middle, self.lower
        )
    }
}

/// Donchian Channel (DC).
///
/// Tracks the highest high and lowest low over a rolling lookback
/// window, forming an upper and lower channel. The middle line is the
/// average of the two extremes.
///
/// ```text
/// upper  = max(high, length bars)
/// lower  = min(low, length bars)
/// middle = (upper + lower) / 2
/// ```
///
/// Returns `None` until the lookback window is full (`length` bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing the window.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Dc, DcConfig};
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
/// let mut dc = Dc::new(DcConfig::builder().build());
/// // Returns None until the lookback window (default 20) is full.
/// assert!(dc.compute(&Bar { o: 10.0, h: 12.0, l: 8.0, c: 11.0, t: 1 }).is_none());
/// ```
#[derive(Clone, Debug)]
pub struct Dc {
    config: DcConfig,
    extremes: RollingExtremes,
    current: Option<DcValue>,
    last_open_time: Option<Timestamp>,
}

impl Indicator for Dc {
    type Config = DcConfig;
    type Output = DcValue;

    fn new(config: Self::Config) -> Self {
        Dc {
            config,
            extremes: RollingExtremes::new(config.length),
            current: None,
            last_open_time: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        debug_assert!(
            self.last_open_time.is_none_or(|t| t <= ohlcv.open_time()),
            "open_time must be non-decreasing: last={}, got={}",
            self.last_open_time.unwrap_or(0),
            ohlcv.open_time(),
        );

        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time());

        let extremes = if is_next_bar {
            self.last_open_time = Some(ohlcv.open_time());
            self.extremes.push(ohlcv)
        } else {
            self.extremes.replace(ohlcv)
        };

        self.current = extremes.map(|(highest_high, lowest_low)| DcValue {
            upper: highest_high,
            middle: (highest_high + lowest_low) * 0.5,
            lower: lowest_low,
        });

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Dc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Dc(l: {})", self.config.length)
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::test_util::{nz, ohlc};

    fn dc(length: usize) -> Dc {
        Dc::new(DcConfig::builder().length(nz(length)).build())
    }

    /// Returns a converged Dc(3) after 3 bars.
    /// Bars: h/l = (12,8), (14,9), (13,10) at times 1–3.
    /// Upper=14, lower=8, middle=11.
    fn seeded_dc() -> Dc {
        let mut d = dc(3);
        d.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
        d.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
        d.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
        d
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_none_during_filling() {
            let mut d = dc(3);
            assert!(d.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1)).is_none());
            assert!(d.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2)).is_none());
        }

        #[test]
        fn first_value_at_length_bars() {
            let mut d = dc(3);
            d.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            d.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            let val = d.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            assert!(val.is_some());
        }

        #[test]
        fn length_one_converges_immediately() {
            let mut d = dc(1);
            let val = d.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            assert!(val.is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn upper_lower_middle_math() {
            // Window bars 1–3: highest_high=14, lowest_low=8
            let d = seeded_dc();
            let val = d.value().unwrap();
            assert_eq!(val.upper(), 14.0);
            assert_eq!(val.lower(), 8.0);
            assert_eq!(val.middle(), 11.0);
        }

        #[test]
        fn sliding_window_drops_oldest() {
            let mut d = seeded_dc();
            // Bar 4: h=15, l=11. Window is now bars 2–4.
            // highest_high = max(14, 13, 15) = 15
            // lowest_low = min(9, 10, 11) = 9
            let val = d.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4)).unwrap();
            assert_eq!(val.upper(), 15.0);
            assert_eq!(val.lower(), 9.0);
            assert_eq!(val.middle(), 12.0);
        }

        #[test]
        fn slides_across_many_bars() {
            let mut d = dc(2);
            d.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            d.compute(&ohlc(10.0, 18.0, 8.0, 12.0, 2));
            d.compute(&ohlc(10.0, 16.0, 10.0, 14.0, 3));
            // Window bars 3–4: h=16,12 → 16, l=10,7 → 7
            let val = d.compute(&ohlc(10.0, 12.0, 7.0, 10.0, 4)).unwrap();
            assert_eq!(val.upper(), 16.0);
            assert_eq!(val.lower(), 7.0);
            assert_eq!(val.middle(), 11.5);
        }

        #[test]
        fn flat_market() {
            let mut d = dc(3);
            for t in 1..=5 {
                let val = d.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
                if let Some(v) = val {
                    assert_eq!(v.upper(), 10.0);
                    assert_eq!(v.lower(), 10.0);
                    assert_eq!(v.middle(), 10.0);
                }
            }
        }
    }

    mod repaints {
        use super::*;

        #[test]
        fn repaint_updates_value() {
            let mut d = seeded_dc();
            let original = d.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 4)).unwrap();
            // Repaint bar 4 with higher high
            let repainted = d.compute(&ohlc(12.0, 20.0, 11.0, 13.0, 4)).unwrap();
            assert!(repainted.upper() > original.upper());
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut d = seeded_dc();
            d.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 4));
            d.compute(&ohlc(12.0, 18.0, 10.0, 15.0, 4)); // repaint 1 (h up, l down)
            d.compute(&ohlc(12.0, 19.0, 9.0, 14.0, 4)); // repaint 2 (h up, l down)
            let final_val = d.compute(&ohlc(12.0, 20.0, 8.0, 12.0, 4)).unwrap();

            let mut clean = seeded_dc();
            let expected = clean.compute(&ohlc(12.0, 20.0, 8.0, 12.0, 4)).unwrap();

            assert_eq!(final_val.upper(), expected.upper());
            assert_eq!(final_val.lower(), expected.lower());
            assert_eq!(final_val.middle(), expected.middle());
        }

        #[test]
        fn repaint_then_advance_uses_repainted() {
            let mut d = seeded_dc();
            d.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 4));
            d.compute(&ohlc(12.0, 16.0, 7.0, 13.0, 4)); // repaint bar 4
            let after = d.compute(&ohlc(14.0, 17.0, 12.0, 14.0, 5)).unwrap();

            let mut clean = seeded_dc();
            clean.compute(&ohlc(12.0, 16.0, 7.0, 13.0, 4));
            let expected = clean.compute(&ohlc(14.0, 17.0, 12.0, 14.0, 5)).unwrap();

            assert_eq!(after.upper(), expected.upper());
            assert_eq!(after.lower(), expected.lower());
            assert_eq!(after.middle(), expected.middle());
        }

        #[test]
        fn repaint_during_filling_has_no_effect_on_convergence() {
            let mut d = dc(3);
            d.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            d.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            d.compute(&ohlc(11.0, 16.0, 7.0, 15.0, 2)); // repaint bar 2
            assert!(d.value().is_none()); // still filling
            let val = d.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            assert!(val.is_some()); // now converged
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut d = seeded_dc();
            let mut cloned = d.clone();

            let orig = d.compute(&ohlc(12.0, 20.0, 11.0, 16.0, 4)).unwrap();
            let clone_val = cloned.compute(&ohlc(12.0, 13.0, 7.0, 9.0, 4)).unwrap();

            assert!(
                (orig.upper() - clone_val.upper()).abs() > 1e-10,
                "divergent inputs should give different upper"
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = DcConfig::builder().length(nz(20)).build();
            assert_eq!(config.to_string(), "DcConfig(l: 20)");
        }

        #[test]
        fn display_dc() {
            let d = dc(14);
            assert_eq!(d.to_string(), "Dc(l: 14)");
        }

        #[test]
        fn display_value() {
            let v = DcValue {
                upper: 100.0,
                middle: 75.0,
                lower: 50.0,
            };
            assert_eq!(v.to_string(), "DcValue(u: 100, m: 75, l: 50)");
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_length_is_20() {
            let config = DcConfig::builder().build();
            assert_eq!(config.length(), 20);
        }

        #[test]
        fn custom_length() {
            let config = DcConfig::builder().length(nz(50)).build();
            assert_eq!(config.length(), 50);
        }

        #[test]
        fn convergence_equals_length() {
            let config = DcConfig::builder().length(nz(14)).build();
            assert_eq!(config.convergence(), 14);

            let config = DcConfig::builder().build();
            assert_eq!(config.convergence(), 20);
        }

        #[test]
        fn source_is_always_close() {
            let config = DcConfig::builder().source(crate::PriceSource::HL2).build();
            assert_eq!(config.source(), crate::PriceSource::Close);
        }

        #[test]
        fn eq_and_hash() {
            let a = DcConfig::builder().length(nz(20)).build();
            let b = DcConfig::builder().length(nz(20)).build();
            let c = DcConfig::builder().length(nz(10)).build();
            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = DcConfig::builder().length(nz(20)).build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let d = dc(3);
            assert_eq!(d.value(), None);
        }

        #[test]
        fn matches_last_compute() {
            let mut d = seeded_dc();
            let computed = d.compute(&ohlc(12.0, 16.0, 11.0, 14.0, 4));
            assert_eq!(d.value(), computed);
        }

        #[test]
        fn accessors_return_correct_values() {
            let d = seeded_dc();
            let val = d.value().unwrap();
            assert!(val.upper().is_finite());
            assert!(val.lower().is_finite());
            assert!(val.middle().is_finite());
            assert!(val.upper() >= val.lower());
            assert!(val.middle() >= val.lower());
            assert!(val.middle() <= val.upper());
        }
    }

    #[cfg(debug_assertions)]
    mod invariants {
        use super::*;

        #[test]
        #[should_panic(expected = "open_time must be non-decreasing")]
        fn panics_on_decreasing_open_time() {
            let mut d = dc(3);
            d.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 2));
            d.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 1));
        }
    }
}
