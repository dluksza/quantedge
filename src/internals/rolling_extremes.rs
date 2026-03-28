use crate::{Ohlcv, Price, internals::RingBuffer};

#[derive(Clone, Debug)]
pub(crate) struct RollingExtremes {
    oldest_pos: usize,
    highs: RingBuffer,
    lows: RingBuffer,
    high_val: Price,
    high_pos: usize,
    low_val: Price,
    low_pos: usize,
    forming_high: Price,
    forming_low: Price,
}

impl RollingExtremes {
    pub(crate) fn new(length: usize) -> Self {
        Self {
            oldest_pos: length - 1,
            highs: RingBuffer::new(length),
            lows: RingBuffer::new(length),
            high_val: -1.0,
            high_pos: 0,
            low_val: f64::MAX,
            low_pos: 0,
            forming_high: -1.0,
            forming_low: f64::MAX,
        }
    }

    #[must_use]
    pub(crate) fn highest_high(&self) -> Price {
        self.high_val.max(self.forming_high)
    }

    #[must_use]
    pub(crate) fn lowest_low(&self) -> Price {
        self.low_val.min(self.forming_low)
    }

    #[must_use]
    pub(crate) fn extremes(&self) -> (Price, Price) {
        (self.highest_high(), self.lowest_low())
    }

    pub(crate) fn push(&mut self, ohlcv: &impl Ohlcv) -> Option<(Price, Price)> {
        if self.high_val < 0.0 {
            self.high_val = ohlcv.high();
        }
        if self.low_val >= f64::MAX {
            self.low_val = ohlcv.low();
        }

        self.highs.push(ohlcv.high());

        if self.forming_high > self.high_val {
            self.high_val = self.forming_high;
            self.high_pos = 1;
        } else if self.high_pos == self.oldest_pos {
            (self.high_val, self.high_pos) = self
                .highs
                .find_value_and_index(|found, candidate| found < candidate);
        } else {
            self.high_pos += 1;
        }

        self.forming_high = ohlcv.high();

        self.lows.push(ohlcv.low());

        if self.forming_low < self.low_val {
            self.low_val = self.forming_low;
            self.low_pos = 1;
        } else if self.low_pos == self.oldest_pos {
            (self.low_val, self.low_pos) = self
                .lows
                .find_value_and_index(|found, candidate| found > candidate);
        } else {
            self.low_pos += 1;
        }

        self.forming_low = ohlcv.low();

        self.value()
    }

    pub(crate) fn replace(&mut self, ohlcv: &impl Ohlcv) -> Option<(Price, Price)> {
        self.highs.replace(ohlcv.high());
        self.lows.replace(ohlcv.low());

        self.forming_high = ohlcv.high();
        self.forming_low = ohlcv.low();

        self.value()
    }

    fn value(&self) -> Option<(Price, Price)> {
        if self.highs.is_ready() {
            Some((self.highest_high(), self.lowest_low()))
        } else {
            None
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::test_util::Bar;

    fn ohlc(high: f64, low: f64, close: f64) -> Bar {
        Bar::new(0.0, high, low, close)
    }

    mod push {
        use super::*;

        #[test]
        fn first_push_sets_extremes() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            assert_eq!(re.highest_high(), 20.0);
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn tracks_highest_high_across_bars() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(25.0, 12.0, 18.0));
            re.push(&ohlc(22.0, 11.0, 16.0));
            assert_eq!(re.highest_high(), 25.0);
        }

        #[test]
        fn tracks_lowest_low_across_bars() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(25.0, 8.0, 18.0));
            re.push(&ohlc(22.0, 11.0, 16.0));
            assert_eq!(re.lowest_low(), 8.0);
        }

        #[test]
        fn extreme_expires_when_pushed_out_of_window() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(50.0, 1.0, 25.0)); // extreme bar
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(22.0, 12.0, 17.0));
            // Window full; extreme still in
            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 1.0);

            // Push again: extreme bar falls out
            re.push(&ohlc(25.0, 11.0, 18.0));
            assert_eq!(re.highest_high(), 25.0);
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn returns_current_extremes() {
            let mut re = RollingExtremes::new(3);
            let actual = re.push(&ohlc(20.0, 10.0, 15.0));
            assert_eq!(actual, None);
        }
    }

    mod replace {
        use super::*;

        #[test]
        fn updates_forming_bar() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(22.0, 12.0, 17.0));
            // Repaint: higher high
            re.replace(&ohlc(30.0, 12.0, 18.0));
            assert_eq!(re.highest_high(), 30.0);
        }

        #[test]
        fn repaint_lower_high_triggers_rescan_if_was_extreme() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(30.0, 12.0, 18.0)); // this bar is the max
            // Repaint: lower high than bar 1
            re.replace(&ohlc(15.0, 12.0, 14.0));
            // Bar 1 (high=20) should now be the max
            assert_eq!(re.highest_high(), 20.0);
        }

        #[test]
        fn repaint_higher_low_triggers_rescan_if_was_extreme() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(22.0, 5.0, 18.0)); // this bar is the min
            // Repaint: higher low than bar 1
            re.replace(&ohlc(22.0, 15.0, 18.0));
            // Bar 1 (low=10) should now be the min
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn multiple_repaints_stable() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(22.0, 12.0, 17.0));
            re.replace(&ohlc(30.0, 8.0, 20.0));
            re.replace(&ohlc(18.0, 14.0, 16.0));
            re.replace(&ohlc(25.0, 11.0, 18.0));
            // Final state: bar 1 = (20,10), forming = (25,11)
            assert_eq!(re.highest_high(), 25.0);
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn returns_current_extremes() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            let actual = re.replace(&ohlc(25.0, 8.0, 18.0));
            assert_eq!(actual, None);
        }

        #[test]
        fn push_after_replace_commits_repainted_value() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));
            re.push(&ohlc(22.0, 12.0, 17.0));
            re.replace(&ohlc(50.0, 3.0, 25.0)); // repaint with extreme values

            // Advance: repainted values should be committed
            re.push(&ohlc(21.0, 11.0, 16.0));
            // Window: bar1=(20,10), repainted_bar2=(50,3), bar3=(21,11)
            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 3.0);
        }
    }

    mod window_size_one {
        use super::*;

        #[test]
        fn always_equals_current_bar() {
            let mut re = RollingExtremes::new(1);
            re.push(&ohlc(20.0, 10.0, 15.0));
            assert_eq!(re.highest_high(), 20.0);
            assert_eq!(re.lowest_low(), 10.0);

            re.push(&ohlc(15.0, 12.0, 13.0));
            assert_eq!(re.highest_high(), 15.0);
            assert_eq!(re.lowest_low(), 12.0);
        }
    }

    mod flat_market {
        use super::*;

        #[test]
        fn all_same_price() {
            let mut re = RollingExtremes::new(5);
            for _ in 0..10 {
                re.push(&ohlc(10.0, 10.0, 10.0));
            }
            assert_eq!(re.highest_high(), 10.0);
            assert_eq!(re.lowest_low(), 10.0);
        }
    }

    mod trending {
        use super::*;

        #[test]
        fn uptrend_max_always_newest() {
            let mut re = RollingExtremes::new(3);
            for i in 1..=6 {
                let f = f64::from(i);
                let h = 10.0 + f * 5.0;
                let l = 10.0 + f * 3.0;
                re.push(&ohlc(h, l, h - 2.0));
            }
            // In a pure uptrend, highest high is always the latest bar
            assert_eq!(re.highest_high(), 40.0); // bar 6: 10 + 6*5
        }

        #[test]
        fn downtrend_min_always_newest() {
            let mut re = RollingExtremes::new(3);
            for i in 1..=6 {
                let f = f64::from(i);
                let h = 50.0 - f * 3.0;
                let l = 50.0 - f * 5.0;
                re.push(&ohlc(h, l, l + 2.0));
            }
            // In a pure downtrend, lowest low is always the latest bar
            assert_eq!(re.lowest_low(), 20.0); // bar 6: 50 - 6*5
        }
    }

    mod corner_cases {
        use super::*;

        #[test]
        fn mid_window_extreme_ages_out() {
            let mut re = RollingExtremes::new(5);
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 1
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 2
            re.push(&ohlc(50.0, 1.0, 25.0)); // bar 3 — extreme
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 4
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 5 — window full

            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 1.0);

            // Push 2 more: extreme at bar 3 should age from pos 2→3→4 then expire
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 6
            assert_eq!(re.highest_high(), 50.0); // still in window
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 7
            assert_eq!(re.highest_high(), 50.0); // still in window (pos 4 = oldest)
            re.push(&ohlc(10.0, 10.0, 10.0)); // bar 8 — bar 3 evicted
            assert_eq!(re.highest_high(), 10.0);
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn consecutive_rescans() {
            // Extreme always at oldest position → rescan every push
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(50.0, 1.0, 25.0)); // extreme, will age to oldest
            re.push(&ohlc(10.0, 10.0, 10.0));
            re.push(&ohlc(10.0, 10.0, 10.0)); // now at oldest, triggers rescan
            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 1.0);

            // Next push evicts extreme
            re.push(&ohlc(10.0, 10.0, 10.0));
            assert_eq!(re.highest_high(), 10.0);
            assert_eq!(re.lowest_low(), 10.0);

            // Multiple pushes of flat data → rescans each time, stays correct
            for _ in 0..20 {
                re.push(&ohlc(10.0, 10.0, 10.0));
                assert_eq!(re.highest_high(), 10.0);
                assert_eq!(re.lowest_low(), 10.0);
            }
        }

        #[test]
        fn interleaved_replace_and_push() {
            let mut re = RollingExtremes::new(3);
            re.push(&ohlc(20.0, 10.0, 15.0));

            // Repaint forming to extreme, then push, repeat
            re.replace(&ohlc(50.0, 2.0, 25.0));
            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 2.0);

            re.push(&ohlc(15.0, 12.0, 13.0));
            // Previous forming (50, 2) now committed
            assert_eq!(re.highest_high(), 50.0);
            assert_eq!(re.lowest_low(), 2.0);

            re.replace(&ohlc(60.0, 1.0, 30.0));
            assert_eq!(re.highest_high(), 60.0);
            assert_eq!(re.lowest_low(), 1.0);

            re.push(&ohlc(10.0, 10.0, 10.0));
            // Window: (50,2), (60,1), forming=(10,10)
            assert_eq!(re.highest_high(), 60.0);
            assert_eq!(re.lowest_low(), 1.0);

            // Push enough to evict both extremes
            re.push(&ohlc(10.0, 10.0, 10.0));
            re.push(&ohlc(10.0, 10.0, 10.0));
            assert_eq!(re.highest_high(), 10.0);
            assert_eq!(re.lowest_low(), 10.0);
        }

        #[test]
        fn large_window_stress() {
            let mut re = RollingExtremes::new(100);
            // Fill window with ascending highs, constant low=0
            for i in 0..100 {
                let f = f64::from(i);
                re.push(&ohlc(f, 0.0, f));
            }
            assert_eq!(re.highest_high(), 99.0);
            assert_eq!(re.lowest_low(), 0.0);

            // Push 99 flat bars: one 0.0-low bar still in window
            for _ in 0..99 {
                re.push(&ohlc(5.0, 5.0, 5.0));
            }
            assert_eq!(re.lowest_low(), 0.0);

            // 100th flat push evicts the last 0.0 low
            re.push(&ohlc(5.0, 5.0, 5.0));
            assert_eq!(re.highest_high(), 5.0);
            assert_eq!(re.lowest_low(), 5.0);
        }
    }
}
