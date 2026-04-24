mod fixtures;

use fixtures::*;
use quantedge_ta::*;

const REF_PATH: &str = "tests/fixtures/data/vwap-day-hlc3.csv";
const TOLERANCE: f64 = 1e-4;

fn vwap_config() -> VwapConfig {
    VwapConfig::default() // Day anchor, HLC3 source
}

#[test]
fn matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_ref_values(REF_PATH);
    let mut ind = Vwap::new(vwap_config());

    let mut ref_idx = 0;
    for bar in &bars {
        ind.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = ind
                .value()
                .unwrap_or_else(|| panic!("vwap returned None at t={}", bar.open_time));
            assert_near(
                value.vwap,
                reference[ref_idx].expected,
                TOLERANCE,
                &format!("vwap at bar {ref_idx} (t={})", bar.open_time),
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
    let mut closed = Vwap::new(vwap_config());
    let mut repainted = Vwap::new(vwap_config());

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);
        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }
        let c = closed.value().map(|v| v.vwap);
        let r = repainted.value().map(|v| v.vwap);
        assert_values_match(i, c, r, TOLERANCE);
    }
}
