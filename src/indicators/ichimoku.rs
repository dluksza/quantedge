use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Price,
    internals::{BarAction, BarState, RingBuffer, RollingExtremes},
};

/// Configuration for the Ichimoku Cloud ([`Ichimoku`]) indicator.
///
/// The Ichimoku Cloud (Ichimoku Kinko Hyo) is a comprehensive trend
/// indicator that defines support/resistance, trend direction, and
/// momentum using five lines derived from high/low midpoints over
/// different lookback windows.
///
/// Requires four parameters: `tenkan_length` (conversion line),
/// `kijun_length` (base line), `senkou_b_length` (leading span B),
/// and `displacement` (cloud shift). The standard settings are
/// 9, 26, 52, 26.
///
/// Output begins after `senkou_b_length + displacement + 1` bars.
///
/// # Example
///
/// ```
/// use quantedge_ta::IchimokuConfig;
/// use std::num::NonZero;
///
/// let config = IchimokuConfig::builder()
///     .default()
///     .build();
///
/// assert_eq!(config.tenkan_length(), 9);
/// assert_eq!(config.kijun_length(), 26);
/// assert_eq!(config.senkou_b_length(), 52);
/// assert_eq!(config.displacement(), 26);
/// assert_eq!(config.convergence(), 79);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct IchimokuConfig {
    tenkan_length: usize,
    kijun_length: usize,
    senkou_b_length: usize,
    displacement: usize,
}

impl IndicatorConfig for IchimokuConfig {
    type Builder = IchimokuBuilder;

    fn builder() -> Self::Builder {
        IchimokuBuilder::new()
    }

    fn source(&self) -> crate::PriceSource {
        crate::PriceSource::Close
    }

    fn convergence(&self) -> usize {
        let span_a = self.kijun_length.max(self.tenkan_length) + self.displacement;
        let span_b = self.senkou_b_length + self.displacement + 1;
        span_a.max(span_b)
    }
}

impl IchimokuConfig {
    /// Tenkan-sen (conversion line) lookback length.
    #[must_use]
    pub fn tenkan_length(&self) -> usize {
        self.tenkan_length
    }

    /// Kijun-sen (base line) lookback length.
    #[must_use]
    pub fn kijun_length(&self) -> usize {
        self.kijun_length
    }

    /// Senkou Span B lookback length.
    #[must_use]
    pub fn senkou_b_length(&self) -> usize {
        self.senkou_b_length
    }

    /// Cloud displacement (number of bars to shift forward).
    #[must_use]
    pub fn displacement(&self) -> usize {
        self.displacement
    }
}

impl Display for IchimokuConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IchimokuConfig(tenkan: {}, kijun: {}, senkou_b: {}, displacement: {})",
            self.tenkan_length, self.kijun_length, self.senkou_b_length, self.displacement
        )
    }
}

/// Builder for [`IchimokuConfig`].
///
/// All four parameters must be set before calling
/// [`build`](IndicatorConfigBuilder::build). Use [`default()`](Self::default)
/// to set the standard 9/26/52/26 parameters.
pub struct IchimokuBuilder {
    tenkan_length: Option<usize>,
    kijun_length: Option<usize>,
    senkou_b_length: Option<usize>,
    displacement: Option<usize>,
}

impl IchimokuBuilder {
    fn new() -> Self {
        Self {
            tenkan_length: None,
            kijun_length: None,
            senkou_b_length: None,
            displacement: None,
        }
    }

    /// Sets all parameters to the standard Ichimoku values:
    /// tenkan=9, kijun=26, `senkou_b`=52, displacement=26.
    #[must_use]
    pub fn default(mut self) -> Self {
        self.tenkan_length.replace(9);
        self.kijun_length.replace(26);
        self.senkou_b_length.replace(52);
        self.displacement.replace(26);
        self
    }

    /// Sets the Tenkan-sen (conversion line) lookback length.
    #[must_use]
    pub fn tenkan_length(mut self, value: NonZero<usize>) -> Self {
        self.tenkan_length.replace(value.get());
        self
    }

    /// Sets the Kijun-sen (base line) lookback length.
    #[must_use]
    pub fn kijun_length(mut self, value: NonZero<usize>) -> Self {
        self.kijun_length.replace(value.get());
        self
    }

    /// Sets the Senkou Span B lookback length.
    #[must_use]
    pub fn senkou_b_length(mut self, value: NonZero<usize>) -> Self {
        self.senkou_b_length.replace(value.get());
        self
    }

    /// Sets the cloud displacement (number of bars shifted forward).
    #[must_use]
    pub fn displacement(mut self, value: NonZero<usize>) -> Self {
        self.displacement.replace(value.get());
        self
    }
}

impl IndicatorConfigBuilder<IchimokuConfig> for IchimokuBuilder {
    fn source(self, _source: crate::PriceSource) -> Self {
        self
    }

    fn build(self) -> IchimokuConfig {
        IchimokuConfig {
            tenkan_length: self.tenkan_length.expect("tenkan_length is required"),
            kijun_length: self.kijun_length.expect("kijun_length is required"),
            senkou_b_length: self.senkou_b_length.expect("senkou_b_length is required"),
            displacement: self.displacement.expect("displacement is required"),
        }
    }
}

/// Output of the Ichimoku Cloud indicator.
///
/// Contains five components: Tenkan-sen (conversion line), Kijun-sen
/// (base line), Senkou Span A (leading span A), Senkou Span B
/// (leading span B), and the Chikou close price (lagging span input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IchimokuValue {
    tenkan: Price,
    kijun: Price,
    senkou_a: Price,
    senkou_b: Price,
    chikou_close: Price,
}

impl IchimokuValue {
    /// Tenkan-sen (conversion line): midpoint of highest high and
    /// lowest low over the tenkan lookback window.
    #[inline]
    #[must_use]
    pub fn tenkan(&self) -> Price {
        self.tenkan
    }

    /// Kijun-sen (base line): midpoint of highest high and lowest
    /// low over the kijun lookback window.
    #[inline]
    #[must_use]
    pub fn kijun(&self) -> Price {
        self.kijun
    }

    /// Senkou Span A (leading span A): midpoint of Tenkan-sen and
    /// Kijun-sen, displaced forward.
    #[inline]
    #[must_use]
    pub fn senkou_a(&self) -> Price {
        self.senkou_a
    }

    /// Senkou Span B (leading span B): midpoint of highest high and
    /// lowest low over the `senkou_b` lookback window, displaced forward.
    #[inline]
    #[must_use]
    pub fn senkou_b(&self) -> Price {
        self.senkou_b
    }

    /// Chikou close price (lagging span input): the current bar's
    /// close price, plotted displaced bars back.
    #[inline]
    #[must_use]
    pub fn chikou_close(&self) -> Price {
        self.chikou_close
    }
}

impl Display for IchimokuValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ichimoku(tenkan: {}, kijun: {}, senkou_a: {}, senkou_b: {}, chikou_close: {})",
            self.tenkan, self.kijun, self.senkou_a, self.senkou_b, self.chikou_close
        )
    }
}

/// Ichimoku Cloud (Ichimoku Kinko Hyo).
///
/// A comprehensive trend indicator producing five lines from high/low
/// midpoints over different lookback windows:
///
/// - **Tenkan-sen** (conversion): midpoint of highest high and lowest
///   low over `tenkan_length` bars
/// - **Kijun-sen** (base): midpoint over `kijun_length` bars
/// - **Senkou Span A**: midpoint of Tenkan and Kijun, displaced
///   forward by `displacement` bars
/// - **Senkou Span B**: midpoint over `senkou_b_length` bars,
///   displaced forward by `displacement` bars
/// - **Chikou close**: the current bar's close price (the caller
///   plots it displaced backward)
///
/// Returns `None` until the displacement buffer is full (after
/// `senkou_b_length + displacement + 1` bars).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ichimoku, IchimokuConfig};
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
/// let config = IchimokuConfig::builder()
///     .tenkan_length(NonZero::new(2).unwrap())
///     .kijun_length(NonZero::new(3).unwrap())
///     .senkou_b_length(NonZero::new(4).unwrap())
///     .displacement(NonZero::new(2).unwrap())
///     .build();
/// let mut ich = Ichimoku::new(config);
///
/// // Returns None during convergence
/// assert!(ich.compute(&Bar { o: 10.0, h: 12.0, l: 8.0, c: 11.0, t: 1 }).is_none());
/// ```
#[derive(Clone, Debug)]
pub struct Ichimoku {
    config: IchimokuConfig,
    bar_state: BarState,
    tenkan_extremes: RollingExtremes,
    kijun_extremes: RollingExtremes,
    senkou_b_extremes: RollingExtremes,
    senkou_a_buffer: RingBuffer,
    senkou_b_buffer: RingBuffer,
    current: Option<IchimokuValue>,
}

impl Indicator for Ichimoku {
    type Config = IchimokuConfig;
    type Output = IchimokuValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            bar_state: BarState::new(crate::PriceSource::Close),
            tenkan_extremes: RollingExtremes::new(config.tenkan_length),
            kijun_extremes: RollingExtremes::new(config.kijun_length),
            senkou_b_extremes: RollingExtremes::new(config.senkou_b_length),
            senkou_a_buffer: RingBuffer::new(config.displacement),
            senkou_b_buffer: RingBuffer::new(config.displacement + 1),
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &impl crate::Ohlcv) -> Option<Self::Output> {
        self.current = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                let tenkan_sen = Self::price_midpoint(self.tenkan_extremes.push(ohlcv));
                let kijun_sen = Self::price_midpoint(self.kijun_extremes.push(ohlcv));
                let displaced_b = Self::price_midpoint(self.senkou_b_extremes.push(ohlcv))
                    .and_then(|sb| self.senkou_b_buffer.push(sb));

                if let (Some(tenkan), Some(kijun)) = (tenkan_sen, kijun_sen) {
                    let displaced_a = self.senkou_a_buffer.push(tenkan.midpoint(kijun));

                    if let (Some(senkou_a), Some(senkou_b)) = (displaced_a, displaced_b) {
                        Some(IchimokuValue {
                            tenkan,
                            kijun,
                            senkou_a,
                            senkou_b,
                            chikou_close: price,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            BarAction::Repaint(price) => {
                let tenkan_sen = Self::price_midpoint(self.tenkan_extremes.replace(ohlcv));
                let kijun_sen = Self::price_midpoint(self.kijun_extremes.replace(ohlcv));

                if let Some(senkou_b) = Self::price_midpoint(self.senkou_b_extremes.replace(ohlcv))
                {
                    self.senkou_b_buffer.replace(senkou_b);
                }

                if let (Some(tenkan), Some(kijun)) = (tenkan_sen, kijun_sen) {
                    self.senkou_a_buffer.replace(tenkan.midpoint(kijun));

                    self.current.map(|prev| IchimokuValue {
                        tenkan,
                        kijun,
                        senkou_a: prev.senkou_a,
                        senkou_b: prev.senkou_b,
                        chikou_close: price,
                    })
                } else {
                    None
                }
            }
        };

        self.current
    }

    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Ichimoku {
    fn price_midpoint(extremes: Option<(Price, Price)>) -> Option<Price> {
        extremes.map(|(highest_high, lowest_low)| highest_high.midpoint(lowest_low))
    }
}

impl Display for Ichimoku {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Ichimoku(tenkan: {}, kijun: {}, senkou_b: {}, displacement: {})",
            self.config.tenkan_length,
            self.config.kijun_length,
            self.config.senkou_b_length,
            self.config.displacement
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{nz, ohlc};

    fn default_config() -> IchimokuConfig {
        IchimokuConfig::builder().default().build()
    }

    /// Small config for unit tests: tenkan=2, kijun=3, `senkou_b=4`, displacement=2.
    /// Convergence = 4 + 2 + 1 = 7 bars.
    fn small_config() -> IchimokuConfig {
        IchimokuConfig::builder()
            .tenkan_length(nz(2))
            .kijun_length(nz(3))
            .senkou_b_length(nz(4))
            .displacement(nz(2))
            .build()
    }

    fn ichimoku(config: IchimokuConfig) -> Ichimoku {
        Ichimoku::new(config)
    }

    mod filling {
        use super::*;

        #[test]
        fn none_until_convergence() {
            let mut ich = ichimoku(small_config());
            // convergence = 4 + 2 + 1 = 7
            for t in 1..=6 {
                assert_eq!(
                    ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t)),
                    None,
                    "should be None at bar {t}"
                );
            }
        }

        #[test]
        fn returns_value_at_convergence() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            assert!(ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 7)).is_some());
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn tenkan_is_midpoint_of_window_extremes() {
            // tenkan_length=2: midpoint of last 2 bars' high/low
            let mut ich = ichimoku(small_config());
            // Feed enough bars to converge (7)
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            ich.compute(&ohlc(10.0, 18.0, 6.0, 14.0, 2));
            ich.compute(&ohlc(10.0, 22.0, 4.0, 16.0, 3));
            ich.compute(&ohlc(10.0, 19.0, 7.0, 13.0, 4));
            ich.compute(&ohlc(10.0, 21.0, 3.0, 12.0, 5));
            ich.compute(&ohlc(10.0, 23.0, 6.0, 16.0, 6));
            let val = ich.compute(&ohlc(10.0, 25.0, 8.0, 17.0, 7)).unwrap();

            // tenkan window = bars 6,7: HH=max(23,25)=25, LL=min(6,8)=6
            // tenkan = (25 + 6) / 2 = 15.5
            assert!((val.tenkan() - 15.5).abs() < 1e-10);
        }

        #[test]
        fn kijun_is_midpoint_of_window_extremes() {
            let mut ich = ichimoku(small_config());
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            ich.compute(&ohlc(10.0, 18.0, 6.0, 14.0, 2));
            ich.compute(&ohlc(10.0, 22.0, 4.0, 16.0, 3));
            ich.compute(&ohlc(10.0, 19.0, 7.0, 13.0, 4));
            ich.compute(&ohlc(10.0, 21.0, 3.0, 12.0, 5));
            ich.compute(&ohlc(10.0, 23.0, 6.0, 16.0, 6));
            let val = ich.compute(&ohlc(10.0, 25.0, 8.0, 17.0, 7)).unwrap();

            // kijun window = bars 5,6,7: HH=max(21,23,25)=25, LL=min(3,6,8)=3
            // kijun = (25 + 3) / 2 = 14.0
            assert!((val.kijun() - 14.0).abs() < 1e-10);
        }

        #[test]
        fn chikou_close_equals_current_close() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            let val = ich.compute(&ohlc(10.0, 20.0, 5.0, 42.0, 7)).unwrap();
            assert!((val.chikou_close() - 42.0).abs() < 1e-10);
        }

        #[test]
        fn senkou_a_and_b_are_displaced() {
            // displacement=2, senkou_b=2: convergence = max(2+2, 2+3) = 5
            // senkou_a displaced by 2 bars, senkou_b displaced by 3 bars.
            let config = IchimokuConfig::builder()
                .tenkan_length(nz(2))
                .kijun_length(nz(2))
                .senkou_b_length(nz(2))
                .displacement(nz(2))
                .build();
            let mut ich = ichimoku(config);

            // Bars 1-2: identical bars → tenkan=kijun=12.5
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 2));
            // Bar 3: tenkan=kijun=(30+2)/2=16.0
            ich.compute(&ohlc(10.0, 30.0, 2.0, 15.0, 3));
            // Bar 4: tenkan=(28+2)/2=15.0 (wait: bars 3,4)
            ich.compute(&ohlc(10.0, 28.0, 3.0, 15.0, 4));

            // Bar 5: convergence reached
            let v5 = ich.compute(&ohlc(10.0, 26.0, 4.0, 15.0, 5)).unwrap();
            // senkou_a displaced by 2: from bar 3's senkou_a_raw = (16+16)/2 = 16.0
            assert!((v5.senkou_a() - 16.0).abs() < 1e-10);
            // senkou_b displaced by 3: from bar 2's senkou_b_raw = (20+5)/2 = 12.5
            assert!((v5.senkou_b() - 12.5).abs() < 1e-10);
        }
    }

    mod sliding {
        use super::*;

        #[test]
        fn values_change_as_window_slides() {
            let mut ich = ichimoku(small_config());
            for t in 1..=7 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            let v1 = ich.value().unwrap();

            // Advance with a very different bar
            let v2 = ich.compute(&ohlc(50.0, 60.0, 40.0, 55.0, 8)).unwrap();

            // tenkan should change (new extremes)
            assert!(
                (v1.tenkan() - v2.tenkan()).abs() > 1e-10,
                "tenkan should change as window slides"
            );
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            let v1 = ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 7)).unwrap();
            let v2 = ich.compute(&ohlc(10.0, 40.0, 1.0, 15.0, 7)).unwrap();

            assert!(
                (v1.tenkan() - v2.tenkan()).abs() > 1e-10,
                "repaint with different extremes should change tenkan"
            );
        }

        #[test]
        fn multiple_repaints_match_single() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            // Intermediate repaints stay within existing extremes (H<=20, L>=5)
            // to avoid evicting deque entries that can't be restored.
            ich.compute(&ohlc(10.0, 18.0, 7.0, 14.0, 7));
            ich.compute(&ohlc(10.0, 19.0, 6.0, 16.0, 7)); // repaint
            let final_val = ich.compute(&ohlc(10.0, 17.0, 8.0, 13.0, 7)).unwrap();

            // Clean computation
            let mut clean = ichimoku(small_config());
            for t in 1..=6 {
                clean.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            let expected = clean.compute(&ohlc(10.0, 17.0, 8.0, 13.0, 7)).unwrap();

            assert!((final_val.tenkan() - expected.tenkan()).abs() < 1e-10);
            assert!((final_val.kijun() - expected.kijun()).abs() < 1e-10);
            assert!((final_val.chikou_close() - expected.chikou_close()).abs() < 1e-10);
        }

        #[test]
        fn repaint_during_filling() {
            let mut ich = ichimoku(small_config());
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
            ich.compute(&ohlc(10.0, 25.0, 3.0, 14.0, 1)); // repaint bar 1
            assert_eq!(ich.compute(&ohlc(10.0, 18.0, 6.0, 14.0, 2)), None);
        }

        #[test]
        fn advance_after_repaint() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 7));
            ich.compute(&ohlc(10.0, 25.0, 3.0, 18.0, 7)); // repaint
            let after = ich.compute(&ohlc(10.0, 22.0, 7.0, 16.0, 8));
            assert!(after.is_some());
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut ich = ichimoku(small_config());

            // Bars 1-5: open then close (repaint each)
            for t in 1..=5 {
                ich.compute(&ohlc(10.0, 18.0, 6.0, 14.0, t));
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t)); // repaint
            }

            // Bar 6: still filling
            assert_eq!(ich.compute(&ohlc(10.0, 19.0, 4.0, 13.0, 6)), None);
            ich.compute(&ohlc(10.0, 21.0, 3.0, 12.0, 6)); // repaint

            // Bar 7: open
            let v1 = ich.compute(&ohlc(10.0, 22.0, 6.0, 16.0, 7)).unwrap();
            // Bar 7: close (repaint)
            let v2 = ich.compute(&ohlc(10.0, 25.0, 2.0, 17.0, 7)).unwrap();

            assert!(
                (v1.tenkan() - v2.tenkan()).abs() > 1e-10,
                "repaint should change tenkan"
            );

            // Bar 8: advance
            let v3 = ich.compute(&ohlc(10.0, 24.0, 4.0, 18.0, 8));
            assert!(v3.is_some());
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut ich = ichimoku(small_config());
            for t in 1..=7 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }

            let mut cloned = ich.clone();

            // Advance original with extreme bar
            let orig = ich.compute(&ohlc(50.0, 80.0, 1.0, 60.0, 8)).unwrap();

            // Clone advances with narrow bar
            let clone_val = cloned.compute(&ohlc(14.0, 16.0, 14.0, 15.0, 8)).unwrap();

            assert!(
                (orig.tenkan() - clone_val.tenkan()).abs() > 1e-10,
                "divergent inputs should give different tenkan"
            );
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_sets_standard_values() {
            let config = default_config();
            assert_eq!(config.tenkan_length(), 9);
            assert_eq!(config.kijun_length(), 26);
            assert_eq!(config.senkou_b_length(), 52);
            assert_eq!(config.displacement(), 26);
        }

        #[test]
        fn convergence_is_senkou_b_plus_displacement_plus_one() {
            let config = default_config();
            assert_eq!(config.convergence(), 79);

            let config = small_config();
            assert_eq!(config.convergence(), 7);
        }

        #[test]
        fn source_is_close() {
            let config = default_config();
            assert_eq!(config.source(), crate::PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "tenkan_length is required")]
        fn panics_without_tenkan() {
            let _ = IchimokuConfig::builder()
                .kijun_length(nz(26))
                .senkou_b_length(nz(52))
                .displacement(nz(26))
                .build();
        }

        #[test]
        #[should_panic(expected = "kijun_length is required")]
        fn panics_without_kijun() {
            let _ = IchimokuConfig::builder()
                .tenkan_length(nz(9))
                .senkou_b_length(nz(52))
                .displacement(nz(26))
                .build();
        }

        #[test]
        #[should_panic(expected = "senkou_b_length is required")]
        fn panics_without_senkou_b() {
            let _ = IchimokuConfig::builder()
                .tenkan_length(nz(9))
                .kijun_length(nz(26))
                .displacement(nz(26))
                .build();
        }

        #[test]
        #[should_panic(expected = "displacement is required")]
        fn panics_without_displacement() {
            let _ = IchimokuConfig::builder()
                .tenkan_length(nz(9))
                .kijun_length(nz(26))
                .senkou_b_length(nz(52))
                .build();
        }

        #[test]
        fn eq_and_hash() {
            let a = default_config();
            let b = default_config();
            let c = small_config();

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn custom_parameters() {
            let config = IchimokuConfig::builder()
                .tenkan_length(nz(7))
                .kijun_length(nz(22))
                .senkou_b_length(nz(44))
                .displacement(nz(22))
                .build();
            assert_eq!(config.tenkan_length(), 7);
            assert_eq!(config.kijun_length(), 22);
            assert_eq!(config.senkou_b_length(), 44);
            assert_eq!(config.displacement(), 22);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_indicator() {
            let ich = ichimoku(default_config());
            assert_eq!(
                ich.to_string(),
                "Ichimoku(tenkan: 9, kijun: 26, senkou_b: 52, displacement: 26)"
            );
        }

        #[test]
        fn formats_config() {
            let config = default_config();
            assert_eq!(
                config.to_string(),
                "IchimokuConfig(tenkan: 9, kijun: 26, senkou_b: 52, displacement: 26)"
            );
        }

        #[test]
        fn formats_value() {
            let val = IchimokuValue {
                tenkan: 1.0,
                kijun: 2.0,
                senkou_a: 3.0,
                senkou_b: 4.0,
                chikou_close: 5.0,
            };
            assert_eq!(
                val.to_string(),
                "Ichimoku(tenkan: 1, kijun: 2, senkou_a: 3, senkou_b: 4, chikou_close: 5)"
            );
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let ich = ichimoku(small_config());
            assert_eq!(ich.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut ich = ichimoku(small_config());
            for t in 1..=7 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            assert!(ich.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut ich = ichimoku(small_config());
            for t in 1..=6 {
                ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, t));
            }
            let computed = ich.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 7));
            assert_eq!(ich.value(), computed);
        }
    }
}
