//! Core types and traits shared across the quantedge crates.
//!
//! Defines the [`Ohlcv`] bar trait and its [`Price`] and [`Timestamp`]
//! aliases. Implement [`Ohlcv`] on your own kline/candle type to feed
//! it into downstream crates without per-tick conversion.

mod indicator;
mod instrument;
mod ohlcv;
mod price_source;
mod timeframe;

#[cfg(any(test, feature = "test-util"))]
pub mod test_util;

pub use crate::indicator::{Indicator, IndicatorConfig, IndicatorConfigBuilder};
pub use crate::instrument::{
    Asset, AssetError, Instrument, MarketKind, MarketKindError, Ticker, TickerError, Venue,
    VenueError,
};
pub use crate::ohlcv::{Ohlcv, Price, Timestamp};
pub use crate::price_source::PriceSource;
pub use crate::timeframe::{TimeUnit, Timeframe};
