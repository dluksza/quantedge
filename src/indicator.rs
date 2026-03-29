use crate::{Ohlcv, PriceSource};

use std::{
    fmt::{Debug, Display},
    hash::Hash,
};

/// Configuration for a technical [`Indicator`].
///
/// Every indicator has a corresponding config type that holds its parameters
/// (length, price source, etc). Configs are value types: cheap to clone,
/// compare, and hash.
pub trait IndicatorConfig: Sized + PartialEq + Eq + Hash + Display + Debug {
    /// Builder type for constructing this config.
    type Builder: IndicatorConfigBuilder<Self>;

    /// Returns a new builder with default values.
    fn builder() -> Self::Builder;

    /// Price source to extract from each bar.
    fn source(&self) -> PriceSource;

    /// Bars until [`compute`](Indicator::compute) returns Some
    fn convergence(&self) -> usize;

    /// Returns a builder pre-filled with this config's values.
    fn to_builder(&self) -> Self::Builder;
}

/// Builder for an [`IndicatorConfig`].
pub trait IndicatorConfigBuilder<Config>
where
    Config: IndicatorConfig,
{
    /// Sets the price source.
    #[must_use]
    fn source(self, source: PriceSource) -> Self;

    /// Builds the config. Panics if required fields are missing.
    #[must_use]
    fn build(self) -> Config;
}

/// A streaming technical indicator.
///
/// Indicators maintain internal state and update incrementally on each call to
/// [`compute`](Indicator::compute). Output is `None` until enough data has been
/// received for convergence.
///
/// # Example
///
/// ```
/// use quantedge_ta::{Sma, SmaConfig};
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
/// let mut sma = Sma::new(SmaConfig::close(NonZero::new(3).unwrap()));
///
/// assert_eq!(sma.compute(&Bar(10.0, 1)), None);
/// assert_eq!(sma.compute(&Bar(20.0, 2)), None);
/// assert_eq!(sma.compute(&Bar(30.0, 3)), Some(20.0));
/// ```
pub trait Indicator: Sized + Clone + Display + Debug {
    /// Configuration type for this indicator.
    type Config: IndicatorConfig;

    /// Computed output type. `f64` for simple indicators,
    /// a struct for composite ones (e.g. Bollinger Bands).
    type Output: Send + Sync + Display + Debug;

    /// Creates a new indicator from the given config.
    fn new(config: Self::Config) -> Self;

    /// Feeds a bar and returns the updated indicator value,
    /// or `None` if not yet converged.
    ///
    /// Prefer using the return value directly over calling
    /// [`value()`](Self::value) separately.
    fn compute(&mut self, ohlcv: &impl Ohlcv) -> Option<Self::Output>;

    /// Returns the last computed indicator value without advancing state,
    /// or `None` if not yet converged.
    ///
    /// This is a cached field read — O(1) with no computation.
    fn value(&self) -> Option<Self::Output>;
}
