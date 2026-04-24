mod fixtures;

use fixtures::{
    assert_near, assert_psar_values_match, load_psar_ref, load_reference_ohlcvs, repaint_sequence,
};
use quantedge_ta::{Multiplier, ParabolicSar, ParabolicSarConfig};

const REF_PATH: &str = "tests/fixtures/data/psar-0.02-0.2.csv";

const TOLERANCE: f64 = 1e-6;

fn psar_config() -> ParabolicSarConfig {
    ParabolicSarConfig::builder()
        .af_step(Multiplier::new(0.02))
        .af_max(Multiplier::new(0.2))
        .build()
}

#[test]
fn psar_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_psar_ref(REF_PATH);

    let mut psar = ParabolicSar::new(psar_config());

    let mut ref_idx = 0;
    for bar in &bars {
        psar.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = psar
                .value()
                .unwrap_or_else(|| panic!("PSAR returned None at t={}", bar.open_time));
            let ctx = format!("PSAR(0.02,0.2) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.sar,
                reference[ref_idx].sar,
                TOLERANCE,
                &format!("{ctx} sar"),
            );
            let expected_long = reference[ref_idx].is_long == 1;
            assert_eq!(
                value.is_long, expected_long,
                "{ctx} direction: expected is_long={expected_long}, got is_long={}",
                value.is_long
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
fn psar_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = psar_config();
    let mut closed = ParabolicSar::new(config);
    let mut repainted = ParabolicSar::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_psar_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
