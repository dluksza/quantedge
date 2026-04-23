use std::{fmt::Display, hash::Hash, num::NonZero};

use crate::{
    Atr, AtrConfig, Ema, EmaConfig, Indicator, IndicatorConfig, IndicatorConfigBuilder, Multiplier,
    Price, PriceSource, internals::EmaCore,
};

/// Configuration for the Keltner Channel ([`Kc`]) indicator.
///
/// The Keltner Channel uses an EMA as the centre line and ATR-based
/// bands above and below it. The `length` controls the EMA period,
/// `atr_length` controls the ATR period, and `multiplier` scales the
/// ATR to set band width.
///
/// # Convergence
///
/// Output begins when both the EMA and ATR have converged, i.e. after
/// `max(length, atr_length)` bars. Use [`full_convergence`](Self::full_convergence)
/// for the stricter threshold where the EMA seed influence has decayed
/// below 1%.
///
/// # Example
///
/// ```
/// use quantedge_ta::{KcConfig, Multiplier};
/// use std::num::NonZero;
///
/// let config = KcConfig::builder()
///     .length(NonZero::new(20).unwrap())
///     .atr_length(NonZero::new(10).unwrap())
///     .multiplier(Multiplier::new(2.0))
///     .build();
///
/// assert_eq!(config.length(), 20);
/// assert_eq!(config.atr_length(), 10);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct KcConfig {
    length: usize,
    atr_length: usize,
    multiplier: Multiplier,
    source: PriceSource,
}

impl IndicatorConfig for KcConfig {
    type Builder = KcConfigBuilder;

    fn builder() -> Self::Builder {
        KcConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length.max(self.atr_length)
    }

    fn to_builder(&self) -> Self::Builder {
        KcConfigBuilder {
            length: Some(self.length),
            atr_length: Some(self.atr_length),
            multiplier: self.multiplier,
            source: self.source,
        }
    }
}

impl KcConfig {
    /// EMA window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// ATR window length (number of bars).
    #[must_use]
    pub fn atr_length(&self) -> usize {
        self.atr_length
    }

    /// Band width multiplier.
    #[must_use]
    pub fn multiplier(&self) -> Multiplier {
        self.multiplier
    }

    /// Bars until all outputs are fully converged (EMA seed influence
    /// below 1%).
    #[must_use]
    pub fn full_convergence(&self) -> usize {
        EmaCore::bars_to_converge(self.length).max(self.atr_length)
    }
}

impl Default for KcConfig {
    /// Default: length=20, `atr_length`=10, multiplier=1.5, source=Close (`TradingView` default).
    fn default() -> Self {
        Self {
            length: 20,
            atr_length: 10,
            multiplier: Multiplier::new(1.5),
            source: PriceSource::Close,
        }
    }
}

impl Display for KcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KcConfig(l: {}, atr_l: {}, m: {}, s: {})",
            self.length,
            self.atr_length,
            self.multiplier.value(),
            self.source
        )
    }
}

/// Builder for [`KcConfig`].
///
/// Defaults: source = [`PriceSource::Close`],
/// multiplier = `1.5`.
/// `length` and `atr_length` must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct KcConfigBuilder {
    length: Option<usize>,
    atr_length: Option<usize>,
    multiplier: Multiplier,
    source: PriceSource,
}

impl KcConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            atr_length: None,
            multiplier: Multiplier::new(1.5),
            source: PriceSource::Close,
        }
    }
}

impl IndicatorConfigBuilder<KcConfig> for KcConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> KcConfig {
        KcConfig {
            length: self.length.expect("length is required"),
            atr_length: self.atr_length.expect("atr_length is required"),
            multiplier: self.multiplier,
            source: self.source,
        }
    }
}

impl KcConfigBuilder {
    /// Sets the EMA window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }

    /// Sets the ATR window length.
    #[must_use]
    pub fn atr_length(mut self, atr_length: NonZero<usize>) -> Self {
        self.atr_length.replace(atr_length.get());
        self
    }

    /// Sets the band width multiplier.
    #[must_use]
    pub fn multiplier(mut self, multiplier: Multiplier) -> Self {
        self.multiplier = multiplier;
        self
    }
}

/// Keltner Channel output: upper band, middle line, and lower band.
///
/// The middle line is the EMA value. The upper and lower bands are
/// offset from the middle by `multiplier × ATR`.
///
/// ```text
/// middle = EMA(length, price)
/// upper  = middle + multiplier × ATR
/// lower  = middle − multiplier × ATR
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KcValue {
    upper: Price,
    middle: Price,
    lower: Price,
}

impl KcValue {
    /// Upper band: `EMA + multiplier × ATR`.
    #[inline]
    #[must_use]
    pub fn upper(&self) -> Price {
        self.upper
    }

    /// Middle line: EMA value.
    #[inline]
    #[must_use]
    pub fn middle(&self) -> Price {
        self.middle
    }

    /// Lower band: `EMA − multiplier × ATR`.
    #[inline]
    #[must_use]
    pub fn lower(&self) -> Price {
        self.lower
    }
}

impl Display for KcValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "KcValue(u: {}, m: {}, l: {})",
            self.upper, self.middle, self.lower
        )
    }
}

/// Keltner Channel (KC).
///
/// A volatility-based envelope indicator that places bands around an
/// EMA using Average True Range (ATR). The centre line is an EMA of
/// the configured price source; the upper and lower bands are offset
/// by `multiplier × ATR`.
///
/// ```text
/// middle = EMA(length, price)
/// upper  = middle + multiplier × ATR(atr_length)
/// lower  = middle − multiplier × ATR(atr_length)
/// ```
///
/// Returns `None` until both the EMA and ATR have converged (after
/// `max(length, atr_length)` bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Kc, KcConfig, Multiplier};
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
/// let config = KcConfig::builder()
///     .length(NonZero::new(2).unwrap())
///     .atr_length(NonZero::new(2).unwrap())
///     .multiplier(Multiplier::new(1.5))
///     .build();
/// let mut kc = Kc::new(config);
///
/// assert!(kc.compute(&Bar { o: 10.0, h: 20.0, l: 5.0, c: 15.0, t: 1 }).is_none());
///
/// let val = kc.compute(&Bar { o: 16.0, h: 22.0, l: 12.0, c: 18.0, t: 2 }).unwrap();
/// assert!(val.upper() > val.middle());
/// assert!(val.middle() > val.lower());
/// ```
#[derive(Clone, Debug)]
pub struct Kc {
    config: KcConfig,
    ema: Ema,
    atr: Atr,
    current: Option<KcValue>,
}

impl Indicator for Kc {
    type Config = KcConfig;
    type Output = KcValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            ema: Ema::new(
                EmaConfig::builder()
                    .length(NonZero::new(config.length).unwrap())
                    .source(config.source)
                    .build(),
            ),
            atr: Atr::new(
                AtrConfig::builder()
                    .length(NonZero::new(config.atr_length).unwrap())
                    .build(),
            ),
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        self.current = match (self.ema.compute(ohlcv), self.atr.compute(ohlcv)) {
            (Some(ema), Some(atr)) => {
                let diff = atr * self.config.multiplier.value();

                Some(KcValue {
                    upper: ema + diff,
                    middle: ema,
                    lower: ema - diff,
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

impl Display for Kc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Kc(l: {}, atr_l: {}, m: {}, s: {})",
            self.config.length,
            self.config.atr_length,
            self.config.multiplier.value(),
            self.config.source
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{nz, ohlc};

    /// KC(2, 2, 1.5) — small windows for tractable hand calculations.
    fn kc_2_2() -> Kc {
        Kc::new(
            KcConfig::builder()
                .length(nz(2))
                .atr_length(nz(2))
                .multiplier(Multiplier::new(1.5))
                .build(),
        )
    }

    /// Returns a converged KC(2, 2, 1.5) after 2 bars.
    fn seeded_kc() -> Kc {
        let mut kc = kc_2_2();
        kc.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
        kc.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2));
        kc
    }

    mod convergence {
        use super::*;

        #[test]
        fn none_before_both_converge() {
            let mut kc = kc_2_2();
            assert!(kc.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)).is_none());
        }

        #[test]
        fn first_value_at_max_length_atr_length() {
            let kc = seeded_kc();
            assert!(kc.value().is_some());
        }

        #[test]
        fn none_when_ema_leads_atr() {
            // EMA length=1 converges at bar 1, ATR length=3 converges at bar 3
            let mut kc = Kc::new(KcConfig::builder().length(nz(1)).atr_length(nz(3)).build());
            assert!(kc.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none());
            assert!(kc.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2)).is_none());
            assert!(kc.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3)).is_some());
        }

        #[test]
        fn none_when_atr_leads_ema() {
            // EMA length=3, ATR length=1
            let mut kc = Kc::new(KcConfig::builder().length(nz(3)).atr_length(nz(1)).build());
            assert!(kc.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none());
            assert!(kc.compute(&ohlc(12.0, 18.0, 8.0, 14.0, 2)).is_none());
            assert!(kc.compute(&ohlc(14.0, 20.0, 10.0, 16.0, 3)).is_some());
        }

        #[test]
        fn value_none_before_convergence() {
            let kc = kc_2_2();
            assert_eq!(kc.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut kc = seeded_kc();
            let computed = kc.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 3));
            assert_eq!(kc.value(), computed);
        }
    }

    mod computation {
        use super::*;
        use quantedge_core::test_util::assert_approx;

        #[test]
        fn middle_equals_ema() {
            // Run EMA(2) on close and KC(2,2) in parallel, compare middle
            let mut kc = seeded_kc();
            let mut ema = Ema::new(EmaConfig::close(nz(2)));

            let bars = [
                ohlc(10.0, 20.0, 5.0, 15.0, 1),
                ohlc(16.0, 22.0, 12.0, 18.0, 2),
                ohlc(18.0, 25.0, 14.0, 20.0, 3),
            ];
            for b in &bars {
                ema.compute(b);
            }
            kc.compute(&bars[2]);

            assert_approx!(kc.value().unwrap().middle(), ema.value().unwrap());
        }

        #[test]
        fn bands_symmetric_around_middle() {
            let val = seeded_kc().value().unwrap();
            let upper_dist = val.upper() - val.middle();
            let lower_dist = val.middle() - val.lower();
            assert!((upper_dist - lower_dist).abs() < 1e-10);
        }

        #[test]
        fn upper_above_middle_above_lower() {
            let val = seeded_kc().value().unwrap();
            assert!(val.upper() > val.middle());
            assert!(val.middle() > val.lower());
        }

        #[test]
        fn band_width_equals_2_times_multiplier_times_atr() {
            let val = seeded_kc().value().unwrap();
            let width = val.upper() - val.lower();
            let half_width = val.upper() - val.middle();
            // width should be 2 * half_width
            assert!((width - 2.0 * half_width).abs() < 1e-10);
        }

        #[test]
        fn constant_ohlc_collapses_bands() {
            // When high == low == close, ATR = 0 → bands collapse to EMA
            let mut kc = kc_2_2();
            kc.compute(&ohlc(50.0, 50.0, 50.0, 50.0, 1));
            let val = kc.compute(&ohlc(50.0, 50.0, 50.0, 50.0, 2)).unwrap();
            assert!((val.upper() - val.middle()).abs() < 1e-10);
            assert!((val.middle() - val.lower()).abs() < 1e-10);
        }
    }

    mod multiplier {
        use super::*;

        #[test]
        fn wider_multiplier_wider_bands() {
            let bars = [
                ohlc(10.0, 20.0, 5.0, 15.0, 1),
                ohlc(16.0, 22.0, 12.0, 18.0, 2),
            ];

            let mut narrow = Kc::new(
                KcConfig::builder()
                    .length(nz(2))
                    .atr_length(nz(2))
                    .multiplier(Multiplier::new(1.0))
                    .build(),
            );
            let mut wide = Kc::new(
                KcConfig::builder()
                    .length(nz(2))
                    .atr_length(nz(2))
                    .multiplier(Multiplier::new(3.0))
                    .build(),
            );

            for b in &bars {
                narrow.compute(b);
                wide.compute(b);
            }

            let nv = narrow.value().unwrap();
            let wv = wide.value().unwrap();

            assert!(wv.upper() - wv.lower() > nv.upper() - nv.lower());
            // Middle should be the same (same EMA)
            assert!((nv.middle() - wv.middle()).abs() < 1e-10);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_value() {
            let mut kc = seeded_kc();
            let original = kc.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 3)).unwrap();
            let repainted = kc.compute(&ohlc(18.0, 30.0, 10.0, 28.0, 3)).unwrap();
            assert_ne!(original, repainted);
        }

        #[test]
        fn multiple_repaints_match_clean() {
            let mut kc = seeded_kc();
            kc.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 3));
            kc.compute(&ohlc(18.0, 30.0, 10.0, 28.0, 3)); // repaint 1
            kc.compute(&ohlc(18.0, 24.0, 13.0, 19.0, 3)); // repaint 2
            let final_val = kc.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 3));

            let mut clean = seeded_kc();
            let expected = clean.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 3));

            assert_eq!(final_val, expected);
        }

        #[test]
        fn repaint_then_advance() {
            let mut kc = seeded_kc();
            kc.compute(&ohlc(18.0, 25.0, 14.0, 20.0, 3));
            kc.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 3)); // repaint
            let after = kc.compute(&ohlc(22.0, 28.0, 18.0, 24.0, 4));

            let mut clean = seeded_kc();
            clean.compute(&ohlc(18.0, 26.0, 12.0, 22.0, 3));
            let expected = clean.compute(&ohlc(22.0, 28.0, 18.0, 24.0, 4));

            assert_eq!(after, expected);
        }

        #[test]
        fn repaint_during_filling() {
            let mut kc = kc_2_2();
            kc.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            kc.compute(&ohlc(10.0, 22.0, 4.0, 18.0, 1)); // repaint
            assert!(kc.value().is_none()); // still filling
            assert!(kc.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)).is_some());
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut kc = seeded_kc();
            let mut cloned = kc.clone();

            let orig = kc.compute(&ohlc(18.0, 30.0, 10.0, 28.0, 3)).unwrap();
            let clone_val = cloned.compute(&ohlc(18.0, 19.0, 17.0, 17.5, 3)).unwrap();

            assert!(
                (orig.middle() - clone_val.middle()).abs() > 1e-10,
                "divergent inputs should give different middle"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn accessors() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .multiplier(Multiplier::new(2.0))
                .build();
            assert_eq!(config.length(), 20);
            assert_eq!(config.atr_length(), 10);
            assert!((config.multiplier().value() - 2.0).abs() < f64::EPSILON);
        }

        #[test]
        fn default_source_is_close() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn default_multiplier_is_1_5() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            assert!((config.multiplier().value() - 1.5).abs() < f64::EPSILON);
        }

        #[test]
        fn convergence_is_max_of_lengths() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            assert_eq!(config.convergence(), 20);

            let config = KcConfig::builder()
                .length(nz(10))
                .atr_length(nz(20))
                .build();
            assert_eq!(config.convergence(), 20);
        }

        #[test]
        fn full_convergence_uses_ema_bars_to_converge() {
            // EmaCore::bars_to_converge(20) = 3 * (20 + 1) = 63
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            assert_eq!(config.full_convergence(), 63);
        }

        #[test]
        fn full_convergence_capped_by_atr() {
            // EmaCore::bars_to_converge(2) = 3 * 3 = 9, atr_length=100
            let config = KcConfig::builder()
                .length(nz(2))
                .atr_length(nz(100))
                .build();
            assert_eq!(config.full_convergence(), 100);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = KcConfig::builder().atr_length(nz(10)).build();
        }

        #[test]
        #[should_panic(expected = "atr_length is required")]
        fn panics_without_atr_length() {
            let _ = KcConfig::builder().length(nz(20)).build();
        }

        #[test]
        fn eq_and_hash() {
            let a = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            let b = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build();
            let c = KcConfig::builder()
                .length(nz(10))
                .atr_length(nz(10))
                .build();

            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .multiplier(Multiplier::new(2.0))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .multiplier(Multiplier::new(1.5))
                .build();
            assert_eq!(
                config.to_string(),
                "KcConfig(l: 20, atr_l: 10, m: 1.5, s: Close)"
            );
        }

        #[test]
        fn display_kc() {
            let kc = Kc::new(
                KcConfig::builder()
                    .length(nz(20))
                    .atr_length(nz(10))
                    .multiplier(Multiplier::new(1.5))
                    .build(),
            );
            assert_eq!(kc.to_string(), "Kc(l: 20, atr_l: 10, m: 1.5, s: Close)");
        }

        #[test]
        fn display_value() {
            let v = KcValue {
                upper: 105.0,
                middle: 100.0,
                lower: 95.0,
            };
            assert_eq!(v.to_string(), "KcValue(u: 105, m: 100, l: 95)");
        }
    }

    mod price_source {
        use super::*;
        use quantedge_core::test_util::assert_approx;

        #[test]
        fn uses_configured_source() {
            // KC with HL2 source vs KC with close source fed HL2 values
            let config_hl2 = KcConfig::builder()
                .length(nz(2))
                .atr_length(nz(2))
                .source(PriceSource::HL2)
                .build();
            let mut kc_hl2 = Kc::new(config_hl2);

            let config_close = KcConfig::builder().length(nz(2)).atr_length(nz(2)).build();
            let mut kc_close = Kc::new(config_close);

            // Feed HL2 KC with bars where close != HL2
            let bars = [
                ohlc(10.0, 20.0, 10.0, 5.0, 1), // HL2=15
                ohlc(16.0, 22.0, 12.0, 8.0, 2), // HL2=17
            ];
            for b in &bars {
                kc_hl2.compute(b);
            }

            // Feed close KC with bars where close = HL2
            let hl2_bars = [
                ohlc(15.0, 20.0, 10.0, 15.0, 1),
                ohlc(17.0, 22.0, 12.0, 17.0, 2),
            ];
            for b in &hl2_bars {
                kc_close.compute(b);
            }

            // Middle lines should match (same EMA input values)
            let hl2_val = kc_hl2.value().unwrap();
            let close_val = kc_close.value().unwrap();
            assert_approx!(hl2_val.middle(), close_val.middle());
        }
    }
}
