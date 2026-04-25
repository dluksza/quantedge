//! Streaming technical analysis indicators for Rust.
//!
//! Indicators accept any type implementing [`Ohlcv`] and return
//! typed results. Values are `None` until enough data has been
//! received for convergence.
//!
//! Each indicator type ([`Sma`], [`Ema`], [`Bb`]), its config, and
//! its builder expose trait methods as inherent methods — no trait
//! import needed. Import [`Indicator`], [`IndicatorConfig`], or
//! [`IndicatorConfigBuilder`] only for generic code.

pub use quantedge_core::{
    Indicator, IndicatorConfig, IndicatorConfigBuilder, Ohlcv, Price, PriceSource, Timestamp,
};

mod indicators;
mod internals;
mod types;

pub use crate::indicators::*;
pub use crate::types::*;

macro_rules! impl_inherent_methods {
    ($indicator:ty, $config:ty, $builder:ty) => {
        // --- Indicator ---
        impl $indicator {
            /// See [`Indicator::new`].
            #[must_use]
            pub fn new(config: <Self as Indicator>::Config) -> Self {
                <Self as Indicator>::new(config)
            }

            /// See [`Indicator::compute`].
            #[inline]
            pub fn compute(&mut self, ohlcv: &Ohlcv) -> Option<<Self as Indicator>::Output> {
                <Self as Indicator>::compute(self, ohlcv)
            }

            /// See [`Indicator::value`].
            #[must_use]
            #[inline]
            pub fn value(&self) -> Option<<Self as Indicator>::Output> {
                <Self as Indicator>::value(self)
            }
        }

        // --- IndicatorConfig ---
        impl $config {
            /// See [`IndicatorConfig::builder`].
            #[must_use]
            pub fn builder() -> <Self as IndicatorConfig>::Builder {
                <Self as IndicatorConfig>::builder()
            }

            /// See [`IndicatorConfig::source`].
            #[must_use]
            pub fn source(&self) -> PriceSource {
                <Self as IndicatorConfig>::source(self)
            }

            /// See [`IndicatorConfig::convergence`].
            #[must_use]
            pub fn convergence(&self) -> usize {
                <Self as IndicatorConfig>::convergence(self)
            }

            /// See [`IndicatorConfig::to_builder`].
            #[must_use]
            pub fn to_builder(&self) -> <Self as IndicatorConfig>::Builder {
                <Self as IndicatorConfig>::to_builder(self)
            }
        }

        // --- IndicatorConfigBuilder ---
        impl $builder {
            /// See [`IndicatorConfigBuilder::source`].
            #[must_use]
            pub fn source(self, source: PriceSource) -> Self {
                <Self as IndicatorConfigBuilder<$config>>::source(self, source)
            }

            /// See [`IndicatorConfigBuilder::build`].
            #[must_use]
            pub fn build(self) -> $config {
                <Self as IndicatorConfigBuilder<$config>>::build(self)
            }
        }
    };
}

impl_inherent_methods!(Adx, AdxConfig, AdxConfigBuilder);
impl_inherent_methods!(Atr, AtrConfig, AtrConfigBuilder);
impl_inherent_methods!(Cci, CciConfig, CciConfigBuilder);
impl_inherent_methods!(Chop, ChopConfig, ChopConfigBuilder);
impl_inherent_methods!(Dc, DcConfig, DcConfigBuilder);
impl_inherent_methods!(Sma, SmaConfig, SmaConfigBuilder);
impl_inherent_methods!(Rsi, RsiConfig, RsiConfigBuilder);
impl_inherent_methods!(Ema, EmaConfig, EmaConfigBuilder);
impl_inherent_methods!(Kc, KcConfig, KcConfigBuilder);
impl_inherent_methods!(Bb, BbConfig, BbConfigBuilder);
impl_inherent_methods!(Ichimoku, IchimokuConfig, IchimokuConfigBuilder);
impl_inherent_methods!(Macd, MacdConfig, MacdConfigBuilder);
impl_inherent_methods!(Obv, ObvConfig, ObvConfigBuilder);
impl_inherent_methods!(ParabolicSar, ParabolicSarConfig, ParabolicSarConfigBuilder);
impl_inherent_methods!(Stoch, StochConfig, StochConfigBuilder);
impl_inherent_methods!(StochRsi, StochRsiConfig, StochRsiConfigBuilder);
impl_inherent_methods!(Supertrend, SupertrendConfig, SupertrendConfigBuilder);
impl_inherent_methods!(Vwap, VwapConfig, VwapConfigBuilder);
impl_inherent_methods!(WillR, WillRConfig, WillRConfigBuilder);

#[cfg(test)]
mod inherent_methods {
    use super::{Bb, BbConfig, BbValue, Ema, EmaConfig, Sma, SmaConfig};
    use quantedge_core::test_util::{bar, nz};

    #[test]
    fn sma_without_indicator_import() {
        let mut sma = Sma::new(SmaConfig::close(nz(2)));
        assert_eq!(sma.compute(&bar(10.0, 1)), None);
        assert_eq!(sma.compute(&bar(20.0, 2)), Some(15.0));
        assert_eq!(sma.value(), Some(15.0));
    }

    #[test]
    fn ema_without_indicator_import() {
        let mut ema = Ema::new(EmaConfig::close(nz(2)));
        assert_eq!(ema.compute(&bar(10.0, 1)), None);
        assert!(ema.compute(&bar(20.0, 2)).is_some());
        assert!(ema.value().is_some());
    }

    #[test]
    fn bb_without_indicator_import() {
        let mut bb = Bb::new(BbConfig::close(nz(2)));
        assert!(bb.compute(&bar(10.0, 1)).is_none());
        let v: Option<BbValue> = bb.compute(&bar(20.0, 2));
        assert!(v.is_some());
        assert!(bb.value().is_some());
    }

    #[test]
    fn config_methods_without_trait_import() {
        let config = SmaConfig::close(nz(20));
        assert_eq!(config.length(), 20);
        assert_eq!(config.source(), super::PriceSource::Close);

        let config = EmaConfig::close(nz(10));
        assert_eq!(config.length(), 10);

        let config = BbConfig::close(nz(5));
        assert_eq!(config.length(), 5);
    }

    #[test]
    fn builder_methods_without_trait_import() {
        let config = SmaConfig::builder()
            .length(nz(14))
            .source(super::PriceSource::HL2)
            .build();
        assert_eq!(config.length(), 14);
        assert_eq!(config.source(), super::PriceSource::HL2);

        let config = EmaConfig::builder().length(nz(20)).build();
        assert_eq!(config.length(), 20);

        let config = BbConfig::builder().length(nz(20)).build();
        assert_eq!(config.length(), 20);
    }
}
