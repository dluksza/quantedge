//! Instrument identifiers for subscribing to venue data streams.
//!
//! [`Instrument`] combines a [`Venue`], a [`Ticker`] (base/quote asset
//! pair), and a [`MarketKind`]. The leaf newtypes wrap `Arc<str>`, so
//! each leaf clone is a single atomic increment; an [`Instrument`]
//! clone is four. No heap allocation on the clone path — instruments
//! flow cheaply through strategies, order paths, and log lines.
//!
//! All string-valued types share the same construction rules: leading
//! and trailing whitespace is trimmed; the remainder must be ASCII
//! alphanumeric or `_`. Case is normalized per type (lowercase for
//! [`Venue`] and [`MarketKind`], uppercase for [`Asset`]).

use std::{error::Error, fmt::Display, str::FromStr};

#[doc(hidden)]
macro_rules! ascii_ident {
    (
        $(#[$ty_meta:meta])*
        $ty:ident,
        $(#[$err_meta:meta])*
        $err:ident,
        $case:ident,
        $label:literal,
    ) => {
        $(#[$err_meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[non_exhaustive]
        pub enum $err {
            /// Input was empty or entirely whitespace.
            Empty,
            /// Input contained characters outside the permitted charset.
            Invalid,
        }

        impl std::fmt::Display for $err {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Empty => f.write_str(concat!($label, " is empty")),
                    Self::Invalid => f.write_str(concat!($label, " is invalid")),
                }
            }
        }

        impl std::error::Error for $err {}

        $(#[$ty_meta])*
        #[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $ty(std::sync::Arc<str>);

        impl $ty {
            /// Constructs a new value, trimming whitespace, validating
            /// the charset, and normalizing case.
            ///
            /// # Errors
            ///
            /// Returns the `Empty` variant for empty or whitespace-only
            /// input, and `Invalid` when any character falls outside
            /// the permitted charset.
            pub fn new(name: impl AsRef<str>) -> Result<Self, $err> {
                let name = name.as_ref().trim();

                if name.is_empty() {
                    return Err($err::Empty);
                }

                if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err($err::Invalid);
                }

                Ok(Self(name.$case().into()))
            }

            /// Returns the underlying string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $ty {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::borrow::Borrow<str> for $ty {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::str::FromStr for $ty {
            type Err = $err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl TryFrom<String> for $ty {
            type Error = $err;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

ascii_ident!(
    /// Trading venue identifier (e.g. `binance`, `coinbase`, `bybit_us`).
    ///
    /// Normalized to lowercase. Backed by `Arc<str>`; clones are O(1).
    Venue,
    /// Error returned when constructing a [`Venue`].
    VenueError,
    to_ascii_lowercase,
    "venue",
);

ascii_ident!(
    /// Single-asset identifier (e.g. `BTC`, `USDT`, `1000PEPE`).
    ///
    /// Normalized to uppercase. Backed by `Arc<str>`; clones are O(1).
    Asset,
    /// Error returned when constructing an [`Asset`].
    AssetError,
    to_ascii_uppercase,
    "asset",
);

ascii_ident!(
    /// Venue-specific market label (e.g. `spot`, `perp`, `usdm_perp`).
    ///
    /// The set of valid values is venue-defined and opaque to this
    /// crate; only the charset and case policy are enforced here.
    /// Normalized to lowercase. Backed by `Arc<str>`; clones are O(1).
    MarketKind,
    /// Error returned when constructing a [`MarketKind`].
    MarketKindError,
    to_ascii_lowercase,
    "market kind",
);

/// Error returned when constructing or parsing a [`Ticker`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum TickerError {
    /// Base and quote resolved to the same asset.
    SameAsset,
    /// Input did not match `<base>/<quote>`.
    Malformed,
    /// The base side failed to parse as an [`Asset`].
    InvalidBase(AssetError),
    /// The quote side failed to parse as an [`Asset`].
    InvalidQuote(AssetError),
}

impl Display for TickerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SameAsset => f.write_str("base and quote cannot be the same asset"),
            Self::Malformed => f.write_str("invalid ticker format, expected: <base>/<quote>"),
            Self::InvalidBase(err) => write!(f, "base asset error: {err}"),
            Self::InvalidQuote(err) => write!(f, "quote asset error: {err}"),
        }
    }
}

impl Error for TickerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidBase(e) | Self::InvalidQuote(e) => Some(e),
            Self::Malformed | Self::SameAsset => None,
        }
    }
}

/// A base/quote asset pair (e.g. `BTC/USDT`).
///
/// Construct via [`Ticker::new`] for typed input, or `parse()` for the
/// textual form `<base>/<quote>`. Prefer `parse()` when working with
/// untrusted input — it cannot silently swap base and quote.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ticker {
    base: Asset,
    quote: Asset,
}

impl Ticker {
    /// Separator used by [`Display`] and [`FromStr`].
    pub const SEPARATOR: char = '/';

    /// Constructs a ticker from typed base and quote assets.
    ///
    /// Callers working with textual input should prefer
    /// `"BTC/USDT".parse()` to eliminate ordering mistakes at the
    /// call site.
    ///
    /// # Errors
    ///
    /// Returns [`TickerError::SameAsset`] if base and quote are equal
    /// after [`Asset`] normalization.
    pub fn new(base: Asset, quote: Asset) -> Result<Self, TickerError> {
        if base == quote {
            return Err(TickerError::SameAsset);
        }

        Ok(Self { base, quote })
    }

    /// The base asset (what is being priced).
    #[must_use]
    pub fn base(&self) -> &Asset {
        &self.base
    }

    /// The quote asset (what it is priced in).
    #[must_use]
    pub fn quote(&self) -> &Asset {
        &self.quote
    }
}

impl Display for Ticker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{}", self.base, Self::SEPARATOR, self.quote)
    }
}

impl FromStr for Ticker {
    type Err = TickerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, quote) = s
            .split_once(Ticker::SEPARATOR)
            .ok_or(TickerError::Malformed)?;

        Ticker::new(
            base.parse().map_err(TickerError::InvalidBase)?,
            quote.parse().map_err(TickerError::InvalidQuote)?,
        )
    }
}

/// A fully qualified market identifier: venue, ticker, and market kind.
///
/// Used as the subscription key for data streams. Cloning is cheap —
/// one atomic increment per inner `Arc<str>` handle.
///
/// [`Display`] renders `<venue>:<base>/<quote>@<market>` for logging
/// and debugging. There is intentionally no `FromStr`; construct
/// instruments from their typed parts rather than parsing compound
/// strings.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Instrument {
    venue: Venue,
    ticker: Ticker,
    market: MarketKind,
}

impl Instrument {
    /// Constructs an instrument from its parts.
    #[must_use]
    pub fn new(venue: Venue, ticker: Ticker, market: MarketKind) -> Self {
        Self {
            venue,
            ticker,
            market,
        }
    }

    /// The venue.
    #[must_use]
    pub fn venue(&self) -> &Venue {
        &self.venue
    }

    /// The base/quote ticker.
    #[must_use]
    pub fn ticker(&self) -> &Ticker {
        &self.ticker
    }

    /// Shorthand for `self.ticker().base()`.
    #[must_use]
    pub fn base(&self) -> &Asset {
        &self.ticker.base
    }

    /// Shorthand for `self.ticker().quote()`.
    #[must_use]
    pub fn quote(&self) -> &Asset {
        &self.ticker.quote
    }

    /// The market kind.
    #[must_use]
    pub fn market(&self) -> &MarketKind {
        &self.market
    }
}

impl Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}@{}", self.venue, self.ticker, self.market)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn asset(s: &str) -> Asset {
        Asset::new(s).unwrap()
    }

    // --- Shared macro-generated behavior (exercised via Venue) ---

    #[test]
    fn venue_normalizes_to_lowercase() {
        assert_eq!(Venue::new("BINANCE").unwrap().as_str(), "binance");
    }

    #[test]
    fn venue_trims_whitespace() {
        assert_eq!(Venue::new("  binance  ").unwrap().as_str(), "binance");
    }

    #[test]
    fn venue_allows_digits_and_underscore() {
        assert!(Venue::new("bybit_us").is_ok());
        assert!(Venue::new("venue123").is_ok());
        assert!(Venue::new("1000venue").is_ok());
    }

    #[test]
    fn venue_rejects_empty() {
        assert_eq!(Venue::new(""), Err(VenueError::Empty));
        assert_eq!(Venue::new("   "), Err(VenueError::Empty));
    }

    #[test]
    fn venue_rejects_grammar_separators() {
        assert_eq!(Venue::new("a/b"), Err(VenueError::Invalid));
        assert_eq!(Venue::new("a:b"), Err(VenueError::Invalid));
        assert_eq!(Venue::new("a@b"), Err(VenueError::Invalid));
        assert_eq!(Venue::new("a b"), Err(VenueError::Invalid));
        assert_eq!(Venue::new("a-b"), Err(VenueError::Invalid));
    }

    #[test]
    fn venue_rejects_non_ascii() {
        assert_eq!(Venue::new("vénue"), Err(VenueError::Invalid));
        assert_eq!(Venue::new("日本"), Err(VenueError::Invalid));
    }

    #[test]
    fn venue_from_str_and_try_from_agree() {
        let a: Venue = "binance".parse().unwrap();
        let b = Venue::try_from(String::from("binance")).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn venue_display_outputs_normalized_value() {
        assert_eq!(Venue::new("BiNaNcE").unwrap().to_string(), "binance");
    }

    #[test]
    fn venue_borrow_enables_str_keyed_hashmap_lookup() {
        let mut map: HashMap<Venue, i32> = HashMap::new();
        map.insert(Venue::new("binance").unwrap(), 1);
        assert_eq!(map.get("binance"), Some(&1));
    }

    #[test]
    fn venue_error_display() {
        assert_eq!(VenueError::Empty.to_string(), "venue is empty");
        assert_eq!(VenueError::Invalid.to_string(), "venue is invalid");
    }

    // --- Asset (uppercase normalization) ---

    #[test]
    fn asset_normalizes_to_uppercase() {
        assert_eq!(Asset::new("btc").unwrap().as_str(), "BTC");
    }

    #[test]
    fn asset_allows_digit_prefixed_names() {
        assert!(Asset::new("1000PEPE").is_ok());
        assert!(Asset::new("1INCH").is_ok());
    }

    #[test]
    fn asset_error_display() {
        assert_eq!(AssetError::Empty.to_string(), "asset is empty");
        assert_eq!(AssetError::Invalid.to_string(), "asset is invalid");
    }

    // --- MarketKind (lowercase normalization) ---

    #[test]
    fn market_kind_normalizes_to_lowercase() {
        assert_eq!(MarketKind::new("SPOT").unwrap().as_str(), "spot");
    }

    #[test]
    fn market_kind_allows_venue_specific_labels() {
        assert!(MarketKind::new("usdm_perp").is_ok());
        assert!(MarketKind::new("coinm_futures").is_ok());
    }

    #[test]
    fn market_kind_error_display() {
        assert_eq!(MarketKindError::Empty.to_string(), "market kind is empty");
        assert_eq!(
            MarketKindError::Invalid.to_string(),
            "market kind is invalid"
        );
    }

    // --- Ticker ---

    #[test]
    fn ticker_new_succeeds_with_distinct_assets() {
        let t = Ticker::new(asset("btc"), asset("usdt")).unwrap();
        assert_eq!(t.base().as_str(), "BTC");
        assert_eq!(t.quote().as_str(), "USDT");
    }

    #[test]
    fn ticker_new_rejects_same_asset() {
        assert_eq!(
            Ticker::new(asset("btc"), asset("btc")),
            Err(TickerError::SameAsset)
        );
    }

    #[test]
    fn ticker_new_rejects_assets_equal_after_normalization() {
        // "btc" and "BTC" both normalize to "BTC".
        assert_eq!(
            Ticker::new(asset("btc"), asset("BTC")),
            Err(TickerError::SameAsset)
        );
    }

    #[test]
    fn ticker_from_str_parses_valid_pair() {
        let t: Ticker = "btc/usdt".parse().unwrap();
        assert_eq!(t.base().as_str(), "BTC");
        assert_eq!(t.quote().as_str(), "USDT");
    }

    #[test]
    fn ticker_from_str_rejects_empty_input() {
        assert_eq!("".parse::<Ticker>(), Err(TickerError::Malformed));
    }

    #[test]
    fn ticker_from_str_rejects_missing_separator() {
        assert_eq!("btcusdt".parse::<Ticker>(), Err(TickerError::Malformed));
    }

    #[test]
    fn ticker_from_str_rejects_extra_separator() {
        // split_once yields ("btc", "usdt/foo"); the '/' in the quote half
        // is rejected by Asset's charset.
        let err = "btc/usdt/foo".parse::<Ticker>().unwrap_err();
        assert!(matches!(
            err,
            TickerError::InvalidQuote(AssetError::Invalid)
        ));
    }

    #[test]
    fn ticker_from_str_rejects_empty_base() {
        let err = "/usdt".parse::<Ticker>().unwrap_err();
        assert!(matches!(err, TickerError::InvalidBase(AssetError::Empty)));
    }

    #[test]
    fn ticker_from_str_rejects_empty_quote() {
        let err = "btc/".parse::<Ticker>().unwrap_err();
        assert!(matches!(err, TickerError::InvalidQuote(AssetError::Empty)));
    }

    #[test]
    fn ticker_from_str_distinguishes_base_vs_quote_invalid_char() {
        let base_err = "b!c/usdt".parse::<Ticker>().unwrap_err();
        assert!(matches!(
            base_err,
            TickerError::InvalidBase(AssetError::Invalid)
        ));

        let quote_err = "btc/u!dt".parse::<Ticker>().unwrap_err();
        assert!(matches!(
            quote_err,
            TickerError::InvalidQuote(AssetError::Invalid)
        ));
    }

    #[test]
    fn ticker_from_str_rejects_same_asset() {
        assert_eq!("btc/BTC".parse::<Ticker>(), Err(TickerError::SameAsset));
    }

    #[test]
    fn ticker_display_round_trips_through_parse() {
        let t = Ticker::new(asset("btc"), asset("usdt")).unwrap();
        assert_eq!(t.to_string(), "BTC/USDT");
        assert_eq!(t.to_string().parse::<Ticker>().unwrap(), t);
    }

    #[test]
    fn ticker_error_source_exposes_inner_asset_error() {
        let base_err = TickerError::InvalidBase(AssetError::Invalid);
        assert_eq!(
            base_err.source().unwrap().to_string(),
            AssetError::Invalid.to_string()
        );

        let quote_err = TickerError::InvalidQuote(AssetError::Empty);
        assert_eq!(
            quote_err.source().unwrap().to_string(),
            AssetError::Empty.to_string()
        );
    }

    #[test]
    fn ticker_error_source_is_none_for_leaf_variants() {
        assert!(TickerError::SameAsset.source().is_none());
        assert!(TickerError::Malformed.source().is_none());
    }

    #[test]
    fn ticker_error_display_names_the_failing_side() {
        assert_eq!(
            TickerError::InvalidBase(AssetError::Invalid).to_string(),
            "base asset error: asset is invalid"
        );
        assert_eq!(
            TickerError::InvalidQuote(AssetError::Empty).to_string(),
            "quote asset error: asset is empty"
        );
    }

    // --- Instrument ---

    fn instrument() -> Instrument {
        Instrument::new(
            Venue::new("binance").unwrap(),
            Ticker::new(asset("btc"), asset("usdt")).unwrap(),
            MarketKind::new("perp").unwrap(),
        )
    }

    #[test]
    fn instrument_accessors_return_parts() {
        let i = instrument();
        assert_eq!(i.venue().as_str(), "binance");
        assert_eq!(i.ticker().base().as_str(), "BTC");
        assert_eq!(i.base().as_str(), "BTC");
        assert_eq!(i.quote().as_str(), "USDT");
        assert_eq!(i.market().as_str(), "perp");
    }

    #[test]
    fn instrument_display_renders_canonical_form() {
        assert_eq!(instrument().to_string(), "binance:BTC/USDT@perp");
    }

    #[test]
    fn instrument_equal_instances_hash_equal() {
        use std::hash::{DefaultHasher, Hash, Hasher};

        let a = instrument();
        let b = instrument();
        let mut ha = DefaultHasher::new();
        let mut hb = DefaultHasher::new();
        a.hash(&mut ha);
        b.hash(&mut hb);
        assert_eq!(a, b);
        assert_eq!(ha.finish(), hb.finish());
    }

    #[test]
    fn instrument_clone_equals_original() {
        let i = instrument();
        assert_eq!(i, i.clone());
    }
}
