use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price,
    internals::{BarState, EmaCore},
};

/// Configuration for the Average Directional Index ([`Adx`]) indicator.
///
/// ADX uses Wilder's smoothing (`α = 1 / length`) for both the
/// directional movement lines (+DI / −DI) and the final ADX value.
/// The price source is always
/// [`PriceSource::Close`](crate::PriceSource::Close) (ignored by the
/// builder). Output begins after `length × 2` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::AdxConfig;
/// use std::num::NonZero;
///
/// let config = AdxConfig::builder()
///     .length(NonZero::new(14).unwrap())
///     .build();
///
/// assert_eq!(config.length(), 14);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct AdxConfig {
    length: usize,
}

impl IndicatorConfig for AdxConfig {
    type Builder = AdxConfigBuilder;

    #[inline]
    fn builder() -> Self::Builder {
        AdxConfigBuilder::new()
    }

    #[inline]
    fn source(&self) -> crate::PriceSource {
        crate::PriceSource::Close
    }

    #[inline]
    fn convergence(&self) -> usize {
        self.length * 2
    }
}

impl AdxConfig {
    /// Window length (number of bars).
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }
}

impl Display for AdxConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdxConfig(l: {})", self.length)
    }
}

/// Builder for [`AdxConfig`].
///
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build). The price source is
/// always [`PriceSource::Close`](crate::PriceSource::Close) and
/// cannot be overridden.
pub struct AdxConfigBuilder {
    length: Option<usize>,
}

impl AdxConfigBuilder {
    fn new() -> Self {
        AdxConfigBuilder { length: None }
    }

    #[inline]
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }
}

impl IndicatorConfigBuilder<AdxConfig> for AdxConfigBuilder {
    #[inline]
    fn source(self, _source: crate::PriceSource) -> Self {
        self
    }

    #[inline]
    fn build(self) -> AdxConfig {
        AdxConfig {
            length: self.length.expect("length is required"),
        }
    }
}

/// Average Directional Index output: ADX, +DI, and −DI.
///
/// ADX measures trend strength on a 0–100 scale. +DI and −DI
/// indicate the direction of the trend. All three values are
/// smoothed with Wilder's method.
///
/// ```text
/// +DM = max(high − prev_high, 0) if > −DM, else 0
/// −DM = max(prev_low − low, 0)   if > +DM, else 0
/// +DI = smoothed(+DM) / smoothed(TR) × 100
/// −DI = smoothed(−DM) / smoothed(TR) × 100
/// DX  = |+DI − −DI| / (+DI + −DI) × 100
/// ADX = smoothed(DX)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdxValue {
    adx: Price,
    plus_di: Price,
    minus_di: Price,
}

impl AdxValue {
    #[must_use]
    #[inline]
    pub fn adx(&self) -> Price {
        self.adx
    }

    #[must_use]
    #[inline]
    pub fn plus_di(&self) -> Price {
        self.plus_di
    }

    #[must_use]
    #[inline]
    pub fn minus_di(&self) -> Price {
        self.minus_di
    }
}

impl Display for AdxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdxValue(adx: {}, +di: {}, -di: {})",
            self.adx, self.plus_di, self.minus_di
        )
    }
}

/// Average Directional Index (ADX).
///
/// Measures trend strength regardless of direction on a 0–100
/// scale. Values above 25 conventionally indicate a trending
/// market; below 20, a ranging market.
///
/// Internally computes +DI and −DI from smoothed directional
/// movement (+DM / −DM) relative to smoothed True Range, then
/// smooths the DX (directional index) to produce ADX. All
/// smoothing uses Wilder's method (`α = 1 / length`).
///
/// The first `length` bars build the smoothing seeds. A second
/// `length` bars are needed to seed the ADX smoother. Output
/// begins after `length × 2` bars.
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Adx, AdxConfig};
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
/// let config = AdxConfig::builder()
///     .length(NonZero::new(3).unwrap())
///     .build();
/// let mut adx = Adx::new(config);
///
/// // Seeding phase (convergence = length × 2 = 6 bars)
/// assert!(adx.compute(&Bar { o: 10.0, h: 15.0, l: 8.0, c: 12.0, t: 1 }).is_none());
/// assert!(adx.compute(&Bar { o: 12.0, h: 18.0, l: 10.0, c: 16.0, t: 2 }).is_none());
/// assert!(adx.compute(&Bar { o: 16.0, h: 20.0, l: 14.0, c: 18.0, t: 3 }).is_none());
/// assert!(adx.compute(&Bar { o: 18.0, h: 22.0, l: 15.0, c: 20.0, t: 4 }).is_none());
/// assert!(adx.compute(&Bar { o: 20.0, h: 25.0, l: 17.0, c: 23.0, t: 5 }).is_none());
///
/// // ADX output begins
/// let value = adx.compute(&Bar { o: 23.0, h: 28.0, l: 19.0, c: 26.0, t: 6 });
/// assert!(value.is_some());
/// ```
#[derive(Clone, Debug)]
pub struct Adx {
    config: AdxConfig,
    bar_state: BarState,
    prev_high: Option<Price>,
    prev_low: Option<Price>,
    current_high: Price,
    current_low: Price,
    dm_started: bool,
    smoothed_plus_dm: EmaCore, // Wilder's α = 1/length
    smoothed_minus_dm: EmaCore,
    smoothed_tr: EmaCore,
    smoother: EmaCore,
    current: Option<AdxValue>,
}

impl Indicator for Adx {
    type Config = AdxConfig;
    type Output = AdxValue;

    fn new(config: Self::Config) -> Self {
        let length = config.length;
        #[allow(clippy::cast_precision_loss)]
        let alpha = 1.0 / length as f64;

        Adx {
            config,
            bar_state: BarState::new(crate::PriceSource::TrueRange),
            prev_high: None,
            prev_low: None,
            current_high: 0.0,
            current_low: 0.0,
            dm_started: false,
            smoothed_plus_dm: EmaCore::with_alpha(length, alpha),
            smoothed_minus_dm: EmaCore::with_alpha(length, alpha),
            smoothed_tr: EmaCore::with_alpha(length, alpha),
            smoother: EmaCore::with_alpha(length, alpha),
            current: None,
        }
    }

    #[inline]
    #[allow(clippy::similar_names)]
    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        self.current = match self.bar_state.handle(ohlcv) {
            crate::internals::BarAction::Advance(true_range) => {
                let smooth_tr = self.smoothed_tr.push(true_range);

                if self.prev_high.is_some() {
                    let (plus_dm, minus_dm) = Adx::dm(ohlcv, self.current_high, self.current_low);

                    self.prev_high = Some(self.current_high);
                    self.prev_low = Some(self.current_low);
                    self.dm_started = true;

                    let smooth_plus_dm = self.smoothed_plus_dm.push(plus_dm);
                    let smooth_minus_dm = self.smoothed_minus_dm.push(minus_dm);

                    match (smooth_plus_dm, smooth_minus_dm) {
                        (Some(smooth_plus_dm), Some(smooth_minus_dm)) => {
                            let (plus_di, minus_di, dx) = Adx::compute_adx(
                                smooth_tr.unwrap_or(0.0),
                                smooth_plus_dm,
                                smooth_minus_dm,
                            );

                            self.smoother.push(dx).map(|adx| AdxValue {
                                adx,
                                plus_di,
                                minus_di,
                            })
                        }
                        _ => None,
                    }
                } else {
                    self.prev_high = Some(self.current_high);
                    self.prev_low = Some(self.current_low);
                    None
                }
            }
            crate::internals::BarAction::Repaint(true_range) => {
                let smooth_tr = self.smoothed_tr.replace(true_range);

                if self.dm_started {
                    match (self.prev_high, self.prev_low) {
                        (Some(prev_high), Some(prev_low)) => {
                            let (plus_dm, minus_dm) = Adx::dm(ohlcv, prev_high, prev_low);

                            let smooth_plus_dm = self.smoothed_plus_dm.replace(plus_dm);
                            let smooth_minus_dm = self.smoothed_minus_dm.replace(minus_dm);

                            match (smooth_plus_dm, smooth_minus_dm) {
                                (Some(smooth_plus_dm), Some(smooth_minus_dm)) => {
                                    let (plus_di, minus_di, dx) = Adx::compute_adx(
                                        smooth_tr.unwrap_or(0.0),
                                        smooth_plus_dm,
                                        smooth_minus_dm,
                                    );

                                    self.smoother.replace(dx).map(|adx| AdxValue {
                                        adx,
                                        plus_di,
                                        minus_di,
                                    })
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
        };

        self.current_high = ohlcv.high();
        self.current_low = ohlcv.low();

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Adx {
    #[inline]
    fn d_move(a: Price, b: Price) -> Price {
        if a > b && a > 0.0 { a } else { 0.0 }
    }

    #[inline]
    fn dm(ohlcv: &impl Ohlcv, prev_high: Price, prev_low: Price) -> (Price, Price) {
        let move_up = ohlcv.high() - prev_high;
        let move_down = prev_low - ohlcv.low();

        let plus_dm = Adx::d_move(move_up, move_down);
        let minus_dm = Adx::d_move(move_down, move_up);

        (plus_dm, minus_dm)
    }

    #[inline]
    fn compute_adx(
        smooth_tr: Price,
        smooth_plus_dm: Price,
        smooth_minus_dm: Price,
    ) -> (Price, Price, Price) {
        if smooth_tr < f64::EPSILON {
            return (0.0, 0.0, 0.0); // plus_di, minus_di, dx
        }

        let inv_tr_100 = 100.0 / smooth_tr;
        let plus_di = smooth_plus_dm * inv_tr_100;
        let minus_di = smooth_minus_dm * inv_tr_100;
        let denom = plus_di + minus_di;

        let dx = if denom.abs() < f64::EPSILON {
            0.0
        } else {
            (plus_di - minus_di).abs() / denom * 100.0
        };

        (plus_di, minus_di, dx)
    }
}

impl Display for Adx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Adx(l: {})", self.config.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{nz, ohlc};

    fn adx(length: usize) -> Adx {
        Adx::new(AdxConfig::builder().length(nz(length)).build())
    }

    /// Returns a converged ADX(3) after 7 trending-up bars.
    fn seeded_adx3() -> Adx {
        let mut a = adx(3);
        a.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
        a.compute(&ohlc(12.0, 18.0, 10.0, 16.0, 2));
        a.compute(&ohlc(16.0, 20.0, 14.0, 18.0, 3));
        a.compute(&ohlc(18.0, 22.0, 15.0, 20.0, 4));
        a.compute(&ohlc(20.0, 25.0, 17.0, 23.0, 5));
        a.compute(&ohlc(23.0, 28.0, 19.0, 26.0, 6));
        a.compute(&ohlc(26.0, 31.0, 22.0, 29.0, 7));
        a
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_none_during_seeding() {
            let mut a = adx(3);
            assert!(a.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1)).is_none());
            assert!(a.compute(&ohlc(12.0, 18.0, 10.0, 16.0, 2)).is_none());
            assert!(a.compute(&ohlc(16.0, 20.0, 14.0, 18.0, 3)).is_none());
            assert!(a.compute(&ohlc(18.0, 22.0, 15.0, 20.0, 4)).is_none());
            assert!(a.compute(&ohlc(20.0, 25.0, 17.0, 23.0, 5)).is_none());
        }

        #[test]
        fn first_value_after_seeding() {
            let a = seeded_adx3();
            assert!(a.value().is_some());
        }

        #[test]
        fn value_is_none_before_convergence() {
            let a = adx(14);
            assert_eq!(a.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut a = seeded_adx3();
            let computed = a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8));
            assert_eq!(a.value(), computed);
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn uptrend_has_higher_plus_di() {
            // Strong uptrend: each bar higher than previous
            let a = seeded_adx3();
            let val = a.value().unwrap();
            assert!(
                val.plus_di() > val.minus_di(),
                "+DI should exceed -DI in uptrend: +DI={}, -DI={}",
                val.plus_di(),
                val.minus_di()
            );
        }

        #[test]
        fn downtrend_has_higher_minus_di() {
            let mut a = adx(3);
            // Downtrend: each bar lower
            a.compute(&ohlc(30.0, 35.0, 28.0, 32.0, 1));
            a.compute(&ohlc(32.0, 33.0, 25.0, 27.0, 2));
            a.compute(&ohlc(27.0, 28.0, 20.0, 22.0, 3));
            a.compute(&ohlc(22.0, 23.0, 15.0, 17.0, 4));
            a.compute(&ohlc(17.0, 18.0, 10.0, 12.0, 5));
            a.compute(&ohlc(12.0, 13.0, 6.0, 8.0, 6));
            let val = a.compute(&ohlc(8.0, 9.0, 3.0, 5.0, 7)).unwrap();
            assert!(
                val.minus_di() > val.plus_di(),
                "-DI should exceed +DI in downtrend: -DI={}, +DI={}",
                val.minus_di(),
                val.plus_di()
            );
        }

        #[test]
        fn adx_positive_in_trend() {
            let a = seeded_adx3();
            let val = a.value().unwrap();
            assert!(val.adx() > 0.0, "ADX should be positive in a trend");
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn adx_between_0_and_100() {
            let mut a = adx(3);
            let bars = [
                ohlc(10.0, 15.0, 5.0, 12.0, 1),
                ohlc(12.0, 20.0, 8.0, 18.0, 2),
                ohlc(18.0, 22.0, 6.0, 7.0, 3),
                ohlc(7.0, 25.0, 4.0, 24.0, 4),
                ohlc(24.0, 30.0, 3.0, 5.0, 5),
                ohlc(5.0, 10.0, 2.0, 9.0, 6),
                ohlc(9.0, 50.0, 1.0, 45.0, 7),
                ohlc(45.0, 48.0, 40.0, 42.0, 8),
                ohlc(42.0, 55.0, 38.0, 50.0, 9),
            ];
            for b in &bars {
                if let Some(v) = a.compute(b) {
                    assert!(
                        (0.0..=100.0).contains(&v.adx()),
                        "ADX out of bounds: {}",
                        v.adx()
                    );
                    assert!(v.plus_di() >= 0.0, "+DI negative: {}", v.plus_di());
                    assert!(v.minus_di() >= 0.0, "-DI negative: {}", v.minus_di());
                }
            }
        }
    }

    mod flat_market {
        use super::*;

        #[test]
        fn flat_bars_give_zero_di() {
            let mut a = adx(3);
            for t in 1..=10 {
                a.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
            }
            let val = a.value().unwrap();
            assert!(
                val.plus_di().abs() < 1e-10,
                "flat market +DI should be 0, got {}",
                val.plus_di()
            );
            assert!(
                val.minus_di().abs() < 1e-10,
                "flat market -DI should be 0, got {}",
                val.minus_di()
            );
        }
    }

    mod repaints {
        use super::*;

        #[test]
        fn repaint_updates_value() {
            let mut a = seeded_adx3();
            let original = a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8)).unwrap();
            // Repaint with much higher high — +DI should increase
            let repainted = a.compute(&ohlc(29.0, 46.0, 25.0, 44.0, 8)).unwrap();
            assert!(
                repainted.plus_di() > original.plus_di(),
                "higher high should increase +DI: {} vs {}",
                repainted.plus_di(),
                original.plus_di()
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut a = seeded_adx3();
            a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8));
            a.compute(&ohlc(29.0, 41.0, 21.0, 36.0, 8)); // repaint 1
            a.compute(&ohlc(29.0, 36.0, 24.0, 31.0, 8)); // repaint 2
            let final_val = a.compute(&ohlc(29.0, 33.0, 26.0, 30.0, 8)).unwrap();

            let mut clean = seeded_adx3();
            let expected = clean.compute(&ohlc(29.0, 33.0, 26.0, 30.0, 8)).unwrap();

            assert!((final_val.adx() - expected.adx()).abs() < 1e-10);
            assert!((final_val.plus_di() - expected.plus_di()).abs() < 1e-10);
            assert!((final_val.minus_di() - expected.minus_di()).abs() < 1e-10);
        }

        #[test]
        fn repaint_then_advance_uses_repainted() {
            let mut a = seeded_adx3();
            a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8));
            a.compute(&ohlc(29.0, 36.0, 24.0, 34.0, 8)); // repaint bar 8
            let after = a.compute(&ohlc(34.0, 39.0, 30.0, 37.0, 9)).unwrap();

            let mut clean = seeded_adx3();
            clean.compute(&ohlc(29.0, 36.0, 24.0, 34.0, 8));
            let expected = clean.compute(&ohlc(34.0, 39.0, 30.0, 37.0, 9)).unwrap();

            assert!((after.adx() - expected.adx()).abs() < 1e-10);
            assert!((after.plus_di() - expected.plus_di()).abs() < 1e-10);
            assert!((after.minus_di() - expected.minus_di()).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling_stays_none() {
            let mut a = adx(3);
            a.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
            a.compute(&ohlc(12.0, 18.0, 10.0, 16.0, 2));
            a.compute(&ohlc(16.0, 22.0, 12.0, 20.0, 2)); // repaint bar 2
            assert!(a.value().is_none());
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut a = seeded_adx3();
            let mut cloned = a.clone();

            let orig = a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8)).unwrap();
            let clone_val = cloned.compute(&ohlc(29.0, 21.0, 11.0, 14.0, 8)).unwrap();

            assert!(
                (orig.adx() - clone_val.adx()).abs() > 1e-10,
                "divergent inputs should give different ADX"
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_adx() {
            let a = adx(14);
            assert_eq!(a.to_string(), "Adx(l: 14)");
        }

        #[test]
        fn formats_config() {
            let config = AdxConfig::builder().length(nz(14)).build();
            assert_eq!(config.to_string(), "AdxConfig(l: 14)");
        }

        #[test]
        fn formats_value() {
            let v = AdxValue {
                adx: 25.5,
                plus_di: 30.0,
                minus_di: 15.0,
            };
            assert_eq!(v.to_string(), "AdxValue(adx: 25.5, +di: 30, -di: 15)");
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn length_accessor() {
            let config = AdxConfig::builder().length(nz(14)).build();
            assert_eq!(config.length(), 14);
        }

        #[test]
        fn convergence_is_double_length() {
            let config = AdxConfig::builder().length(nz(14)).build();
            assert_eq!(config.convergence(), 28);

            let config = AdxConfig::builder().length(nz(3)).build();
            assert_eq!(config.convergence(), 6);
        }

        #[test]
        fn source_is_close() {
            let config = AdxConfig::builder().length(nz(14)).build();
            assert_eq!(config.source(), crate::PriceSource::Close);
        }

        #[test]
        fn source_builder_is_noop() {
            let a = AdxConfig::builder().length(nz(14)).build();
            let b = AdxConfig::builder()
                .length(nz(14))
                .source(crate::PriceSource::HL2)
                .build();
            assert_eq!(a, b);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = AdxConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = AdxConfig::builder().length(nz(14)).build();
            let b = AdxConfig::builder().length(nz(14)).build();
            let c = AdxConfig::builder().length(nz(7)).build();

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let a = adx(3);
            assert_eq!(a.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let a = seeded_adx3();
            assert!(a.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut a = seeded_adx3();
            let computed = a.compute(&ohlc(29.0, 34.0, 25.0, 32.0, 8));
            assert_eq!(a.value(), computed);
        }

        #[test]
        fn accessors_return_components() {
            let a = seeded_adx3();
            let val = a.value().unwrap();
            assert!(val.adx().is_finite());
            assert!(val.plus_di().is_finite());
            assert!(val.minus_di().is_finite());
        }
    }
}
