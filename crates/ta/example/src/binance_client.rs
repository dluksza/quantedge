use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
    },
    thread,
};

use binance::{
    api::Binance,
    errors::Error,
    market::Market,
    model::{Kline, KlineEvent, KlineSummaries, KlineSummary},
    websockets::{WebSockets, WebsocketEvent},
};

use quantedge_ta::{Ohlcv, Price, Timestamp};

/// NOTE: panics on malformed price data, intentional for demo purposes.
/// Production callers must validate input before this point
fn to_price(value: &str) -> Price {
    match value.parse() {
        Ok(value) => value,
        Err(e) => panic!("cannot parse {value}: {e}"),
    }
}

fn kline_to_ohlcv(kline: &Kline) -> Ohlcv {
    Ohlcv {
        open: to_price(&kline.open),
        high: to_price(&kline.high),
        low: to_price(&kline.low),
        close: to_price(&kline.close),
        volume: to_price(&kline.volume),
        open_time: kline.open_time as Timestamp,
    }
}

fn summary_to_ohlcv(kline: &KlineSummary) -> Ohlcv {
    Ohlcv {
        open: to_price(&kline.open),
        high: to_price(&kline.high),
        low: to_price(&kline.low),
        close: to_price(&kline.close),
        volume: to_price(&kline.volume),
        open_time: kline.open_time as Timestamp,
    }
}

pub(crate) fn stream_binance_klines(
    symbol: &str,
    interval: &str,
    history: u16,
    running: Arc<AtomicBool>,
) -> Result<impl Iterator<Item = Ohlcv>, Box<Error>> {
    let (tx, rx) = channel::<KlineEvent>();
    let subscription = subscription_cmd(symbol, interval);

    thread::spawn(move || {
        let mut ws = WebSockets::new(|event| match event {
            WebsocketEvent::Kline(kline) => {
                if tx.send(kline).is_err() {
                    running.store(false, Ordering::Relaxed);
                }

                Ok(())
            }
            _ => Ok(()),
        });

        if let Err(e) = ws
            .connect(&subscription)
            .and_then(|_| ws.event_loop(&running))
            .and_then(|_| ws.disconnect())
        {
            eprintln!("websocket worker failed: {e}");
        }
    });

    let klines = history_data(symbol, interval, history)?;
    let symbol = symbol.to_uppercase();
    let live = rx
        .into_iter()
        .filter(move |k| k.symbol == symbol)
        .map(|k| kline_to_ohlcv(&k.kline));

    Ok(klines.chain(live))
}

fn history_data(
    symbol: &str,
    interval: &str,
    history: u16,
) -> Result<impl Iterator<Item = Ohlcv>, Box<Error>> {
    let market: Market = Binance::new(None, None);
    let KlineSummaries::AllKlineSummaries(klines) =
        market.get_klines(symbol, interval, history, None, None)?;

    Ok(klines.into_iter().map(|k| summary_to_ohlcv(&k)))
}

fn subscription_cmd(symbol: &str, interval: &str) -> String {
    format!(
        "{}@kline_{}",
        symbol.to_lowercase(),
        interval.to_lowercase()
    )
}
