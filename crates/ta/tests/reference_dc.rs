mod fixtures;

use fixtures::{
    assert_channel_values_match, assert_near, dc_bands, load_channel_ref, load_reference_ohlcvs,
    repaint_sequence,
};
use quantedge_ta::{Dc, DcConfig};

const REF_PATH: &str = "tests/fixtures/data/dc-20.csv";

/// Tolerance: 1e-10 — DC is pure min/max with only a midpoint average.
const TOLERANCE: f64 = 1e-10;

#[test]
fn dc_20_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_channel_ref(REF_PATH);

    let mut dc = Dc::new(DcConfig::builder().build());

    let mut ref_idx = 0;
    for bar in &bars {
        dc.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = dc
                .value()
                .unwrap_or_else(|| panic!("DC returned None at t={}", bar.open_time));
            let ctx = format!("DC(20) at bar {ref_idx} (t={})", bar.open_time);

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
fn dc_20_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = DcConfig::builder().build();
    let mut closed = Dc::new(config);
    let mut repainted = Dc::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_channel_values_match(
            "DC",
            i,
            closed.value(),
            repainted.value(),
            TOLERANCE,
            dc_bands,
        );
    }
}
