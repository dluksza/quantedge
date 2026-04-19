#![allow(dead_code)]

use std::num::NonZero;

use quantedge_ta::{Ohlcv, Price, Timestamp};
use serde::{Deserialize, de::DeserializeOwned};

pub fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).unwrap()
}

/// OHLCV bar parsed from Binance CSV.
#[derive(Debug, Clone, Deserialize)]
pub struct RefBar {
    pub open_time: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Ohlcv for RefBar {
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

/// Reference value with timestamp.
#[derive(Debug, Deserialize)]
pub struct RefValue {
    pub open_time: u64,
    pub expected: f64,
}

/// Reference channel value with timestamp (upper/middle/lower bands).
/// Used by BB, DC, and KC reference tests.
#[derive(Debug, Deserialize)]
pub struct RefChannelValue {
    pub open_time: u64,
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

/// Reference Stoch value with timestamp (both %K and %D present).
#[derive(Debug, Deserialize)]
pub struct RefStochValue {
    pub open_time: u64,
    pub k: f64,
    pub d: f64,
}

/// Reference ADX value with timestamp (adx, +DI, -DI).
#[derive(Debug, Deserialize)]
pub struct RefAdxValue {
    pub open_time: u64,
    pub adx: f64,
    pub plus_di: f64,
    pub minus_di: f64,
}

/// Reference Ichimoku value with timestamp (`tenkan`, `kijun`, `senkou_a`, `senkou_b`).
#[derive(Debug, Deserialize)]
pub struct RefIchimokuValue {
    pub open_time: u64,
    pub tenkan: f64,
    pub kijun: f64,
    pub senkou_a: f64,
    pub senkou_b: f64,
}

/// Reference MACD value with timestamp (fully converged: all 3 fields present).
#[derive(Debug, Deserialize)]
pub struct RefMacdValue {
    pub open_time: u64,
    pub macd: f64,
    pub signal: f64,
    pub histogram: f64,
}

/// Reference Supertrend value with timestamp (value and direction).
#[derive(Debug, Deserialize)]
pub struct RefSupertrendValue {
    pub open_time: u64,
    pub value: f64,
    pub is_bullish: u8,
}

/// Reference Parabolic SAR value with timestamp (SAR level and direction).
#[derive(Debug, Deserialize)]
pub struct RefPsarValue {
    pub open_time: u64,
    pub sar: f64,
    pub is_long: u8,
}

const OHLCV_PATH: &str = "tests/fixtures/data/btcusdt-1h.csv";

/// Load reference OHLCV bars from Binance.
pub fn load_reference_ohlcvs() -> Vec<RefBar> {
    load_records(OHLCV_PATH, "invalid OHLCV record")
}

/// Load single-value reference data (SMA, EMA).
pub fn load_ref_values(path: &str) -> Vec<RefValue> {
    load_records(path, "invalid reference record")
}

/// Load channel reference data (upper, middle, lower). Used by BB, DC, KC.
pub fn load_channel_ref(path: &str) -> Vec<RefChannelValue> {
    load_records(path, "invalid channel reference record")
}

/// Load Stoch reference data (k, d).
pub fn load_stoch_ref(path: &str) -> Vec<RefStochValue> {
    load_records(path, "invalid Stoch reference record")
}

/// Load ADX reference data (`adx`, `plus_di`, `minus_di`).
pub fn load_adx_ref(path: &str) -> Vec<RefAdxValue> {
    load_records(path, "invalid ADX reference record")
}

/// Load Ichimoku reference data (`tenkan`, `kijun`, `senkou_a`, `senkou_b`).
pub fn load_ichimoku_ref(path: &str) -> Vec<RefIchimokuValue> {
    load_records(path, "invalid Ichimoku reference record")
}

/// Load MACD reference data (macd, signal, histogram).
pub fn load_macd_ref(path: &str) -> Vec<RefMacdValue> {
    load_records(path, "invalid MACD reference record")
}

/// Load Supertrend reference data (value, `is_bullish`).
pub fn load_supertrend_ref(path: &str) -> Vec<RefSupertrendValue> {
    load_records(path, "invalid Supertrend reference record")
}

/// Load Parabolic SAR reference data (sar, `is_long`).
pub fn load_psar_ref(path: &str) -> Vec<RefPsarValue> {
    load_records(path, "invalid PSAR reference record")
}

/// Assert two f64 values are within tolerance.
pub fn assert_near(actual: f64, expected: f64, tolerance: f64, context: &str) {
    let diff = (actual - expected).abs();
    assert!(
        diff <= tolerance,
        "{context}: expected {expected:.10}, got {actual:.10}, diff {diff:.2e} > tolerance {tolerance:.2e}"
    );
}

/// Creates perturbed versions of a bar to simulate live repaints.
///
/// Returns 2 intermediate bars (with shifted close/high/low) followed
/// by the original bar. All share the same `open_time`.
///
/// High is monotonically non-decreasing and low is monotonically
/// non-increasing across ticks, matching real OHLCV semantics where
/// each new trade can only extend the bar's range.
pub fn repaint_sequence(bar: &RefBar) -> Vec<RefBar> {
    let t = bar.open_time;
    let tick1_high = bar.high.min(bar.open * 1.001);
    let tick1_low = bar.low.max(bar.open * 0.999);

    let tick2_high = bar.high.min(tick1_high.max(bar.open.midpoint(bar.high)));
    let tick2_low = bar.low.max(tick1_low.min(bar.open.midpoint(bar.low)));

    vec![
        // First tick: only open is known, close near open
        RefBar {
            open: bar.open,
            high: tick1_high,
            low: tick1_low,
            close: bar.open * 1.0005,
            volume: bar.volume - 2.0,
            open_time: t,
        },
        // Mid-bar: partial movement toward final values
        RefBar {
            open: bar.open,
            high: tick2_high,
            low: tick2_low,
            close: bar.open.midpoint(bar.close),
            volume: bar.volume - 1.0,
            open_time: t,
        },
        // Final: real OHLCV values
        RefBar {
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_time: t,
        },
    ]
}

pub fn assert_values_match(
    bar_idx: usize,
    closed: Option<f64>,
    repainted: Option<f64>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {} // both pre-convergence, fine
        (Some(c), Some(r)) => {
            let diff = (c - r).abs();
            assert!(
                diff <= tolerance,
                "diverged at bar {bar_idx}: closed={c:.10}, repainted={r:.10}, diff={diff:.2e}"
            );
        }
        (c, r) => {
            panic!("convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert channel values (upper/middle/lower) match between closed and repainted indicators.
///
/// Extracts bands via the provided closure to work with any channel type (BB, DC, KC).
pub fn assert_channel_values_match<V: std::fmt::Debug>(
    label: &str,
    bar_idx: usize,
    closed: Option<V>,
    repainted: Option<V>,
    tolerance: f64,
    bands: fn(&V) -> [(&str, f64); 3],
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            for ((band, cv), (_, rv)) in bands(&c).into_iter().zip(bands(&r)) {
                let diff = (cv - rv).abs();
                assert!(
                    diff <= tolerance,
                    "{label} {band} diverged at bar {bar_idx}: closed={cv:.10}, repainted={rv:.10}, diff={diff:.2e}"
                );
            }
        }
        (c, r) => {
            panic!("{label} convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

pub fn bb_bands(v: &quantedge_ta::BbValue) -> [(&str, f64); 3] {
    [
        ("upper", v.upper()),
        ("middle", v.middle()),
        ("lower", v.lower()),
    ]
}

pub fn dc_bands(v: &quantedge_ta::DcValue) -> [(&str, f64); 3] {
    [
        ("upper", v.upper()),
        ("middle", v.middle()),
        ("lower", v.lower()),
    ]
}

pub fn kc_bands(v: &quantedge_ta::KcValue) -> [(&str, f64); 3] {
    [
        ("upper", v.upper()),
        ("middle", v.middle()),
        ("lower", v.lower()),
    ]
}

/// Assert ADX values match between closed and repainted indicators.
pub fn assert_adx_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::AdxValue>,
    repainted: Option<quantedge_ta::AdxValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            for (label, cv, rv) in [
                ("ADX", c.adx(), r.adx()),
                ("+DI", c.plus_di(), r.plus_di()),
                ("-DI", c.minus_di(), r.minus_di()),
            ] {
                let diff = (cv - rv).abs();
                assert!(
                    diff <= tolerance,
                    "ADX {label} diverged at bar {bar_idx}: closed={cv:.10}, repainted={rv:.10}, diff={diff:.2e}"
                );
            }
        }
        (c, r) => {
            panic!("ADX convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert Ichimoku values match between closed and repainted indicators.
pub fn assert_ichimoku_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::IchimokuValue>,
    repainted: Option<quantedge_ta::IchimokuValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            for (label, cv, rv) in [
                ("tenkan", c.tenkan(), r.tenkan()),
                ("kijun", c.kijun(), r.kijun()),
                ("senkou_a", c.senkou_a(), r.senkou_a()),
                ("senkou_b", c.senkou_b(), r.senkou_b()),
                ("chikou_close", c.chikou_close(), r.chikou_close()),
            ] {
                let diff = (cv - rv).abs();
                assert!(
                    diff <= tolerance,
                    "Ichimoku {label} diverged at bar {bar_idx}: closed={cv:.10}, repainted={rv:.10}, diff={diff:.2e}"
                );
            }
        }
        (c, r) => {
            panic!("Ichimoku convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert MACD values match between closed and repainted indicators.
pub fn assert_macd_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::MacdValue>,
    repainted: Option<quantedge_ta::MacdValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            let diff = (c.macd() - r.macd()).abs();
            assert!(
                diff <= tolerance,
                "MACD line diverged at bar {bar_idx}: closed={:.10}, repainted={:.10}, diff={diff:.2e}",
                c.macd(),
                r.macd()
            );
            match (c.signal(), r.signal()) {
                (Some(cs), Some(rs)) => {
                    let diff = (cs - rs).abs();
                    assert!(
                        diff <= tolerance,
                        "MACD signal diverged at bar {bar_idx}: closed={cs:.10}, repainted={rs:.10}, diff={diff:.2e}"
                    );
                }
                (None, None) => {}
                (cs, rs) => {
                    panic!(
                        "MACD signal convergence mismatch at bar {bar_idx}: closed={cs:?}, repainted={rs:?}"
                    );
                }
            }
            match (c.histogram(), r.histogram()) {
                (Some(ch), Some(rh)) => {
                    let diff = (ch - rh).abs();
                    assert!(
                        diff <= tolerance,
                        "MACD histogram diverged at bar {bar_idx}: closed={ch:.10}, repainted={rh:.10}, diff={diff:.2e}"
                    );
                }
                (None, None) => {}
                (ch, rh) => {
                    panic!(
                        "MACD histogram convergence mismatch at bar {bar_idx}: closed={ch:?}, repainted={rh:?}"
                    );
                }
            }
        }
        (c, r) => {
            panic!("MACD convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert Stoch values match between closed and repainted indicators.
pub fn assert_stoch_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::StochValue>,
    repainted: Option<quantedge_ta::StochValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            let diff = (c.k() - r.k()).abs();
            assert!(
                diff <= tolerance,
                "Stoch %K diverged at bar {bar_idx}: closed={:.10}, repainted={:.10}, diff={diff:.2e}",
                c.k(),
                r.k()
            );
            match (c.d(), r.d()) {
                (Some(cd), Some(rd)) => {
                    let diff = (cd - rd).abs();
                    assert!(
                        diff <= tolerance,
                        "Stoch %D diverged at bar {bar_idx}: closed={cd:.10}, repainted={rd:.10}, diff={diff:.2e}"
                    );
                }
                (None, None) => {}
                (cd, rd) => {
                    panic!(
                        "Stoch %D convergence mismatch at bar {bar_idx}: closed={cd:?}, repainted={rd:?}"
                    );
                }
            }
        }
        (c, r) => {
            panic!("Stoch convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert Supertrend values match between closed and repainted indicators.
pub fn assert_supertrend_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::SupertrendValue>,
    repainted: Option<quantedge_ta::SupertrendValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            let diff = (c.value() - r.value()).abs();
            assert!(
                diff <= tolerance,
                "Supertrend value diverged at bar {bar_idx}: closed={:.10}, repainted={:.10}, diff={diff:.2e}",
                c.value(),
                r.value()
            );
            assert_eq!(
                c.is_bullish(),
                r.is_bullish(),
                "Supertrend direction diverged at bar {bar_idx}: closed={}, repainted={}",
                c.is_bullish(),
                r.is_bullish()
            );
        }
        (c, r) => {
            panic!(
                "Supertrend convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}"
            );
        }
    }
}

/// Assert Parabolic SAR values match between closed and repainted indicators.
pub fn assert_psar_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::ParabolicSarValue>,
    repainted: Option<quantedge_ta::ParabolicSarValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            let diff = (c.sar() - r.sar()).abs();
            assert!(
                diff <= tolerance,
                "PSAR value diverged at bar {bar_idx}: closed={:.10}, repainted={:.10}, diff={diff:.2e}",
                c.sar(),
                r.sar()
            );
            assert_eq!(
                c.is_long(),
                r.is_long(),
                "PSAR direction diverged at bar {bar_idx}: closed={}, repainted={}",
                c.is_long(),
                r.is_long()
            );
        }
        (c, r) => {
            panic!("PSAR convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Assert `StochRsi` values match between closed and repainted indicators.
pub fn assert_stoch_rsi_values_match(
    bar_idx: usize,
    closed: Option<quantedge_ta::StochRsiValue>,
    repainted: Option<quantedge_ta::StochRsiValue>,
    tolerance: f64,
) {
    match (closed, repainted) {
        (None, None) => {}
        (Some(c), Some(r)) => {
            let diff = (c.k() - r.k()).abs();
            assert!(
                diff <= tolerance,
                "StochRsi %K diverged at bar {bar_idx}: closed={:.10}, repainted={:.10}, diff={diff:.2e}",
                c.k(),
                r.k()
            );
            match (c.d(), r.d()) {
                (Some(cd), Some(rd)) => {
                    let diff = (cd - rd).abs();
                    assert!(
                        diff <= tolerance,
                        "StochRsi %D diverged at bar {bar_idx}: closed={cd:.10}, repainted={rd:.10}, diff={diff:.2e}"
                    );
                }
                (None, None) => {}
                (cd, rd) => {
                    panic!(
                        "StochRsi %D convergence mismatch at bar {bar_idx}: closed={cd:?}, repainted={rd:?}"
                    );
                }
            }
        }
        (c, r) => {
            panic!("StochRsi convergence mismatch at bar {bar_idx}: closed={c:?}, repainted={r:?}");
        }
    }
}

/// Generate reference match + repaint tests for a single-value indicator.
///
/// Usage: `reference_test!(sma_20, Sma, SmaConfig::close(nz(20)), "tests/fixtures/data/sma-20-close.csv", 1e-6);`
#[allow(unused_macros)]
macro_rules! reference_test {
    ($name:ident, $ind:ty, $config:expr, $ref_path:expr, $tolerance:expr) => {
        mod $name {
            use super::fixtures::*;
            use quantedge_ta::*;

            #[test]
            fn matches_reference() {
                let bars = load_reference_ohlcvs();
                let reference = load_ref_values($ref_path);
                let config = $config;
                let mut ind = <$ind>::new(config);

                let mut ref_idx = 0;
                for bar in &bars {
                    ind.compute(bar);

                    if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
                        let value = ind.value().unwrap_or_else(|| {
                            panic!("{} returned None at t={}", stringify!($name), bar.open_time)
                        });
                        assert_near(
                            value,
                            reference[ref_idx].expected,
                            $tolerance,
                            &format!(
                                "{} at bar {ref_idx} (t={})",
                                stringify!($name),
                                bar.open_time
                            ),
                        );
                        ref_idx += 1;
                    }
                }

                assert_eq!(
                    ref_idx,
                    reference.len(),
                    "not all reference values checked: {ref_idx}/{}",
                    reference.len()
                );
            }

            #[test]
            fn repaint_matches_closed() {
                let bars = load_reference_ohlcvs();
                let config = $config;
                let mut closed = <$ind>::new(config);
                let mut repainted = <$ind>::new(config);

                for (i, bar) in bars.iter().enumerate() {
                    closed.compute(bar);
                    for tick in repaint_sequence(bar) {
                        repainted.compute(&tick);
                    }
                    assert_values_match(i, closed.value(), repainted.value(), $tolerance);
                }
            }
        }
    };
}

#[allow(unused_imports)]
pub(crate) use reference_test;

fn load_records<D>(path: &str, expect_msg: &str) -> Vec<D>
where
    D: DeserializeOwned,
{
    let mut rdr =
        csv::Reader::from_path(path).unwrap_or_else(|e| panic!("failed to open {path}: {e}"));

    rdr.deserialize().map(|r| r.expect(expect_msg)).collect()
}
