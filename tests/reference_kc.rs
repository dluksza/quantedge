mod fixtures;

use fixtures::{assert_near, load_kc_ref, nz};
use quantedge_ta::{Kc, KcConfig, KcMultiplier};

use crate::fixtures::{assert_kc_values_match, load_reference_ohlcvs, repaint_sequence};

const REF_PATH: &str = "tests/fixtures/data/kc-20-10-1.5.csv";

/// Tolerance: 1e-6 (~$0.000001 for BTC prices).
/// KC involves EMA + ATR which adds minor FP noise.
const TOLERANCE: f64 = 1e-6;

fn kc_config() -> KcConfig {
    KcConfig::builder()
        .length(nz(20))
        .atr_length(nz(10))
        .multiplier(KcMultiplier::new(1.5))
        .build()
}

#[test]
fn kc_20_10_1_5_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_kc_ref(REF_PATH);

    let mut kc = Kc::new(kc_config());

    let mut ref_idx = 0;
    for bar in &bars {
        kc.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = kc
                .value()
                .unwrap_or_else(|| panic!("KC returned None at t={}", bar.open_time));
            let ctx = format!("KC(20,10,1.5) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.upper(),
                reference[ref_idx].upper,
                TOLERANCE,
                &format!("{ctx} upper"),
            );
            assert_near(
                value.middle(),
                reference[ref_idx].middle,
                TOLERANCE,
                &format!("{ctx} middle"),
            );
            assert_near(
                value.lower(),
                reference[ref_idx].lower,
                TOLERANCE,
                &format!("{ctx} lower"),
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
fn kc_20_10_1_5_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = kc_config();
    let mut closed = Kc::new(config);
    let mut repainted = Kc::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_kc_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
