#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use crate::fixtures::{load_reference_ohlcvs, repaint_sequence};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use quantedge_ta::{
    Adx, AdxConfig, Atr, AtrConfig, Bb, BbConfig, Cci, CciConfig, Dc, DcConfig, Ema, EmaConfig, Kc,
    KcConfig, Macd, MacdConfig, Rsi, RsiConfig, Sma, SmaConfig, Stoch, StochConfig, WillR,
    WillRConfig,
};
use std::{hint::black_box, num::NonZero, time::Duration};

fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).expect("non zero value")
}

/// Calls `$m!(name, Type, config)` for every indicator configuration.
macro_rules! all_indicators {
    ($m:ident) => {
        $m!("sma20", Sma, SmaConfig::close(nz(20)));
        $m!("sma200", Sma, SmaConfig::close(nz(200)));
        $m!("ema20", Ema, EmaConfig::close(nz(20)));
        $m!("ema200", Ema, EmaConfig::close(nz(200)));
        $m!("bb20", Bb, BbConfig::close(nz(20)));
        $m!("bb200", Bb, BbConfig::close(nz(200)));
        $m!("rsi14", Rsi, RsiConfig::close(nz(14)));
        $m!("rsi140", Rsi, RsiConfig::close(nz(140)));
        $m!("macd12269", Macd, MacdConfig::close(nz(12), nz(26), nz(9)));
        $m!(
            "macd12026090",
            Macd,
            MacdConfig::close(nz(120), nz(260), nz(90))
        );
        $m!("atr14", Atr, AtrConfig::builder().length(nz(14)).build());
        $m!("atr140", Atr, AtrConfig::builder().length(nz(140)).build());
        $m!(
            "stoch1433",
            Stoch,
            StochConfig::builder()
                .length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build()
        );
        $m!(
            "stoch140303",
            Stoch,
            StochConfig::builder()
                .length(nz(140))
                .k_smooth(nz(30))
                .d_smooth(nz(30))
                .build()
        );
        $m!(
            "kc2010",
            Kc,
            KcConfig::builder()
                .length(nz(20))
                .atr_length(nz(10))
                .build()
        );
        $m!(
            "kc200100",
            Kc,
            KcConfig::builder()
                .length(nz(200))
                .atr_length(nz(100))
                .build()
        );
        $m!("dc20", Dc, DcConfig::builder().length(nz(20)).build());
        $m!("dc200", Dc, DcConfig::builder().length(nz(200)).build());
        $m!("adx14", Adx, AdxConfig::builder().length(nz(14)).build());
        $m!("adx140", Adx, AdxConfig::builder().length(nz(140)).build());
        $m!(
            "willr14",
            WillR,
            WillRConfig::builder().length(nz(14)).build()
        );
        $m!(
            "willr140",
            WillR,
            WillRConfig::builder().length(nz(140)).build()
        );
        $m!("cci20", Cci, CciConfig::builder().length(nz(20)).build());
        $m!("cci200", Cci, CciConfig::builder().length(nz(200)).build());
    };
}

fn stream_benchmarks(c: &mut Criterion) {
    let bars = load_reference_ohlcvs();
    let mut group = c.benchmark_group("stream");
    group.throughput(Throughput::Elements(bars.len() as u64));
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    macro_rules! stream_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                b.iter_batched(
                    || <$ind_type>::new($config),
                    |mut ind| {
                        for bar in &bars {
                            black_box(ind.compute(bar));
                        }
                    },
                    BatchSize::SmallInput,
                );
            });
        };
    }

    all_indicators!(stream_bench);

    group.finish();
}

fn tick_benchmarks(c: &mut Criterion) {
    let bars = load_reference_ohlcvs();
    let mut group = c.benchmark_group("tick");
    group.sample_size(200);
    group.noise_threshold(0.03);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    // Pre-feed all bars except the last, then benchmark a single compute() call.
    let (warmup, last) = bars.split_at(bars.len() - 1);

    macro_rules! tick_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                b.iter_batched(
                    || {
                        let mut ind = <$ind_type>::new($config);
                        for bar in warmup {
                            ind.compute(bar);
                        }
                        ind
                    },
                    |mut ind| {
                        black_box(ind.compute(&last[0]));
                    },
                    BatchSize::SmallInput,
                );
            });
        };
    }

    all_indicators!(tick_bench);

    group.finish();
}

fn repaint_benchmarks(c: &mut Criterion) {
    let bars = load_reference_ohlcvs();
    let mut group = c.benchmark_group("repaint");
    group.sample_size(200);
    group.noise_threshold(0.03);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    // Pre-feed all bars, then benchmark a single repaint tick (same open_time, perturbed close).
    let last = bars.last().unwrap();
    let repaint_bar = {
        let mut b = last.clone();
        b.close *= 1.001;
        b
    };

    macro_rules! repaint_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                b.iter_batched(
                    || {
                        let mut ind = <$ind_type>::new($config);
                        for bar in &bars {
                            ind.compute(bar);
                        }
                        ind
                    },
                    |mut ind| {
                        black_box(ind.compute(&repaint_bar));
                    },
                    BatchSize::SmallInput,
                );
            });
        };
    }

    all_indicators!(repaint_bench);

    group.finish();
}

fn repaint_stream_benchmarks(c: &mut Criterion) {
    let bars = load_reference_ohlcvs();
    let mut group = c.benchmark_group("repaint_stream");
    group.throughput(Throughput::Elements(bars.len() as u64 * 3));
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    // Pre-build repaint sequences: 3 ticks per bar (2 repaints + final).
    let sequences: Vec<_> = bars.iter().flat_map(repaint_sequence).collect();

    macro_rules! repaint_stream_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                b.iter_batched(
                    || <$ind_type>::new($config),
                    |mut ind| {
                        for bar in &sequences {
                            black_box(ind.compute(bar));
                        }
                    },
                    BatchSize::SmallInput,
                );
            });
        };
    }

    all_indicators!(repaint_stream_bench);

    group.finish();
}

criterion_group!(
    benches,
    stream_benchmarks,
    tick_benchmarks,
    repaint_benchmarks,
    repaint_stream_benchmarks
);
criterion_main!(benches);
