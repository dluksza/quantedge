mod fixtures;

use fixtures::{
    assert_near, assert_supertrend_values_match, load_reference_ohlcvs, load_supertrend_ref, nz,
    repaint_sequence,
};
use quantedge_ta::{Multiplier, Supertrend, SupertrendConfig};

const REF_PATH: &str = "tests/fixtures/data/supertrend-10-3.csv";

/// Tolerance: 1e-6 — Supertrend uses EMA-smoothed ATR,
/// so minor FP noise accumulates across bars.
const TOLERANCE: f64 = 1e-6;

fn st_config() -> SupertrendConfig {
    SupertrendConfig::builder()
        .length(nz(10))
        .multiplier(Multiplier::new(3.0))
        .build()
}

#[test]
fn supertrend_10_3_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_supertrend_ref(REF_PATH);

    let mut st = Supertrend::new(st_config());

    let mut ref_idx = 0;
    for bar in &bars {
        st.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = st
                .value()
                .unwrap_or_else(|| panic!("Supertrend returned None at t={}", bar.open_time));
            let ctx = format!("Supertrend(10,3) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.value(),
                reference[ref_idx].value,
                TOLERANCE,
                &format!("{ctx} value"),
            );
            let expected_bullish = reference[ref_idx].is_bullish == 1;
            assert_eq!(
                value.is_bullish(),
                expected_bullish,
                "{ctx} direction: expected bullish={expected_bullish}, got bullish={}",
                value.is_bullish()
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
fn supertrend_10_3_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = st_config();
    let mut closed = Supertrend::new(config);
    let mut repainted = Supertrend::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_supertrend_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
