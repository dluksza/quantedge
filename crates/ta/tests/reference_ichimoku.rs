mod fixtures;

use fixtures::{
    assert_ichimoku_values_match, assert_near, load_ichimoku_ref, load_reference_ohlcvs, nz,
    repaint_sequence,
};
use quantedge_ta::{Ichimoku, IchimokuConfig};

const REF_PATH: &str = "tests/fixtures/data/ichimoku-9-26-52-26.csv";

/// Tolerance: 1e-6 — Ichimoku involves rolling extremes over
/// multiple windows plus a displacement buffer.
const TOLERANCE: f64 = 1e-6;

fn ichimoku_config() -> IchimokuConfig {
    IchimokuConfig::builder()
        .tenkan_length(nz(9))
        .kijun_length(nz(26))
        .senkou_b_length(nz(52))
        .displacement(nz(26))
        .build()
}

#[test]
fn ichimoku_9_26_52_26_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_ichimoku_ref(REF_PATH);

    let mut ich = Ichimoku::new(ichimoku_config());

    let mut ref_idx = 0;
    for bar in &bars {
        ich.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = ich
                .value()
                .unwrap_or_else(|| panic!("Ichimoku returned None at t={}", bar.open_time));
            let ctx = format!("Ichimoku at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.tenkan,
                reference[ref_idx].tenkan,
                TOLERANCE,
                &format!("{ctx} tenkan"),
            );
            assert_near(
                value.kijun,
                reference[ref_idx].kijun,
                TOLERANCE,
                &format!("{ctx} kijun"),
            );
            assert_near(
                value.senkou_a,
                reference[ref_idx].senkou_a,
                TOLERANCE,
                &format!("{ctx} senkou_a"),
            );
            assert_near(
                value.senkou_b,
                reference[ref_idx].senkou_b,
                TOLERANCE,
                &format!("{ctx} senkou_b"),
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
fn ichimoku_9_26_52_26_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = ichimoku_config();
    let mut closed = Ichimoku::new(config);
    let mut repainted = Ichimoku::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_ichimoku_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
