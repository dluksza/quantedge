use crate::{Ohlcv, Price, Timestamp};

#[doc(inline)]
pub use crate::nz;

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

pub type Bar = Ohlcv;

/// Test-only builder-style constructors on [`Ohlcv`]. Gated behind the
/// `test-util` feature so they do not leak into the public API — production
/// callers should build [`Ohlcv`] with a struct literal or a `From`
/// conversion from their own kline type.
impl Ohlcv {
    /// Construct a bar from OHLC values. `open_time` and `volume` default
    /// to `0`; override via [`at`](Self::at) and [`vol`](Self::vol).
    #[must_use]
    pub const fn new(open: Price, high: Price, low: Price, close: Price) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume: 0.0,
            open_time: 0,
        }
    }

    /// Sets `open_time`, consuming `self`.
    #[must_use]
    pub const fn at(mut self, open_time: Timestamp) -> Self {
        self.open_time = open_time;
        self
    }

    /// Sets `volume`, consuming `self`.
    #[must_use]
    pub const fn vol(mut self, volume: f64) -> Self {
        self.volume = volume;
        self
    }
}

/// Convenience: build a [`Bar`] with OHLC collapsed to `close` at `open_time`.
///
/// Used in ta unit tests where only the close price matters.
#[must_use]
pub fn bar(close: f64, open_time: Timestamp) -> Bar {
    Ohlcv::new(close, close, close, close).at(open_time)
}

/// Convenience: build a [`Bar`] with explicit OHLC at `open_time`.
#[must_use]
pub fn ohlc(open: f64, high: f64, low: f64, close: f64, open_time: Timestamp) -> Bar {
    Ohlcv::new(open, high, low, close).at(open_time)
}

/// Convenience shim for tests still using the previous 5-arg constructor.
#[must_use]
pub fn bar_at(open: f64, high: f64, low: f64, close: f64, open_time: Timestamp) -> Bar {
    Ohlcv::new(open, high, low, close).at(open_time)
}
