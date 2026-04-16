#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use crate::fixtures::{RefBar, load_reference_ohlcvs, repaint_sequence};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use quantedge_ta::{
    Adx, AdxConfig, Atr, AtrConfig, Bb, BbConfig, Cci, CciConfig, Chop, ChopConfig, Dc, DcConfig,
    Ema, EmaConfig, Ichimoku, IchimokuConfig, IndicatorConfig, Kc, KcConfig, Macd, MacdConfig,
    Multiplier, Obv, ObvConfig, ParabolicSar, ParabolicSarConfig, Rsi, RsiConfig, Sma, SmaConfig,
    Stoch, StochConfig, StochRsi, StochRsiConfig, Supertrend, SupertrendConfig, Vwap, VwapConfig,
    WillR, WillRConfig,
};
use std::{hint::black_box, num::NonZero, sync::OnceLock, time::Duration};

fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).expect("non zero value")
}

/// CSV-backed reference bars, parsed on first access.
fn bars() -> &'static [RefBar] {
    static BARS: OnceLock<Vec<RefBar>> = OnceLock::new();
    BARS.get_or_init(load_reference_ohlcvs)
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
        $m!("chop14", Chop, ChopConfig::builder().length(nz(14)).build());
        $m!(
            "chop140",
            Chop,
            ChopConfig::builder().length(nz(140)).build()
        );
        $m!(
            "ichimoku9265226",
            Ichimoku,
            IchimokuConfig::builder()
                .tenkan_length(nz(9))
                .kijun_length(nz(26))
                .senkou_b_length(nz(52))
                .displacement(nz(26))
                .build()
        );
        $m!(
            "ichimoku36104208104",
            Ichimoku,
            IchimokuConfig::builder()
                .tenkan_length(nz(36))
                .kijun_length(nz(104))
                .senkou_b_length(nz(208))
                .displacement(nz(104))
                .build()
        );
        $m!(
            "stochrsi141433",
            StochRsi,
            StochRsiConfig::builder()
                .rsi_length(nz(14))
                .stoch_length(nz(14))
                .k_smooth(nz(3))
                .d_smooth(nz(3))
                .build()
        );
        $m!(
            "stochrsi1401403030",
            StochRsi,
            StochRsiConfig::builder()
                .rsi_length(nz(140))
                .stoch_length(nz(140))
                .k_smooth(nz(30))
                .d_smooth(nz(30))
                .build()
        );
        $m!("obv", Obv, ObvConfig::builder().build());
        $m!("vwap", Vwap, VwapConfig::builder().build());
        $m!("supertrend20", Supertrend, SupertrendConfig::default());
        $m!(
            "supertrend200",
            Supertrend,
            SupertrendConfig::builder().length(nz(200)).build()
        );
        $m!(
            "parabolicsar0.02_0.2",
            ParabolicSar,
            ParabolicSarConfig::default()
        );
        $m!(
            "parabolicsar0.01_0.4",
            ParabolicSar,
            ParabolicSarConfig::builder()
                .af_step(Multiplier::new(0.01))
                .af_max(Multiplier::new(0.4))
                .build()
        );
    };
}

/// Largest bar index across every config in `all_indicators!` at which the
/// hot path of `compute()` still takes a pre-steady-state branch. For `Macd`
/// and `StochRsi` the signal/D line keeps seeding past the first `Some`
/// output; for every other indicator the nominal `convergence()` is also the
/// point where all output branches stabilize.
fn max_convergence() -> usize {
    let mut max_conv = 0usize;
    macro_rules! hot_path_conv {
        (Macd, $cfg:expr) => {
            $cfg.full_convergence()
        };
        (StochRsi, $cfg:expr) => {
            $cfg.rsi_length() + $cfg.stoch_length() + $cfg.k_smooth() + $cfg.d_smooth() - 1
        };
        ($_t:ident, $cfg:expr) => {
            IndicatorConfig::convergence(&$cfg)
        };
    }
    macro_rules! collect_conv {
        ($name:expr, $ind_type:ident, $config:expr) => {{
            let _ = $name;
            let cfg = $config;
            let c = hot_path_conv!($ind_type, cfg);
            if c > max_conv {
                max_conv = c;
            }
        }};
    }
    all_indicators!(collect_conv);
    max_conv
}

fn stream_benchmarks(c: &mut Criterion) {
    let bars = bars();
    let warmup_len = max_convergence();
    assert!(
        warmup_len < bars.len(),
        "fixture has {} bars, needs > {} for steady-state measurement",
        bars.len(),
        warmup_len,
    );
    let (warmup, measured) = bars.split_at(warmup_len);
    let mut group = c.benchmark_group("stream");
    group.throughput(Throughput::Elements(measured.len() as u64));
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    macro_rules! stream_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                let mut seed = <$ind_type>::new($config);
                for bar in warmup {
                    seed.compute(bar);
                }
                b.iter_batched(
                    || seed.clone(),
                    |mut ind| {
                        for bar in measured {
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
    let bars = bars();
    let warmup_len = max_convergence();
    assert!(
        warmup_len < bars.len(),
        "fixture has {} bars, needs > {} for steady-state measurement",
        bars.len(),
        warmup_len,
    );
    let warmup = &bars[..warmup_len];
    let next = &bars[warmup_len];

    let mut group = c.benchmark_group("tick");
    group.sample_size(200);
    group.noise_threshold(0.03);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    macro_rules! tick_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                let mut seed = <$ind_type>::new($config);
                for bar in warmup {
                    seed.compute(bar);
                }
                b.iter_batched(
                    || seed.clone(),
                    |mut ind| {
                        black_box(ind.compute(next));
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
    let bars = bars();
    let warmup_len = max_convergence();
    assert!(
        warmup_len < bars.len(),
        "fixture has {} bars, needs > {} for steady-state measurement",
        bars.len(),
        warmup_len,
    );
    let warmup = &bars[..warmup_len];
    // Repaint the last warmed bar (same open_time, perturbed close).
    let repaint_bar = {
        let mut b = warmup.last().unwrap().clone();
        b.close *= 1.001;
        b
    };

    let mut group = c.benchmark_group("repaint");
    group.sample_size(200);
    group.noise_threshold(0.03);
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    macro_rules! repaint_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                let mut seed = <$ind_type>::new($config);
                for bar in warmup {
                    seed.compute(bar);
                }
                b.iter_batched(
                    || seed.clone(),
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
    let bars = bars();
    let warmup_len = max_convergence();
    assert!(
        warmup_len < bars.len(),
        "fixture has {} bars, needs > {} for steady-state measurement",
        bars.len(),
        warmup_len,
    );
    let (warmup_bars, measured_bars) = bars.split_at(warmup_len);
    // Split on bar boundaries so each repaint triple stays whole.
    let warmup_sequences: Vec<_> = warmup_bars.iter().flat_map(repaint_sequence).collect();
    let measured_sequences: Vec<_> = measured_bars.iter().flat_map(repaint_sequence).collect();

    let mut group = c.benchmark_group("repaint_stream");
    group.throughput(Throughput::Elements(measured_sequences.len() as u64));
    group.warm_up_time(Duration::from_secs(5));
    group.measurement_time(Duration::from_secs(10));

    macro_rules! repaint_stream_bench {
        ($name:expr, $ind_type:ty, $config:expr) => {
            group.bench_function($name, |b| {
                let mut seed = <$ind_type>::new($config);
                for bar in &warmup_sequences {
                    seed.compute(bar);
                }
                b.iter_batched(
                    || seed.clone(),
                    |mut ind| {
                        for bar in &measured_sequences {
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
