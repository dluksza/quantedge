mod fixtures;

use fixtures::{
    assert_macd_values_match, assert_near, load_macd_ref, load_reference_ohlcvs, repaint_sequence,
};
use quantedge_ta::{Macd, MacdConfig};

const REF_PATH: &str = "tests/fixtures/data/macd-12-26-9-close.csv";

/// Tolerance: 1e-6 — MACD involves two EMAs plus a signal EMA,
/// so minor FP noise accumulates beyond a single EMA.
const TOLERANCE: f64 = 1e-6;

#[test]
fn macd_12_26_9_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_macd_ref(REF_PATH);

    let config = MacdConfig::default_close();
    let mut macd = Macd::new(config);

    let mut ref_idx = 0;
    for bar in &bars {
        macd.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = macd
                .value()
                .unwrap_or_else(|| panic!("MACD returned None at t={}", bar.open_time));
            let ctx = format!("MACD(12,26,9) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.macd(),
                reference[ref_idx].macd,
                TOLERANCE,
                &format!("{ctx} macd"),
            );
            assert_near(
                value.signal().unwrap(),
                reference[ref_idx].signal,
                TOLERANCE,
                &format!("{ctx} signal"),
            );
            assert_near(
                value.histogram().unwrap(),
                reference[ref_idx].histogram,
                TOLERANCE,
                &format!("{ctx} histogram"),
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
fn macd_12_26_9_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = MacdConfig::default_close();
    let mut closed = Macd::new(config);
    let mut repainted = Macd::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_macd_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
