use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

/// Standard deviation multiplier for Bollinger Bands.
///
/// Wraps a positive, non-NaN `f64`. The constructor panics if the value is
/// zero, negative, or NaN.
///
/// Defaults to `2.0` (the standard Bollinger Bands setting).
///
/// Implements `Eq` and `Hash` via bit-level comparison, which is safe because
/// NaN is rejected at construction.
#[derive(Clone, Copy, Debug)]
pub struct StdDev(f64);

impl StdDev {
    /// Creates a new standard deviation multiplier.
    ///
    /// # Panics
    ///
    /// Panics if `value` is zero, negative, or NaN.
    #[must_use]
    pub fn new(value: f64) -> Self {
        assert!(!value.is_nan(), "std_dev must not be NaN");
        assert!(value > 0.0, "std_dev must be positive");
        Self(value)
    }

    /// Returns the standard deviation multiplier value.
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl PartialEq for StdDev {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for StdDev {}

impl Hash for StdDev {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Default for StdDev {
    fn default() -> Self {
        Self(2.0)
    }
}

impl Display for StdDev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StdDev({})", self.0)
    }
}
