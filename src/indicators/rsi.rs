use std::{fmt::Display, num::NonZero};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price, PriceSource, Timestamp,
};

/// Configuration for the Relative Strength Index ([`Rsi`])
/// indicator.
///
/// RSI uses Wilder's smoothing, which has infinite memory: the
/// SMA seed (first `length` price changes) influences all
/// subsequent values. Output begins at bar `length + 1`.
///
/// # Example
///
/// ```
/// use quantedge_ta::RsiConfig;
/// use std::num::NonZero;
///
/// let config = RsiConfig::close(NonZero::new(14).unwrap());
/// assert_eq!(config.length(), 14);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct RsiConfig {
    length: usize,
    source: PriceSource,
}

impl IndicatorConfig for RsiConfig {
    type Builder = RsiConfigBuilder;

    #[inline]
    fn builder() -> Self::Builder {
        RsiConfigBuilder::new()
    }

    #[inline]
    fn source(&self) -> PriceSource {
        self.source
    }
}

impl RsiConfig {
    /// Window length (number of bars).
    #[inline]
    #[must_use]
    pub fn length(&self) -> usize {
        self.length
    }

    /// RSI on closing price.
    #[must_use]
    pub fn close(length: NonZero<usize>) -> Self {
        Self::builder().length(length).build()
    }
}

impl Display for RsiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RsiConfig({}, {})", self.length, self.source)
    }
}

/// Builder for [`RsiConfig`].
///
/// Defaults: source = [`PriceSource::Close`].
/// Length must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct RsiConfigBuilder {
    length: Option<usize>,
    source: PriceSource,
}

impl RsiConfigBuilder {
    #[must_use]
    fn new() -> Self {
        Self {
            length: None,
            source: PriceSource::Close,
        }
    }

    /// Sets the indicator window length.
    #[inline]
    #[must_use]
    pub fn length(mut self, length: std::num::NonZero<usize>) -> Self {
        self.length = Some(length.get());
        self
    }
}

impl IndicatorConfigBuilder<RsiConfig> for RsiConfigBuilder {
    #[inline]
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    #[inline]
    fn build(self) -> RsiConfig {
        let length = self.length.expect("length is required");

        RsiConfig {
            length,
            source: self.source,
        }
    }
}

#[derive(Clone, Debug)]
enum RsiPhase {
    Seeding {
        sum_gain: f64,
        sum_loss: f64,
        prev_gain: f64,
        prev_loss: f64,
        seen_bars: usize,
    },
    Active {
        prev_avg_gain: f64,
        prev_avg_loss: f64,
        avg_gain: f64,
        avg_loss: f64,
    },
}

/// Relative Strength Index (RSI) with Wilder's smoothing.
///
/// Measures the speed and magnitude of recent price changes on
/// a 0–100 scale. Values above 70 are conventionally considered
/// overbought; below 30, oversold.
///
/// The first `length` price changes are averaged with a simple
/// mean (SMA seed). After seeding, gains and losses are smoothed
/// with Wilder's method (`α = 1 / length`):
///
/// ```text
/// avg_gain = (prev_avg_gain × (length − 1) + gain) / length
/// avg_loss = (prev_avg_loss × (length − 1) + loss) / length
/// RSI      = 100 × avg_gain / (avg_gain + avg_loss)
/// ```
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Rsi, RsiConfig};
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
/// let mut rsi = Rsi::new(RsiConfig::close(NonZero::new(3).unwrap()));
///
/// // Seeding: need 3 price changes (4 bars)
/// assert_eq!(rsi.compute(&Bar(10.0, 1)), None);
/// assert_eq!(rsi.compute(&Bar(12.0, 2)), None);
/// assert_eq!(rsi.compute(&Bar(11.0, 3)), None);
///
/// // Bar 4: changes = +2, −1, +2 → avg_gain=4/3, avg_loss=1/3 → RSI=80
/// assert_eq!(rsi.compute(&Bar(13.0, 4)), Some(80.0));
/// ```
#[derive(Clone, Debug)]
pub struct Rsi {
    config: RsiConfig,
    prev_price: f64,
    cur_price: f64,
    cur_close: Option<Price>,
    prev_close: Option<Price>,
    phase: RsiPhase,
    current: Option<Price>,
    last_open_time: Option<Timestamp>,
    length_reciprocal: f64,
    length_minus_one: f64,
}

impl Indicator for Rsi {
    type Config = RsiConfig;
    type Output = Price;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            phase: RsiPhase::Seeding {
                sum_gain: 0.0,
                sum_loss: 0.0,
                prev_gain: 0.0,
                prev_loss: 0.0,
                seen_bars: 0,
            },
            cur_close: None,
            prev_close: None,
            prev_price: 0.0,
            cur_price: 0.0,
            current: None,
            last_open_time: None,
            #[allow(clippy::cast_precision_loss)]
            length_reciprocal: 1.0 / config.length() as f64,
            #[allow(clippy::cast_precision_loss)]
            length_minus_one: (config.length() - 1) as f64,
        }
    }

    #[inline]
    fn compute(&mut self, ohlcv: &impl Ohlcv) -> Option<Self::Output> {
        debug_assert!(
            self.last_open_time.is_none_or(|t| t <= ohlcv.open_time()),
            "open_time must be non-decreasing: last={}, got={}",
            self.last_open_time.unwrap_or(0),
            ohlcv.open_time(),
        );

        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time());

        if is_next_bar {
            self.prev_close = self.cur_close;
            self.prev_price = self.cur_price;
            self.last_open_time = Some(ohlcv.open_time());
        }

        let price = self.config.source().extract(ohlcv, self.prev_close);
        self.cur_price = price;
        self.cur_close = Some(ohlcv.close());

        self.current = match &mut self.phase {
            RsiPhase::Seeding {
                sum_gain,
                sum_loss,
                prev_gain,
                prev_loss,
                seen_bars,
            } if *seen_bars <= self.config.length() => {
                if is_next_bar {
                    // Compute change (skip first bar: no previous price)
                    if *seen_bars > 0 {
                        (*prev_gain, *prev_loss) = Self::gain_and_loss(self.prev_price, price);
                        *sum_gain += *prev_gain;
                        *sum_loss += *prev_loss;
                    }

                    *seen_bars += 1;
                } else if *seen_bars > 1 {
                    // Adjust sums (skip bar 1: no valid prev bar)
                    let (gain, loss) = Self::gain_and_loss(self.prev_price, price);

                    *sum_gain = *sum_gain - *prev_gain + gain;
                    *sum_loss = *sum_loss - *prev_loss + loss;
                    *prev_gain = gain;
                    *prev_loss = loss;
                }

                // Seeding complete: output SMA-based RSI
                if *seen_bars > self.config.length() {
                    Some(Rsi::rsi_from_averages(
                        *sum_gain * self.length_reciprocal,
                        *sum_loss * self.length_reciprocal,
                    ))
                } else {
                    None
                }
            }

            // seen_bars > length: transition bar repaint or advance to Active
            RsiPhase::Seeding {
                sum_gain,
                sum_loss,
                prev_gain,
                prev_loss,
                ..
            } => {
                if is_next_bar {
                    // Advance to Active phase: compute first Wilder-smoothed value
                    let prev_avg_gain = *sum_gain * self.length_reciprocal;
                    let prev_avg_loss = *sum_loss * self.length_reciprocal;

                    let (gain, loss) = Self::gain_and_loss(self.prev_price, price);

                    let avg_gain =
                        prev_avg_gain.mul_add(self.length_minus_one, gain) * self.length_reciprocal;
                    let avg_loss =
                        prev_avg_loss.mul_add(self.length_minus_one, loss) * self.length_reciprocal;

                    self.phase = RsiPhase::Active {
                        prev_avg_gain,
                        prev_avg_loss,
                        avg_gain,
                        avg_loss,
                    };

                    Some(Rsi::rsi_from_averages(avg_gain, avg_loss))
                } else {
                    let (gain, loss) = Self::gain_and_loss(self.prev_price, price);

                    *sum_gain = *sum_gain - *prev_gain + gain;
                    *sum_loss = *sum_loss - *prev_loss + loss;
                    *prev_gain = gain;
                    *prev_loss = loss;

                    Some(Rsi::rsi_from_averages(
                        *sum_gain * self.length_reciprocal,
                        *sum_loss * self.length_reciprocal,
                    ))
                }
            }

            RsiPhase::Active {
                prev_avg_gain,
                prev_avg_loss,
                avg_gain,
                avg_loss,
            } => {
                if is_next_bar {
                    *prev_avg_gain = *avg_gain;
                    *prev_avg_loss = *avg_loss;
                }

                let (gain, loss) = Self::gain_and_loss(self.prev_price, price);

                *avg_gain =
                    prev_avg_gain.mul_add(self.length_minus_one, gain) * self.length_reciprocal;
                *avg_loss =
                    prev_avg_loss.mul_add(self.length_minus_one, loss) * self.length_reciprocal;

                Some(Rsi::rsi_from_averages(*avg_gain, *avg_loss))
            }
        };

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl Rsi {
    #[inline]
    fn gain_and_loss(prev_price: Price, price: Price) -> (Price, Price) {
        let change = price - prev_price;
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);

        (gain, loss)
    }

    #[inline]
    fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
        let sum = avg_gain + avg_loss;
        if sum == 0.0 {
            50.0
        } else {
            100.0 * avg_gain / sum
        }
    }
}

impl Display for Rsi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RSI({}, {})", self.config.length, self.config.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{Bar, nz};

    fn bar(price: f64, time: u64) -> Bar {
        Bar::new_with_open_time(price, price, price, price, time)
    }

    /// Returns a seeded RSI(3) after bars: 10, 12, 11, 13 at times 1–4.
    fn seeded_rsi3() -> Rsi {
        let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
        rsi.compute(&bar(10.0, 1));
        rsi.compute(&bar(12.0, 2));
        rsi.compute(&bar(11.0, 3));
        rsi.compute(&bar(13.0, 4));
        rsi
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_none_during_seed() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            assert_eq!(rsi.compute(&bar(10.0, 1)), None);
            assert_eq!(rsi.compute(&bar(12.0, 2)), None);
            assert_eq!(rsi.compute(&bar(11.0, 3)), None);
        }

        #[test]
        fn first_value_at_period_plus_one_bars() {
            let rsi = seeded_rsi3();
            assert!(rsi.value().is_some());
        }

        #[test]
        fn value_is_none_before_convergence() {
            let rsi = Rsi::new(RsiConfig::close(nz(14)));
            assert_eq!(rsi.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut rsi = seeded_rsi3();
            let computed = rsi.compute(&bar(14.0, 5));
            assert_eq!(rsi.value(), computed);
        }
    }

    mod seed_values {
        use super::*;

        #[test]
        fn all_gains_gives_100() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            rsi.compute(&bar(10.0, 1));
            rsi.compute(&bar(11.0, 2));
            rsi.compute(&bar(12.0, 3));
            assert_eq!(rsi.compute(&bar(13.0, 4)), Some(100.0));
        }
        #[test]
        fn all_losses_gives_0() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            rsi.compute(&bar(13.0, 1));
            rsi.compute(&bar(12.0, 2));
            rsi.compute(&bar(11.0, 3));
            assert_eq!(rsi.compute(&bar(10.0, 4)), Some(0.0));
        }

        #[test]
        fn equal_gains_and_losses_gives_50() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(2)));
            rsi.compute(&bar(10.0, 1));
            rsi.compute(&bar(11.0, 2));
            assert_eq!(rsi.compute(&bar(10.0, 3)), Some(50.0));
        }

        #[test]
        fn seed_rsi_computation() {
            // RSI(3): prices 10, 12, 11, 13
            // Changes: +2, -1, +2
            // avg_gain=4/3, avg_loss=1/3, RS=4, RSI=80
            let rsi = seeded_rsi3();
            assert!((rsi.value().unwrap() - 80.0).abs() < 1e-10);
        }
    }

    mod wilder_smoothing {
        use super::*;

        #[test]
        fn first_smoothed_value() {
            // Seed: avg_g=4/3, avg_l=1/3
            // Bar 5: change=+1, gain=1, loss=0
            // avg_g=(4/3*2+1)/3=11/9, avg_l=(1/3*2+0)/3=2/9
            // RS=11/2=5.5, RSI=100-100/6.5
            let mut rsi = seeded_rsi3();
            let value = rsi.compute(&bar(14.0, 5)).unwrap();
            let expected = 100.0 - (100.0 / (1.0 + 11.0 / 2.0));
            assert!((value - expected).abs() < 1e-10);
        }

        #[test]
        fn converges_toward_50_with_alternating() {
            // Alternating +1/-1 should push RSI toward 50
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            rsi.compute(&bar(10.0, 1));
            rsi.compute(&bar(11.0, 2));
            rsi.compute(&bar(10.0, 3));
            rsi.compute(&bar(11.0, 4)); // seed

            let mut prev = rsi.value().unwrap();
            for i in 0..20 {
                let price = if i % 2 == 0 { 10.0 } else { 11.0 };
                let val = rsi.compute(&bar(price, 5 + i)).unwrap();
                // Should be approaching 50
                if i > 10 {
                    assert!((val - 50.0).abs() < (prev - 50.0).abs() + 1.0);
                }
                prev = val;
            }
        }
    }

    mod bounds {
        use super::*;

        #[test]
        fn always_between_0_and_100() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            let prices = [
                100.0, 102.0, 99.0, 101.0, 98.0, 103.0, 97.0, 105.0, 96.0, 104.0, 50.0, 150.0,
            ];
            for (i, &p) in prices.iter().enumerate() {
                if let Some(value) = rsi.compute(&bar(p, i as u64 + 1)) {
                    assert!((0.0..=100.0).contains(&value), "RSI out of bounds: {value}");
                }
            }
        }
    }
    mod repaints {
        use super::*;

        #[test]
        fn active_repaint_updates_value() {
            let mut rsi = seeded_rsi3();

            let original = rsi.compute(&bar(14.0, 5)).unwrap();
            let repainted = rsi.compute(&bar(16.0, 5)).unwrap();
            assert!(repainted > original, "Higher price should give higher RSI");
        }

        #[test]
        fn multiple_repaints_match_single_computation() {
            let mut rsi = seeded_rsi3();
            rsi.compute(&bar(14.0, 5));
            rsi.compute(&bar(16.0, 5)); // repaint 1
            rsi.compute(&bar(12.0, 5)); // repaint 2
            let final_val = rsi.compute(&bar(15.0, 5)).unwrap();

            // Fresh computation with final price only
            let mut clean = seeded_rsi3();
            let expected = clean.compute(&bar(15.0, 5)).unwrap();

            assert!((final_val - expected).abs() < 1e-10);
        }

        #[test]
        fn repaint_then_advance_uses_repainted_price() {
            let mut rsi = seeded_rsi3();
            rsi.compute(&bar(14.0, 5));
            rsi.compute(&bar(15.0, 5)); // repaint bar 5
            let after_advance = rsi.compute(&bar(13.0, 6)).unwrap();

            // Compare: clean run with repainted price
            let mut clean = seeded_rsi3();
            clean.compute(&bar(15.0, 5)); // final price
            let expected = clean.compute(&bar(13.0, 6)).unwrap();
            assert!((after_advance - expected).abs() < 1e-10);
        }

        #[test]
        fn seed_repaint_adjusts_sum() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            rsi.compute(&bar(10.0, 1));
            rsi.compute(&bar(12.0, 2));
            rsi.compute(&bar(14.0, 2)); // repaint bar 2
            rsi.compute(&bar(11.0, 3));
            let value = rsi.compute(&bar(13.0, 4)).unwrap();

            // Clean run with repainted price
            let mut clean = Rsi::new(RsiConfig::close(nz(3)));
            clean.compute(&bar(10.0, 1));
            clean.compute(&bar(14.0, 2));
            clean.compute(&bar(11.0, 3));
            let expected = clean.compute(&bar(13.0, 4)).unwrap();

            assert!((value - expected).abs() < 1e-10);
        }
    }

    mod transition_repaint {
        use super::*;

        #[test]
        fn transition_repaint_matches_clean() {
            // Repaint bar 4 (transition bar) with a different price, then advance.
            // Exercises the second Seeding match arm's repaint path.
            let mut rsi = seeded_rsi3();
            rsi.compute(&bar(15.0, 4)); // repaint transition bar

            // Now advance to trigger Active phase with repainted value
            let value = rsi.compute(&bar(14.0, 5)).unwrap();

            // Clean run: seed with repainted bar 4 price directly
            let mut clean = Rsi::new(RsiConfig::close(nz(3)));
            clean.compute(&bar(10.0, 1));
            clean.compute(&bar(12.0, 2));
            clean.compute(&bar(11.0, 3));
            clean.compute(&bar(15.0, 4)); // bar 4 at repainted price
            let expected = clean.compute(&bar(14.0, 5)).unwrap();

            assert!((value - expected).abs() < 1e-10);
        }
    }

    mod flat_price {
        use super::*;

        #[test]
        fn flat_price_gives_50() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            for t in 1..=10 {
                let val = rsi.compute(&bar(100.0, t));
                if let Some(v) = val {
                    assert!((v - 50.0).abs() < 1e-10, "flat price should give RSI=50");
                }
            }
        }
    }

    mod length_one {
        use super::*;

        #[test]
        fn produces_on_bar_two() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(1)));
            assert_eq!(rsi.compute(&bar(10.0, 1)), None);
            assert!(rsi.compute(&bar(12.0, 2)).is_some());
        }

        #[test]
        fn length_one_values_correct() {
            // RSI(1): only 1 change in seed. gain=2, loss=0 → RSI=100
            let mut rsi = Rsi::new(RsiConfig::close(nz(1)));
            rsi.compute(&bar(10.0, 1));
            assert_eq!(rsi.compute(&bar(12.0, 2)), Some(100.0));

            // Next bar: loss. Wilder smoothing: avg_g=(2*0+0)/1=0, avg_l=(0*0+1)/1=1 → RSI=0
            assert_eq!(rsi.compute(&bar(11.0, 3)), Some(0.0));
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut rsi = seeded_rsi3();

            let mut cloned = rsi.clone();
            let orig_val = rsi.compute(&bar(14.0, 5)).unwrap();
            let clone_val = cloned.compute(&bar(9.0, 5)).unwrap();

            assert!(
                (orig_val - clone_val).abs() > 1e-10,
                "divergent inputs should give different RSI"
            );
        }
    }

    mod price_source {
        use super::*;
        use crate::test_util::Bar;

        #[test]
        fn uses_configured_source() {
            // HL2 = (high + low) / 2
            // Bar with high=20, low=10 → HL2=15, regardless of close
            let config = RsiConfig::builder()
                .length(nz(2))
                .source(PriceSource::HL2)
                .build();
            let mut rsi = Rsi::new(config);
            rsi.compute(&Bar::new_with_open_time(10.0, 20.0, 10.0, 5.0, 1)); // HL2=15
            rsi.compute(&Bar::new_with_open_time(10.0, 24.0, 12.0, 5.0, 2)); // HL2=18, change=+3
            let val = rsi
                .compute(&Bar::new_with_open_time(10.0, 22.0, 8.0, 5.0, 3))
                .unwrap(); // HL2=15, change=-3

            // equal gain and loss → RSI=50
            assert!((val - 50.0).abs() < 1e-10);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = RsiConfig::close(nz(14));
            assert_eq!(config.to_string(), "RsiConfig(14, Close)");
        }

        #[test]
        fn display_rsi() {
            let rsi = Rsi::new(RsiConfig::close(nz(14)));
            assert_eq!(rsi.to_string(), "RSI(14, Close)");
        }
    }

    mod config {
        use super::*;

        #[test]
        fn default_source_is_close() {
            let config = RsiConfig::builder().length(nz(14)).build();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "length is required")]
        fn panics_without_length() {
            let _ = RsiConfig::builder().build();
        }

        #[test]
        fn eq_and_hash() {
            use std::collections::HashSet;
            let a = RsiConfig::close(nz(14));
            let b = RsiConfig::close(nz(14));
            let c = RsiConfig::close(nz(7));
            assert_eq!(a, b);
            assert_ne!(a, c);

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }
    }

    #[cfg(debug_assertions)]
    mod invariants {
        use super::*;
        #[test]
        #[should_panic(expected = "open_time must be non-decreasing")]
        fn panics_on_decreasing_open_time() {
            let mut rsi = Rsi::new(RsiConfig::close(nz(3)));
            rsi.compute(&bar(10.0, 2));
            rsi.compute(&bar(12.0, 1));
        }
    }
}
