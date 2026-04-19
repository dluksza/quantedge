use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

/// Wraps a positive, non-NaN `f64`.
///
/// Used by indicators that scale a volatility measure (standard deviation,
/// ATR, etc.) by a constant factor — e.g. Bollinger Bands' `k`, Keltner
/// Channels' ATR multiplier, VWAP band distances, Supertrend's ATR multiplier.
///
/// Implements `Eq` and `Hash` via bit-level comparison, which is safe because
/// NaN is rejected at construction.
///
/// # Panics
///
/// [`Multiplier::new`] panics if the value is NaN or non-positive.
#[derive(Clone, Copy, Debug)]
pub struct Multiplier(f64);

impl Multiplier {
    /// Creates a new multiplier from the given value.
    ///
    /// # Panics
    ///
    /// Panics if `value` is NaN or not positive.
    #[must_use]
    pub fn new(value: f64) -> Self {
        assert!(!value.is_nan(), "multiplier must not be NaN");
        assert!(value > 0.0, "multiplier must be positive");
        Self(value)
    }

    /// Returns the inner `f64` value.
    #[must_use]
    #[inline]
    pub fn value(self) -> f64 {
        self.0
    }
}

impl PartialEq for Multiplier {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for Multiplier {}

impl Hash for Multiplier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl Display for Multiplier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Multiplier({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_accessor() {
        let m = Multiplier::new(2.5);
        assert!((m.value() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    #[should_panic(expected = "multiplier must not be NaN")]
    fn rejects_nan() {
        let _ = Multiplier::new(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "multiplier must be positive")]
    fn rejects_zero() {
        let _ = Multiplier::new(0.0);
    }

    #[test]
    #[should_panic(expected = "multiplier must be positive")]
    fn rejects_negative() {
        let _ = Multiplier::new(-1.0);
    }

    #[test]
    fn eq_and_hash() {
        use std::collections::HashSet;
        let a = Multiplier::new(1.5);
        let b = Multiplier::new(1.5);
        let c = Multiplier::new(2.0);
        assert_eq!(a, b);
        assert_ne!(a, c);

        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn display() {
        let m = Multiplier::new(2.0);
        assert_eq!(m.to_string(), "Multiplier(2)");
    }
}
