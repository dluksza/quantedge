use std::sync::{Arc, atomic::AtomicBool};

use quantedge_ta::{Sma, SmaConfig};

use crate::{
    binance_client::stream_binance_klines,
    utils::{print_data, register_sigint},
};

mod binance_client;
mod utils;

// Each kline coming from Binance is converted into an `Ohlcv` struct by
// the binance_client module. `open_time` is the bar-boundary key: two
// klines with the same `open_time` are treated as the *same* bar (a
// repaint); a new `open_time` advances the window.

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
