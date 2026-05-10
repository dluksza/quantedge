use crate::{Ohlcv, PriceSource};

use std::{
    any::{Any, TypeId},
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
};

/// Configuration for a technical [`Indicator`].
///
/// Every indicator has a corresponding config type that holds its parameters
/// (length, price source, etc). Configs are value types: cheap to clone,
/// compare, and hash.
pub trait IndicatorConfig:
    Sized + Default + Clone + Eq + Hash + Debug + Display + Send + Sync + 'static
{
    /// The [`Indicator`] this config produces. Lets generic code resolve
    /// the indicator from the config alone.
    type Indicator: Indicator<Config = Self, Output = Self::Output>;

    /// Builder type for constructing this config.
    type Builder: IndicatorConfigBuilder<Self>;

    /// Computed output type. [`crate::Price`] for simple indicators,
    /// a struct for composite ones (e.g. Bollinger Bands).
    type Output: Copy + PartialEq + Debug + Display + Send + Sync + 'static;

    /// Returns a new builder with default values.
    fn builder() -> Self::Builder;

    /// Price source to extract from each bar.
    fn source(&self) -> PriceSource;

    /// Bars until [`compute`](Indicator::compute) returns Some
    fn convergence(&self) -> usize;

    /// Returns a builder pre-filled with this config's values.
    fn to_builder(&self) -> Self::Builder;
}

/// Object-safe view of an [`IndicatorConfig`].
///
/// Plumbing for crates that store heterogeneous configs (engines,
/// test utilities). Strategy code should use the typed
/// [`IndicatorConfig`] API. Identity is the pair `(TypeId, hash)`:
/// two boxes compare equal iff they hold the same concrete type with
/// equal contents.
///
/// Blanket-impl'd for every [`IndicatorConfig`]; downstream crates
/// that define new config types automatically participate.
#[doc(hidden)]
pub trait ErasedIndicatorConfig: Any + Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn dyn_eq(&self, other: &dyn ErasedIndicatorConfig) -> bool;
    fn dyn_hash(&self, hasher: &mut dyn Hasher);
    fn clone_erased(&self) -> Box<dyn ErasedIndicatorConfig>;
}

impl<C: IndicatorConfig> ErasedIndicatorConfig for C {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn ErasedIndicatorConfig) -> bool {
        other.as_any().downcast_ref::<C>() == Some(self)
    }

    fn dyn_hash(&self, mut hasher: &mut dyn Hasher) {
        TypeId::of::<C>().hash(&mut hasher);
        Hash::hash(self, &mut hasher);
    }

    fn clone_erased(&self) -> Box<dyn ErasedIndicatorConfig> {
        Box::new(self.clone())
    }
}

impl PartialEq for dyn ErasedIndicatorConfig {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other)
    }
}

impl Eq for dyn ErasedIndicatorConfig {}

impl Hash for dyn ErasedIndicatorConfig {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
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
pub trait Indicator: Sized + Clone + Debug + Display + Send + Sync + 'static {
    /// Configuration type for this indicator.
    type Config: IndicatorConfig<Output = Self::Output>;

    /// Computed output type. [`crate::Price`] for simple indicators,
    /// a struct for composite ones (e.g. Bollinger Bands).
    type Output: Copy + PartialEq + Debug + Display + Send + Sync + 'static;

    /// Creates a new indicator from the given config.
    fn new(config: Self::Config) -> Self;

    /// Feeds a bar and returns the updated indicator value,
    /// or `None` if not yet converged.
    ///
    /// Prefer using the return value directly over calling
    /// [`value()`](Self::value) separately.
    fn compute(&mut self, ohlcv: &Ohlcv) -> Option<Self::Output>;

    /// Returns the last computed indicator value without advancing state,
    /// or `None` if not yet converged.
    ///
    /// This is a cached field read — O(1) with no computation.
    fn value(&self) -> Option<Self::Output>;
}
