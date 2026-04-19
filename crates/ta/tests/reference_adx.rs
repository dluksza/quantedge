mod fixtures;

use fixtures::{
    assert_adx_values_match, assert_near, load_adx_ref, load_reference_ohlcvs, nz, repaint_sequence,
};
use quantedge_ta::{Adx, AdxConfig};

const REF_PATH: &str = "tests/fixtures/data/adx-14.csv";

/// Tolerance: 1e-6 — ADX involves multiple Wilder smoothers,
/// so minor FP noise accumulates.
const TOLERANCE: f64 = 1e-6;

#[test]
fn adx_14_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_adx_ref(REF_PATH);

    let config = AdxConfig::builder().length(nz(14)).build();
    let mut adx = Adx::new(config);

    let mut ref_idx = 0;
    for bar in &bars {
        adx.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = adx
                .value()
                .unwrap_or_else(|| panic!("ADX returned None at t={}", bar.open_time));
            let ctx = format!("ADX(14) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.adx(),
                reference[ref_idx].adx,
                TOLERANCE,
                &format!("{ctx} adx"),
            );
            assert_near(
                value.plus_di(),
                reference[ref_idx].plus_di,
                TOLERANCE,
                &format!("{ctx} +DI"),
            );
            assert_near(
                value.minus_di(),
                reference[ref_idx].minus_di,
                TOLERANCE,
                &format!("{ctx} -DI"),
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
fn adx_14_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = AdxConfig::builder().length(nz(14)).build();
    let mut closed = Adx::new(config);
    let mut repainted = Adx::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_adx_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
