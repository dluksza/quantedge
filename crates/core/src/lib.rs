//! Core types and traits shared across the quantedge crates.
//!
//! Defines the [`Ohlcv`] bar trait and its [`Price`] and [`Timestamp`]
//! aliases. Implement [`Ohlcv`] on your own kline/candle type to feed
//! it into downstream crates without per-tick conversion.

use std::num::NonZero;

mod indicator;
mod instrument;
mod ohlcv;
mod price_source;
mod snapshots;
mod timeframe;

#[cfg(any(test, feature = "test-util"))]
pub mod test_util;

pub use crate::indicator::{Indicator, IndicatorConfig, IndicatorConfigBuilder};
#[doc(hidden)]
pub use crate::indicator::{ErasedIndicatorConfig, ErasedIndicatorOutput};
pub use crate::instrument::{
    Asset, AssetError, Instrument, MarketKind, MarketKindError, Ticker, TickerError, Venue,
    VenueError,
};
pub use crate::ohlcv::{Ohlcv, Price, Timestamp};
pub use crate::price_source::PriceSource;
pub use crate::snapshots::{Bar, MarketSnapshot, TimeframeSnapshot};
pub use crate::timeframe::{TimeUnit, Timeframe};

/// Shorthand for constructing a [`NonZero<usize>`] from a literal.
///
/// Intended for indicator config call sites such as
/// `EmaConfig::builder().length(nz(9))`. Const-evaluable, so passing a
/// zero literal fails at compile time inside `const` contexts.
///
/// # Panics
///
/// Panics if `n == 0`.
#[must_use]
pub const fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).expect("nz requires a non-zero value")
}
