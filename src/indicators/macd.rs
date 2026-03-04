use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Price, PriceSource, Timestamp,
    internals::EmaCore,
};

/// Configuration for the Moving Average Convergence Divergence ([`Macd`])
/// indicator.
///
/// # Convergence
///
/// MACD output begins when both EMAs have converged (after `slow_period`
/// bars). The signal line requires an additional `signal_period` bars of
/// MACD values to seed its own EMA. Full output (MACD, signal, histogram)
/// is available at bar `slow_period + signal_period - 1`.
///
/// # Panics
///
/// Building a config panics if `fast_period >= slow_period`.
///
/// # Example
///
/// ```
/// use quantedge_ta::MacdConfig;
/// use std::num::NonZero;
///
/// let config = MacdConfig::close(
///     NonZero::new(12).unwrap(),
///     NonZero::new(26).unwrap(),
///     NonZero::new(9).unwrap(),
/// );
/// assert_eq!(config.fast_period(), 12);
/// assert_eq!(config.slow_period(), 26);
/// assert_eq!(config.signal_period(), 9);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct MacdConfig {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    source: PriceSource,
}

impl IndicatorConfig for MacdConfig {
    type Builder = MacdConfigBuilder;

    #[inline]
    fn builder() -> Self::Builder {
        MacdConfigBuilder::new()
    }

    #[inline]
    fn source(&self) -> PriceSource {
        self.source
    }
}

impl MacdConfig {
    /// MACD on closing price with the given periods.
    #[must_use]
    pub fn close(fast: NonZero<usize>, slow: NonZero<usize>, signal: NonZero<usize>) -> Self {
        Self::builder()
            .fast_period(fast)
            .slow_period(slow)
            .signal_period(signal)
            .build()
    }

    /// MACD(12, 26, 9) on closing price — the standard setting.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn default_close() -> Self {
        Self::close(
            NonZero::new(12).unwrap(),
            NonZero::new(26).unwrap(),
            NonZero::new(9).unwrap(),
        )
    }

    /// Fast EMA period.
    #[must_use]
    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    /// Slow EMA period.
    #[must_use]
    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    /// Signal EMA period.
    #[must_use]
    pub fn signal_period(&self) -> usize {
        self.signal_period
    }

    /// Bars until MACD line values are fully converged.
    #[must_use]
    pub fn convergence_bars(&self) -> usize {
        EmaCore::bars_to_converge(self.slow_period)
    }

    /// Bars until all outputs (including signal) are fully converged.
    #[must_use]
    pub fn full_convergence_bars(&self) -> usize {
        self.convergence_bars() + self.signal_period - 1
    }
}

impl Display for MacdConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MacdConfig({}, {}, {}, {})",
            self.fast_period, self.slow_period, self.signal_period, self.source
        )
    }
}

/// Builder for [`MacdConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// `fast_period`, `slow_period`, and `signal_period` must be set before
/// calling [`build`](IndicatorConfigBuilder::build).
pub struct MacdConfigBuilder {
    fast_period: Option<usize>,
    slow_period: Option<usize>,
    signal_period: Option<usize>,
    source: PriceSource,
}

impl MacdConfigBuilder {
    fn new() -> Self {
        Self {
            fast_period: None,
            slow_period: None,
            signal_period: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the fast EMA period.
    #[inline]
    #[must_use]
    pub fn fast_period(mut self, period: NonZero<usize>) -> Self {
        self.fast_period.replace(period.get());
        self
    }

    /// Sets the slow EMA period.
    #[inline]
    #[must_use]
    pub fn slow_period(mut self, period: NonZero<usize>) -> Self {
        self.slow_period.replace(period.get());
        self
    }

    /// Sets the signal EMA period.
    #[inline]
    #[must_use]
    pub fn signal_period(mut self, period: NonZero<usize>) -> Self {
        self.signal_period.replace(period.get());
        self
    }
}

impl IndicatorConfigBuilder<MacdConfig> for MacdConfigBuilder {
    #[inline]
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    #[inline]
    fn build(self) -> MacdConfig {
        let fast = self.fast_period.expect("fast_period is required");
        let slow = self.slow_period.expect("slow_period is required");
        assert!(fast < slow, "fast_period must be less than slow_period");
        MacdConfig {
            fast_period: fast,
            slow_period: slow,
            signal_period: self.signal_period.expect("signal_period is required"),
            source: self.source,
        }
    }
}

/// MACD indicator output: line, signal, and histogram.
///
/// The MACD line is always present once the indicator has converged.
/// The signal line and histogram are `None` until the signal EMA
/// has accumulated enough MACD values to complete its SMA seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdValue {
    macd: f64,
    signal: Option<f64>,
    histogram: Option<f64>,
}

impl MacdValue {
    /// MACD line: fast EMA minus slow EMA.
    #[must_use]
    pub fn macd(&self) -> f64 {
        self.macd
    }

    /// Signal line: EMA of the MACD line.
    /// `None` during the signal seeding phase.
    #[must_use]
    pub fn signal(&self) -> Option<f64> {
        self.signal
    }

    /// Histogram: MACD minus signal.
    /// `None` when signal is not yet available.
    #[must_use]
    pub fn histogram(&self) -> Option<f64> {
        self.histogram
    }
}

impl Display for MacdValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let signal = self.signal.map_or("-".to_string(), |v| format!("{v:.4}"));
        let histogram = self
            .histogram
            .map_or("-".to_string(), |v| format!("{v:.4}"));

        write!(
            f,
            "MacdValue(m: {}, s: {}, h: {})",
            self.macd, signal, histogram
        )
    }
}

/// Moving Average Convergence Divergence (MACD).
///
/// A trend-following momentum indicator that shows the relationship
/// between two exponential moving averages. The MACD line is the
/// difference between the fast and slow EMAs; the signal line is an
/// EMA of the MACD line; the histogram is MACD minus signal.
///
/// ```text
/// MACD      = EMA(fast) − EMA(slow)
/// signal    = EMA(signal_period, MACD)
/// histogram = MACD − signal
/// ```
///
/// Each EMA is seeded with an SMA of its first `period` values.
/// After seeding, all updates are O(1) per tick.
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Macd, MacdConfig};
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
/// let mut macd = Macd::new(MacdConfig::close(
///     NonZero::new(3).unwrap(),
///     NonZero::new(6).unwrap(),
///     NonZero::new(4).unwrap(),
/// ));
///
/// // Feed bars until the slow EMA converges
/// for i in 1..=5 {
///     assert_eq!(macd.compute(&Bar(i as f64 * 10.0, i as u64)), None);
/// }
///
/// // MACD line available at bar 6 (slow_period)
/// let val = macd.compute(&Bar(60.0, 6));
/// assert!(val.is_some());
/// ```
#[derive(Clone, Debug)]
pub struct Macd {
    fast: EmaCore,
    slow: EmaCore,
    signal: EmaCore,
    config: MacdConfig,
    current: Option<MacdValue>,
    last_open_time: Option<Timestamp>,
    curr_close: Option<Price>,
    prev_close: Option<Price>,
}

impl Indicator for Macd {
    type Config = MacdConfig;
    type Output = MacdValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            fast: EmaCore::new(config.fast_period(), false),
            slow: EmaCore::new(config.slow_period(), false),
            signal: EmaCore::new(config.signal_period(), false),
            current: None,
            last_open_time: None,
            curr_close: None,
            prev_close: None,
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

        if is_next_bar {
            self.prev_close = self.curr_close;
            self.last_open_time = Some(ohlcv.open_time());
        }

        let price = self.config.source().extract(ohlcv, self.prev_close);
        self.curr_close = Some(ohlcv.close());

        let signal_value = if is_next_bar {
            let fast_value = self.fast.push(price);
            let slow_value = self.slow.push(price);

            match (fast_value, slow_value) {
                (Some(fast), Some(slow)) => {
                    let macd = fast - slow;
                    let signal = self.signal.push(macd);

                    Some((macd, signal))
                }
                _ => None,
            }
        } else {
            let fast_value = self.fast.replace(price);
            let slow_value = self.slow.replace(price);

            match (fast_value, slow_value) {
                (Some(fast), Some(slow)) => {
                    let macd = fast - slow;
                    let signal = self.signal.replace(macd);

                    Some((macd, signal))
                }
                _ => None,
            }
        };

        self.current = match signal_value {
            Some((macd, signal)) => Some(MacdValue {
                macd,
                signal,
                histogram: signal.map(|v| macd - v),
            }),
            _ => None,
        };

        self.current
    }

    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Display for Macd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MACD({}, {}, {}, {})",
            self.config.fast_period,
            self.config.slow_period,
            self.config.signal_period,
            self.config.source
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Bar, bar, nz};

    /// Standard MACD(3, 6, 4) on close for tractable hand calculations.
    /// Fast α = 2/4 = 0.5, Slow α = 2/7 ≈ 0.2857, Signal α = 2/5 = 0.4
    fn macd_3_6_4() -> Macd {
        Macd::new(MacdConfig::close(nz(3), nz(6), nz(4)))
    }

    /// Standard MACD(12, 26, 9) on close.
    fn macd_default() -> Macd {
        Macd::new(MacdConfig::default_close())
    }

    /// Feed bars 1..=n with incrementing prices, return the MACD.
    fn feed_sequential(macd: &mut Macd, n: u64) {
        for i in 1..=n {
            #[allow(clippy::cast_precision_loss)]
            macd.compute(&bar(i as f64 * 10.0, i));
        }
    }

    mod convergence {
        use super::*;

        #[test]
        fn none_before_slow_ema_converges() {
            // MACD(3,6,4): slow EMA needs 6 bars to seed
            let mut macd = macd_3_6_4();
            for i in 1..=5 {
                #[allow(clippy::cast_precision_loss)]
                let f = i as f64;
                assert_eq!(
                    macd.compute(&bar(f * 10.0, i)),
                    None,
                    "expected None at bar {i}"
                );
            }
        }

        #[test]
        fn macd_line_available_at_slow_period() {
            let mut macd = macd_3_6_4();
            for i in 1..=5 {
                #[allow(clippy::cast_precision_loss)]
                macd.compute(&bar(i as f64 * 10.0, i));
            }
            let val = macd.compute(&bar(60.0, 6));
            assert!(val.is_some(), "MACD should produce output at bar 6");
        }

        #[test]
        fn signal_none_during_signal_seed() {
            // MACD(3,6,4): MACD line at bar 6, signal needs 4 MACD values
            // Signal available at bar 6 + 4 - 1 = 9
            let mut macd = macd_3_6_4();
            for i in 1..=7 {
                #[allow(clippy::cast_precision_loss)]
                macd.compute(&bar(i as f64 * 10.0, i));
            }
            let val = macd.value().unwrap();
            assert!(val.signal().is_none(), "signal should be None during seed");
            assert!(
                val.histogram().is_none(),
                "histogram should be None during seed"
            );
        }

        #[test]
        fn full_output_at_slow_plus_signal_minus_one() {
            // MACD(3,6,4): full output at bar 9
            let mut macd = macd_3_6_4();
            for i in 1..=9 {
                #[allow(clippy::cast_precision_loss)]
                macd.compute(&bar(i as f64 * 10.0, i));
            }
            let val = macd.value().unwrap();
            assert!(val.signal().is_some(), "signal should be Some at bar 9");
            assert!(
                val.histogram().is_some(),
                "histogram should be Some at bar 9"
            );
        }

        #[test]
        fn value_none_before_convergence() {
            let macd = macd_3_6_4();
            assert_eq!(macd.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut macd = macd_3_6_4();
            feed_sequential(&mut macd, 10);
            let computed = macd.compute(&bar(110.0, 11));
            assert_eq!(macd.value(), computed);
        }
    }

    mod macd_line {
        use super::*;

        #[test]
        fn equals_fast_minus_slow() {
            // Verify MACD line = EMA(fast) - EMA(slow) by running
            // separate EMAs and comparing
            let mut macd = macd_3_6_4();
            let mut fast = crate::Ema::new(crate::EmaConfig::close(nz(3)));
            let mut slow = crate::Ema::new(crate::EmaConfig::close(nz(6)));
            let prices = [10.0, 25.0, 18.0, 30.0, 22.0, 35.0, 28.0, 40.0, 33.0, 45.0];

            for (i, &p) in prices.iter().enumerate() {
                let b = bar(p, (i + 1) as u64);
                macd.compute(&b);
                fast.compute(&b);
                slow.compute(&b);

                if let (Some(f), Some(s)) = (fast.value(), slow.value()) {
                    let expected_macd = f - s;
                    let actual = macd.value().unwrap().macd();
                    assert!(
                        (actual - expected_macd).abs() < 1e-10,
                        "MACD line mismatch at bar {}: expected {expected_macd}, got {actual}",
                        i + 1
                    );
                }
            }
        }

        #[test]
        fn zero_when_constant_price() {
            let mut macd = macd_3_6_4();
            for i in 1..=10 {
                macd.compute(&bar(50.0, i));
            }
            let val = macd.value().unwrap();
            assert!(
                (val.macd()).abs() < 1e-10,
                "MACD should be 0 for constant price"
            );
        }
    }

    mod signal_line {
        use super::*;

        #[test]
        fn signal_is_ema_of_macd_line() {
            // Collect MACD values manually, compute signal EMA, compare
            let mut macd = macd_3_6_4();
            let mut macd_values: Vec<f64> = Vec::new();
            let prices = [
                10.0, 25.0, 18.0, 30.0, 22.0, 35.0, 28.0, 40.0, 33.0, 45.0, 38.0, 50.0, 43.0, 55.0,
                48.0,
            ];

            for (i, &p) in prices.iter().enumerate() {
                macd.compute(&bar(p, (i + 1) as u64));
                if let Some(val) = macd.value() {
                    macd_values.push(val.macd());
                }
            }

            // Manually compute EMA(4) of macd_values
            // SMA seed = first 4 MACD values
            assert!(macd_values.len() >= 4, "need at least 4 MACD values");
            let sma_seed: f64 = macd_values[..4].iter().sum::<f64>() / 4.0;
            let alpha = 2.0 / 5.0; // signal_period = 4

            let mut expected_signal = sma_seed;
            // Compareat the point where signal becomes available (4th MACD value)
            // The signal becomes available partway through our feed, verify last value
            // by continuing the manual EMA
            for &mv in &macd_values[4..] {
                expected_signal = alpha * mv + (1.0 - alpha) * expected_signal;
            }
            let last_val = macd.value().unwrap();
            let actual_signal = last_val.signal().unwrap();
            assert!(
                (actual_signal - expected_signal).abs() < 1e-10,
                "signal mismatch: expected {expected_signal}, got {actual_signal}"
            );
        }

        #[test]
        fn zero_when_constant_price() {
            let mut macd = macd_3_6_4();
            for i in 1..=15 {
                macd.compute(&bar(50.0, i));
            }
            let val = macd.value().unwrap();
            assert!((val.signal().unwrap()).abs() < 1e-10);
        }
    }
    mod histogram {
        use super::*;

        #[test]
        fn equals_macd_minus_signal() {
            let mut macd = macd_3_6_4();
            for i in 1..=15 {
                #[allow(clippy::cast_precision_loss)]
                let f = i as f64;
                macd.compute(&bar(f * 10.0 + f.sin() * 5.0, i));
            }
            let val = macd.value().unwrap();
            if let (Some(sig), Some(hist)) = (val.signal(), val.histogram()) {
                let expected = val.macd() - sig;
                assert!(
                    (hist - expected).abs() < 1e-10,
                    "histogram should equal macd - signal"
                );
            }
        }

        #[test]
        fn zero_when_constant_price() {
            let mut macd = macd_3_6_4();
            for i in 1..=15 {
                macd.compute(&bar(50.0, i));
            }
            let val = macd.value().unwrap();
            assert!((val.histogram().unwrap()).abs() < 1e-10);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn forming_bar_updates_value() {
            let mut macd = macd_3_6_4();
            feed_sequential(&mut macd, 9);

            let original = macd.compute(&bar(100.0, 10));
            let repainted = macd.compute(&bar(200.0, 10));

            assert_ne!(original, repainted, "repaint should change the value");
        }

        #[test]
        fn repaint_matches_clean() {
            // Repainted path
            let mut repainted = macd_3_6_4();
            feed_sequential(&mut repainted, 9);
            repainted.compute(&bar(100.0, 10));
            repainted.compute(&bar(150.0, 10)); // repaint
            let val_repaint = repainted.compute(&bar(110.0, 11));
            // Clean path (bar 10 was always 150)
            let mut clean = macd_3_6_4();
            feed_sequential(&mut clean, 9);
            clean.compute(&bar(150.0, 10));
            let val_clean = clean.compute(&bar(110.0, 11));

            assert_eq!(val_repaint, val_clean);
        }

        #[test]
        fn multiple_repaints_stable() {
            let mut macd = macd_3_6_4();
            feed_sequential(&mut macd, 9);
            macd.compute(&bar(100.0, 10));
            macd.compute(&bar(120.0, 10));
            macd.compute(&bar(140.0, 10));
            let final_val = macd.compute(&bar(130.0, 10));
            let mut clean = macd_3_6_4();
            feed_sequential(&mut clean, 9);
            let expected = clean.compute(&bar(130.0, 10));

            assert_eq!(final_val, expected);
        }

        #[test]
        fn repaint_during_signal_seed() {
            // Repaint when MACD line exists but signal is still seeding
            let mut macd = macd_3_6_4();
            feed_sequential(&mut macd, 7); // bar 7: MACD line exists, signal seeding

            let original = macd.compute(&bar(80.0, 8));
            let repainted = macd.compute(&bar(90.0, 8));
            assert_ne!(original, repainted);

            // Verify signal still None
            assert!(macd.value().unwrap().signal().is_none());
        }

        #[test]
        fn repaint_then_signal_seed_completes_correctly() {
            let mut repainted = macd_3_6_4();
            feed_sequential(&mut repainted, 7);
            repainted.compute(&bar(80.0, 8));
            repainted.compute(&bar(85.0, 8)); // repaint
            let val_r = repainted.compute(&bar(90.0, 9)); // signal should appear
            let mut clean = macd_3_6_4();
            feed_sequential(&mut clean, 7);
            clean.compute(&bar(85.0, 8));
            let val_c = clean.compute(&bar(90.0, 9));

            assert_eq!(val_r, val_c);
        }
    }

    mod edge_cases {
        use super::*;

        #[test]
        fn constant_price_all_zeros() {
            let mut macd = macd_3_6_4();
            for i in 1..=20 {
                macd.compute(&bar(100.0, i));
            }
            let val = macd.value().unwrap();
            assert!((val.macd()).abs() < 1e-10);
            assert!((val.signal().unwrap()).abs() < 1e-10);
            assert!((val.histogram().unwrap()).abs() < 1e-10);
        }
        #[test]
        fn trending_up_positive_macd() {
            let mut macd = macd_3_6_4();
            // Strong uptrend: fast EMA > slow EMA → positive MACD
            for i in 1..=15 {
                #[allow(clippy::cast_precision_loss)]
                macd.compute(&bar(i as f64 * 10.0, i));
            }
            assert!(macd.value().unwrap().macd() > 0.0);
        }

        #[test]
        fn trending_down_negative_macd() {
            let mut macd = macd_3_6_4();
            // Strong downtrend: fast EMA < slow EMA → negative MACD
            for i in 1..=15 {
                #[allow(clippy::cast_precision_loss)]
                macd.compute(&bar(200.0 - i as f64 * 10.0, i));
            }
            assert!(macd.value().unwrap().macd() < 0.0);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut macd = macd_3_6_4();
            feed_sequential(&mut macd, 10);

            let mut cloned = macd.clone();
            let orig = macd.compute(&bar(200.0, 11)).unwrap();
            let clone = cloned.compute(&bar(50.0, 11)).unwrap();

            assert!(
                (orig.macd() - clone.macd()).abs() > 1e-10,
                "divergent inputs should give different MACD"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_is_12_26_9() {
            let config = MacdConfig::default_close();
            assert_eq!(config.fast_period(), 12);
            assert_eq!(config.slow_period(), 26);
            assert_eq!(config.signal_period(), 9);
        }

        #[test]
        fn custom_periods() {
            let config = MacdConfig::close(nz(5), nz(15), nz(7));
            assert_eq!(config.fast_period(), 5);
            assert_eq!(config.slow_period(), 15);
            assert_eq!(config.signal_period(), 7);
        }

        #[test]
        fn default_source_is_close() {
            let config = MacdConfig::default_close();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "fast_period must be less than slow_period")]
        fn panics_when_fast_equals_slow() {
            let _ = MacdConfig::close(nz(12), nz(12), nz(9));
        }

        #[test]
        #[should_panic(expected = "fast_period must be less than slow_period")]
        fn panics_when_fast_greater_than_slow() {
            let _ = MacdConfig::close(nz(26), nz(12), nz(9));
        }
        #[test]
        fn eq_and_hash() {
            let a = MacdConfig::default_close();
            let b = MacdConfig::default_close();
            let c = MacdConfig::close(nz(5), nz(15), nz(7));

            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = MacdConfig::default_close();
            assert_eq!(config.to_string(), "MacdConfig(12, 26, 9, Close)");
        }

        #[test]
        fn display_macd() {
            let macd = macd_default();
            assert_eq!(macd.to_string(), "MACD(12, 26, 9, Close)");
        }
        #[test]
        fn display_value_full() {
            let val = MacdValue {
                macd: 1.5,
                signal: Some(1.2),
                histogram: Some(0.3),
            };
            let s = val.to_string();
            assert!(s.contains("1.5"));
            assert!(s.contains("1.2"));
            assert!(s.contains("0.3"));
        }

        #[test]
        fn display_value_partial() {
            let val = MacdValue {
                macd: 1.5,
                signal: None,
                histogram: None,
            };
            let s = val.to_string();
            assert!(s.contains("1.5"));
            assert!(s.contains('-'));
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_configured_source() {
            let config = MacdConfig::builder()
                .fast_period(nz(3))
                .slow_period(nz(6))
                .signal_period(nz(4))
                .source(PriceSource::HL2)
                .build();
            let mut macd = Macd::new(config);
            // HL2 = (high + low) / 2
            // Feed bars where close differs from HL2
            for i in 1..=10 {
                #[allow(clippy::cast_precision_loss)]
                let f = i as f64;
                let b = Bar::new(
                    f * 10.0, // open
                    f * 12.0, // high
                    f * 8.0,  // low
                    f * 11.0, // close (different from HL2)
                )
                .at(i);
                macd.compute(&b);
            }

            // Compare with MACD using close on the same HL2 values
            let mut macd_close = Macd::new(MacdConfig::close(nz(3), nz(6), nz(4)));
            for i in 1..=10 {
                #[allow(clippy::cast_precision_loss)]
                let hl2 = i as f64 * 10.0;
                macd_close.compute(&bar(hl2, i));
            }
            let val_hl2 = macd.value().unwrap();
            let val_close = macd_close.value().unwrap();
            assert!(
                (val_hl2.macd() - val_close.macd()).abs() < 1e-10,
                "HL2 source should match close source fed with HL2 values"
            );
        }
    }

    #[cfg(debug_assertions)]
    mod invariants {
        use super::*;

        #[test]
        #[should_panic(expected = "open_time must be non-decreasing")]
        fn panics_on_decreasing_open_time() {
            let mut macd = macd_3_6_4();
            macd.compute(&bar(10.0, 2));
            macd.compute(&bar(12.0, 1));
        }
    }
}
