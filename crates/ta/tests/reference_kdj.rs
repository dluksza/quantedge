mod fixtures;

use fixtures::{
    assert_kdj_values_match, assert_near, load_kdj_ref, load_reference_ohlcvs, repaint_sequence,
};
use quantedge_ta::{Kdj, KdjConfig};

use std::num::NonZero;

const REF_PATH: &str = "tests/fixtures/data/kdj-9-3-3-close.csv";

/// Tolerance: 1e-6 — KDJ stacks rolling extremes and two SMAs.
const TOLERANCE: f64 = 1e-6;

fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).unwrap()
}

fn kdj_config() -> KdjConfig {
    KdjConfig::builder()
        .period(nz(9))
        .k_smooth(nz(3))
        .d_smooth(nz(3))
        .build()
}

#[test]
fn kdj_9_3_3_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_kdj_ref(REF_PATH);

    let mut kdj = Kdj::new(kdj_config());

    let mut ref_idx = 0;
    for bar in &bars {
        kdj.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = kdj
                .value()
                .unwrap_or_else(|| panic!("KDJ returned None at t={}", bar.open_time));
            let ctx = format!("KDJ(9,3,3) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.k,
                reference[ref_idx].k,
                TOLERANCE,
                &format!("{ctx} %K"),
            );
            assert_near(
                value.d,
                reference[ref_idx].d,
                TOLERANCE,
                &format!("{ctx} %D"),
            );
            assert_near(
                value.j,
                reference[ref_idx].j,
                TOLERANCE,
                &format!("{ctx} %J"),
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
fn kdj_9_3_3_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = kdj_config();
    let mut closed = Kdj::new(config);
    let mut repainted = Kdj::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_kdj_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
