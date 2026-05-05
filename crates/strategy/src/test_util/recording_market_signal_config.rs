//! Spy [`MarketSignalConfig`] for inspecting what a [`SignalGenerator`]
//! declared in its [`configure`] step.
//!
//! Pass a [`RecordingMarketSignalConfig`] through `configure` and assert
//! against the recorded required timeframes, required closed-bar
//! history, and registered indicator configs.
//!
//! [`SignalGenerator`]: crate::SignalGenerator
//! [`configure`]: crate::SignalGenerator::configure

use std::collections::HashSet;

use quantedge_core::{ErasedIndicatorConfig, IndicatorConfig, Timeframe};

use crate::MarketSignalConfig;

/// Spy implementation of [`MarketSignalConfig`] that records every
/// dependency a [`SignalGenerator`] declares in its
/// [`configure`](crate::SignalGenerator::configure), so tests can assert
/// the declared shape.
///
/// Duplicate declarations (the same timeframe or the same indicator
/// config registered more than once) silently coalesce — the trait
/// contract is additive and the recorder treats it as such.
///
/// [`SignalGenerator`]: crate::SignalGenerator
#[derive(Debug, Default)]
pub struct RecordingMarketSignalConfig {
    required_closed_bars: usize,
    timeframes: HashSet<Timeframe>,
    indicators: HashSet<Box<dyn ErasedIndicatorConfig>>,
}

impl RecordingMarketSignalConfig {
    /// Creates an empty recorder.
    ///
    /// All declarations are zero/empty until populated by passing this
    /// recorder through [`SignalGenerator::configure`].
    ///
    /// [`SignalGenerator::configure`]: crate::SignalGenerator::configure
    pub fn new() -> Self {
        Self::default()
    }

    /// Largest value passed to
    /// [`require_closed_bars`](MarketSignalConfig::require_closed_bars)
    /// across all calls. `0` if never called.
    pub fn required_closed_bars(&self) -> usize {
        self.required_closed_bars
    }

    /// Whether `timeframe` was declared via
    /// [`require_timeframes`](MarketSignalConfig::require_timeframes).
    pub fn has_timeframe(&self, timeframe: &Timeframe) -> bool {
        self.timeframes.contains(timeframe)
    }

    /// Whether the indicator identified by `config` was declared via
    /// [`register`](MarketSignalConfig::register).
    ///
    /// Identity is by [`IndicatorConfig`] equality — an indicator with
    /// different parameters (e.g. a different length) is a different
    /// indicator and will not match.
    pub fn has_indicator(&self, config: &impl IndicatorConfig) -> bool {
        self.indicators.contains(&config.clone_erased())
    }
}

impl MarketSignalConfig for RecordingMarketSignalConfig {
    fn require_timeframes(mut self, timeframes: &[Timeframe]) -> Self {
        for timeframe in timeframes {
            self.timeframes.insert(*timeframe);
        }
        self
    }

    fn require_closed_bars(mut self, bars: usize) -> Self {
        self.required_closed_bars = self.required_closed_bars.max(bars);
        self
    }

    fn register(mut self, config: &impl IndicatorConfig) -> Self {
        self.indicators.insert(config.clone_erased());
        self
    }
}

#[cfg(test)]
mod tests {
    use quantedge_core::{Timeframe, nz};
    use quantedge_ta::{EmaConfig, SmaConfig};

    use crate::{MarketSignalConfig, test_util::RecordingMarketSignalConfig};

    #[test]
    fn require_timeframes() {
        let recorder = RecordingMarketSignalConfig::new()
            .require_timeframes(&[Timeframe::HOUR_1, Timeframe::DAY_1]);

        assert!(recorder.has_timeframe(&Timeframe::HOUR_1));
        assert!(recorder.has_timeframe(&Timeframe::DAY_1));
        assert!(!recorder.has_timeframe(&Timeframe::MIN_1));
    }

    #[test]
    fn require_timeframes_duplicate_coalesces() {
        let recorder = RecordingMarketSignalConfig::new()
            .require_timeframes(&[Timeframe::HOUR_1])
            .require_timeframes(&[Timeframe::HOUR_1, Timeframe::DAY_1]);

        assert!(recorder.has_timeframe(&Timeframe::HOUR_1));
        assert!(recorder.has_timeframe(&Timeframe::DAY_1));
    }

    #[test]
    fn require_closed_bars() {
        let recorder = RecordingMarketSignalConfig::new().require_closed_bars(3);

        assert_eq!(recorder.required_closed_bars(), 3);
    }

    #[test]
    fn require_closed_bars_takes_max_across_calls() {
        let recorder = RecordingMarketSignalConfig::new()
            .require_closed_bars(3)
            .require_closed_bars(1)
            .require_closed_bars(5)
            .require_closed_bars(2);

        assert_eq!(recorder.required_closed_bars(), 5);
    }

    #[test]
    fn register_indicators() {
        let recorder = RecordingMarketSignalConfig::new()
            .register(&EmaConfig::close(nz(5)))
            .register(&SmaConfig::close(nz(50)));

        assert!(recorder.has_indicator(&EmaConfig::close(nz(5))));
        assert!(recorder.has_indicator(&SmaConfig::close(nz(50))));
        assert!(!recorder.has_indicator(&EmaConfig::close(nz(13))));
    }

    #[test]
    fn register_duplicate_coalesces() {
        let recorder = RecordingMarketSignalConfig::new()
            .register(&EmaConfig::close(nz(5)))
            .register(&EmaConfig::close(nz(5)));

        assert!(recorder.has_indicator(&EmaConfig::close(nz(5))));
    }
}
