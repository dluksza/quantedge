//! Common instrument fixtures for tests.
//!
//! [`tickers`], [`market_kinds`], and [`venues`] expose constructors
//! for [`Ticker`](quantedge_core::Ticker),
//! [`MarketKind`](quantedge_core::MarketKind), and
//! [`Venue`](quantedge_core::Venue) values used across `test_util`'s
//! own tests and available to consumers building [`Instrument`]s for
//! their own snapshot fakes.
//!
//! [`Instrument`]: quantedge_core::Instrument

// Each fixture parses a static literal that cannot fail; the panic
// branch is unreachable, so a `# Panics` section would be noise.
#![allow(clippy::missing_panics_doc)]

/// Common [`Ticker`](quantedge_core::Ticker) fixtures for tests.
pub mod tickers {
    use quantedge_core::Ticker;

    /// `BTC/USDT` ticker.
    #[must_use]
    pub fn btcusdt() -> Ticker {
        "BTC/USDT".parse().expect("static literal must parse")
    }

    /// `ETH/USDT` ticker.
    #[must_use]
    pub fn ethusdt() -> Ticker {
        "ETH/USDT".parse().expect("static literal must parse")
    }
}

/// Common [`MarketKind`](quantedge_core::MarketKind) fixtures for tests.
pub mod market_kinds {
    use quantedge_core::MarketKind;

    /// Spot market.
    #[must_use]
    pub fn spot() -> MarketKind {
        "spot".parse().expect("static literal must parse")
    }

    /// Margin market.
    #[must_use]
    pub fn margin() -> MarketKind {
        "margin".parse().expect("static literal must parse")
    }
}

/// Common [`Venue`](quantedge_core::Venue) fixtures for tests.
pub mod venues {
    use quantedge_core::Venue;

    /// Synthetic `test` venue used by
    /// [`FakeMarketSnapshot::btcusdt`](super::super::FakeMarketSnapshot::btcusdt).
    #[must_use]
    pub fn test() -> Venue {
        "test".parse().expect("static literal must parse")
    }
}
