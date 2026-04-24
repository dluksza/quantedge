use crate::{
    Ohlcv, Price, PriceSource,
    internals::{BarAction, BarState, RingBuffer},
};

#[derive(Clone, Debug)]
pub(crate) struct PriceWindow<const SUM_OF_SQUARES: bool = false> {
    bar_state: BarState,
    window: RingBuffer,
    sum: Price,
    sum_of_squares: f64,
}

pub(crate) type PriceWindowWithSumOfSquares = PriceWindow<true>;

impl PriceWindow {
    pub fn new(size: usize, source: PriceSource) -> Self {
        Self {
            sum: 0.0,
            sum_of_squares: 0.0,
            bar_state: BarState::new(source),
            window: RingBuffer::new(size),
        }
    }
}

impl PriceWindow<true> {
    pub fn with_sum_of_squares(size: usize, source: PriceSource) -> Self {
        Self {
            sum: 0.0,
            sum_of_squares: 0.0,
            bar_state: BarState::new(source),
            window: RingBuffer::new(size),
        }
    }
}

impl<const SUM_OF_SQUARES: bool> PriceWindow<SUM_OF_SQUARES> {
    pub fn add(&mut self, ohlcv: &Ohlcv) -> Price {
        let price = match self.bar_state.handle(ohlcv) {
            BarAction::Advance(price) => {
                if let Some(old_price) = self.window.push(price) {
                    self.maintain_sums(old_price);
                }

                price
            }
            BarAction::Repaint(price) => {
                let old_price = self.window.replace(price);
                self.maintain_sums(old_price);

                price
            }
        };

        self.sum += price;
        if SUM_OF_SQUARES {
            self.sum_of_squares += price * price;
        }

        price
    }

    pub fn sum(&self) -> Option<Price> {
        self.is_ready().then_some(self.sum)
    }

    pub fn sum_of_squares(&self) -> Option<Price> {
        assert!(SUM_OF_SQUARES, "sum_of_squares requires PriceWindow<true>");
        self.is_ready().then_some(self.sum_of_squares)
    }

    pub fn fold<B, F>(&self, init: B, cb: F) -> B
    where
        F: FnMut(B, &Price) -> B,
    {
        self.window.fold(init, cb)
    }

    fn maintain_sums(&mut self, old_price: Price) {
        self.sum -= old_price;
        if SUM_OF_SQUARES {
            self.sum_of_squares -= old_price * old_price;
        }
    }

    fn is_ready(&self) -> bool {
        self.window.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod price_window {
        use super::*;
        use quantedge_core::test_util::{Bar, bar};

        fn close_window(size: usize) -> PriceWindow {
            PriceWindow::new(size, PriceSource::Close)
        }

        mod filling {
            use super::*;

            #[test]
            fn sum_is_none_when_empty() {
                let w = close_window(3);
                assert_eq!(w.sum(), None);
            }

            #[test]
            fn sum_is_none_until_window_full() {
                let mut w = close_window(3);
                w.add(&bar(10.0, 1));
                assert_eq!(w.sum(), None);
                w.add(&bar(20.0, 2));
                assert_eq!(w.sum(), None);
            }

            #[test]
            fn sum_returns_value_when_full() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 1));
                w.add(&bar(20.0, 2));
                assert_eq!(w.sum(), Some(30.0));
            }
        }

        mod sliding {
            use super::*;

            #[test]
            fn oldest_value_drops_on_advance() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 1));
                w.add(&bar(20.0, 2));
                w.add(&bar(30.0, 3));
                // 10 dropped, 20 + 30 = 50
                assert_eq!(w.sum(), Some(50.0));
            }

            #[test]
            fn slides_across_many_bars() {
                let mut w = close_window(2);
                w.add(&bar(1.0, 1));
                w.add(&bar(2.0, 2));
                w.add(&bar(3.0, 3));
                w.add(&bar(4.0, 4));
                w.add(&bar(5.0, 5));
                // 4 + 5 = 9
                assert_eq!(w.sum(), Some(9.0));
            }
        }

        mod repaint {
            use super::*;

            #[test]
            fn replaces_value_in_unfilled_window() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 1));
                w.add(&bar(15.0, 1)); // same timestamp, repaint
                w.add(&bar(20.0, 2));
                // 15 + 20 = 35
                assert_eq!(w.sum(), Some(35.0));
            }

            #[test]
            fn sum_stays_none_during_repaint_of_unfilled() {
                let mut w = close_window(3);
                w.add(&bar(10.0, 1));
                w.add(&bar(15.0, 1)); // repaint, still only 1 bar
                assert_eq!(w.sum(), None);
            }

            #[test]
            fn replaces_value_in_full_window() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 1));
                w.add(&bar(20.0, 2));
                assert_eq!(w.sum(), Some(30.0));

                w.add(&bar(25.0, 2)); // repaint bar 2
                assert_eq!(w.sum(), Some(35.0));
            }

            #[test]
            fn multiple_repaints_same_bar() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 1));
                w.add(&bar(20.0, 2));
                w.add(&bar(25.0, 2));
                w.add(&bar(30.0, 2));
                // 10 + 30 = 40
                assert_eq!(w.sum(), Some(40.0));
            }
        }

        mod open_time_zero {
            use super::*;

            #[test]
            fn first_bar_at_time_zero() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 0));
                w.add(&bar(20.0, 1));
                assert_eq!(w.sum(), Some(30.0));
            }

            #[test]
            fn repaint_at_time_zero() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 0));
                w.add(&bar(15.0, 0)); // repaint
                w.add(&bar(20.0, 1));
                assert_eq!(w.sum(), Some(35.0));
            }
        }

        mod window_size_one {
            use super::*;

            #[test]
            fn ready_after_one_bar() {
                let mut w = close_window(1);
                w.add(&bar(42.0, 1));
                assert_eq!(w.sum(), Some(42.0));
            }

            #[test]
            fn slides_with_size_one() {
                let mut w = close_window(1);
                w.add(&bar(10.0, 1));
                w.add(&bar(20.0, 2));
                assert_eq!(w.sum(), Some(20.0));
            }
        }

        mod true_range {
            use super::*;

            fn tr_window(size: usize) -> PriceWindow {
                PriceWindow::new(size, PriceSource::TrueRange)
            }

            fn ohlc(open: f64, high: f64, low: f64, close: f64, time: u64) -> Bar {
                Bar::new(open, high, low, close).at(time)
            }

            #[test]
            fn first_bar_uses_high_minus_low() {
                let mut w = tr_window(1);
                w.add(&ohlc(10.0, 30.0, 5.0, 20.0, 1));
                // No prev_close, falls back to 30 - 5 = 25
                assert_eq!(w.sum(), Some(25.0));
            }

            #[test]
            fn uses_prev_close_on_second_bar() {
                let mut w = tr_window(1);
                w.add(&ohlc(10.0, 30.0, 5.0, 20.0, 1));
                w.add(&ohlc(21.0, 25.0, 18.0, 22.0, 2));
                // hl = 7, |25 - 20| = 5, |18 - 20| = 2 → max = 7
                assert_eq!(w.sum(), Some(7.0));
            }

            #[test]
            fn gap_up_high_vs_prev_close_wins() {
                let mut w = tr_window(1);
                w.add(&ohlc(10.0, 15.0, 5.0, 10.0, 1));
                w.add(&ohlc(25.0, 30.0, 20.0, 28.0, 2));
                // hl = 10, |30 - 10| = 20, |20 - 10| = 10 → max = 20
                assert_eq!(w.sum(), Some(20.0));
            }

            #[test]
            fn gap_down_low_vs_prev_close_wins() {
                let mut w = tr_window(1);
                w.add(&ohlc(40.0, 50.0, 35.0, 45.0, 1));
                w.add(&ohlc(10.0, 15.0, 5.0, 12.0, 2));
                // hl = 10, |15 - 45| = 30, |5 - 45| = 40 → max = 40
                assert_eq!(w.sum(), Some(40.0));
            }

            #[test]
            fn prev_close_not_updated_on_repaint() {
                let mut w = tr_window(1);
                w.add(&ohlc(10.0, 15.0, 5.0, 10.0, 1)); // close = 10
                w.add(&ohlc(20.0, 25.0, 18.0, 22.0, 2)); // prev_close = 10
                w.add(&ohlc(20.0, 26.0, 19.0, 24.0, 2)); // repaint, prev_close still 10
                // hl = 7, |26 - 10| = 16, |19 - 10| = 9 → max = 16
                assert_eq!(w.sum(), Some(16.0));
            }

            #[test]
            fn prev_close_updates_on_advance() {
                let mut w = tr_window(1);
                w.add(&ohlc(10.0, 15.0, 5.0, 10.0, 1)); // close = 10
                w.add(&ohlc(20.0, 25.0, 18.0, 22.0, 2)); // prev_close = 10
                w.add(&ohlc(23.0, 28.0, 20.0, 25.0, 3)); // prev_close = 22 (bar 2's close)
                // hl = 8, |28 - 22| = 6, |20 - 22| = 2 → max = 8
                assert_eq!(w.sum(), Some(8.0));
            }

            #[test]
            fn sum_accumulates_true_range_values() {
                let mut w = tr_window(2);
                w.add(&ohlc(10.0, 20.0, 5.0, 15.0, 1));
                // TR1 = 15 (hl, no prev_close)
                assert_eq!(w.sum(), None);

                w.add(&ohlc(16.0, 22.0, 12.0, 18.0, 2));
                // hl = 10, |22 - 15| = 7, |12 - 15| = 3 → TR2 = 10
                // sum = 15 + 10 = 25
                assert_eq!(w.sum(), Some(25.0));
            }
        }

        mod invariants {
            use super::*;

            #[cfg(debug_assertions)]
            #[test]
            #[should_panic(expected = "open_time must be non-decreasing")]
            fn panics_on_decreasing_open_time() {
                let mut w = close_window(2);
                w.add(&bar(10.0, 2));
                w.add(&bar(20.0, 1));
            }
        }
    }
}
