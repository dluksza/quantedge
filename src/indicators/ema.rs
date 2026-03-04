use std::{
    fmt::{Debug, Display},
    num::NonZero,
};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price, PriceSource, Timestamp,
    internals::EmaCore,
};

/// Configuration for the Exponential Moving Average ([`Ema`])
/// indicator.
///
/// # Convergence
///
/// EMA has infinite memory: the initial seed value (SMA of the
/// first `length` bars) influences all subsequent values. With
/// `enforce_convergence` enabled, [`Ema::compute`] returns
/// `None` until the seed's contribution decays below 1%.
///
/// For EMA(20), that's 63 bars (`3 × (length + 1)`).
/// Without enforcement, values are returned as soon as the
/// SMA seed is ready (after `length` bars).
///
/// # Example
///
/// ```
/// use quantedge_ta::EmaConfig;
/// use std::num::NonZero;
///
/// let config = EmaConfig::builder()
///     .length(NonZero::new(20).unwrap())
///     .enforce_convergence(true)
///     .build();
///
/// assert_eq!(config.length(), 20);
/// assert_eq!(config.required_bars_to_converge(), 63);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct EmaConfig {
    length: usize,
    source: PriceSource,
    convergence: bool,
    bars_to_converge: usize,
}

impl IndicatorConfig for EmaConfig {
    type Builder = EmaConfigBuilder;

    #[inline]
    fn builder() -> Self::Builder {
        EmaConfigBuilder::new()
    }

    #[inline]
    fn source(&self) -> PriceSource {
        self.source
    }
}

impl EmaConfig {
    /// Window length (number of bars).
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// When `true`, [`Ema::compute`] returns `None` until
    /// [`required_bars_to_converge`](Self::required_bars_to_converge) bars have
    /// been processed. Default: `false`.
    #[inline]
    #[must_use]
    pub fn enforce_convergence(&self) -> bool {
        self.convergence
    }

    /// Number of bars needed before the EMA output is fully converged.
    ///
    /// When convergence is not enforced, this equals the window length.
    /// When enforced, this is `3 × (length + 1)` — the number of bars
    /// until the SMA seed's influence decays below 1%.
    #[must_use]
    pub fn required_bars_to_converge(&self) -> usize {
        self.bars_to_converge
    }

    /// EMA on closing price.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }

    /// EMA on median price: `(high + low) / 2`.
    #[must_use]
    pub fn hl2(length: NonZero<usize>) -> Self {
        Self::builder()
            .length(length)
            .source(PriceSource::HL2)
            .build()
    }

    /// EMA on average price: `(open + high + low + close) / 4`.
    #[must_use]
    pub fn ohlc4(length: NonZero<usize>) -> Self {
        Self::builder()
            .length(length)
            .source(PriceSource::OHLC4)
            .build()
    }
}

impl Display for EmaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmaConfig({}, {})", self.length, self.source)
    }
}

/// Builder for [`EmaConfig`].
///
/// Defaults: source = [`PriceSource::Close`],
/// convergence enforcement = `false`.
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct EmaConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
    convergence: bool,
}

impl EmaConfigBuilder {
    fn new() -> Self {
        Self {
            length: None,
            source: PriceSource::Close,
            convergence: false,
        }
    }

    /// Sets the indicator window length.
    #[inline]
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length.replace(length.get());
        self
    }

    /// Enables or disables convergence enforcement.
    #[inline]
    #[must_use]
    pub fn enforce_convergence(mut self, enforce: bool) -> Self {
        self.convergence = enforce;
        self
    }
}

impl IndicatorConfigBuilder<EmaConfig> for EmaConfigBuilder {
    #[inline]
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    #[inline]
    fn build(self) -> EmaConfig {
        let length = self.length.expect("length is required");
        let bars_to_converge = if self.convergence {
            EmaCore::bars_to_converge(length)
        } else {
            length
        };

        EmaConfig {
            length,
            source: self.source,
            convergence: self.convergence,
            bars_to_converge,
        }
    }
}

/// Exponential Moving Average (EMA).
///
/// A weighted moving average that gives more weight to recent
/// prices. Uses the standard smoothing factor
/// `α = 2 / (length + 1)`. Each value is computed as:
///
/// ```text
/// EMA = α × price + (1 − α) × prev_EMA
/// ```
///
/// The first `length` bars are collected to compute an SMA
/// seed value. After seeding, the SMA state is dropped and
/// the EMA runs with O(1) constant memory per tick via a
/// single fused multiply-add.
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous EMA without
/// advancing state.
///
/// # Convergence
///
/// Without `enforce_convergence`, values are returned as soon
/// as the SMA seed is ready (after `length` bars). The seed's
/// influence is still present but decays exponentially.
///
/// With `enforce_convergence` enabled, `None` is returned
/// until `3 × (length + 1)` bars have been processed, at
/// which point the seed's contribution is below 1%.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ema, EmaConfig};
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
/// let config = EmaConfig::builder()
///     .length(NonZero::new(3).unwrap())
///     .build();
/// let mut ema = Ema::new(config);
///
/// // Seeding phase: collecting SMA
/// assert_eq!(ema.compute(&Bar(2.0, 1)), None);
/// assert_eq!(ema.compute(&Bar(4.0, 2)), None);
///
/// // SMA seed = (2 + 4 + 6) / 3 = 4.0
/// assert_eq!(ema.compute(&Bar(6.0, 3)), Some(4.0));
///
/// // EMA(3) α = 0.5: 8 × 0.5 + 4 × 0.5 = 6.0
/// assert_eq!(ema.compute(&Bar(8.0, 4)), Some(6.0));
/// ```
#[derive(Clone, Debug)]
pub struct Ema {
    config: EmaConfig,
    last_open_time: Option<Timestamp>,
    cur_close: Option<Price>,
    prev_close: Option<Price>,
    core: EmaCore,
}

impl Indicator for Ema {
    type Config = EmaConfig;
    type Output = Price;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            last_open_time: None,
            cur_close: None,
            prev_close: None,
            core: EmaCore::new(config.length(), config.convergence),
        }
    }

    #[inline]
    fn compute(&mut self, ohlcv: &impl Ohlcv) -> Option<Price> {
        debug_assert!(
            self.last_open_time.is_none_or(|t| t <= ohlcv.open_time()),
            "open_time must be non-decreasing: last={}, got={}",
            self.last_open_time.unwrap_or(0),
            ohlcv.open_time(),
        );

        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time());

        if is_next_bar {
            self.last_open_time = Some(ohlcv.open_time());
            self.prev_close = self.cur_close;

            let price = self.config.source.extract(ohlcv, self.prev_close);
            self.core.push(price);
        } else {
            let price = self.config.source.extract(ohlcv, self.prev_close);
            self.core.replace(price);
        }

        self.cur_close = Some(ohlcv.close());

        self.core.value()
    }

    #[inline]
    fn value(&self) -> Option<Price> {
        self.core.value()
    }
}

impl Display for Ema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EMA({}, {})", self.config.length, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Bar, assert_approx, bar, nz};

    fn ema(length: usize) -> Ema {
        Ema::new(EmaConfig::builder().length(nz(length)).build())
    }

    mod seeding {
        use super::*;

        #[test]
        fn none_during_seeding() {
            let mut ema = ema(3);
            assert_eq!(ema.compute(&bar(10.0, 1)), None);
            assert_eq!(ema.compute(&bar(20.0, 2)), None);
        }

        #[test]
        fn first_value_is_sma_seed() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            // SMA(3) = (2 + 4 + 6) / 3 = 4.0
            assert_eq!(ema.compute(&bar(6.0, 3)), Some(4.0));
        }

        #[test]
        fn repaint_during_seeding() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(5.0, 1)); // repaint bar 1
            ema.compute(&bar(4.0, 2));
            // SMA seed = (5 + 4 + 6) / 3 = 5.0
            assert_eq!(ema.compute(&bar(6.0, 3)), Some(5.0));
        }
    }

    mod alpha {
        use super::*;

        #[test]
        fn ema_2_alpha_is_two_thirds() {
            // α = 2/(2+1) = 2/3
            // seed [3, 6] → SMA = 4.5
            // bar 3: 9 * 2/3 + 4.5 * 1/3 = 6 + 1.5 = 7.5
            let mut ema = ema(2);
            ema.compute(&bar(3.0, 1));
            ema.compute(&bar(6.0, 2));
            assert_eq!(ema.compute(&bar(9.0, 3)), Some(7.5));
        }

        #[test]
        fn ema_4_alpha_is_two_fifths() {
            // α = 2/(4+1) = 0.4
            // seed [10, 20, 30, 40] → SMA = 25
            // bar 5: 50 * 0.4 + 25 * 0.6 = 20 + 15 = 35
            let mut ema = ema(4);
            ema.compute(&bar(10.0, 1));
            ema.compute(&bar(20.0, 2));
            ema.compute(&bar(30.0, 3));
            ema.compute(&bar(40.0, 4));
            assert_eq!(ema.compute(&bar(50.0, 5)), Some(35.0));
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn recomputes_from_prev_ema() {
            // EMA(3): α = 0.5
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3)); // seed = 4.0
            ema.compute(&bar(8.0, 4)); // EMA = 6.0
            // Repaint bar 4: 12 * 0.5 + 4.0 * 0.5 = 8.0
            assert_eq!(ema.compute(&bar(12.0, 4)), Some(8.0));
        }

        #[test]
        fn multiple_repaints_same_bar() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3)); // seed = 4.0
            ema.compute(&bar(8.0, 4)); // 6.0
            ema.compute(&bar(10.0, 4)); // 10*0.5 + 4*0.5 = 7.0
            // 12 * 0.5 + 4.0 * 0.5 = 8.0
            assert_eq!(ema.compute(&bar(12.0, 4)), Some(8.0));
        }

        #[test]
        fn advance_after_repaint() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3)); // seed = 4.0
            ema.compute(&bar(8.0, 4)); // 6.0
            ema.compute(&bar(10.0, 4)); // repaint: 10*0.5 + 4*0.5 = 7.0
            // Advance: prev_ema = 7.0 (repainted value)
            // EMA = 12 * 0.5 + 7.0 * 0.5 = 9.5
            assert_eq!(ema.compute(&bar(12.0, 5)), Some(9.5));
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            // EMA(3): α = 0.5
            let mut ema = ema(3);

            // Bar 1: opens at 10, closes at 12
            assert_eq!(ema.compute(&bar(10.0, 1)), None);
            assert_eq!(ema.compute(&bar(12.0, 1)), None);

            // Bar 2: opens at 8, closes at 6
            assert_eq!(ema.compute(&bar(8.0, 2)), None);
            assert_eq!(ema.compute(&bar(6.0, 2)), None);

            // Bar 3: opens at 9 → seed = (12+6+9)/3 = 9.0
            assert_eq!(ema.compute(&bar(9.0, 3)), Some(9.0));
            // Bar 3 repaint: seed = (12+6+15)/3 = 11.0
            assert_eq!(ema.compute(&bar(15.0, 3)), Some(11.0));

            // Bar 4: 20 * 0.5 + 11.0 * 0.5 = 15.5
            assert_eq!(ema.compute(&bar(20.0, 4)), Some(15.5));
            // Bar 4 repaint: 14 * 0.5 + 11.0 * 0.5 = 12.5
            assert_eq!(ema.compute(&bar(14.0, 4)), Some(12.5));

            // Bar 5: prev_ema = 12.5
            // 10 * 0.5 + 12.5 * 0.5 = 11.25
            assert_eq!(ema.compute(&bar(10.0, 5)), Some(11.25));
        }
    }

    mod window_size_one {
        use super::*;

        #[test]
        fn first_bar_returns_value() {
            let mut ema = ema(1);
            assert_eq!(ema.compute(&bar(42.0, 1)), Some(42.0));
        }

        #[test]
        fn always_equals_latest_price() {
            // EMA(1): α = 2/(1+1) = 1.0
            let mut ema = ema(1);
            ema.compute(&bar(10.0, 1));
            assert_eq!(ema.compute(&bar(20.0, 2)), Some(20.0));
            assert_eq!(ema.compute(&bar(5.0, 3)), Some(5.0));
        }
    }

    mod price_source {
        use super::*;

        #[test]
        fn uses_configured_source() {
            // EMA(2) HL2: α = 2/3
            let mut ema = Ema::new(
                EmaConfig::builder()
                    .length(nz(2))
                    .source(PriceSource::HL2)
                    .build(),
            );
            // HL2 bar 1: midpoint(20, 10) = 15
            // HL2 bar 2: midpoint(30, 20) = 25
            // SMA seed = (15 + 25) / 2 = 20
            let b1 = Bar::new(0.0, 20.0, 10.0, 0.0).at(1);
            let b2 = Bar::new(0.0, 30.0, 20.0, 0.0).at(2);
            ema.compute(&b1);
            assert_eq!(ema.compute(&b2), Some(20.0));

            // Post-seed: HL2 bar 3 = midpoint(40, 30) = 35
            // EMA = 35 * 2/3 + 20 * 1/3 = 30
            let b3 = Bar::new(0.0, 40.0, 30.0, 0.0).at(3);
            assert_eq!(ema.compute(&b3), Some(30.0));
        }
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_value_at_seed_without_enforcement() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            assert!(ema.compute(&bar(6.0, 3)).is_some());
        }

        #[test]
        fn none_until_converged_when_enforced() {
            let mut ema = Ema::new(
                EmaConfig::builder()
                    .length(nz(3))
                    .enforce_convergence(true)
                    .build(),
            );
            // required_bars = 3 * (3 + 1) = 12
            for i in 1..=11 {
                assert_eq!(ema.compute(&bar(50.0, i)), None, "expected None at bar {i}");
            }
            assert!(ema.compute(&bar(50.0, 12)).is_some());
        }

        #[test]
        fn required_bars_scales_with_length() {
            let c10 = EmaConfig::builder()
                .length(nz(10))
                .enforce_convergence(true)
                .build();
            assert_eq!(c10.required_bars_to_converge(), 33);

            let c50 = EmaConfig::builder()
                .length(nz(50))
                .enforce_convergence(true)
                .build();
            assert_eq!(c50.required_bars_to_converge(), 153);
        }

        #[test]
        #[allow(clippy::cast_precision_loss)]
        fn values_match_with_and_without_enforcement() {
            let mut free = Ema::new(EmaConfig::builder().length(nz(3)).build());
            let mut enforced = Ema::new(
                EmaConfig::builder()
                    .length(nz(3))
                    .enforce_convergence(true)
                    .build(),
            );

            for i in 1..=20 {
                free.compute(&bar(i as f64 * 10.0, i));
                enforced.compute(&bar(i as f64 * 10.0, i));
            }

            let f = free.compute(&bar(210.0, 21));
            let e = enforced.compute(&bar(210.0, 21));
            assert_eq!(f, e);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3)); // seed = 4.0

            let mut cloned = ema.clone();

            // Advance original past seed
            assert_eq!(ema.compute(&bar(8.0, 4)), Some(6.0));

            // Clone still at seed value
            assert_eq!(cloned.value(), Some(4.0));

            // Clone advances independently
            assert_eq!(cloned.compute(&bar(20.0, 4)), Some(12.0));
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_source_is_close() {
            let config = EmaConfig::builder().length(nz(10)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn convergence_disabled_by_default() {
            let config = EmaConfig::builder().length(nz(10)).build();
            assert!(!config.enforce_convergence());
        }

        #[test]
        fn custom_source() {
            let config = EmaConfig::builder()
                .length(nz(10))
                .source(PriceSource::HL2)
                .build();
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = EmaConfig::builder().build();
        }

        #[test]
        fn close_helper() {
            let config = EmaConfig::close(nz(20));
            assert_eq!(config.length(), 20);
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        fn hl2_helper() {
            let config = EmaConfig::hl2(nz(10));
            assert_eq!(config.length(), 10);
            assert_eq!(config.source(), PriceSource::HL2);
        }

        #[test]
        fn ohlc4_helper() {
            let config = EmaConfig::ohlc4(nz(10));
            assert_eq!(config.length(), 10);
            assert_eq!(config.source(), PriceSource::OHLC4);
        }

        #[test]
        fn eq_and_hash() {
            let a = EmaConfig::close(nz(20));
            let b = EmaConfig::close(nz(20));
            let c = EmaConfig::close(nz(10));

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let ema = ema(20);
            assert_eq!(ema.to_string(), "EMA(20, Close)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = EmaConfig::close(nz(20));
            assert_eq!(config.to_string(), "EmaConfig(20, Close)");
        }
    }

    mod true_range {
        use super::*;

        fn tr_ema(length: usize) -> Ema {
            Ema::new(
                EmaConfig::builder()
                    .length(nz(length))
                    .source(PriceSource::TrueRange)
                    .build(),
            )
        }

        fn ohlc(open: f64, high: f64, low: f64, close: f64, time: u64) -> Bar {
            Bar::new(open, high, low, close).at(time)
        }

        #[test]
        fn seeds_with_true_range_sma() {
            // EMA(2) on TrueRange, α = 2/3
            let mut ema = tr_ema(2);
            ema.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)); // TR=15
            // TR2: hl=10, |22-15|=7, |12-15|=3 → 10
            // SMA seed = (15 + 10) / 2 = 12.5
            assert_eq!(ema.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)), Some(12.5),);
        }

        #[test]
        fn applies_ema_after_seed() {
            // EMA(2) on TrueRange, α = 2/3
            let mut ema = tr_ema(2);
            ema.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)); // TR=15
            ema.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)); // seed=12.5
            // TR3: hl=8, |28-18|=10, |20-18|=2 → 10
            // EMA = 10 * 2/3 + 12.5 * 1/3 = 10.833...
            let result = ema.compute(&ohlc(23.0, 28.0, 20.0, 25.0, 3)).unwrap();
            let expected = 10.0 * (2.0 / 3.0) + 12.5 * (1.0 / 3.0);
            assert_approx!(result, expected);
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let ema = ema(3);
            assert_eq!(ema.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3)); // seed = 4.0
            assert_eq!(ema.value(), Some(4.0));
        }

        #[test]
        fn matches_last_compute() {
            let mut ema = ema(3);
            ema.compute(&bar(2.0, 1));
            ema.compute(&bar(4.0, 2));
            ema.compute(&bar(6.0, 3));
            let computed = ema.compute(&bar(8.0, 4));
            assert_eq!(ema.value(), computed);
        }

        #[test]
        fn none_during_convergence_enforcement() {
            let mut ema = Ema::new(
                EmaConfig::builder()
                    .length(nz(3))
                    .enforce_convergence(true)
                    .build(),
            );
            for i in 1..=5 {
                ema.compute(&bar(50.0, i));
            }
            // Required bars = 12, only fed 5
            assert_eq!(ema.value(), None);
        }
    }
}
