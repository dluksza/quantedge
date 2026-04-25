use std::{fmt::Display, hint::cold_path};

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Multiplier, Ohlcv, Price, PriceSource,
    internals::{BarAction, BarState},
};

/// Configuration for the Parabolic SAR ([`ParabolicSar`]) indicator.
///
/// Parabolic SAR (Stop and Reverse) uses an acceleration factor that
/// increases each time the extreme point makes a new high (long) or
/// low (short). The `af_step` controls the initial and incremental
/// acceleration, and `af_max` caps the maximum acceleration.
///
/// # Convergence
///
/// Output begins after 2 bars. The first bar establishes the initial
/// high/low/close, and the second bar determines the initial trend
/// direction.
///
/// # Example
///
/// ```
/// use quantedge_ta::{ParabolicSarConfig, Multiplier};
///
/// let config = ParabolicSarConfig::builder()
///     .af_step(Multiplier::new(0.02))
///     .af_max(Multiplier::new(0.2))
///     .build();
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ParabolicSarConfig {
    af_step: Multiplier,
    af_max: Multiplier,
}

impl IndicatorConfig for ParabolicSarConfig {
    type Builder = ParabolicSarConfigBuilder;
    type Output = ParabolicSarValue;

    fn builder() -> Self::Builder {
        ParabolicSarConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        PriceSource::Close
    }

    fn convergence(&self) -> usize {
        2
    }

    fn to_builder(&self) -> Self::Builder {
        ParabolicSarConfigBuilder {
            af_step: Some(self.af_step),
            af_max: Some(self.af_max),
        }
    }
}

impl Default for ParabolicSarConfig {
    fn default() -> Self {
        ParabolicSarConfig {
            af_step: Multiplier::new(0.02),
            af_max: Multiplier::new(0.2),
        }
    }
}

impl Display for ParabolicSarConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParabolicSarConfig(s: {}, m: {})",
            self.af_step, self.af_max
        )
    }
}

/// Builder for [`ParabolicSarConfig`].
///
/// Both `af_step` and `af_max` must be set before calling
/// [`build`](IndicatorConfigBuilder::build).
pub struct ParabolicSarConfigBuilder {
    af_step: Option<Multiplier>,
    af_max: Option<Multiplier>,
}

impl ParabolicSarConfigBuilder {
    fn new() -> Self {
        Self {
            af_step: None,
            af_max: None,
        }
    }

    #[must_use]
    pub fn af_step(mut self, value: Multiplier) -> Self {
        self.af_step.replace(value);
        self
    }

    #[must_use]
    pub fn af_max(mut self, value: Multiplier) -> Self {
        self.af_max.replace(value);
        self
    }
}

impl IndicatorConfigBuilder<ParabolicSarConfig> for ParabolicSarConfigBuilder {
    fn source(self, _source: PriceSource) -> Self {
        self
    }

    fn build(self) -> ParabolicSarConfig {
        ParabolicSarConfig {
            af_step: self.af_step.expect("af_step is required"),
            af_max: self.af_max.expect("af_max is required"),
        }
    }
}

/// Parabolic SAR output: SAR price level and trend direction.
///
/// When long (bullish), SAR trails below price as a stop-loss.
/// When short (bearish), SAR trails above price as a stop-loss.
///
/// ```text
/// SAR = prev_SAR + AF × (EP − prev_SAR)
/// ```
///
/// The SAR reverses when price penetrates the current SAR level.
/// On reversal, SAR resets to the extreme point of the prior trend.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParabolicSarValue {
    /// The current SAR price level.
    pub sar: Price,
    /// `true` when the trend is long (bullish, SAR below price).
    pub is_long: bool,
}

impl Display for ParabolicSarValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParabolicSarValue(s: {}, is_long: {})",
            self.sar, self.is_long
        )
    }
}

/// SAR algorithm state for the running phase.
#[derive(Clone, Copy, Debug)]
struct SarState {
    sar: f64,
    ep: f64,
    af: f64,
    is_long: bool,
    high: f64,
    low: f64,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    New,
    First { high: f64, low: f64, close: f64 },
    Seeding { high: f64, low: f64 },
    Running(SarState),
}

/// Parabolic SAR (Stop and Reverse) indicator.
///
/// A trend-following overlay that places trailing stops below price
/// in an uptrend (long) and above price in a downtrend (short). The
/// stop accelerates toward price using Wilder's acceleration factor:
///
/// ```text
/// SAR = prev_SAR + AF × (EP − prev_SAR)
/// ```
///
/// where EP is the extreme point (highest high in a long trend,
/// lowest low in a short trend) and AF starts at `af_step` and
/// increments by `af_step` each time EP makes a new extreme, up
/// to `af_max`.
///
/// When price penetrates SAR, the trend reverses: SAR resets to
/// the prior EP, and AF resets to `af_step`. The next SAR is
/// clamped against the prior two bars' highs (short) or lows
/// (long) to prevent the stop from jumping inside the price range.
///
/// Returns `None` until the second bar (convergence = 2).
///
/// Supports live repainting: feeding a bar with the same `open_time`
/// recomputes from the previous state without advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ohlcv, ParabolicSar, ParabolicSarConfig};
///
/// fn ohlc(o: f64, h: f64, l: f64, c: f64, t: u64) -> Ohlcv {
///     Ohlcv { open: o, high: h, low: l, close: c, volume: 0.0, open_time: t }
/// }
///
/// let mut sar = ParabolicSar::new(ParabolicSarConfig::default());
///
/// // First bar: collecting initial state
/// assert!(sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none());
///
/// // Second bar: first SAR value
/// let val = sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2)).unwrap();
/// assert!(val.is_long);  // close rose → long trend
/// assert!((val.sar - 5.0).abs() < f64::EPSILON);  // SAR = init low
/// ```
#[derive(Clone, Debug)]
pub struct ParabolicSar {
    config: ParabolicSarConfig,
    bar_state: BarState,
    phase: Phase,
    pending: SarState,
    current: Option<ParabolicSarValue>,
}

impl Indicator for ParabolicSar {
    type Config = ParabolicSarConfig;
    type Output = ParabolicSarValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            bar_state: BarState::new(PriceSource::Close),
            phase: Phase::New,
            pending: SarState {
                sar: 0.0,
                ep: 0.0,
                af: 0.0,
                is_long: true,
                high: 0.0,
                low: 0.0,
            },
            current: None,
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        self.current = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(close) => match self.phase {
                Phase::New => {
                    self.phase = Phase::First {
                        high: ohlcv.high,
                        low: ohlcv.low,
                        close,
                    };

                    None
                }
                Phase::First { high, low, .. } => {
                    self.phase = Phase::Seeding { high, low };

                    Some(self.initialize(ohlcv))
                }
                Phase::Seeding { .. } | Phase::Running(_) => {
                    self.phase = Phase::Running(self.pending);

                    Some(self.step(ohlcv))
                }
            },
            BarAction::Repaint(price) => match self.phase {
                Phase::New => None,
                Phase::First {
                    ref mut high,
                    ref mut low,
                    ref mut close,
                } => {
                    *high = ohlcv.high;
                    *low = ohlcv.low;
                    *close = price;

                    None
                }
                Phase::Seeding { .. } => Some(self.initialize(ohlcv)),
                Phase::Running(_) => Some(self.step(ohlcv)),
            },
        };

        self.current
    }

    #[inline]
    fn value(&self) -> Option<Self::Output> {
        self.current
    }
}

impl ParabolicSar {
    fn initialize(&mut self, ohlcv: &Ohlcv) -> ParabolicSarValue {
        cold_path();
        let Phase::Seeding { high, low } = self.phase else {
            unreachable!()
        };

        let ohlcv_high = ohlcv.high;
        let ohlcv_low = ohlcv.low;

        let plus_dm = ohlcv_high - high;
        let minus_dm = low - ohlcv_low;
        let is_long = !(minus_dm > plus_dm && minus_dm > 0.0);
        let af = self.config.af_step.value();

        let (output_sar, next_sar, ep) = if is_long {
            (
                low,
                Self::next_long(af, ohlcv_high, low, ohlcv_low, ohlcv_low),
                ohlcv_high,
            )
        } else {
            (
                high,
                Self::next_short(af, ohlcv_low, high, ohlcv_high, ohlcv_high),
                ohlcv_low,
            )
        };

        self.pending = SarState {
            sar: next_sar,
            ep,
            af,
            is_long,
            high: ohlcv_high,
            low: ohlcv_low,
        };

        ParabolicSarValue {
            sar: output_sar,
            is_long,
        }
    }

    fn step(&mut self, ohlcv: &Ohlcv) -> ParabolicSarValue {
        let Phase::Running(c) = self.phase else {
            unreachable!()
        };

        let ohlcv_high = ohlcv.high;
        let ohlcv_low = ohlcv.low;
        let af_step = self.config.af_step.value();
        let af_max = self.config.af_max.value();
        let mut sar = c.sar;
        let mut ep = c.ep;
        let mut af = c.af;
        let mut is_long = c.is_long;

        let (output_sar, next_sar) = if is_long {
            if ohlcv_low <= sar {
                is_long = false;
                ep = ep.max(ohlcv_high);
                sar = ep.max(c.high);
                ep = ohlcv_low;
                af = af_step;

                (sar, Self::next_short(af, ep, sar, c.high, ohlcv_high))
            } else {
                if ohlcv_high > ep {
                    ep = ohlcv_high;
                    af = (af + af_step).min(af_max);
                }

                (sar, Self::next_long(af, ep, sar, c.low, ohlcv_low))
            }
        } else if ohlcv_high >= sar {
            is_long = true;
            ep = ep.min(ohlcv_low);
            sar = ep.min(c.low);
            ep = ohlcv_high;
            af = af_step;

            (sar, Self::next_long(af, ep, sar, c.low, ohlcv_low))
        } else {
            if ohlcv_low < ep {
                ep = ohlcv_low;
                af = (af + af_step).min(af_max);
            }

            (sar, Self::next_short(af, ep, sar, c.high, ohlcv_high))
        };

        self.pending = SarState {
            sar: next_sar,
            ep,
            af,
            is_long,
            high: ohlcv_high,
            low: ohlcv_low,
        };

        ParabolicSarValue {
            sar: output_sar,
            is_long,
        }
    }

    #[inline]
    fn next_long(af: f64, ep: f64, sar: f64, prev_low: f64, curr_low: f64) -> f64 {
        af.mul_add(ep - sar, sar).min(prev_low).min(curr_low)
    }

    #[inline]
    fn next_short(af: f64, ep: f64, sar: f64, prev_high: f64, curr_high: f64) -> f64 {
        af.mul_add(ep - sar, sar).max(prev_high).max(curr_high)
    }
}

impl Display for ParabolicSar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ParabolicSar(s: {}, m: {})",
            self.config.af_step, self.config.af_max
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantedge_core::test_util::ohlc;

    fn default_sar() -> ParabolicSar {
        ParabolicSar::new(ParabolicSarConfig::default())
    }

    mod convergence {
        use super::*;

        #[test]
        fn none_on_first_bar() {
            let mut sar = default_sar();
            assert!(sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none());
        }

        #[test]
        fn some_on_second_bar() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            assert!(sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2)).is_some());
        }

        #[test]
        fn value_none_before_convergence() {
            let sar = default_sar();
            assert_eq!(sar.value(), None);
        }

        #[test]
        fn value_matches_last_compute() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            let computed = sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            assert_eq!(sar.value(), computed);
        }
    }

    mod initialization {
        use super::*;

        #[test]
        fn uptrend_sar_is_init_low() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            let val = sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2)).unwrap();
            // close[1]=16 > close[0]=12 → long, SAR = init_low = 5.0
            assert!(val.is_long);
            assert!((val.sar - 5.0).abs() < f64::EPSILON);
        }

        #[test]
        fn downtrend_sar_is_init_high() {
            let mut sar = default_sar();
            // -DM = 8 - 2 = 6, +DM = 14 - 15 = -1 → falling (6 > -1 && 6 > 0)
            sar.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
            let val = sar.compute(&ohlc(12.0, 14.0, 2.0, 8.0, 2)).unwrap();
            assert!(!val.is_long);
            assert!((val.sar - 15.0).abs() < f64::EPSILON);
        }

        #[test]
        fn equal_close_defaults_to_long() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            let val = sar.compute(&ohlc(12.0, 18.0, 8.0, 12.0, 2)).unwrap();
            assert!(val.is_long);
        }
    }

    mod computation {
        use super::*;
        use quantedge_core::test_util::assert_approx;

        #[test]
        fn sar_advances_toward_ep_in_uptrend() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2)).unwrap();
            // Long, SAR=5.0, EP=20.0, AF=0.02
            // Next SAR = 5.0 + 0.02*(20.0-5.0) = 5.3, clamped to min(8.0, 8.0) = 5.3
            let v2 = sar.compute(&ohlc(18.0, 22.0, 12.0, 20.0, 3)).unwrap();
            assert!(v2.is_long);
            assert_approx!(v2.sar, 5.3);
        }

        #[test]
        fn af_increments_on_new_ep() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            // Bar 2: long, EP=20, AF=0.02
            sar.compute(&ohlc(18.0, 25.0, 15.0, 22.0, 3));
            // Bar 3: high=25 > EP=20 → EP=25, AF=0.04
            // SAR for bar 4 = 5.3 + 0.04*(25.0-5.3) = 6.088
            let v = sar.compute(&ohlc(22.0, 30.0, 18.0, 28.0, 4)).unwrap();
            assert!(v.is_long);
            assert_approx!(v.sar, 6.088);
        }

        #[test]
        fn af_does_not_increment_without_new_ep() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            // Bar 2: long, EP=20, AF=0.02
            sar.compute(&ohlc(18.0, 19.0, 15.0, 17.0, 3));
            // Bar 3: high=19 < EP=20 → no EP update, AF stays 0.02
            // SAR for bar 4 = 5.3 + 0.02*(20.0-5.3) = 5.594
            let v = sar.compute(&ohlc(17.0, 21.0, 14.0, 19.0, 4)).unwrap();
            assert_approx!(v.sar, 5.594);
        }

        #[test]
        fn af_capped_at_max() {
            let mut sar = ParabolicSar::new(
                ParabolicSarConfig::builder()
                    .af_step(Multiplier::new(0.1))
                    .af_max(Multiplier::new(0.2))
                    .build(),
            );
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            // AF=0.1, EP=20
            sar.compute(&ohlc(18.0, 25.0, 15.0, 22.0, 3));
            // New EP=25, AF=0.2 (0.1+0.1, capped at 0.2)
            sar.compute(&ohlc(22.0, 30.0, 18.0, 28.0, 4));
            // New EP=30, AF still 0.2 (already at max)
            // SAR for bar 4 = 5.0 + 0.2*(25.0-5.0) = 9.0, clamped to min(8.0,15.0) = 8.0
            // SAR for bar 5 = 8.0 + 0.2*(30.0-8.0) = 12.4, clamped to min(15.0,18.0) = 12.4
            let v = sar.compute(&ohlc(28.0, 35.0, 22.0, 32.0, 5)).unwrap();
            assert_approx!(v.sar, 12.4);
        }

        #[test]
        fn reversal_long_to_short() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            assert!(sar.value().unwrap().is_long);

            // Bar 3: low=2.0 penetrates SAR → reverse to short
            // Confirmed SAR entering bar 3 = 5.0 (clamped)
            // low=2.0 <= 5.0 → reverse
            // New SAR = old EP = 20.0, clamped max(prev_high=15.0, prev_prev_high=15.0) = 20.0
            let v = sar.compute(&ohlc(8.0, 10.0, 2.0, 4.0, 3)).unwrap();
            assert!(!v.is_long);
            assert!((v.sar - 20.0).abs() < f64::EPSILON);
        }

        #[test]
        fn reversal_short_to_long() {
            let mut sar = default_sar();
            // -DM = 8 - 2 = 6, +DM = 14 - 15 = -1 → falling
            sar.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 14.0, 2.0, 8.0, 2));
            assert!(!sar.value().unwrap().is_long);

            // Short: SAR=15.0, EP=2.0
            // Next SAR for bar 3 = 15.0 + 0.02*(2.0-15.0) = 14.74
            // clamped max(15.0, 14.0) = 15.0
            // Bar 3: high=18.0 >= 15.0 → reverse to long
            // New SAR = old EP = 2.0, clamped min(2.0, prev_low=8.0) = 2.0
            let v = sar.compute(&ohlc(10.0, 18.0, 7.0, 16.0, 3)).unwrap();
            assert!(v.is_long);
            assert!((v.sar - 2.0).abs() < f64::EPSILON);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_value() {
            let mut sar = default_sar();
            // bar 1: H=15, L=8
            sar.compute(&ohlc(10.0, 15.0, 8.0, 12.0, 1));
            // Original: +DM=3, -DM=-2 → long
            let original = sar.compute(&ohlc(12.0, 18.0, 6.0, 16.0, 2)).unwrap();
            // Repaint: +DM=-1, -DM=6 → falling (short)
            let repainted = sar.compute(&ohlc(12.0, 14.0, 2.0, 8.0, 2)).unwrap();
            assert_ne!(original.is_long, repainted.is_long);
        }

        #[test]
        fn multiple_repaints_match_clean() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            sar.compute(&ohlc(12.0, 20.0, 6.0, 14.0, 2)); // repaint
            let final_val = sar.compute(&ohlc(12.0, 17.0, 9.0, 15.0, 2));

            let mut clean = default_sar();
            clean.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            let expected = clean.compute(&ohlc(12.0, 17.0, 9.0, 15.0, 2));

            assert_eq!(final_val, expected);
        }

        #[test]
        fn repaint_then_advance() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            sar.compute(&ohlc(12.0, 20.0, 7.0, 17.0, 2)); // repaint
            let after = sar.compute(&ohlc(17.0, 22.0, 14.0, 20.0, 3));

            let mut clean = default_sar();
            clean.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            clean.compute(&ohlc(12.0, 20.0, 7.0, 17.0, 2));
            let expected = clean.compute(&ohlc(17.0, 22.0, 14.0, 20.0, 3));

            assert_eq!(after, expected);
        }

        #[test]
        fn repaint_during_filling() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(10.0, 16.0, 4.0, 13.0, 1)); // repaint bar 1
            assert!(sar.value().is_none());

            let val = sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            assert!(val.is_some());

            // Should use repainted bar 1 data
            let mut clean = default_sar();
            clean.compute(&ohlc(10.0, 16.0, 4.0, 13.0, 1));
            let expected = clean.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            assert_eq!(val, expected);
        }

        #[test]
        fn repaint_bar3_matches_clean() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            sar.compute(&ohlc(18.0, 25.0, 15.0, 22.0, 3));
            sar.compute(&ohlc(18.0, 23.0, 16.0, 21.0, 3)); // repaint
            let after = sar.compute(&ohlc(21.0, 28.0, 18.0, 26.0, 4));

            let mut clean = default_sar();
            clean.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            clean.compute(&ohlc(12.0, 20.0, 8.0, 18.0, 2));
            clean.compute(&ohlc(18.0, 23.0, 16.0, 21.0, 3));
            let expected = clean.compute(&ohlc(21.0, 28.0, 18.0, 26.0, 4));

            assert_eq!(after, expected);
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut sar = default_sar();

            // Bar 1: open then close
            assert!(sar.compute(&ohlc(10.0, 14.0, 6.0, 11.0, 1)).is_none());
            assert!(sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1)).is_none()); // repaint

            // Bar 2: first value
            let v = sar.compute(&ohlc(12.0, 17.0, 9.0, 15.0, 2));
            assert!(v.is_some());
            let v = sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2)); // repaint
            assert!(v.is_some());

            // Bar 3
            let v3 = sar.compute(&ohlc(16.0, 22.0, 13.0, 20.0, 3));
            assert!(v3.is_some());

            // Verify against clean run with final prices
            let mut clean = default_sar();
            clean.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            clean.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            let expected = clean.compute(&ohlc(16.0, 22.0, 13.0, 20.0, 3));

            assert_eq!(v3, expected);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            let mut cloned = sar.clone();

            let orig = sar.compute(&ohlc(16.0, 25.0, 14.0, 22.0, 3)).unwrap();
            let clone_val = cloned.compute(&ohlc(8.0, 10.0, 2.0, 4.0, 3)).unwrap();

            assert_ne!(orig, clone_val);
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_values() {
            let config = ParabolicSarConfig::default();
            assert!((config.af_step.value() - 0.02).abs() < f64::EPSILON);
            assert!((config.af_max.value() - 0.2).abs() < f64::EPSILON);
        }

        #[test]
        fn convergence_is_2() {
            let config = ParabolicSarConfig::default();
            assert_eq!(config.convergence(), 2);
        }

        #[test]
        fn source_is_close() {
            let config = ParabolicSarConfig::default();
            assert_eq!(config.source(), PriceSource::Close);
        }

        #[test]
        #[should_panic(expected = "af_step is required")]
        fn panics_without_af_step() {
            let _ = ParabolicSarConfig::builder()
                .af_max(Multiplier::new(0.2))
                .build();
        }

        #[test]
        #[should_panic(expected = "af_max is required")]
        fn panics_without_af_max() {
            let _ = ParabolicSarConfig::builder()
                .af_step(Multiplier::new(0.02))
                .build();
        }

        #[test]
        fn eq_and_hash() {
            let a = ParabolicSarConfig::default();
            let b = ParabolicSarConfig::default();
            let c = ParabolicSarConfig::builder()
                .af_step(Multiplier::new(0.01))
                .af_max(Multiplier::new(0.2))
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
            let config = ParabolicSarConfig::builder()
                .af_step(Multiplier::new(0.03))
                .af_max(Multiplier::new(0.25))
                .build();
            assert_eq!(config.to_builder().build(), config);
        }
    }

    mod display {
        use super::*;

        #[test]
        fn display_config() {
            let config = ParabolicSarConfig::default();
            let s = config.to_string();
            assert!(s.starts_with("ParabolicSarConfig(s: "));
            assert!(s.contains("0.02"));
            assert!(s.contains("0.2"));
        }

        #[test]
        fn display_indicator() {
            let sar = default_sar();
            let s = sar.to_string();
            assert!(s.starts_with("ParabolicSar(s: "));
            assert!(s.contains("0.02"));
            assert!(s.contains("0.2"));
        }

        #[test]
        fn display_value() {
            let v = ParabolicSarValue {
                sar: 100.5,
                is_long: true,
            };
            assert_eq!(v.to_string(), "ParabolicSarValue(s: 100.5, is_long: true)");
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_convergence() {
            let sar = default_sar();
            assert_eq!(sar.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            assert!(sar.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut sar = default_sar();
            sar.compute(&ohlc(10.0, 15.0, 5.0, 12.0, 1));
            sar.compute(&ohlc(12.0, 18.0, 8.0, 16.0, 2));
            let computed = sar.compute(&ohlc(16.0, 22.0, 13.0, 20.0, 3));
            assert_eq!(sar.value(), computed);
        }
    }
}
