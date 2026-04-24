use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use chrono::{Local, TimeZone};

use quantedge_ta::{Ohlcv, Price};

pub(crate) fn register_sigint(running: Arc<AtomicBool>) {
    ctrlc::set_handler(move || {
        println!("\nShutting down...");
        running.store(false, Ordering::Relaxed);
    })
    .expect("install SIGINT handler");
}

pub(crate) fn print_data(kline: &Ohlcv, sma_value: Option<Price>) {
    let date_format = "%y.%m.%d %H:%M:%S";
    let now = Local::now().format(date_format);
    let open_time = Local
        .timestamp_millis_opt(kline.open_time as i64)
        .unwrap()
        .format(date_format);
    let sma = sma_value.map_or_else(|| "-".into(), |v| format!("{v:.2}"));

    println!(
        "{now}> open time: {open_time}, price: {:.2}: sma: {}",
        kline.close, sma
    );
}
