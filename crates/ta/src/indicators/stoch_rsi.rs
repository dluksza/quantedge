use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, PriceSource, Rsi, RsiConfig, Stoch,
    Timestamp,
    internals::{RollingMaxMin, RollingSum},
};

/// Configuration for the Stochastic RSI ([`StochRsi`]) indicator.
///
/// Stochastic RSI applies the Stochastic Oscillator formula to RSI
/// values instead of raw prices. It requires four parameters:
/// `rsi_length` for the RSI lookback, `stoch_length` for the
/// highest/lowest RSI window, `k_smooth` for smoothing %K, and
/// `d_smooth` for the %D signal line (SMA of smoothed %K).
///
/// Output begins after `rsi_length + stoch_length + k_smooth` bars.
/// The %D line requires an additional `d_smooth` bars after %K
/// converges.
///
/// # Example
///
/// ```
/// use quantedge_ta::StochRsiConfig;
/// use std::num::NonZero;
///
/// let config = StochRsiConfig::builder()
///     .rsi_length(NonZero::new(14).unwrap())
///     .stoch_length(NonZero::new(14).unwrap())
///     .k_smooth(NonZero::new(3).unwrap())
///     .d_smooth(NonZero::new(3).unwrap())
///     .build();
///
/// assert_eq!(config.rsi_length(), 14);
/// assert_eq!(config.stoch_length(), 14);
/// assert_eq!(config.k_smooth(), 3);
/// assert_eq!(config.d_smooth(), 3);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct StochRsiConfig {
    rsi_length: usize,
    stoch_length: usize,
    k_smooth: usize,
    d_smooth: usize,
    source: PriceSource,
}

impl StochRsiConfig {
    /// RSI lookback length.
    #[must_use]
    pub fn rsi_length(&self) -> usize {
        self.rsi_length
    }

    /// Stochastic lookback length (highest/lowest RSI window).
    #[must_use]
    pub fn stoch_length(&self) -> usize {
        self.stoch_length
    }

    /// Smoothing period for %K (SMA of raw stochastic values).
    #[must_use]
    pub fn k_smooth(&self) -> usize {
        self.k_smooth
    }

    /// Smoothing period for %D (SMA of smoothed %K values).
    #[must_use]
    pub fn d_smooth(&self) -> usize {
        self.d_smooth
    }
}

impl IndicatorConfig for StochRsiConfig {
    type Builder = StochRsiConfigBuilder;

    fn builder() -> Self::Builder {
        StochRsiConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.rsi_length + self.stoch_length + self.k_smooth
    }

    fn to_builder(&self) -> Self::Builder {
        StochRsiConfigBuilder {
            rsi_length: Some(self.rsi_length),
            stoch_length: Some(self.stoch_length),
            k_smooth: Some(self.k_smooth),
            d_smooth: Some(self.d_smooth),
            source: self.source,
        }
    }
}

impl Default for StochRsiConfig {
    /// Default: `rsi_length`=14, `stoch_length`=14, `k_smooth`=3,
    /// `d_smooth`=3, source=Close (`TradingView` default).
    fn default() -> Self {
        Self {
            rsi_length: 14,
            stoch_length: 14,
            k_smooth: 3,
            d_smooth: 3,
            source: PriceSource::Close,
        }
    }
}

impl Display for StochRsiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StochRsiConfig(rsi: {}, stoch: {}, k_smooth: {}, d_smooth: {}, s: {})",
            self.rsi_length, self.stoch_length, self.k_smooth, self.d_smooth, self.source
        )
    }
}

/// Builder for [`StochRsiConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// `rsi_length`, `stoch_length`, `k_smooth`, and `d_smooth` must
/// all be set before calling [`build`](IndicatorConfigBuilder::build).
pub struct StochRsiConfigBuilder {
    rsi_length: Option<usize>,
    stoch_length: Option<usize>,
    k_smooth: Option<usize>,
    d_smooth: Option<usize>,
    source: PriceSource,
}

impl StochRsiConfigBuilder {
    fn new() -> Self {
        Self {
            rsi_length: None,
            stoch_length: None,
            k_smooth: None,
            d_smooth: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the RSI lookback length.
    #[must_use]
    pub fn rsi_length(mut self, value: NonZero<usize>) -> Self {
        self.rsi_length.replace(value.get());
        self
    }

    /// Sets the stochastic lookback length.
    #[must_use]
    pub fn stoch_length(mut self, value: NonZero<usize>) -> Self {
        self.stoch_length.replace(value.get());
        self
    }

    /// Sets the %K smoothing period.
    #[must_use]
    pub fn k_smooth(mut self, value: NonZero<usize>) -> Self {
        self.k_smooth.replace(value.get());
        self
    }

    /// Sets the %D smoothing period.
    #[must_use]
    pub fn d_smooth(mut self, value: NonZero<usize>) -> Self {
        self.d_smooth.replace(value.get());
        self
    }
}

impl IndicatorConfigBuilder<StochRsiConfig> for StochRsiConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> StochRsiConfig {
        StochRsiConfig {
            rsi_length: self.rsi_length.expect("rsi_length is required"),
            stoch_length: self.stoch_length.expect("stoch_length is required"),
            k_smooth: self.k_smooth.expect("k_smooth is required"),
            d_smooth: self.d_smooth.expect("d_smooth is required"),
            source: self.source,
        }
    }
}

/// Stochastic RSI output: %K and %D lines.
///
/// %K is the smoothed stochastic RSI value (0–100). %D is the
/// signal line (SMA of %K), which may be `None` until enough %K
/// values have been collected.
///
/// ```text
/// RSI        = RSI(price, rsi_length)
/// raw %K     = (RSI − lowest_RSI) / (highest_RSI − lowest_RSI) × 100
/// %K         = SMA(raw %K, k_smooth)
/// %D         = SMA(%K, d_smooth)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StochRsiValue {
    k: f64,
    d: Option<f64>,
}

impl StochRsiValue {
    /// Smoothed %K value (0–100).
    #[inline]
    #[must_use]
    pub fn k(&self) -> f64 {
        self.k
    }

    /// %D signal line, or `None` if not yet converged.
    #[inline]
    #[must_use]
    pub fn d(&self) -> Option<f64> {
        self.d
    }
}

impl Display for StochRsiValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.d {
            Some(d) => write!(f, "StochRsi(k: {}, d: {d})", self.k),
            None => write!(f, "StochRsi(k: {}, d: -)", self.k),
        }
    }
}

/// Stochastic RSI (`StochRSI`).
///
/// Applies the Stochastic Oscillator formula to RSI values,
/// producing a 0–100 scale that measures whether RSI is near its
/// recent high or low. Values above 80 are conventionally
/// considered overbought; below 20, oversold.
///
/// ```text
/// RSI        = RSI(price, rsi_length)
/// raw %K     = (RSI − lowest_RSI) / (highest_RSI − lowest_RSI) × 100
/// %K         = SMA(raw %K, k_smooth)
/// %D         = SMA(%K, d_smooth)
/// ```
///
/// Returns `None` until the RSI, stochastic window, and %K
/// smoothing are all full (`rsi_length + stoch_length + k_smooth`
/// bars). The %D line within the output is `None` until an
/// additional `d_smooth` bars have been processed.
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{StochRsi, StochRsiConfig};
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
/// let config = StochRsiConfig::builder()
///     .rsi_length(NonZero::new(3).unwrap())
///     .stoch_length(NonZero::new(3).unwrap())
///     .k_smooth(NonZero::new(1).unwrap())
///     .d_smooth(NonZero::new(1).unwrap())
///     .build();
/// let mut stoch_rsi = StochRsi::new(config);
///
/// // Feed bars until convergence
/// for (i, price) in [10.0, 12.0, 11.0, 13.0, 14.0, 12.0, 15.0].iter().enumerate() {
///     let result = stoch_rsi.compute(&Bar(*price, i as u64 + 1));
///     if i < 6 {
///         assert!(result.is_none());
///     } else {
///         assert!(result.is_some());
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub struct StochRsi {
    config: StochRsiConfig,
    rsi: Rsi,
    k_sum: RollingSum,
    d_sum: RollingSum,
    max_min: RollingMaxMin,
    current: Option<StochRsiValue>,
    last_open_time: Option<Timestamp>,
    k_reciprocal: f64,
    d_reciprocal: f64,
}

impl Indicator for StochRsi {
    type Config = StochRsiConfig;
    type Output = StochRsiValue;

    fn new(config: Self::Config) -> Self {
        StochRsi {
            config,
            rsi: Rsi::new(
                RsiConfig::builder()
                    .length(NonZero::new(config.rsi_length).unwrap())
                    .source(config.source)
                    .build(),
            ),
            k_sum: RollingSum::new(config.k_smooth),
            d_sum: RollingSum::new(config.d_smooth),
            max_min: RollingMaxMin::new(config.stoch_length),
            current: None,
            last_open_time: None,
            #[allow(clippy::cast_precision_loss)]
            k_reciprocal: 1.0 / config.k_smooth as f64,
            #[allow(clippy::cast_precision_loss)]
            d_reciprocal: 1.0 / config.d_smooth as f64,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        if let Some(rsi) = self.rsi.compute(ohlcv) {
            let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time());

            self.current = if is_next_bar {
                self.last_open_time = Some(ohlcv.open_time());

                let k_sum = self
                    .max_min
                    .push(rsi)
                    .and_then(|(max, min)| self.k_sum.push(Stoch::k_value(rsi, max, min)));

                match k_sum {
                    Some(k_sum) => {
                        let k = k_sum * self.k_reciprocal;
                        let d = self.d_sum.push(k).map(|sum| sum * self.d_reciprocal);

                        Some(StochRsiValue { k, d })
                    }
                    None => None,
                }
            } else {
                let k_sum = self
                    .max_min
                    .replace(rsi)
                    .and_then(|(max, min)| self.k_sum.replace(Stoch::k_value(rsi, max, min)));

                self.current.map(|current| {
                    let k = k_sum.expect("k_sum must be ready when current is Some")
                        * self.k_reciprocal;
                    let d_sum = self.d_sum.replace(k);

                    StochRsiValue {
                        k,
                        d: current.d.map(|_| {
                            d_sum.expect("d_sum must be ready when d is Some") * self.d_reciprocal
                        }),
                    }
                })
            }
        }

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for StochRsi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let config = self.config;

        write!(
            f,
            "StochRsi(rsi: {}, stoch: {}, k_smooth: {}, d_smooth: {}, s: {})",
            config.rsi_length, config.stoch_length, config.k_smooth, config.d_smooth, config.source
        )
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::cast_precision_loss)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{bar, nz};

    /// StochRsi(rsi=3, stoch=3, k=1, d=1) — simplest config.
    fn stoch_rsi_3_3_1_1() -> StochRsi {
        let config = StochRsiConfig::builder()
            .rsi_length(nz(3))
            .stoch_length(nz(3))
            .k_smooth(nz(1))
            .d_smooth(nz(1))
            .build();
        StochRsi::new(config)
    }

    /// Returns a converged StochRsi(3,3,1,1) after feeding enough bars.
    /// RSI(3) converges at bar 4. `stoch_length=3` needs 3 RSI values.
    /// `k_smooth=1` needs 1 more. Total: 3+3+1 = 7 bars.
    fn seeded_stoch_rsi() -> StochRsi {
        let mut s = stoch_rsi_3_3_1_1();
        // Feed 7 bars with varying prices
        let prices = [10.0, 12.0, 11.0, 13.0, 14.0, 12.0, 15.0];
        for (i, &p) in prices.iter().enumerate() {
            s.compute(&bar(p, i as u64 + 1));
        }
        assert!(s.value().is_some(), "should be converged after 7 bars");
        s
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_none_during_filling() {
            let mut s = stoch_rsi_3_3_1_1();
            // convergence = 3+3+1 = 7, so bars 1–6 return None
            for t in 1..=6 {
                assert!(
                    s.compute(&bar(10.0 + t as f64, t)).is_none(),
                    "bar {t} should be None"
                );
            }
        }

        #[test]
        fn first_value_at_convergence() {
            let mut s = stoch_rsi_3_3_1_1();
            // convergence = 3+3+1 = 7
            for t in 1..=6 {
                s.compute(&bar(10.0 + t as f64, t));
            }
            assert!(s.compute(&bar(20.0, 7)).is_some());
        }

        #[test]
        fn convergence_matches_config() {
            let config = StochRsiConfig::builder()
                .rsi_length(nz(3))
                .stoch_length(nz(3))
                .k_smooth(nz(1))
                .d_smooth(nz(1))
                .build();
            assert_eq!(config.convergence(), 7);
        }

        #[test]
        fn k_smooth_delays_output() {
            // rsi=3, stoch=2, k_smooth=3 → convergence = 3+2+3 = 8
            let config = StochRsiConfig::builder()
                .rsi_length(nz(3))
                .stoch_length(nz(2))
                .k_smooth(nz(3))
                .d_smooth(nz(1))
                .build();
            let mut s = StochRsi::new(config);
            assert_eq!(config.convergence(), 8);
            for t in 1..=7 {
                assert!(
                    s.compute(&bar(10.0 + t as f64, t)).is_none(),
                    "bar {t} should be None"
                );
            }
            assert!(s.compute(&bar(20.0, 8)).is_some());
        }

        #[test]
        fn d_none_until_d_smooth_bars_after_k() {
            // rsi=3, stoch=3, k_smooth=1, d_smooth=3
            // First %K at bar 7 (3+3+1). First %D at bar 7+3-1=9? depends on RollingSum
            let config = StochRsiConfig::builder()
                .rsi_length(nz(3))
                .stoch_length(nz(3))
                .k_smooth(nz(1))
                .d_smooth(nz(3))
                .build();
            let mut s = StochRsi::new(config);

            // Feed until first %K
            for t in 1..=7 {
                s.compute(&bar(10.0 + t as f64, t));
            }
            let v7 = s.value().unwrap();
            assert!(v7.d().is_none());

            let v8 = s.compute(&bar(20.0, 8)).unwrap();
            assert!(v8.d().is_none());

            let v9 = s.compute(&bar(18.0, 9)).unwrap();
            assert!(v9.d().is_none());

            // Bar 10: 4th d_sum push → first %D
            let v10 = s.compute(&bar(22.0, 10)).unwrap();
            assert!(v10.d().is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn flat_rsi_gives_50() {
            // All same price → RSI=50, stochastic of flat RSI=50
            let mut s = stoch_rsi_3_3_1_1();
            for t in 1..=20 {
                let val = s.compute(&bar(100.0, t));
                if let Some(v) = val {
                    assert!(
                        (v.k() - 50.0).abs() < 1e-10,
                        "flat price should give %K=50, got {}",
                        v.k()
                    );
                }
            }
        }

        #[test]
        fn monotonic_increase_gives_constant_rsi_hence_50() {
            // Strictly increasing by constant step → RSI converges to
            // 100 (all gains). Constant RSI means max=min → %K=50.
            let mut s = stoch_rsi_3_3_1_1();
            let mut last_val = None;
            for t in 1..=20 {
                last_val = s.compute(&bar(t as f64, t));
            }
            let v = last_val.unwrap();
            assert!(
                (v.k() - 50.0).abs() < 1e-10,
                "constant RSI should give %K=50, got {}",
                v.k()
            );
        }

        #[test]
        fn varying_prices_give_intermediate_k() {
            // Alternating gains/losses produce RSI values that vary
            // within the stochastic window, giving %K between 0–100.
            let mut s = stoch_rsi_3_3_1_1();
            let prices = [10.0, 12.0, 11.0, 13.0, 11.5, 14.0, 12.0, 15.0, 13.0, 16.0];
            let mut last_val = None;
            for (i, &p) in prices.iter().enumerate() {
                last_val = s.compute(&bar(p, i as u64 + 1));
            }
            let v = last_val.unwrap();
            assert!(
                v.k() > 0.0 && v.k() < 100.0,
                "expected intermediate %K, got {}",
                v.k()
            );
        }
    }

    mod repaints {
        use super::*;

        #[test]
        fn repaint_updates_value() {
            let mut s = seeded_stoch_rsi();
            // Use a drop first so RSI is mid-range, then repaint with a gain
            let original = s.compute(&bar(11.0, 8)).unwrap();
            let repainted = s.compute(&bar(20.0, 8)).unwrap();
            assert!(
                (original.k() - repainted.k()).abs() > 1e-10,
                "repaint should change %K: original={}, repainted={}",
                original.k(),
                repainted.k()
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut s = seeded_stoch_rsi();
            s.compute(&bar(20.0, 8));
            s.compute(&bar(22.0, 8)); // repaint 1
            s.compute(&bar(18.0, 8)); // repaint 2
            let final_val = s.compute(&bar(21.0, 8)).unwrap();

            let mut clean = seeded_stoch_rsi();
            let expected = clean.compute(&bar(21.0, 8)).unwrap();

            assert!(
                (final_val.k() - expected.k()).abs() < 1e-10,
                "multiple repaints should match single: got {} vs {}",
                final_val.k(),
                expected.k()
            );
        }

        #[test]
        fn repaint_then_advance_uses_repainted() {
            let mut s = seeded_stoch_rsi();
            s.compute(&bar(20.0, 8));
            s.compute(&bar(22.0, 8)); // repaint
            let after = s.compute(&bar(19.0, 9)).unwrap();

            let mut clean = seeded_stoch_rsi();
            clean.compute(&bar(22.0, 8));
            let expected = clean.compute(&bar(19.0, 9)).unwrap();

            assert!(
                (after.k() - expected.k()).abs() < 1e-10,
                "advance after repaint should match clean"
            );
        }

        #[test]
        fn repaint_during_filling_has_no_effect_on_convergence() {
            let mut s = stoch_rsi_3_3_1_1();
            s.compute(&bar(10.0, 1));
            s.compute(&bar(12.0, 2));
            s.compute(&bar(15.0, 2)); // repaint bar 2
            assert!(s.value().is_none());
            for t in 3..=6 {
                s.compute(&bar(10.0 + t as f64, t));
            }
            assert!(s.value().is_none()); // still filling
            assert!(s.compute(&bar(20.0, 7)).is_some()); // now converged
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn k_and_d_always_between_0_and_100() {
            let mut s = stoch_rsi_3_3_1_1();
            let prices = [
                100.0, 102.0, 99.0, 101.0, 98.0, 103.0, 97.0, 105.0, 96.0, 104.0, 50.0, 150.0,
                80.0, 120.0, 90.0, 110.0, 70.0, 130.0, 85.0, 115.0,
            ];
            for (i, &p) in prices.iter().enumerate() {
                if let Some(v) = s.compute(&bar(p, i as u64 + 1)) {
                    assert!(
                        (0.0..=100.0).contains(&v.k()),
                        "%K out of bounds: {}",
                        v.k()
                    );
                    if let Some(d) = v.d() {
                        assert!((0.0..=100.0).contains(&d), "%D out of bounds: {d}");
                    }
                }
            }
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut s = seeded_stoch_rsi();
            let mut cloned = s.clone();

            let orig = s.compute(&bar(25.0, 8)).unwrap();
            let clone_val = cloned.compute(&bar(5.0, 8)).unwrap();

            assert!(
                (orig.k() - clone_val.k()).abs() > 1e-10,
                "divergent inputs should give different %K"
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = StochRsiConfig::default();
            assert_eq!(
                config.to_string(),
                "StochRsiConfig(rsi: 14, stoch: 14, k_smooth: 3, d_smooth: 3, s: Close)"
            );
        }

        #[test]
        fn display_stoch_rsi() {
            let s = StochRsi::new(StochRsiConfig::default());
            assert_eq!(
                s.to_string(),
                "StochRsi(rsi: 14, stoch: 14, k_smooth: 3, d_smooth: 3, s: Close)"
            );
        }

        #[test]
        fn display_value_with_d() {
            let v = StochRsiValue {
                k: 75.5,
                d: Some(60.0),
            };
            assert_eq!(v.to_string(), "StochRsi(k: 75.5, d: 60)");
        }

        #[test]
        fn display_value_without_d() {
            let v = StochRsiValue { k: 75.5, d: None };
            assert_eq!(v.to_string(), "StochRsi(k: 75.5, d: -)");
        }
    }

    mod config {
        use super::*;

        #[test]
        fn default_source_is_close() {
            let config = StochRsiConfig::default();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "rsi_length is required")]
        fn panics_without_rsi_length() {
            let _ = StochRsiConfig::builder()
                .stoch_length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
        }

        #[test]
        #[should_panic(expected = "stoch_length is required")]
        fn panics_without_stoch_length() {
            let _ = StochRsiConfig::builder()
                .rsi_length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
        }

        #[test]
        #[should_panic(expected = "k_smooth is required")]
        fn panics_without_k_smooth() {
            let _ = StochRsiConfig::builder()
                .rsi_length(nz(14))
                .stoch_length(nz(14))
                .d_smooth(nz(3))
                .build();
        }

        #[test]
        #[should_panic(expected = "d_smooth is required")]
        fn panics_without_d_smooth() {
            let _ = StochRsiConfig::builder()
                .rsi_length(nz(14))
                .stoch_length(nz(14))
                .k_smooth(nz(3))
                .build();
        }

        #[test]
        fn eq_and_hash() {
            use std::collections::HashSet;
            let a = StochRsiConfig::default();
            let b = StochRsiConfig::default();
            let c = StochRsiConfig::builder()
                .rsi_length(nz(7))
                .stoch_length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn accessors() {
            let config = StochRsiConfig::builder()
                .rsi_length(nz(14))
                .stoch_length(nz(10))
                .k_smooth(nz(3))
                .d_smooth(nz(5))
                .build();
            assert_eq!(config.rsi_length(), 14);
            assert_eq!(config.stoch_length(), 10);
            assert_eq!(config.k_smooth(), 3);
            assert_eq!(config.d_smooth(), 5);
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = StochRsiConfig::default();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let s = stoch_rsi_3_3_1_1();
            assert_eq!(s.value(), None);
        }

        #[test]
        fn matches_last_compute() {
            let mut s = seeded_stoch_rsi();
            let computed = s.compute(&bar(20.0, 8));
            assert_eq!(s.value(), computed);
        }

        #[test]
        fn value_k_and_d_accessors() {
            // d_smooth=1 needs 2 %K values before %D appears
            let mut s = seeded_stoch_rsi();
            let val = s.value().unwrap();
            assert!(val.k().is_finite());

            // Feed one more bar to get %D
            let v = s.compute(&bar(20.0, 8)).unwrap();
            assert!(v.k().is_finite());
            assert!(v.d().is_some());
            assert!(v.d().unwrap().is_finite());
        }
    }
}
