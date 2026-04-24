use std::fmt::Display;

use crate::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Multiplier, Ohlcv, Price, PriceSource,
    Timestamp, internals::extract_price,
};

/// Anchor period for VWAP session resets.
///
/// Determines when the cumulative sums reset. Fixed anchors
/// (e.g. [`Day`](Self::Day)) reset when `open_time % period == 0`.
/// [`User`](Self::User) disables automatic resets — call
/// [`Vwap::reset()`] manually.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VwapAnchor {
    Hour1,
    Hour2,
    Hour4,
    Hour8,
    Hour12,
    Day,
    User,
}

impl VwapAnchor {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn period_us(&self) -> Option<u64> {
        match self {
            Self::Hour1 => Some(3_600_000_000),
            Self::Hour2 => Some(7_200_000_000),
            Self::Hour4 => Some(14_400_000_000),
            Self::Hour8 => Some(28_800_000_000),
            Self::Hour12 => Some(43_200_000_000),
            Self::Day => Some(86_400_000_000),
            Self::User => None,
        }
    }
}

impl Display for VwapAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VwapAnchor::{}",
            match self {
                Self::Hour1 => "Hour1",
                Self::Hour2 => "Hour2",
                Self::Hour4 => "Hour4",
                Self::Hour8 => "Hour8",
                Self::Hour12 => "Hour12",
                Self::Day => "Day",
                Self::User => "User",
            }
        )
    }
}

/// Configuration for the Volume Weighted Average Price ([`Vwap`])
/// indicator.
///
/// VWAP weights price by volume to produce a running average
/// that resets on a configurable anchor period. Up to three
/// standard-deviation bands can be enabled.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ohlcv, Vwap, VwapAnchor, VwapConfig};
///
/// let mut vwap = Vwap::new(VwapConfig::builder().anchor(VwapAnchor::User).build());
///
/// let bar = Ohlcv {
///     open: 10.0, high: 10.0, low: 10.0, close: 10.0,
///     volume: 100.0, open_time: 1,
/// };
/// let v = vwap.compute(&bar).unwrap();
/// assert_eq!(v.vwap, 10.0);
/// ```
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct VwapConfig {
    band_1: Option<Multiplier>,
    band_2: Option<Multiplier>,
    band_3: Option<Multiplier>,
    source: PriceSource,
    anchor: VwapAnchor,
}

impl VwapConfig {
    /// First standard-deviation band multiplier, if configured.
    #[must_use]
    pub fn band_1(&self) -> Option<Multiplier> {
        self.band_1
    }

    /// Second standard-deviation band multiplier, if configured.
    #[must_use]
    pub fn band_2(&self) -> Option<Multiplier> {
        self.band_2
    }

    /// Third standard-deviation band multiplier, if configured.
    #[must_use]
    pub fn band_3(&self) -> Option<Multiplier> {
        self.band_3
    }

    /// Anchor period for session resets.
    #[must_use]
    pub fn anchor(&self) -> VwapAnchor {
        self.anchor
    }

    fn band_to_str(band: Option<Multiplier>) -> String {
        band.map_or("-".to_string(), |v| v.to_string())
    }
}

impl IndicatorConfig for VwapConfig {
    type Builder = VwapConfigBuilder;
    type Output = VwapValue;

    fn builder() -> Self::Builder {
        VwapConfigBuilder::new()
    }

    fn source(&self) -> PriceSource {
        self.source
    }

    fn convergence(&self) -> usize {
        1
    }

    fn to_builder(&self) -> Self::Builder {
        VwapConfigBuilder {
            band_1: self.band_1,
            band_2: self.band_2,
            band_3: self.band_3,
            source: self.source,
            anchor: self.anchor,
        }
    }
}

impl Default for VwapConfig {
    fn default() -> Self {
        Self {
            band_1: None,
            band_2: None,
            band_3: None,
            source: PriceSource::HLC3,
            anchor: VwapAnchor::Day,
        }
    }
}

impl Display for VwapConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VwapConfig(b1: {}, b2: {}, b3: {}, s: {}, a{})",
            VwapConfig::band_to_str(self.band_1),
            VwapConfig::band_to_str(self.band_2),
            VwapConfig::band_to_str(self.band_3),
            self.source,
            self.anchor,
        )
    }
}

/// Builder for [`VwapConfig`].
///
/// Defaults: source = [`PriceSource::HLC3`],
/// anchor = [`VwapAnchor::Day`], no bands.
pub struct VwapConfigBuilder {
    band_1: Option<Multiplier>,
    band_2: Option<Multiplier>,
    band_3: Option<Multiplier>,
    source: PriceSource,
    anchor: VwapAnchor,
}

impl VwapConfigBuilder {
    fn new() -> Self {
        Self {
            band_1: None,
            band_2: None,
            band_3: None,
            source: PriceSource::HLC3,
            anchor: VwapAnchor::Day,
        }
    }

    /// Sets the first standard-deviation band multiplier.
    #[must_use]
    pub fn band_1(mut self, value: Option<Multiplier>) -> Self {
        self.band_1 = value;
        self
    }

    /// Sets the second standard-deviation band multiplier.
    #[must_use]
    pub fn band_2(mut self, value: Option<Multiplier>) -> Self {
        self.band_2 = value;
        self
    }

    /// Sets the third standard-deviation band multiplier.
    #[must_use]
    pub fn band_3(mut self, value: Option<Multiplier>) -> Self {
        self.band_3 = value;
        self
    }

    /// Sets the anchor period for session resets.
    #[must_use]
    pub fn anchor(mut self, value: VwapAnchor) -> Self {
        self.anchor = value;
        self
    }
}

impl IndicatorConfigBuilder<VwapConfig> for VwapConfigBuilder {
    fn source(mut self, source: PriceSource) -> Self {
        self.source = source;
        self
    }

    fn build(self) -> VwapConfig {
        VwapConfig {
            band_1: self.band_1,
            band_2: self.band_2,
            band_3: self.band_3,
            source: self.source,
            anchor: self.anchor,
        }
    }
}

/// A single standard-deviation band around the VWAP line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VwapBand {
    upper: Price,
    lower: Price,
}

impl VwapBand {
    /// Upper band: `vwap + multiplier × std_dev`.
    #[inline]
    #[must_use]
    pub fn upper(&self) -> Price {
        self.upper
    }

    /// Lower band: `vwap − multiplier × std_dev`.
    #[inline]
    #[must_use]
    pub fn lower(&self) -> Price {
        self.lower
    }
}

impl Display for VwapBand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VwapBand(u: {}, l: {})", self.upper, self.lower)
    }
}

/// Output of [`Vwap::compute`]: the VWAP line and optional bands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VwapValue {
    /// The volume-weighted average price.
    pub vwap: Price,
    /// First standard-deviation band, if configured.
    pub band_1: Option<VwapBand>,
    /// Second standard-deviation band, if configured.
    pub band_2: Option<VwapBand>,
    /// Third standard-deviation band, if configured.
    pub band_3: Option<VwapBand>,
}

impl Display for VwapValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "VwapValue(vwap: {}, b1: {}, b2: {}, b3: {})",
            self.vwap,
            VwapValue::band_to_str(self.band_1),
            VwapValue::band_to_str(self.band_2),
            VwapValue::band_to_str(self.band_3),
        )
    }
}

impl VwapValue {
    fn band_to_str(band: Option<VwapBand>) -> String {
        band.map_or("-".to_string(), |v| v.to_string())
    }
}

/// Volume Weighted Average Price (VWAP).
///
/// Computes the cumulative average price weighted by volume,
/// resetting on a configurable anchor period. Optionally
/// produces up to three standard-deviation bands.
///
/// ```text
/// VWAP = Σ(price × volume) / Σ(volume)
/// ```
///
/// Supports live repainting: feeding a bar with the same
/// `open_time` recomputes from the previous state without
/// advancing.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Ohlcv, Vwap, VwapAnchor, VwapConfig};
///
/// fn bar(close: f64, volume: f64, time: u64) -> Ohlcv {
///     Ohlcv { open: close, high: close, low: close, close, volume, open_time: time }
/// }
///
/// let config = VwapConfig::builder()
///     .anchor(VwapAnchor::User)
///     .build();
/// let mut vwap = Vwap::new(config);
///
/// let v1 = vwap.compute(&bar(10.0, 100.0, 1)).unwrap();
/// assert_eq!(v1.vwap, 10.0);
///
/// // (10×100 + 20×200) / (100+200) = 5000/300 ≈ 16.667
/// let v2 = vwap.compute(&bar(20.0, 200.0, 2)).unwrap();
/// assert!((v2.vwap - 16.666_666_666_666_668).abs() < 1e-10);
/// ```
#[derive(Clone, Debug)]
pub struct Vwap {
    config: VwapConfig,
    cumulative_tpv: f64,
    cumulative_tpv2: f64,
    cumulative_volume: f64,
    pending_price: Price,
    pending_volume: f64,
    current: Option<VwapValue>,
    last_open_time: Option<Timestamp>,
    prev_close: Option<Price>,
    cur_close: Price,
    has_bands: bool,
    anchor: Option<u64>,
}

impl Indicator for Vwap {
    type Config = VwapConfig;
    type Output = VwapValue;

    fn new(config: Self::Config) -> Self {
        Self {
            config,
            cumulative_tpv: 0.0,
            cumulative_tpv2: 0.0,
            cumulative_volume: 0.0,
            pending_price: 0.0,
            pending_volume: 0.0,
            current: None,
            last_open_time: None,
            prev_close: None,
            cur_close: -1.0,
            has_bands: config.band_1.is_some()
                || config.band_2.is_some()
                || config.band_3.is_some(),
            anchor: config.anchor.period_us(),
        }
    }

    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output> {
        let volume = ohlcv.volume;
        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time);

        if is_next_bar {
            self.prev_close = Some(self.cur_close);
            self.last_open_time = Some(ohlcv.open_time);

            if self
                .anchor
                .is_some_and(|v| ohlcv.open_time.is_multiple_of(v))
            {
                self.do_reset();
            } else {
                self.cumulative_tpv += self.pending_price * self.pending_volume;
                self.cumulative_tpv2 += self.pending_price.powi(2) * self.pending_volume;
                self.cumulative_volume += self.pending_volume;
            }
        }

        self.cur_close = ohlcv.close;
        let price = extract_price(self.config.source, ohlcv, self.prev_close);

        self.pending_price = price;
        self.pending_volume = volume;

        let effective_volume = self.cumulative_volume + volume;

        self.current = if effective_volume < f64::EPSILON {
            None
        } else {
            let tpv = self.cumulative_tpv + price * volume;
            let vwap = tpv / effective_volume;

            if self.has_bands {
                let tpv2 = self.cumulative_tpv2 + price.powi(2) * volume;
                let variance = (tpv2 / effective_volume) - vwap.powi(2);
                let std_dev = variance.max(0.0).sqrt();

                Some(VwapValue {
                    vwap,
                    band_1: Self::compute_band(self.config.band_1, vwap, std_dev),
                    band_2: Self::compute_band(self.config.band_2, vwap, std_dev),
                    band_3: Self::compute_band(self.config.band_3, vwap, std_dev),
                })
            } else {
                Some(VwapValue {
                    vwap,
                    band_1: None,
                    band_2: None,
                    band_3: None,
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

impl Vwap {
    /// Manually resets cumulative sums.
    ///
    /// # Panics
    ///
    /// Panics if the anchor is not [`VwapAnchor::User`].
    pub fn reset(&mut self) {
        debug_assert!(
            self.config.anchor == VwapAnchor::User,
            "Only user-controlled VWAP can be reset. Set anchor to VwapAnchor::User"
        );
        self.do_reset();
    }

    fn do_reset(&mut self) {
        self.cumulative_tpv = 0.0;
        self.cumulative_tpv2 = 0.0;
        self.cumulative_volume = 0.0;
        self.pending_price = 0.0;
        self.pending_volume = 0.0;
    }
}

impl Vwap {
    fn compute_band(band: Option<Multiplier>, vwap: Price, std_dev: f64) -> Option<VwapBand> {
        band.map(|b| {
            let offset = b.value() * std_dev;

            VwapBand {
                upper: vwap + offset,
                lower: vwap - offset,
            }
        })
    }
}

impl Display for Vwap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Vwap(b1: {}, b2: {}, b3: {}, s: {})",
            VwapConfig::band_to_str(self.config.band_1),
            VwapConfig::band_to_str(self.config.band_2),
            VwapConfig::band_to_str(self.config.band_3),
            self.config.source,
        )
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use quantedge_core::test_util::{assert_approx, bar};

    fn vbar(close: f64, volume: f64, time: u64) -> quantedge_core::test_util::Bar {
        bar(close, time).vol(volume)
    }

    fn user_vwap() -> Vwap {
        Vwap::new(VwapConfig::builder().anchor(VwapAnchor::User).build())
    }

    fn user_vwap_with_bands() -> Vwap {
        Vwap::new(
            VwapConfig::builder()
                .anchor(VwapAnchor::User)
                .band_1(Some(Multiplier::new(1.0)))
                .band_2(Some(Multiplier::new(2.0)))
                .build(),
        )
    }

    mod convergence {
        use super::*;

        #[test]
        fn returns_value_on_first_bar() {
            let mut vwap = user_vwap();
            assert!(vwap.compute(&vbar(10.0, 100.0, 1)).is_some());
        }

        #[test]
        fn convergence_is_one() {
            assert_eq!(VwapConfig::default().convergence(), 1);
        }

        #[test]
        fn none_with_zero_volume() {
            let mut vwap = user_vwap();
            assert_eq!(vwap.compute(&vbar(10.0, 0.0, 1)), None);
        }
    }

    mod computation {
        use super::*;

        #[test]
        fn single_bar_equals_price() {
            let mut vwap = user_vwap();
            let v = vwap.compute(&vbar(10.0, 100.0, 1)).unwrap();
            assert_eq!(v.vwap, 10.0);
        }

        #[test]
        fn two_bars_weighted_average() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 200.0, 2)).unwrap();
            // (10*100 + 20*200) / (100+200) = 5000/300
            assert_approx!(v.vwap, 5000.0 / 300.0);
        }

        #[test]
        fn three_bars_cumulative() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));
            let v = vwap.compute(&vbar(30.0, 300.0, 3)).unwrap();
            // (10*100 + 20*200 + 30*300) / (100+200+300) = 14000/600
            assert_approx!(v.vwap, 14000.0 / 600.0);
        }

        #[test]
        fn equal_volume_gives_simple_average() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 100.0, 2)).unwrap();
            assert_approx!(v.vwap, 15.0);
        }

        #[test]
        fn no_bands_when_not_configured() {
            let mut vwap = user_vwap();
            let v = vwap.compute(&vbar(10.0, 100.0, 1)).unwrap();
            assert!(v.band_1.is_none());
            assert!(v.band_2.is_none());
            assert!(v.band_3.is_none());
        }
    }

    mod bands {
        use super::*;

        #[test]
        fn bands_present_when_configured() {
            let mut vwap = user_vwap_with_bands();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 200.0, 2)).unwrap();
            assert!(v.band_1.is_some());
            assert!(v.band_2.is_some());
            assert!(v.band_3.is_none());
        }

        #[test]
        fn bands_symmetric_around_vwap() {
            let mut vwap = user_vwap_with_bands();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 200.0, 2)).unwrap();
            let b1 = v.band_1.unwrap();
            let midpoint = b1.upper.midpoint(b1.lower);
            assert_approx!(midpoint, v.vwap);
        }

        #[test]
        fn band_2_wider_than_band_1() {
            let mut vwap = user_vwap_with_bands();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 200.0, 2)).unwrap();
            let b1 = v.band_1.unwrap();
            let b2 = v.band_2.unwrap();
            assert!(b2.upper > b1.upper);
            assert!(b2.lower < b1.lower);
        }

        #[test]
        fn single_price_zero_std_dev() {
            let mut vwap = user_vwap_with_bands();
            let v = vwap.compute(&vbar(10.0, 100.0, 1)).unwrap();
            let b1 = v.band_1.unwrap();
            // All same price → std_dev = 0 → bands collapse to vwap
            assert_eq!(b1.upper, v.vwap);
            assert_eq!(b1.lower, v.vwap);
        }
    }

    mod anchor_reset {
        use super::*;

        #[test]
        fn user_reset_clears_state() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));
            vwap.reset();
            let v = vwap.compute(&vbar(30.0, 300.0, 3)).unwrap();
            assert_eq!(v.vwap, 30.0);
        }

        #[test]
        #[should_panic(expected = "Only user-controlled VWAP can be reset")]
        fn reset_panics_for_non_user_anchor() {
            let mut vwap = Vwap::new(VwapConfig::default());
            vwap.reset();
        }

        #[test]
        fn day_anchor_resets_on_boundary() {
            let day_us: u64 = 86_400_000_000;
            let mut vwap = Vwap::new(VwapConfig::default()); // Day anchor

            // Bar in first day
            vwap.compute(&vbar(10.0, 100.0, day_us));
            vwap.compute(&vbar(20.0, 200.0, day_us + 1000));

            // Bar at next day boundary → reset
            let v = vwap.compute(&vbar(50.0, 500.0, day_us * 2)).unwrap();
            assert_eq!(v.vwap, 50.0);
        }
    }

    mod repaint {
        use super::*;

        #[test]
        fn updates_current_bar() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));
            // Repaint bar 2 with different price
            let v = vwap.compute(&vbar(30.0, 200.0, 2)).unwrap();
            // (10*100 + 30*200) / 300 = 7000/300
            assert_approx!(v.vwap, 7000.0 / 300.0);
        }

        #[test]
        fn multiple_repaints() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));
            vwap.compute(&vbar(25.0, 200.0, 2)); // repaint
            let v = vwap.compute(&vbar(30.0, 200.0, 2)).unwrap();
            assert_approx!(v.vwap, 7000.0 / 300.0);
        }

        #[test]
        fn repaint_then_advance() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));
            vwap.compute(&vbar(30.0, 200.0, 2)); // repaint
            // Advance: cumulative includes repainted bar 2
            let v = vwap.compute(&vbar(40.0, 100.0, 3)).unwrap();
            // (10*100 + 30*200 + 40*100) / (100+200+100) = 11000/400
            assert_approx!(v.vwap, 11000.0 / 400.0);
        }

        #[test]
        fn repaint_first_bar() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            let v = vwap.compute(&vbar(20.0, 200.0, 1)).unwrap();
            assert_eq!(v.vwap, 20.0);
        }
    }

    mod live_data {
        use super::*;

        #[test]
        fn mixed_open_and_closed_bars() {
            let mut vwap = user_vwap();

            // Bar 1: open
            let v = vwap.compute(&vbar(10.0, 100.0, 1)).unwrap();
            assert_eq!(v.vwap, 10.0);
            // Bar 1: close (repaint)
            let v = vwap.compute(&vbar(12.0, 150.0, 1)).unwrap();
            assert_eq!(v.vwap, 12.0);

            // Bar 2: open
            let v = vwap.compute(&vbar(15.0, 200.0, 2)).unwrap();
            // (12*150 + 15*200) / (150+200) = 4800/350
            assert_approx!(v.vwap, 4800.0 / 350.0);

            // Bar 2: close (repaint)
            let v = vwap.compute(&vbar(14.0, 200.0, 2)).unwrap();
            // (12*150 + 14*200) / (150+200) = 4600/350
            assert_approx!(v.vwap, 4600.0 / 350.0);

            // Bar 3
            let v = vwap.compute(&vbar(20.0, 300.0, 3)).unwrap();
            // (12*150 + 14*200 + 20*300) / (150+200+300) = 10600/650
            assert_approx!(v.vwap, 10600.0 / 650.0);
        }
    }

    mod clone {
        use super::*;

        #[test]
        fn produces_independent_state() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            vwap.compute(&vbar(20.0, 200.0, 2));

            let mut cloned = vwap.clone();

            let v_orig = vwap.compute(&vbar(30.0, 300.0, 3)).unwrap();
            let v_clone = cloned.compute(&vbar(50.0, 500.0, 3)).unwrap();

            assert!((v_orig.vwap - v_clone.vwap).abs() > 1e-10);
        }
    }

    mod config {
        use super::*;
        use std::collections::HashSet;

        #[test]
        fn default_values() {
            let config = VwapConfig::default();
            assert_eq!(config.source(), PriceSource::HLC3);
            assert_eq!(config.anchor(), VwapAnchor::Day);
            assert!(config.band_1.is_none());
            assert!(config.band_2.is_none());
            assert!(config.band_3.is_none());
        }

        #[test]
        fn eq_and_hash() {
            let a = VwapConfig::default();
            let b = VwapConfig::default();
            let c = VwapConfig::builder().anchor(VwapAnchor::User).build();

            let mut set = HashSet::new();
            set.insert(a);
            assert!(set.contains(&b));
            assert!(!set.contains(&c));
        }

        #[test]
        fn to_builder_roundtrip() {
            let config = VwapConfig::builder()
                .band_1(Some(Multiplier::new(1.0)))
                .anchor(VwapAnchor::Hour4)
                .build();
            assert_eq!(config.to_builder().build(), config);
        }

        #[test]
        fn display_config() {
            let config = VwapConfig::default();
            let s = config.to_string();
            assert!(s.contains("VwapConfig"));
        }
    }

    mod display {
        use super::*;

        #[test]
        fn formats_correctly() {
            let vwap = user_vwap();
            let s = vwap.to_string();
            assert!(s.contains("Vwap"));
        }

        #[test]
        fn anchor_display() {
            assert_eq!(VwapAnchor::Day.to_string(), "VwapAnchor::Day");
            assert_eq!(VwapAnchor::User.to_string(), "VwapAnchor::User");
        }

        #[test]
        fn band_display() {
            let band = VwapBand {
                upper: 10.0,
                lower: 5.0,
            };
            assert!(band.to_string().contains("10"));
        }

        #[test]
        fn value_display() {
            let val = VwapValue {
                vwap: 10.0,
                band_1: None,
                band_2: None,
                band_3: None,
            };
            assert!(val.to_string().contains("10"));
        }
    }

    mod value_accessor {
        use super::*;

        #[test]
        fn none_before_any_bar() {
            let vwap = user_vwap();
            assert_eq!(vwap.value(), None);
        }

        #[test]
        fn returns_current_value() {
            let mut vwap = user_vwap();
            vwap.compute(&vbar(10.0, 100.0, 1));
            assert!(vwap.value().is_some());
        }

        #[test]
        fn matches_last_compute() {
            let mut vwap = user_vwap();
            let computed = vwap.compute(&vbar(10.0, 100.0, 1));
            assert_eq!(vwap.value(), computed);
        }
    }
}
