use std::{
    fmt::{self, Display},
    num::NonZero,
};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, PriceSource,
    internals::{BarAction, BarState, RollingExtremes, RollingSum},
};

/// Configuration for the Stochastic Oscillator ([`Stoch`]) indicator.
///
/// The Stochastic Oscillator requires three parameters: the lookback
/// `length` for the highest-high / lowest-low window, a `k_smooth`
/// period for smoothing raw %K, and a `d_smooth` period for the %D
/// signal line (SMA of smoothed %K).
///
/// Output begins after `length + k_smooth` bars. The %D line
/// requires an additional `d_smooth` bars after %K converges.
///
/// # Example
///
/// ```
/// use quantedge_ta::StochConfig;
/// use std::num::NonZero;
///
/// let config = StochConfig::builder()
///     .length(NonZero::new(14).unwrap())
///     .k_smooth(NonZero::new(3).unwrap())
///     .d_smooth(NonZero::new(3).unwrap())
///     .build();
///
/// assert_eq!(config.length(), 14);
/// assert_eq!(config.k_smooth(), 3);
/// assert_eq!(config.d_smooth(), 3);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct StochConfig {
    length: usize,
    k_smooth: usize,
    d_smooth: usize,
    source: PriceSource,
}

impl IndicatorConfig for StochConfig {
    type Builder = StochConfigBuilder;

    fn builder() -> Self::Builder {
        StochConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        self.length + self.k_smooth
    }

    fn to_builder(&self) -> Self::Builder {
        StochConfigBuilder {
            length: Some(self.length),
            k_smooth: Some(self.k_smooth),
            d_smooth: Some(self.d_smooth),
            source: self.source,
        }
    }
}

impl StochConfig {
    /// Lookback length for the highest-high / lowest-low window.
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Smoothing period for %K (SMA of raw %K values).
    #[must_use]
    pub fn k_smooth(&self) -> usize {
        self.k_smooth
    }

    /// Smoothing period for %D (SMA of smoothed %K values).
    #[must_use]
    pub fn d_smooth(&self) -> usize {
        self.d_smooth
    }

    /// Stochastic Oscillator on closing price.
    #[must_use]
    pub fn close(
        length: NonZero<usize>,
        k_smooth: NonZero<usize>,
        d_smooth: NonZero<usize>,
    ) -> Self {
        Self::builder()
            .length(length)
            .k_smooth(k_smooth)
            .d_smooth(d_smooth)
            .build()
    }
}

impl Default for StochConfig {
    /// Default: length=14, `k_smooth`=3, `d_smooth`=3, source=Close
    /// (Slow Stochastic, `TradingView` default).
    fn default() -> Self {
        Self {
            length: 14,
            k_smooth: 3,
            d_smooth: 3,
            source: PriceSource::Close,
        }
    }
}

impl Display for StochConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "StochConfig({}, {}, {}, {})",
            self.length, self.k_smooth, self.d_smooth, self.source
        )
    }
}

/// Builder for [`StochConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// Length, `k_smooth`, and `d_smooth` must all be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct StochConfigBuilder {
    length: Option<usize>,
    k_smooth: Option<usize>,
    d_smooth: Option<usize>,
    source: PriceSource,
}

impl StochConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            k_smooth: None,
            d_smooth: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the lookback length for the highest-high / lowest-low window.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }

    /// Sets the %K smoothing period.
    #[must_use]
    pub fn k_smooth(mut self, k_smooth: NonZero<usize>) -> Self {
        self.k_smooth.replace(k_smooth.get());
        self
    }

    /// Sets the %D smoothing period.
    #[must_use]
    pub fn d_smooth(mut self, d_smooth: NonZero<usize>) -> Self {
        self.d_smooth.replace(d_smooth.get());
        self
    }
}

impl IndicatorConfigBuilder<StochConfig> for StochConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> StochConfig {
        StochConfig {
            length: self.length.expect("length is required"),
            k_smooth: self.k_smooth.expect("k_smooth is required"),
            d_smooth: self.d_smooth.expect("d_smooth is required"),
            source: self.source,
        }
    }
}

/// Stochastic Oscillator output: %K and %D lines.
///
/// %K is the smoothed stochastic value (0–100). %D is the signal
/// line (SMA of %K), which may be `None` until enough %K values
/// have been collected.
///
/// ```text
/// raw %K = (price − lowest_low) / (highest_high − lowest_low) × 100
/// %K     = SMA(raw %K, k_smooth)
/// %D     = SMA(%K, d_smooth)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StochValue {
    k: f64,
    d: Option<f64>,
}

impl StochValue {
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

impl Display for StochValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.d {
            Some(d) => write!(f, "Stoch(k: {}, d: {d})", self.k),
            None => write!(f, "Stoch(k: {}, d: -)", self.k),
        }
    }
}

/// Stochastic Oscillator (Stoch).
///
/// Compares the current price to the highest high and lowest low
/// over a lookback window, producing a 0–100 scale. Values above
/// 80 are conventionally considered overbought; below 20, oversold.
///
/// Raw %K is smoothed with an SMA of period `k_smooth` to produce
/// the %K line. The %D signal line is an SMA of %K with period
/// `d_smooth`.
///
/// ```text
/// raw %K = (price − lowest_low) / (highest_high − lowest_low) × 100
/// %K     = SMA(raw %K, k_smooth)
/// %D     = SMA(%K, d_smooth)
/// ```
///
/// Returns `None` until the lookback window and %K smoothing
/// window are both full (`length + k_smooth` bars). The %D
/// line within the output is `None` until an additional
/// `d_smooth` bars have been processed.
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # `TradingView` comparison
///
/// The default config matches `TradingView`'s "Stoch" indicator
/// (**Slow Stochastic**: 14/3/3). Set `k_smooth` to 1 for the
/// **Fast Stochastic**.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Stoch, StochConfig};
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
/// let config = StochConfig::builder()
///     .length(NonZero::new(3).unwrap())
///     .k_smooth(NonZero::new(1).unwrap())
///     .d_smooth(NonZero::new(1).unwrap())
///     .build();
/// let mut stoch = Stoch::new(config);
///
/// // Filling: need length + k_smooth = 4 bars
/// assert!(stoch.compute(&Bar { o: 10.0, h: 12.0, l: 8.0, c: 11.0, t: 1 }).is_none());
/// assert!(stoch.compute(&Bar { o: 11.0, h: 14.0, l: 9.0, c: 13.0, t: 2 }).is_none());
/// assert!(stoch.compute(&Bar { o: 13.0, h: 13.0, l: 10.0, c: 10.0, t: 3 }).is_none());
///
/// // Bar 4: lookback window and %K smoothing both full
/// let value = stoch.compute(&Bar { o: 12.0, h: 15.0, l: 11.0, c: 12.0, t: 4 });
/// assert!(value.is_some());
/// ```
#[derive(Clone, Debug)]
pub struct Stoch {
    config: StochConfig,
    bar_state: BarState,
    extremes: RollingExtremes,
    k_sum: RollingSum,
    d_sum: RollingSum,
    current: Option<StochValue>,
    k_reciprocal: f64,
    d_reciprocal: f64,
}

impl Indicator for Stoch {
    type Config = StochConfig;
    type Output = StochValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            bar_state: BarState::new(config.source),
            extremes: RollingExtremes::new(config.length),
            k_sum: RollingSum::new(config.k_smooth),
            d_sum: RollingSum::new(config.d_smooth),
            current: None,
            #[allow(clippy::cast_precision_loss)]
            k_reciprocal: 1.0 / config.k_smooth as f64,
            #[allow(clippy::cast_precision_loss)]
            d_reciprocal: 1.0 / config.d_smooth as f64,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        self.current = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                let k_sum = self
                    .extremes
                    .push(ohlcv)
                    .and_then(|(highest_high, lowest_low)| {
                        self.k_sum
                            .push(Self::k_value(price, highest_high, lowest_low))
                    });

                match k_sum {
                    Some(k_sum) => {
                        let k = k_sum * self.k_reciprocal;
                        let d = self.d_sum.push(k).map(|sum| sum * self.d_reciprocal);

                        Some(StochValue { k, d })
                    }
                    None => None,
                }
            }
            BarAction::Repaint(price) => {
                let k_sum = self
                    .extremes
                    .replace(ohlcv)
                    .and_then(|(highest_high, lowest_low)| {
                        self.k_sum
                            .replace(Self::k_value(price, highest_high, lowest_low))
                    });

                self.current.map(|current| {
                    let k = k_sum.expect("k_sum must be ready when current is Some")
                        * self.k_reciprocal;
                    let d_sum = self.d_sum.replace(k);

                    StochValue {
                        k,
                        d: current.d.map(|_| {
                            d_sum.expect("d_sum must be ready when d is Some") * self.d_reciprocal
                        }),
                    }
                })
            }
        };

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Stoch {
    pub(crate) fn k_value(price: f64, highest_high: f64, lowest_low: f64) -> f64 {
        let diff = highest_high - lowest_low;

        if diff.abs() < f64::EPSILON {
            50.0
        } else {
            ((price - lowest_low) / diff) * 100.0
        }
    }
}

impl Display for Stoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Stoch({}, {}, {}, {})",
            self.config.length, self.config.k_smooth, self.config.d_smooth, self.config.source
        )
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{nz, ohlc};

    /// Stoch(3,1,1) — simplest config: no smoothing, raw %K output.
    fn stoch_3_1_1() -> Stoch {
        let config = StochConfig::builder()
            .length(nz(3))
            .k_smooth(nz(1))
            .d_smooth(nz(1))
            .build();
        Stoch::new(config)
    }

    /// Returns a converged Stoch(3,1,1) after 4 bars.
    /// Bars: h/l = (12,8), (14,9), (13,10), (15,11) at times 1–4.
    fn seeded_stoch() -> Stoch {
        let mut s = stoch_3_1_1();
        s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
        s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
        s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
        s.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4));
        s
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_none_during_filling() {
            let mut s = stoch_3_1_1();
            assert!(s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1)).is_none());
            assert!(s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2)).is_none());
            assert!(s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3)).is_none());
        }

        #[test]
        fn first_value_at_length_plus_k_smooth() {
            // length=3, k_smooth=1 → first output at bar 4
            let mut s = stoch_3_1_1();
            s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            let val = s.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4));
            assert!(val.is_some());
        }

        #[test]
        fn k_smooth_delays_output() {
            // length=2, k_smooth=3 → first output at bar 5
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(3))
                .d_smooth(nz(1))
                .build();
            let mut s = Stoch::new(config);
            assert!(s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1)).is_none());
            assert!(s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2)).is_none());
            assert!(s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3)).is_none());
            assert!(s.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4)).is_none());
            assert!(s.compute(&ohlc(11.0, 13.0, 9.0, 12.0, 5)).is_some());
        }

        #[test]
        fn d_none_until_d_smooth_bars_after_k() {
            // length=2, k_smooth=1, d_smooth=3
            // First %K at bar 3. First %D at bar 6 (3+3).
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(1))
                .d_smooth(nz(3))
                .build();
            let mut s = Stoch::new(config);
            s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));

            // Bars 3–5: %K available, %D still None
            let v3 = s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3)).unwrap();
            assert!(v3.d().is_none());

            let v4 = s.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4)).unwrap();
            assert!(v4.d().is_none());

            let v5 = s.compute(&ohlc(11.0, 13.0, 9.0, 11.0, 5)).unwrap();
            assert!(v5.d().is_none());

            // Bar 6: 4th d_sum push → first %D
            let v6 = s.compute(&ohlc(10.0, 14.0, 8.0, 12.0, 6)).unwrap();
            assert!(v6.d().is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn raw_k_math() {
            // Stoch(3,1,1): bars with known highs/lows
            // After 4 bars: window bars 2-4, highest_high=15, lowest_low=9
            // close=12, raw %K = (12-9)/(15-9)*100 = 50
            let s = seeded_stoch();
            let val = s.value().unwrap();
            assert!((val.k() - 50.0).abs() < 1e-10);
        }

        #[test]
        fn flat_market_gives_50() {
            let mut s = stoch_3_1_1();
            for t in 1..=10 {
                let val = s.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
                if let Some(v) = val {
                    assert!(
                        (v.k() - 50.0).abs() < 1e-10,
                        "flat market should give %K=50, got {}",
                        v.k()
                    );
                }
            }
        }

        #[test]
        fn close_at_highest_high_gives_100() {
            let mut s = stoch_3_1_1();
            s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            // close=15 = highest_high in window
            let val = s.compute(&ohlc(12.0, 15.0, 11.0, 15.0, 4)).unwrap();
            assert!((val.k() - 100.0).abs() < 1e-10);
        }

        #[test]
        fn close_at_lowest_low_gives_0() {
            let mut s = stoch_3_1_1();
            s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            // close=9 = lowest_low in window
            let val = s.compute(&ohlc(12.0, 15.0, 9.0, 9.0, 4)).unwrap();
            assert!((val.k() - 0.0).abs() < 1e-10);
        }

        #[test]
        fn k_smooth_averages_raw_k() {
            // length=2, k_smooth=2, d_smooth=1
            // First %K at bar length + k_smooth = 4
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(2))
                .d_smooth(nz(1))
                .build();
            let mut s = Stoch::new(config);

            // Bar 1: h=20, l=10
            s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 1));
            // Bar 2: extremes ready. raw_k push 1 → None
            s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 2));
            // Bar 3: raw_k = (15-10)/(20-10)*100 = 50, push 2 → None (k_smooth=2, need 3 pushes)
            assert!(s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 3)).is_none());
            // Bar 4: raw_k = (20-10)/(20-10)*100 = 100, push 3 → Some
            // %K = SMA of last 2 raw_k: (50+100)/2 = 75
            let val = s.compute(&ohlc(15.0, 20.0, 10.0, 20.0, 4)).unwrap();
            assert!((val.k() - 75.0).abs() < 1e-10);
        }

        #[test]
        fn d_is_sma_of_k() {
            // length=2, k_smooth=1, d_smooth=2
            // First %K at bar 3 (2+1). First %D at bar 5 (3+2).
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(1))
                .d_smooth(nz(2))
                .build();
            let mut s = Stoch::new(config);

            s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 1));
            s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 2));

            // Bar 3: first %K, %D=None
            let v3 = s.compute(&ohlc(15.0, 20.0, 10.0, 15.0, 3)).unwrap();
            assert!(v3.d().is_none());

            // Bar 4: second %K, %D=None (need 3 d_sum pushes for d_smooth=2)
            let v4 = s.compute(&ohlc(15.0, 20.0, 10.0, 20.0, 4)).unwrap();
            assert!(v4.d().is_none());
            let k4 = v4.k();

            // Bar 5: third %K, %D = SMA of last 2 %K values
            let v5 = s.compute(&ohlc(15.0, 20.0, 10.0, 17.5, 5)).unwrap();
            let expected_d = f64::midpoint(k4, v5.k());
            assert!((v5.d().unwrap() - expected_d).abs() < 1e-10);
        }
    }

    mod repaints {
        use super::*;

        #[test]
        fn repaint_updates_value() {
            let mut s = seeded_stoch();
            let original = s.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 5)).unwrap();
            let repainted = s.compute(&ohlc(12.0, 16.0, 11.0, 16.0, 5)).unwrap();
            assert!(
                repainted.k() > original.k(),
                "higher close should give higher %K"
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut s = seeded_stoch();
            s.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 5));
            s.compute(&ohlc(12.0, 18.0, 10.0, 15.0, 5)); // repaint 1 (h up, l down)
            s.compute(&ohlc(12.0, 19.0, 9.0, 14.0, 5)); // repaint 2 (h up, l down)
            let final_val = s.compute(&ohlc(12.0, 20.0, 8.0, 12.0, 5)).unwrap();

            let mut clean = seeded_stoch();
            let expected = clean.compute(&ohlc(12.0, 20.0, 8.0, 12.0, 5)).unwrap();

            assert!((final_val.k() - expected.k()).abs() < 1e-10);
            assert_eq!(final_val.d().is_some(), expected.d().is_some());
            if let (Some(fd), Some(ed)) = (final_val.d(), expected.d()) {
                assert!((fd - ed).abs() < 1e-10);
            }
        }

        #[test]
        fn repaint_then_advance_uses_repainted() {
            let mut s = seeded_stoch();
            s.compute(&ohlc(12.0, 16.0, 11.0, 13.0, 5));
            s.compute(&ohlc(12.0, 16.0, 11.0, 15.0, 5)); // repaint bar 5
            let after = s.compute(&ohlc(14.0, 17.0, 12.0, 14.0, 6)).unwrap();

            let mut clean = seeded_stoch();
            clean.compute(&ohlc(12.0, 16.0, 11.0, 15.0, 5));
            let expected = clean.compute(&ohlc(14.0, 17.0, 12.0, 14.0, 6)).unwrap();

            assert!((after.k() - expected.k()).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling_has_no_effect_on_convergence() {
            let mut s = stoch_3_1_1();
            s.compute(&ohlc(10.0, 12.0, 8.0, 11.0, 1));
            s.compute(&ohlc(11.0, 14.0, 9.0, 13.0, 2));
            s.compute(&ohlc(11.0, 16.0, 7.0, 15.0, 2)); // repaint bar 2
            assert!(s.value().is_none()); // still filling
            s.compute(&ohlc(13.0, 13.0, 10.0, 10.0, 3));
            assert!(s.value().is_none()); // still filling
            let val = s.compute(&ohlc(12.0, 15.0, 11.0, 12.0, 4));
            assert!(val.is_some()); // now converged
        }

        #[test]
        fn repaint_during_k_smooth_filling_produces_correct_values() {
            // With k_smooth > 1, repainting while k_sum is still filling
            // must update the stale k_sum entry. Otherwise the smoothed %K
            // after convergence will be wrong.
            //
            // Stoch(length=2, k_smooth=3, d_smooth=1):
            //   extremes ready at bar 2, k_sum fills bars 2–4, first output bar 5.
            //   Repainting bars 2–4 with different closes must propagate
            //   through k_sum so bar 5 matches a clean (no-repaint) run.
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(3))
                .d_smooth(nz(1))
                .build();

            let bars: [(f64, f64, f64, f64, u64); 5] = [
                (10.0, 15.0, 8.0, 12.0, 1),
                (12.0, 16.0, 9.0, 14.0, 2),
                (14.0, 18.0, 11.0, 13.0, 3),
                (13.0, 17.0, 10.0, 15.0, 4),
                (15.0, 19.0, 12.0, 16.0, 5),
            ];

            // Clean run: one tick per bar
            let mut clean = Stoch::new(config);
            for &(o, h, l, c, t) in &bars {
                clean.compute(&ohlc(o, h, l, c, t));
            }
            let expected = clean.value().unwrap();

            // Repainted run: each bar gets an intermediate tick first
            let mut repainted = Stoch::new(config);
            for &(o, h, l, c, t) in &bars {
                // Intermediate tick with close near open
                repainted.compute(&ohlc(o, o + 0.5, o - 0.5, o + 0.1, t));
                // Final tick with real values
                repainted.compute(&ohlc(o, h, l, c, t));
            }
            let actual = repainted.value().unwrap();

            assert!(
                (actual.k() - expected.k()).abs() < 1e-10,
                "repaint during k_sum filling corrupted %K: got {}, expected {}",
                actual.k(),
                expected.k()
            );
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn k_always_between_0_and_100() {
            let mut s = stoch_3_1_1();
            let bars = [
                ohlc(10.0, 15.0, 5.0, 12.0, 1),
                ohlc(12.0, 20.0, 8.0, 18.0, 2),
                ohlc(18.0, 22.0, 6.0, 7.0, 3),
                ohlc(7.0, 25.0, 4.0, 24.0, 4),
                ohlc(24.0, 30.0, 3.0, 5.0, 5),
                ohlc(5.0, 10.0, 2.0, 9.0, 6),
                ohlc(9.0, 50.0, 1.0, 45.0, 7),
                ohlc(45.0, 48.0, 40.0, 42.0, 8),
            ];
            for b in &bars {
                if let Some(v) = s.compute(b) {
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
            let mut s = seeded_stoch();
            let mut cloned = s.clone();

            let orig = s.compute(&ohlc(12.0, 16.0, 11.0, 16.0, 5)).unwrap();
            let clone_val = cloned.compute(&ohlc(12.0, 16.0, 11.0, 9.0, 5)).unwrap();

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
            let config = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            assert_eq!(config.to_string(), "StochConfig(14, 3, 3, Close)");
        }

        #[test]
        fn display_stoch() {
            let config = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            let s = Stoch::new(config);
            assert_eq!(s.to_string(), "Stoch(14, 3, 3, Close)");
        }

        #[test]
        fn display_value_with_d() {
            let v = StochValue {
                k: 75.5,
                d: Some(60.0),
            };
            assert_eq!(v.to_string(), "Stoch(k: 75.5, d: 60)");
        }

        #[test]
        fn display_value_without_d() {
            let v = StochValue { k: 75.5, d: None };
            assert_eq!(v.to_string(), "Stoch(k: 75.5, d: -)");
        }
    }

    mod config {
        use super::*;

        #[test]
        fn default_source_is_close() {
            let config = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = StochConfig::builder()
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
        }

        #[test]
        #[should_panic(expected = "k_smooth is required")]
        fn panics_without_k_smooth() {
            let _ = StochConfig::builder()
                .length(nz(14))
                .d_smooth(nz(3))
                .build();
        }

        #[test]
        #[should_panic(expected = "d_smooth is required")]
        fn panics_without_d_smooth() {
            let _ = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .build();
        }

        #[test]
        fn eq_and_hash() {
            use std::collections::HashSet;
            let a = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            let b = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build();
            let c = StochConfig::builder()
                .length(nz(7))
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
            let config = StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(5))
                .build();
            assert_eq!(config.length(), 14);
            assert_eq!(config.k_smooth(), 3);
            assert_eq!(config.d_smooth(), 5);
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = StochConfig::close(nz(14), nz(3), nz(3));
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_configured_source() {
            // Use HL2 = (high+low)/2 as price source
            // First %K at bar 3 (length=2, k_smooth=1)
            let config = StochConfig::builder()
                .length(nz(2))
                .k_smooth(nz(1))
                .d_smooth(nz(1))
                .source(PriceSource::HL2)
                .build();
            let mut s = Stoch::new(config);

            // Bar 1: h=20, l=10 → HL2=15
            s.compute(&ohlc(15.0, 20.0, 10.0, 5.0, 1));
            // Bar 2: h=20, l=10 → HL2=15 (extremes ready, k_sum push 1 → None)
            assert!(s.compute(&ohlc(15.0, 20.0, 10.0, 5.0, 2)).is_none());
            // Bar 3: h=20, l=10 → HL2=15; hh=20, ll=10
            // raw_k = (15-10)/(20-10)*100 = 50
            let val = s.compute(&ohlc(15.0, 20.0, 10.0, 5.0, 3)).unwrap();
            assert!((val.k() - 50.0).abs() < 1e-10);
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let s = stoch_3_1_1();
            assert_eq!(s.value(), None);
        }

        #[test]
        fn matches_last_compute() {
            let mut s = seeded_stoch();
            let computed = s.compute(&ohlc(12.0, 16.0, 11.0, 14.0, 5));
            assert_eq!(s.value(), computed);
        }

        #[test]
        fn value_k_and_d_accessors() {
            // seeded_stoch has 4 bars. stoch_3_1_1: first %K at bar 4, first %D at bar 5.
            // So after 4 bars, %K is present but %D is None.
            let mut s = seeded_stoch();
            let val = s.value().unwrap();
            assert!(val.k().is_finite());
            assert!(val.d().is_none());

            // Bar 5: %D now available
            let v5 = s.compute(&ohlc(13.0, 16.0, 10.0, 14.0, 5)).unwrap();
            assert!(v5.k().is_finite());
            assert!(v5.d().is_some());
            assert!(v5.d().unwrap().is_finite());
        }
    }
}
