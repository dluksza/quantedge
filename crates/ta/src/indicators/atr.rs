use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price,
    internals::{BarAction, BarState, EmaCore},
};

/// Configuration for the Average True Range ([`Atr`]) indicator.
///
/// ATR always uses [`PriceSource::TrueRange`](crate::PriceSource::TrueRange)
/// as its input source. The price source cannot be changed via the builder.
///
/// # Example
///
/// ```
/// use quantedge_ta::AtrConfig;
/// use std::num::NonZero;
///
/// let config = AtrConfig::builder()
///     .length(NonZero::new(14).unwrap())
///     .build();
///
/// assert_eq!(config.length(), 14);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct AtrConfig {
    length: usize,
}

impl IndicatorConfig for AtrConfig {
    type Builder = AtrConfigBuilder;
    type Indicator = Atr;
    type Output = Price;

    fn builder() -> Self::Builder {
        AtrConfigBuilder::new()
    }

    fn source(&self) -> crate::PriceSource {
        crate::PriceSource::TrueRange
    }

    fn convergence(&self) -> usize {
        self.length
    }

    fn to_builder(&self) -> Self::Builder {
        AtrConfigBuilder {
            length: self.length,
        }
    }
}

impl AtrConfig {
    /// Window length (number of bars).
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// ATR with the given window length.
    #[must_use]
    pub fn period(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }
}

impl Default for AtrConfig {
    /// Default: length=14 (Wilder's original, `TradingView` default).
    fn default() -> Self {
        Self { length: 14 }
    }
}

impl Display for AtrConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AtrConfig({})", self.length)
    }
}

/// Builder for [`AtrConfig`].
///
/// Defaults: length = `14`.
/// The price source is always
/// [`PriceSource::TrueRange`](crate::PriceSource::TrueRange)
/// and cannot be overridden.
pub struct AtrConfigBuilder {
    length: usize,
}

impl AtrConfigBuilder {
    fn new() -> Self {
        AtrConfigBuilder { length: 14 }
    }

    /// Sets the indicator window length.
    #[must_use]
    pub fn length(mut self, length: NonZero<usize>) -> Self {
        self.length = length.get();
        self
    }
}

impl IndicatorConfigBuilder<AtrConfig> for AtrConfigBuilder {
    fn source(self, _source: crate::PriceSource) -> Self {
        self
    }

    fn build(self) -> AtrConfig {
        AtrConfig {
            length: self.length,
        }
    }
}

/// Average True Range (ATR).
///
/// Measures market volatility by smoothing the True Range over a
/// configurable window. True Range is the greatest of:
///
/// - Current high − current low
/// - |current high − previous close|
/// - |current low − previous close|
///
/// The first `length` True Range values are averaged with a simple
/// mean (SMA seed). After seeding, values are smoothed
/// exponentially with Wilder's smoothing factor `α = 1 / length`:
///
/// ```text
/// ATR = α × TR + (1 − α) × prev_ATR
/// ```
///
/// Returns `None` until the SMA seed is ready (after `length` bars).
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Atr, AtrConfig, Ohlcv};
/// use std::num::NonZero;
///
/// fn ohlc(o: f64, h: f64, l: f64, c: f64, t: u64) -> Ohlcv {
///     Ohlcv { open: o, high: h, low: l, close: c, volume: 0.0, open_time: t }
/// }
///
/// let config = AtrConfig::builder()
///     .length(NonZero::new(2).unwrap())
///     .build();
/// let mut atr = Atr::new(config);
///
/// // Bar 1: TR = high − low = 15 (no previous close)
/// assert_eq!(atr.compute(&ohlc(10.0, 20.0, 5.0, 15.0, 1)), None);
///
/// // Bar 2: TR = max(10, |22−15|, |12−15|) = 10
/// // SMA seed = (15 + 10) / 2 = 12.5
/// assert_eq!(atr.compute(&ohlc(16.0, 22.0, 12.0, 18.0, 2)), Some(12.5));
/// ```
#[derive(Clone, Debug)]
pub struct Atr {
    config: AtrConfig,
    bar_state: BarState,
    core: EmaCore,
}

impl Indicator for Atr {
    type Config = AtrConfig;
    type Output = Price;

    fn new(config: Self::Config) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let alpha = 1.0 / config.length() as f64;

        Atr {
            config,
            bar_state: BarState::new(crate::PriceSource::TrueRange),
            core: EmaCore::with_alpha(config.length, alpha),
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => self.core.push(price),
            BarAction::Repaint(price) => self.core.replace(price),
        }
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.core.value()
    }
}

impl Display for Atr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ATR({})", self.config.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{nz, ohlc};

    fn atr(length: usize) -> Atr {
        Atr::new(AtrConfig::builder().length(nz(length)).build())
    }

    mod seeding {
        use super::*;

        #[test]
        fn none_during_seeding() {
            let mut atr = atr(3);
            // Bar 1: TR = 15 - 8 = 7 (no prev_close)
            assert_eq!(atr.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1)), None);
            // Bar 2: TR = max(8, |18-12|, |10-12|) = 8
            assert_eq!(atr.compute(&ohlc(13.0, 18.0, 10.0, 16.0, 2)), None);
        }

        #[test]
        fn first_value_is_sma_of_true_ranges() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1)); // TR = 7
            atr.compute(&ohlc(13.0, 18.0, 10.0, 16.0, 2)); // TR = 8
            // Bar 3: TR = max(6, |20-16|, |14-16|) = 6
            // Seed = (7 + 8 + 6) / 3 = 7.0
            assert_eq!(atr.compute(&ohlc(17.0, 20.0, 14.0, 18.0, 3)), Some(7.0));
        }

        #[test]
        fn repaint_during_seeding() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1)); // TR = 10
            atr.compute(&ohlc(16.0, 25.0, 15.0, 20.0, 2)); // TR = 10
            // Repaint bar 2: H=28, TR = max(13, |28-15|, |15-15|) = 13
            atr.compute(&ohlc(16.0, 28.0, 15.0, 20.0, 2));
            // Bar 3: TR = max(10, |30-20|, |20-20|) = 10
            // Seed = (10 + 13 + 10) / 3 = 11.0
            assert_eq!(atr.compute(&ohlc(21.0, 30.0, 20.0, 25.0, 3)), Some(11.0));
        }
    }

    mod wilders_smoothing {
        use super::*;

        #[test]
        fn applies_wilder_alpha() {
            // Period=3, Wilder's α = 1/3
            // Standard EMA α would be 2/(3+1) = 0.5
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1)); // TR = 6
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2)); // TR = 6
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // TR = 6, seed = 6.0

            // Bar 4: TR = max(12, |31-19|, |19-19|) = 12
            // Wilder: 6 × 2/3 + 12 × 1/3 = 4 + 4 = 8.0
            // Standard EMA would give: 6 × 0.5 + 12 × 0.5 = 9.0
            assert_eq!(atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4)), Some(8.0));
        }

        #[test]
        fn multi_bar_smoothing() {
            // Period=2, Wilder's α = 0.5
            let mut atr = atr(2);
            atr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1)); // TR = 10
            atr.compute(&ohlc(16.0, 24.0, 14.0, 19.0, 2)); // TR = max(10,9,1) = 10
            // Seed = 10.0

            // Bar 3: TR = max(22, |40-19|, |18-19|) = max(22,21,1) = 22
            // ATR = 10 × 0.5 + 22 × 0.5 = 16.0
            assert_eq!(atr.compute(&ohlc(20.0, 40.0, 18.0, 30.0, 3)), Some(16.0));

            // Bar 4: TR = max(6, |35-30|, |29-30|) = 6
            // ATR = 16 × 0.5 + 6 × 0.5 = 11.0
            assert_eq!(atr.compute(&ohlc(31.0, 35.0, 29.0, 32.0, 4)), Some(11.0));

            // Bar 5: TR = max(6, |37-32|, |31-32|) = 6
            // ATR = 11 × 0.5 + 6 × 0.5 = 8.5
            assert_eq!(atr.compute(&ohlc(33.0, 37.0, 31.0, 34.0, 5)), Some(8.5));
        }
    }

    mod gap_detection {
        use super::*;

        #[test]
        fn gap_up_high_vs_prev_close_dominates() {
            // Period=2, α = 0.5
            let mut atr = atr(2);
            atr.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1)); // TR = 7
            atr.compute(&ohlc(13.0, 18.0, 10.0, 16.0, 2)); // TR = 8
            // Seed = 7.5

            // Gap up from 16 → 28..35: prev_close = 16
            // TR = max(7, |35-16|, |28-16|) = max(7, 19, 12) = 19
            // ATR = 7.5 × 0.5 + 19 × 0.5 = 13.25
            assert_eq!(atr.compute(&ohlc(30.0, 35.0, 28.0, 32.0, 3)), Some(13.25));
        }

        #[test]
        fn gap_down_low_vs_prev_close_dominates() {
            // Period=2, α = 0.5
            let mut atr = atr(2);
            atr.compute(&ohlc(50.0, 55.0, 48.0, 52.0, 1)); // TR = 7
            atr.compute(&ohlc(53.0, 58.0, 50.0, 56.0, 2)); // TR = 8
            // Seed = 7.5

            // Gap down from 56 → 28..32: prev_close = 56
            // TR = max(4, |32-56|, |28-56|) = max(4, 24, 28) = 28
            // ATR = 7.5 × 0.5 + 28 × 0.5 = 17.75
            assert_eq!(atr.compute(&ohlc(30.0, 32.0, 28.0, 30.0, 3)), Some(17.75));
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn recomputes_from_prev_atr() {
            // Period=3, α = 1/3
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1)); // TR = 6
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2)); // TR = 6
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed = 6.0

            // Bar 4: TR = 12, ATR = 8.0
            atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4));
            // Repaint bar 4: TR = max(6, |25-19|, |19-19|) = 6
            // ATR = 6.0 × 2/3 + 6 × 1/3 = 6.0
            assert_eq!(atr.compute(&ohlc(20.0, 25.0, 19.0, 22.0, 4)), Some(6.0));
        }

        #[test]
        fn multiple_repaints_same_bar() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1)); // TR = 6
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2)); // TR = 6
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed = 6.0

            // Bar 4: TR = 12 → ATR = 8.0
            atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4));
            // Repaint: TR = max(9, 9, 0) = 9 → ATR = 6×2/3 + 9/3 = 7.0
            atr.compute(&ohlc(20.0, 28.0, 19.0, 23.0, 4));
            // Repaint again: TR = 6 → ATR = 6×2/3 + 6/3 = 6.0
            assert_eq!(atr.compute(&ohlc(20.0, 25.0, 19.0, 22.0, 4)), Some(6.0));
        }

        #[test]
        fn advance_after_repaint() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1)); // TR = 6
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2)); // TR = 6
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed = 6.0

            atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4)); // ATR = 8.0
            // Repaint bar 4: TR = 6, ATR = 6.0, close = 22
            atr.compute(&ohlc(20.0, 25.0, 19.0, 22.0, 4));

            // Bar 5: prev_close = 22 (repainted close)
            // TR = max(12, |31-22|, |19-22|) = max(12, 9, 3) = 12
            // ATR = 6.0 × 2/3 + 12/3 = 4 + 4 = 8.0
            assert_eq!(atr.compute(&ohlc(23.0, 31.0, 19.0, 25.0, 5)), Some(8.0));
        }

        #[test]
        fn repaint_matches_clean_computation() {
            // Repainted path
            let mut repainted = atr(3);
            repainted.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1));
            repainted.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2));
            repainted.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed
            repainted.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4)); // ATR = 8.0
            repainted.compute(&ohlc(20.0, 28.0, 19.0, 24.0, 4)); // repaint
            let val = repainted.compute(&ohlc(25.0, 33.0, 23.0, 28.0, 5));

            // Clean path (bar 4 was always H=28, L=19, C=24)
            let mut clean = atr(3);
            clean.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1));
            clean.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2));
            clean.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3));
            clean.compute(&ohlc(20.0, 28.0, 19.0, 24.0, 4));
            let expected = clean.compute(&ohlc(25.0, 33.0, 23.0, 28.0, 5));

            assert_eq!(val, expected);
        }
    }

    mod flat_market {
        use super::*;

        #[test]
        fn atr_is_zero_on_flat_bars() {
            let mut atr = atr(3);
            for t in 1..=10 {
                atr.compute(&ohlc(10.0, 10.0, 10.0, 10.0, t));
            }
            // TR = 0 for all bars, ATR = 0.0
            assert_eq!(atr.value(), Some(0.0));
        }
    }

    mod spike_decay {
        use super::*;

        #[test]
        fn atr_decays_monotonically_after_spike() {
            // Period=2, α = 0.5
            let mut atr = atr(2);
            atr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1)); // TR = 10
            // Spike: TR = max(42, |56-15|, |14-15|) = 42
            atr.compute(&ohlc(16.0, 56.0, 14.0, 35.0, 2)); // Seed = 26.0

            let mut prev_atr = 26.0;
            // Feed calm bars (TR ≈ 2 each)
            for t in 3..=8 {
                let result = atr.compute(&ohlc(36.0, 37.0, 35.0, 36.0, t)).unwrap();
                assert!(
                    result < prev_atr,
                    "ATR should decay: {result} >= {prev_atr} at bar {t}"
                );
                prev_atr = result;
            }
        }
    }

    mod window_size_one {
        use super::*;

        #[test]
        fn first_bar_returns_value() {
            let mut atr = atr(1);
            // TR = 20 - 10 = 10 (no prev_close)
            assert_eq!(atr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1)), Some(10.0));
        }

        #[test]
        fn always_equals_current_tr() {
            // Period=1, α = 1/1 = 1.0 → ATR = TR
            let mut atr = atr(1);
            atr.compute(&ohlc(10.0, 20.0, 10.0, 15.0, 1)); // TR = 10

            // Bar 2: TR = max(10, |25-15|, |15-15|) = 10
            assert_eq!(atr.compute(&ohlc(16.0, 25.0, 15.0, 20.0, 2)), Some(10.0));

            // Bar 3: TR = max(4, |23-20|, |19-20|) = 4
            assert_eq!(atr.compute(&ohlc(20.0, 23.0, 19.0, 21.0, 3)), Some(4.0));
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1)); // TR = 6
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2)); // TR = 6
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed = 6.0

            let mut cloned = atr.clone();

            // Advance original: TR = 12, ATR = 8.0
            assert_eq!(atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4)), Some(8.0));

            // Clone still at seed
            assert_eq!(cloned.value(), Some(6.0));

            // Clone advances independently: TR = max(18, |37-19|, |19-19|) = 18
            // ATR = 6 × 2/3 + 18/3 = 4 + 6 = 10.0
            assert_eq!(cloned.compute(&ohlc(20.0, 37.0, 19.0, 28.0, 4)), Some(10.0));
        }
    }

    mod config {
        use crate::PriceSource;

        use super::*;
        use std::collections::HashSet;

        #[test]
        fn convergence_equals_length() {
            let config = AtrConfig::builder().length(nz(14)).build();
            assert_eq!(config.convergence(), 14);

            let config = AtrConfig::builder().length(nz(3)).build();
            assert_eq!(config.convergence(), 3);
        }

        #[test]
        fn builder_sets_length() {
            let config = AtrConfig::builder().length(nz(14)).build();
            assert_eq!(config.length(), 14);
        }

        #[test]
        fn source_is_true_range() {
            let config = AtrConfig::builder().length(nz(14)).build();
            assert_eq!(config.source(), PriceSource::TrueRange);
        }

        #[test]
        fn source_builder_is_noop() {
            // Calling source() on the builder should not change behavior
            let a = AtrConfig::builder().length(nz(14)).build();
            let b = AtrConfig::builder()
                .length(nz(14))
                .source(PriceSource::Close)
                .build();

            // Both should produce identical ATR values
            let mut atr_a = Atr::new(a);
            let mut atr_b = Atr::new(b);

            let bars = [
                ohlc(10.0, 20.0, 5.0, 15.0, 1),
                ohlc(16.0, 25.0, 12.0, 20.0, 2),
            ];
            for bar in &bars {
                assert_eq!(atr_a.compute(bar), atr_b.compute(bar));
            }
        }

        #[test]
        fn eq_and_hash() {
            let a = AtrConfig::builder().length(nz(14)).build();
            let b = AtrConfig::builder().length(nz(14)).build();
            let c = AtrConfig::builder().length(nz(7)).build();

            let mut set = HashSet::new();
            set.insert(a);

            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = AtrConfig::builder().length(nz(14)).build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let atr = atr(14);
            assert_eq!(atr.to_string(), "ATR(14)");
        }

        #[test]
        fn config_formats_correctly() {
            let config = AtrConfig::builder().length(nz(14)).build();
            assert_eq!(config.to_string(), "AtrConfig(14)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let atr = atr(3);
            assert_eq!(atr.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1));
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2));
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3)); // seed = 6.0
            assert_eq!(atr.value(), Some(6.0));
        }

        #[test]
        fn matches_last_compute() {
            let mut atr = atr(3);
            atr.compute(&ohlc(10.0, 16.0, 10.0, 13.0, 1));
            atr.compute(&ohlc(14.0, 19.0, 13.0, 16.0, 2));
            atr.compute(&ohlc(17.0, 22.0, 16.0, 19.0, 3));
            let computed = atr.compute(&ohlc(20.0, 31.0, 19.0, 25.0, 4));
            assert_eq!(atr.value(), computed);
        }
    }
}
