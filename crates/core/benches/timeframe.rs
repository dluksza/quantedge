use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use quantedge_core::{Timeframe, Timestamp};
use std::{hint::black_box, time::Duration};

/// Mid-period anchor: Mon Apr 28 2025 00:05:30.123456 UTC.
const TS_BASE: Timestamp = 1_745_798_730_123_456;

/// 256 timestamps spaced ~100 hours apart, sweeping ~3 years.
/// Wide-enough sweep to exercise every dispatch path; large-enough set
/// that the compiler can't constant-fold the entire loop body out.
fn timestamps() -> Vec<Timestamp> {
    (0..256u64)
        .map(|i| TS_BASE.wrapping_add(i * 360_000_000_000))
        .collect()
}

/// One representative per dispatch path:
/// - epoch-aligned uniform (Second/Minute/Hour),
/// - Monday-aligned uniform (Day/Week),
/// - calendar single-month fast path,
/// - calendar N-month general path,
/// - calendar year (delegates to N=12 months).
const TIMEFRAMES: &[(&str, Timeframe)] = &[
    ("min_5", Timeframe::MIN_5),
    ("hour_1", Timeframe::HOUR_1),
    ("hour_4", Timeframe::HOUR_4),
    ("day_1", Timeframe::DAY_1),
    ("week_1", Timeframe::WEEK_1),
    ("month_1", Timeframe::MONTH_1),
    ("month_3", Timeframe::MONTH_3),
    ("year_1", Timeframe::YEAR_1),
];

/// Apply tighter measurement settings borrowed from `quantedge-ta`'s
/// `indicators.rs`: longer warmup/measurement for smoother distributions,
/// 200-sample size for tighter confidence intervals, 3% noise threshold
/// to surface the small (~0.1–0.2 ns) per-call deltas on sub-day paths.
fn configure(g: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>) {
    g.sample_size(200);
    g.noise_threshold(0.03);
    g.warm_up_time(Duration::from_secs(5));
    g.measurement_time(Duration::from_secs(10));
}

fn bench_open_time(c: &mut Criterion) {
    let ts = timestamps();
    let mut g = c.benchmark_group("open_time");
    g.throughput(Throughput::Elements(ts.len() as u64));
    configure(&mut g);
    for &(name, tf) in TIMEFRAMES {
        g.bench_function(name, |b| {
            b.iter(|| {
                for &t in &ts {
                    black_box(tf.open_time(black_box(t)));
                }
            });
        });
    }
    g.finish();
}

fn bench_close_time(c: &mut Criterion) {
    let ts = timestamps();
    let mut g = c.benchmark_group("close_time");
    g.throughput(Throughput::Elements(ts.len() as u64));
    configure(&mut g);
    for &(name, tf) in TIMEFRAMES {
        g.bench_function(name, |b| {
            b.iter(|| {
                for &t in &ts {
                    black_box(tf.close_time(black_box(t)));
                }
            });
        });
    }
    g.finish();
}

/// Realistic per-tick fan-out: each timestamp mapped to every tracked
/// timeframe, computing both open and close per timeframe.
/// Throughput reports per-tick (one tick = one fan-out across all timeframes).
fn bench_fanout(c: &mut Criterion) {
    let ts = timestamps();
    let mut g = c.benchmark_group("fanout");
    g.throughput(Throughput::Elements(ts.len() as u64));
    configure(&mut g);
    g.bench_function("open_then_close", |b| {
        b.iter(|| {
            for &t in &ts {
                for &(_, tf) in TIMEFRAMES {
                    let tf = black_box(tf);
                    black_box(tf.open_time(black_box(t)));
                    black_box(tf.close_time(black_box(t)));
                }
            }
        });
    });
    g.bench_function("bounds", |b| {
        b.iter(|| {
            for &t in &ts {
                for &(_, tf) in TIMEFRAMES {
                    let tf = black_box(tf);
                    black_box(tf.bounds(black_box(t)));
                }
            }
        });
    });
    g.finish();
}

criterion_group!(benches, bench_open_time, bench_close_time, bench_fanout);
criterion_main!(benches);
