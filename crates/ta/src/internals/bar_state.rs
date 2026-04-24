use crate::{Ohlcv, Price, PriceSource, Timestamp, internals::extract_price};

pub(crate) enum BarAction {
    Advance(Price),
    Repaint(Price),
}

#[derive(Clone, Debug)]
pub(crate) struct BarState {
    price_source: PriceSource,
    curr_close: Option<Price>,
    prev_close: Option<Price>,
    last_open_time: Option<Timestamp>,
}

impl BarState {
    pub(crate) fn new(price_source: PriceSource) -> Self {
        Self {
            price_source,
            curr_close: None,
            prev_close: None,
            last_open_time: None,
        }
    }

    pub(crate) fn handle(&mut self, ohlcv: &Ohlcv) -> BarAction {
        debug_assert!(
            self.last_open_time.is_none_or(|t| t <= ohlcv.open_time),
            "open_time must be non-decreasing: last={}, got={}",
            self.last_open_time.unwrap_or(0),
            ohlcv.open_time,
        );

        let is_next_bar = self.last_open_time.is_none_or(|t| t < ohlcv.open_time);

        if is_next_bar {
            self.prev_close = self.curr_close;
            self.last_open_time = Some(ohlcv.open_time);
        }

        self.curr_close = Some(ohlcv.close);

        let price = extract_price(self.price_source, ohlcv, self.prev_close);

        if is_next_bar {
            BarAction::Advance(price)
        } else {
            BarAction::Repaint(price)
        }
    }
}
