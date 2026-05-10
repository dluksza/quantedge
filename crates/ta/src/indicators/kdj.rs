use std::{
    fmt::{self, Display},
    num::NonZero,
};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, PriceSource, Stoch, StochConfig,
};

/// Configuration for the KDJ Oscillator ([`Kdj`]).
///
/// KDJ wraps [`Stoch`] with an added %J line (`3*K - 2*D`) that amplifies
/// the divergence between %K and %D.
///
/// # Parameters
///
/// | Parameter | Default | Description |
/// |-----------|---------|-------------|
/// | `period`  | 9       | RSV lookback — position of close relative to N-day high/low |
/// | `k_smooth`| 3       | MA period applied to RSV for the K line |
/// | `d_smooth`| 3       | MA period applied to K for the D line |
///
/// Output begins after `period + k_smooth + d_smooth` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::KdjConfig;
/// use std::num::NonZero;
///
/// let config = KdjConfig::close(NonZero::new(9).unwrap());
/// assert_eq!(config.period(), 9);
/// assert_eq!(config.k_smooth(), 3);
/// assert_eq!(config.d_smooth(), 3);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct KdjConfig {
    period: usize,
    k_smooth: usize,
    d_smooth: usize,
    source: PriceSource,
}

impl IndicatorConfig for KdjConfig {
    type Builder = KdjConfigBuilder;
    type Indicator = Kdj;
    type Output = KdjValue;

    fn builder() -> Self::Builder {
        KdjConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.period + self.k_smooth + self.d_smooth
    }

    fn to_builder(&self) -> Self::Builder {
        KdjConfigBuilder {
            period: Some(self.period),
            k_smooth: Some(self.k_smooth),
            d_smooth: Some(self.d_smooth),
            source: self.source,
        }
    }
}

impl KdjConfig {
    /// RSV lookback period. Represents the position of the current closing
    /// price relative to the recent N-day highs and lows.
    #[must_use]
    pub fn period(&self) -> usize {
        self.period
    }

    /// MA period applied to RSV for the K line.
    #[must_use]
    pub fn k_smooth(&self) -> usize {
        self.k_smooth
    }

    /// MA period applied to K for the D line.
    #[must_use]
    pub fn d_smooth(&self) -> usize {
        self.d_smooth
    }

    /// KDJ on closing price.
    #[must_use]
    pub fn close(period: NonZero<usize>) -> Self {
        Self::builder().period(period).build()
    }
}

impl Default for KdjConfig {
    /// Default: `period=9`, `k_smooth=3`, `d_smooth=3`, source=`Close`
    /// (standard KDJ(9,3,3) parameters).
    fn default() -> Self {
        Self {
            period: 9,
            k_smooth: 3,
            d_smooth: 3,
            source: PriceSource::Close,
        }
    }
}

impl Display for KdjConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KdjConfig({}, {}, {}, {})",
            self.period, self.k_smooth, self.d_smooth, self.source
        )
    }
}

/// Builder for [`KdjConfig`].
///
/// Defaults: `k_smooth = 3`, `d_smooth = 3`, source = [`PriceSource::Close`].
/// Period must be set before calling [`build`](IndicatorConfigBuilder::build).
pub struct KdjConfigBuilder {
    period: Option<usize>,
    k_smooth: Option<usize>,
    d_smooth: Option<usize>,
    source: PriceSource,
}

impl KdjConfigBuilder {
    fn new() -> Self {
        Self {
            period: None,
            k_smooth: Some(3),
            d_smooth: Some(3),
            source: PriceSource::Close,
        }
    }

    /// Sets the RSV lookback period for the highest-high / lowest-low window.
    #[must_use]
    pub fn period(mut self, period: NonZero<usize>) -> Self {
        self.period.replace(period.get());
        self
    }

    /// Sets the MA period applied to RSV for the K line (default 3).
    #[must_use]
    pub fn k_smooth(mut self, k_smooth: NonZero<usize>) -> Self {
        self.k_smooth.replace(k_smooth.get());
        self
    }

    /// Sets the MA period applied to K for the D line (default 3).
    #[must_use]
    pub fn d_smooth(mut self, d_smooth: NonZero<usize>) -> Self {
        self.d_smooth.replace(d_smooth.get());
        self
    }
}

impl IndicatorConfigBuilder<KdjConfig> for KdjConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> KdjConfig {
        KdjConfig {
            period: self.period.expect("period is required"),
            k_smooth: self.k_smooth.unwrap_or(3),
            d_smooth: self.d_smooth.unwrap_or(3),
            source: self.source,
        }
    }
}

/// KDJ Oscillator output: %K, %D, and %J lines.
///
/// %K and %D are on a 0–100 scale (though %J can exceed that range).
/// %J = 3×%K - 2×%D amplifies the divergence between the two lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KdjValue {
    /// %K line (MA-smoothed stochastic, 0–100 typical).
    pub k: f64,
    /// %D line (MA of %K, 0–100 typical).
    pub d: f64,
    /// %J line (3K - 2D, can exceed 0–100).
    pub j: f64,
}

impl Display for KdjValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Kdj(k: {}, d: {}, j: {})", self.k, self.d, self.j)
    }
}

/// KDJ Oscillator.
///
/// Thin wrapper around [`Stoch`] that adds a %J line: `%J = 3×%K - 2×%D`.
/// RSV is computed from an N-day high/low window, then smoothed with
/// configurable MA periods for K and D.
///
/// ```text
/// RSV = (close - lowest_low) / (highest_high - lowest_low) × 100
/// K   = MA(RSV, k_smooth)
/// D   = MA(K, d_smooth)
/// J   = 3×K - 2×D
/// ```
///
/// Returns `None` until `period + k_smooth + d_smooth` bars have been
/// processed.
///
/// Supports live repainting through the underlying [`Stoch`] indicator.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ohlcv, Kdj, KdjConfig};
/// use std::num::NonZero;
///
/// fn ohlc(o: f64, h: f64, l: f64, c: f64, t: u64) -> Ohlcv {
///     Ohlcv { open: o, high: h, low: l, close: c, volume: 0.0, open_time: t }
/// }
///
/// // KDJ(2,3,3) → convergence at bar 8 (2 + 3 + 3)
/// let mut kdj = Kdj::new(KdjConfig::close(NonZero::new(2).unwrap()));
///
/// for t in 1..=7u64 {
///     let tf = t as f64;
///     assert_eq!(kdj.compute(&ohlc(tf, tf + 2.0, tf, tf + 1.0, t)), None);
/// }
/// let value = kdj.compute(&ohlc(8.0, 10.0, 7.0, 9.0, 8));
/// assert!(value.is_some());
/// ```
#[derive(Clone, Debug)]
pub struct Kdj {
    stoch: Stoch,
    config: KdjConfig,
}

impl Indicator for Kdj {
    type Config = KdjConfig;
    type Output = KdjValue;

    fn new(config: Self::Config) -> Self {
        let stoch_config = StochConfig::builder()
            .length(NonZero::new(config.period).unwrap())
            .k_smooth(NonZero::new(config.k_smooth).unwrap())
            .d_smooth(NonZero::new(config.d_smooth).unwrap())
            .source(config.source)
            .build();

        Kdj {
            stoch: Stoch::new(stoch_config),
            config,
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        let sv = self.stoch.compute(ohlcv)?;
        let d = sv.d?;
        Some(KdjValue {
            k: sv.k,
            d,
            j: 3.0 * sv.k - 2.0 * d,
        })
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        let sv = self.stoch.value()?;
        let d = sv.d?;
        Some(KdjValue {
            k: sv.k,
            d,
            j: 3.0 * sv.k - 2.0 * d,
        })
    }
}

impl Display for Kdj {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KDJ({}, {}, {}, {})",
            self.config.period, self.config.k_smooth, self.config.d_smooth, self.config.source
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{nz, ohlc};

    fn kdj(period: usize) -> Kdj {
        Kdj::new(KdjConfig::close(nz(period)))
    }

    fn kdj_custom(period: usize, k_smooth: usize, d_smooth: usize) -> Kdj {
        Kdj::new(
            KdjConfig::builder()
                .period(nz(period))
                .k_smooth(nz(k_smooth))
                .d_smooth(nz(d_smooth))
                .build(),
        )
    }

    // ---------------------------------------------------------------------------
    // Reference tests verify:
    //   - correct output count (len - convergence values)
    //   - K, D in [0, 100]
    //   - J = 3K - 2D
    //   - positive K, D for ascending close data
    // ---------------------------------------------------------------------------

    mod reference {
        use super::*;

        /// Reference input data:
        /// highs  = [11, 12, 13, 14, 15, 16]
        /// lows   = [10, 9,  8,  7,  6,  5]
        /// closes = [10.5, 11, 12, 13, 14, 15]
        /// period = 3
        fn reference_bars() -> [Ohlcv; 6] {
            [
                ohlc(11.0, 11.0, 10.0, 10.5, 1),
                ohlc(12.0, 12.0, 9.0, 11.0, 2),
                ohlc(13.0, 13.0, 8.0, 12.0, 3),
                ohlc(14.0, 14.0, 7.0, 13.0, 4),
                ohlc(15.0, 15.0, 6.0, 14.0, 5),
                ohlc(16.0, 16.0, 5.0, 15.0, 6),
            ]
        }

        #[test]
        fn correct_output_count() {
            // KDJ(3,3,3): convergence at bar 3+3+3=9; 6 bars < 9 → 0 outputs
            // Feed 12 bars for convergence
            let mut kdj = kdj(3);
            let mut all_bars = reference_bars().to_vec();

            for t in 7..=12u32 {
                let tf = f64::from(t);
                all_bars.push(ohlc(tf + 5.0, tf + 7.0, tf + 3.0, tf + 6.0, u64::from(t)));
            }

            let mut results = Vec::new();
            for bar in &all_bars {
                if let Some(v) = kdj.compute(bar) {
                    results.push(v);
                }
            }

            // 12 bars - 9 convergence = 4 outputs (bars 9..=12)
            assert_eq!(results.len(), 4, "expected 4 output values from 12 bars");
        }

        #[test]
        fn k_d_in_0_100_range() {
            // Need 9+ bars for KDJ(3,3,3) to produce output
            let mut kdj = kdj(3);
            for bar in &reference_bars() {
                kdj.compute(bar);
            }
            kdj.compute(&ohlc(17.0, 17.0, 4.0, 16.0, 7));
            kdj.compute(&ohlc(18.0, 18.0, 3.0, 17.0, 8));
            kdj.compute(&ohlc(19.0, 19.0, 2.0, 18.0, 9));

            let v = kdj
                .value()
                .expect("expected KDJ output after 9 bars (period=3)");
            assert!(v.k >= 0.0 && v.k <= 100.0, "K out of range: {}", v.k);
            assert!(v.d >= 0.0 && v.d <= 100.0, "D out of range: {}", v.d);
        }

        #[test]
        fn j_equals_3k_minus_2d() {
            let mut kdj = kdj(3);
            for bar in &reference_bars() {
                kdj.compute(bar);
            }
            kdj.compute(&ohlc(17.0, 17.0, 4.0, 16.0, 7));
            kdj.compute(&ohlc(18.0, 18.0, 3.0, 17.0, 8));
            kdj.compute(&ohlc(19.0, 19.0, 2.0, 18.0, 9));

            let v = kdj
                .value()
                .expect("expected KDJ output after 9 bars (period=3)");
            assert!(
                (v.j - (3.0 * v.k - 2.0 * v.d)).abs() < 1e-14,
                "J = 3K - 2D relation failed: J={} expected={}",
                v.j,
                3.0 * v.k - 2.0 * v.d
            );
        }

        #[test]
        fn positive_k_d_values() {
            // Same data with k_smooth=1, d_smooth=1 for faster convergence
            // to verify the reference bars produce positive values
            let mut kdj = kdj_custom(3, 1, 1);
            let mut results = Vec::new();
            for bar in &reference_bars() {
                if let Some(v) = kdj.compute(bar) {
                    results.push(v);
                }
            }

            // KDJ(3,1,1): convergence at bar 5 → bars 5,6 → 2 outputs
            assert!(
                !results.is_empty(),
                "expected KDJ output from reference data"
            );
            for (i, v) in results.iter().enumerate() {
                assert!(v.k > 0.0, "K[{}] should be positive: {}", i, v.k);
                assert!(v.d > 0.0, "D[{}] should be positive: {}", i, v.d);
            }
        }
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_window_full() {
            // KDJ(2,3,3) → convergence at bar 8
            let mut kdj = kdj(2);
            for t in 1..=7u32 {
                let tf = f64::from(t);
                assert_eq!(
                    kdj.compute(&ohlc(tf, tf + 2., tf, tf + 1., u64::from(t))),
                    None,
                    "should be None at bar {t}"
                );
            }
        }

        #[test]
        fn returns_value_at_convergence() {
            let mut kdj = kdj(2);
            for t in 1..=7u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf, tf + 2., tf, tf + 1., u64::from(t)));
            }
            let val = kdj.compute(&ohlc(8.0, 10.0, 7.0, 9.0, 8));
            assert!(
                val.is_some(),
                "expected Some at bar 8 (period=2, convergence=8)"
            );
        }

        #[test]
        fn custom_smoothing_convergence() {
            // KDJ(2,1,1) → convergence at bar 4
            let mut kdj = kdj_custom(2, 1, 1);
            kdj.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
            kdj.compute(&ohlc(12.0, 18.0, 10.0, 16.0, 2));
            kdj.compute(&ohlc(14.0, 20.0, 12.0, 18.0, 3));
            // Bar 4: first output
            assert!(
                kdj.compute(&ohlc(16.0, 22.0, 14.0, 20.0, 4)).is_some(),
                "expected Some at bar 4 (2+1+1)"
            );
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn flat_market_stabilizes_at_50() {
            let mut kdj = kdj(3);
            for t in 1..=30u64 {
                kdj.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
            }
            let val = kdj.value().unwrap();
            assert!((val.k - 50.0).abs() < 1e-10, "K should be 50: {}", val.k);
            assert!((val.d - 50.0).abs() < 1e-10, "D should be 50: {}", val.d);
            assert!((val.j - 50.0).abs() < 1e-10, "J should be 50: {}", val.j);
        }

        #[test]
        fn values_respond_to_price() {
            let mut kdj = kdj(2);
            for t in 1..=7u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf, tf + 5.0, tf, tf + 2.5, u64::from(t)));
            }
            kdj.compute(&ohlc(8.0, 13.0, 8.0, 12.0, 8));

            for t in 9..=20u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf + 10., tf + 15., tf, tf + 13., u64::from(t)));
            }
            let v = kdj.value().unwrap();

            assert!((v.k >= 0.0) && (v.k <= 100.0));
            assert!((v.d >= 0.0) && (v.d <= 100.0));
            assert!((v.j - (3.0 * v.k - 2.0 * v.d)).abs() < 1e-14);
        }

        #[test]
        fn k_and_d_in_reasonable_range() {
            let mut kdj = kdj(3);
            let bars = [
                ohlc(100.0, 110.0, 90.0, 105.0, 1),
                ohlc(102.0, 115.0, 88.0, 98.0, 2),
                ohlc(99.0, 108.0, 92.0, 103.0, 3),
                ohlc(101.0, 120.0, 85.0, 95.0, 4),
                ohlc(96.0, 105.0, 80.0, 100.0, 5),
                ohlc(98.0, 130.0, 75.0, 110.0, 6),
                ohlc(100.0, 125.0, 82.0, 108.0, 7),
                ohlc(102.0, 118.0, 88.0, 105.0, 8),
                ohlc(99.0, 122.0, 78.0, 112.0, 9),
                ohlc(101.0, 128.0, 80.0, 115.0, 10),
                ohlc(97.0, 135.0, 76.0, 120.0, 11),
                ohlc(95.0, 130.0, 74.0, 118.0, 12),
            ];
            for b in &bars {
                if let Some(v) = kdj.compute(b) {
                    assert!(v.k >= 0.0 && v.k <= 100.0, "K out of range: {}", v.k);
                    assert!(v.d >= 0.0 && v.d <= 100.0, "D out of range: {}", v.d);
                }
            }
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn old_extreme_expires() {
            let mut kdj = kdj(2);
            kdj.compute(&ohlc(10.0, 50.0, 1.0, 15.0, 1));
            for t in 2..=7u64 {
                kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, t));
            }
            let v1 = kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 8)).unwrap();

            kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 9));
            let v2 = kdj.compute(&ohlc(10.0, 25.0, 10.0, 24.0, 10)).unwrap();

            assert!(
                v2.k > v1.k,
                "K should increase when close moves toward high: K1={} K2={}",
                v1.k,
                v2.k
            );
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut kdj = kdj(2);
            for t in 1..=7u64 {
                kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, t));
            }
            let v1 = kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 8)).unwrap();
            let v2 = kdj.compute(&ohlc(10.0, 20.0, 10.0, 19.0, 8)).unwrap();
            assert!(
                v2.k > v1.k,
                "higher close should give higher K: K1={} K2={}",
                v1.k,
                v2.k
            );
        }

        #[test]
        fn repaint_during_filling() {
            let mut kdj = kdj(3);
            kdj.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1));
            kdj.compute(&ohlc(12.0, 22.0, 8.0, 16.0, 1)); // repaint bar 1
            for t in 2..=10u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf + 5.0, tf + 15.0, tf, tf + 10.0, u64::from(t)));
            }
            assert!(kdj.value().is_some(), "expected value after convergence");
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut kdj = kdj(2);
            for t in 1..=8u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf, tf + 5.0, tf, tf + 2.5, u64::from(t)));
            }

            let mut cloned = kdj.clone();

            let orig = kdj.compute(&ohlc(20.0, 25.0, 15.0, 24.0, 9)).unwrap();
            let clone_val = cloned.compute(&ohlc(5.0, 10.0, 5.0, 6.0, 9)).unwrap();

            assert!(
                (orig.k - clone_val.k).abs() > 1e-10,
                "divergent inputs should give different K"
            );
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_indicator() {
            let kdj = kdj(9);
            assert_eq!(kdj.to_string(), "KDJ(9, 3, 3, Close)");
        }

        #[test]
        fn formats_indicator_custom() {
            let kdj = kdj_custom(9, 1, 1);
            assert_eq!(kdj.to_string(), "KDJ(9, 1, 1, Close)");
        }

        #[test]
        fn formats_config() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.to_string(), "KdjConfig(9, 3, 3, Close)");
        }

        #[test]
        fn formats_value() {
            let v = KdjValue {
                k: 75.5,
                d: 60.0,
                j: 106.5,
            };
            assert_eq!(v.to_string(), "Kdj(k: 75.5, d: 60, j: 106.5)");
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn close_helper_uses_close_source() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn period_accessor() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.period(), 9);
        }

        #[test]
        fn k_smooth_accessor() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.k_smooth(), 3);
        }

        #[test]
        fn d_smooth_accessor() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.d_smooth(), 3);
        }

        #[test]
        fn convergence_equals_sum() {
            // KDJ(9,3,3): 9+3+3 = 15
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.convergence(), 15);

            // KDJ(9,1,1): 9+1+1 = 11
            let config = KdjConfig::builder()
                .period(nz(9))
                .k_smooth(nz(1))
                .d_smooth(nz(1))
                .build();
            assert_eq!(config.convergence(), 11);
        }

        #[test]
        fn default_source_is_close() {
            let config = KdjConfig::builder().period(nz(9)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn default_k_d_smooth_are_3() {
            let config = KdjConfig::builder().period(nz(9)).build();
            assert_eq!(config.k_smooth(), 3);
            assert_eq!(config.d_smooth(), 3);
        }

        #[test]
        fn custom_source() {
            let config = KdjConfig::builder()
                .period(nz(9))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        fn custom_k_d_smooth() {
            let config = KdjConfig::builder()
                .period(nz(9))
                .k_smooth(nz(5))
                .d_smooth(nz(3))
                .build();
            assert_eq!(config.period(), 9);
            assert_eq!(config.k_smooth(), 5);
            assert_eq!(config.d_smooth(), 3);
        }

        #[test]
        #[should_panic(expected = "period is required")]
        fn panics_without_period() {
            let _ = KdjConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            let a = KdjConfig::close(nz(9));
            let b = KdjConfig::close(nz(9));
            let c = KdjConfig::close(nz(5));

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn eq_and_hash_with_custom_smoothing() {
            let a = KdjConfig::builder()
                .period(nz(9))
                .k_smooth(nz(5))
                .d_smooth(nz(3))
                .build();
            let b = KdjConfig::builder()
                .period(nz(9))
                .k_smooth(nz(5))
                .d_smooth(nz(3))
                .build();
            let c = KdjConfig::close(nz(9)); // default 3,3

            assert_eq!(a, b);
            assert_ne!(a, c);
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = KdjConfig::close(nz(9));
            assert_eq!(config.to_builder().build(), config);

            let custom = KdjConfig::builder()
                .period(nz(9))
                .k_smooth(nz(5))
                .d_smooth(nz(3))
                .build();
            assert_eq!(custom.to_builder().build(), custom);
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_hl2_source() {
            let mut kdj = Kdj::new(
                KdjConfig::builder()
                    .period(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            for t in 1..=7u64 {
                kdj.compute(&ohlc(15.0, 20.0, 10.0, 15.0, t));
            }
            let val = kdj.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 8));
            assert!(val.is_some(), "expected KDJ value at convergence");
            if let Some(v) = val {
                assert!(
                    (v.k - 50.0).abs() < 1e-10,
                    "K should be 50 for uniform HL2: {}",
                    v.k
                );
            }
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            assert_eq!(kdj(3).value(), None);
        }

        #[test]
        fn matches_last_compute() {
            let mut kdj = kdj(2);
            for t in 1..=7u32 {
                let tf = f64::from(t);
                kdj.compute(&ohlc(tf, tf + 2., tf, tf + 1., u64::from(t)));
            }
            let computed = kdj.compute(&ohlc(8.0, 10.0, 7.0, 9.0, 8));
            assert_eq!(kdj.value(), computed);
        }
    }
}
