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

use quantedge_ta::{Price, Timestamp};

fn to_price(value: &str) -> Price {
    match value.parse() {
        Ok(value) => value,
        Err(e) => panic!("cannot parse {value}: {e}"),
    }
}

pub(crate) struct BinanceOhlcv {
    pub(crate) open: Price,
    pub(crate) high: Price,
    pub(crate) low: Price,
    pub(crate) close: Price,
    pub(crate) volume: f64,
    pub(crate) open_time: Timestamp,
}

impl BinanceOhlcv {
    pub(crate) fn from_kline(kline: Kline) -> Self {
        Self {
            open: to_price(&kline.open),
            high: to_price(&kline.high),
            low: to_price(&kline.low),
            close: to_price(&kline.close),
            volume: to_price(&kline.volume),
            open_time: kline.open_time as Timestamp,
        }
    }

    pub(crate) fn from_kline_summary(kline: KlineSummary) -> Self {
        Self {
            open: to_price(&kline.open),
            high: to_price(&kline.high),
            low: to_price(&kline.low),
            close: to_price(&kline.close),
            volume: to_price(&kline.volume),
            open_time: kline.open_time as Timestamp,
        }
    }
}

pub(crate) fn stream_binance_klines(
    symbol: &str,
    interval: &str,
    history: u16,
    running: Arc<AtomicBool>,
) -> Result<impl Iterator<Item = BinanceOhlcv>, Box<Error>> {
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
        .map(|k| BinanceOhlcv::from_kline(k.kline));

    Ok(klines.chain(live))
}

fn history_data(
    symbol: &str,
    interval: &str,
    history: u16,
) -> Result<impl Iterator<Item = BinanceOhlcv>, Box<Error>> {
    let market: Market = Binance::new(None, None);
    let KlineSummaries::AllKlineSummaries(klines) =
        market.get_klines(symbol, interval, history, None, None)?;

    Ok(klines.into_iter().map(BinanceOhlcv::from_kline_summary))
}

fn subscription_cmd(symbol: &str, interval: &str) -> String {
    format!(
        "{}@kline_{}",
        symbol.to_lowercase(),
        interval.to_lowercase()
    )
}
