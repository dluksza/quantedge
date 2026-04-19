use std::sync::{Arc, atomic::AtomicBool};

use quantedge_ta::{Ohlcv, Price, Sma, SmaConfig, Timestamp};

use crate::{
    binance_client::{BinanceOhlcv, stream_binance_klines},
    utils::{print_data, register_sigint},
};

mod binance_client;
mod utils;

// Implement the `Ohlcv` trait on the caller's own candle type. quantedge-ta
// never forces a conversion to a library-specific struct — any type that
// exposes the five required fields (`open`, `high`, `low`, `close`,
// `open_time`) works. `volume` is optional and defaults to 0.0.
//
// `open_time` is the bar-boundary key: two klines with the same `open_time`
// are treated as the *same* bar (a repaint); a new `open_time` advances the
// window.
impl Ohlcv for BinanceOhlcv {
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
    fn open_time(&self) -> Timestamp {
        self.open_time
    }
    fn volume(&self) -> f64 {
        self.volume
    }
}

fn main() {
    // `SmaConfig::default()` yields SMA(20) over Close. Every indicator config
    // exposes `convergence()`, the number of bars the indicator must see
    // before `compute()` begins returning `Some`. Used below to size the
    // history fetch so the live stream starts already converged.
    let config = SmaConfig::default();
    let mut sma = Sma::new(config);

    let running = Arc::new(AtomicBool::new(true));
    register_sigint(running.clone());

    // Binance emits a kline snapshot for the *forming* bar roughly every 2s
    // over the WebSocket. For a 5m interval that means ~150 updates per bar,
    // all sharing the same `open_time`, with `close`/`high`/`low`/`volume`
    // moving as new trades print. Only the final update for a bar is closed;
    // the rest are intra-bar repaints.
    //
    // `stream_binance_klines` prefixes `convergence()` historical closed bars
    // so the indicator is warm by the time live ticks arrive.
    let klines = stream_binance_klines("BTCUSDT", "5m", config.convergence() as u16, running);

    match klines {
        Ok(klines) => {
            for kline in klines {
                // `compute()` is O(1) and handles repainting internally:
                //   - same `open_time` as last call → replaces the current
                //     bar's contribution (the ~2s live snapshot case)
                //   - new `open_time` → advances the window
                // Output is `Option<f64>`: `None` until converged, never NaN.
                let sma_value = sma.compute(&kline);

                // Each printed line reflects the current state of the bar,
                // including in-progress ones. Expect the same `open_time` to
                // appear ~150 times per 5m bar with an evolving SMA value,
                // this is live repainting, not a bug.
                print_data(&kline, sma_value);
            }
        }
        Err(e) => {
            println!("err: {e}");
        }
    }
}
