use std::num::NonZero;

use crate::{Ohlcv, Price, Timestamp};

/// # Panics
/// Panics if `n` is zero.
pub fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).unwrap()
}

/// Asserts that two `f64` values are approximately equal using a
/// relative epsilon of `4 * f64::EPSILON`.
#[macro_export]
macro_rules! assert_approx {
    ($actual:expr, $expected:expr) => {{
        let (a, e) = ($actual, $expected);
        assert!(
            (a - e).abs() < e.abs() * 4.0 * f64::EPSILON,
            "assert_approx failed: actual={a}, expected={e}, diff={}",
            (a - e).abs(),
        );
    }};
}

pub use crate::assert_approx;

pub struct Bar {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub open_time: u64,
}

impl Bar {
    pub fn new(open: f64, high: f64, low: f64, close: f64) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume: 0.0,
            open_time: 0,
        }
    }

    pub fn new_with_open_time(
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        open_time: Timestamp,
    ) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume: 0.0,
            open_time,
        }
    }

    #[must_use]
    pub fn at(mut self, open_time: u64) -> Self {
        self.open_time = open_time;
        self
    }

    #[must_use]
    pub fn vol(mut self, volume: f64) -> Self {
        self.volume = volume;
        self
    }
}

/// Convenience: bar with just a close price and timestamp (OHLC all equal to close).
pub fn bar(close: f64, time: u64) -> Bar {
    Bar::new(close, close, close, close).at(time)
}

/// Convenience: bar with explicit OHLC and timestamp.
pub fn ohlc(open: f64, high: f64, low: f64, close: f64, time: u64) -> Bar {
    Bar::new(open, high, low, close).at(time)
}

impl Ohlcv for Bar {
    fn open(&self) -> Price {
        self.open
    }
    fn high(&self) -> Price {
        self.high
    }
    fn low(&self) -> Price {
        self.low
    }
    fn close(&self) -> Price {
        self.close
    }
    fn volume(&self) -> f64 {
        self.volume
    }
    fn open_time(&self) -> Timestamp {
        self.open_time
    }
}
