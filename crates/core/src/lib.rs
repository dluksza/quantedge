//! Core types and traits shared across the quantedge crates.
//!
//! Defines the [`Ohlcv`] bar trait and its [`Price`] and [`Timestamp`]
//! aliases. Implement [`Ohlcv`] on your own kline/candle type to feed
//! it into downstream crates without per-tick conversion.

mod instrument;
mod ohlcv;
mod timeframe;

pub use crate::instrument::{
    Asset, AssetError, Instrument, MarketKind, MarketKindError, Ticker, TickerError, Venue,
    VenueError,
};
pub use crate::ohlcv::{Ohlcv, Price, Timestamp};
pub use crate::timeframe::{TimeUnit, Timeframe};
