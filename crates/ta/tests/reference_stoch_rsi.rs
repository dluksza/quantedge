mod fixtures;

use fixtures::{
    assert_near, assert_stoch_rsi_values_match, load_reference_ohlcvs, load_stoch_ref,
    repaint_sequence,
};
use quantedge_ta::{StochRsi, StochRsiConfig};

use std::num::NonZero;

const REF_PATH: &str = "tests/fixtures/data/stoch-rsi-14-14-3-3-close.csv";

/// Tolerance: 1e-6 — `StochRSI` chains RSI + rolling extremes + two SMAs.
const TOLERANCE: f64 = 1e-6;

fn nz(n: usize) -> NonZero<usize> {
    NonZero::new(n).unwrap()
}

fn stoch_rsi_config() -> StochRsiConfig {
    StochRsiConfig::builder()
        .rsi_length(nz(14))
        .stoch_length(nz(14))
        .k_smooth(nz(3))
        .d_smooth(nz(3))
        .build()
}

#[test]
fn stoch_rsi_14_14_3_3_matches_reference() {
    let bars = load_reference_ohlcvs();
    let reference = load_stoch_ref(REF_PATH);

    let mut stoch_rsi = StochRsi::new(stoch_rsi_config());

    let mut ref_idx = 0;
    let mut d_checked = 0;
    for bar in &bars {
        stoch_rsi.compute(bar);

        if ref_idx < reference.len() && bar.open_time == reference[ref_idx].open_time {
            let value = stoch_rsi
                .value()
                .unwrap_or_else(|| panic!("StochRsi returned None at t={}", bar.open_time));
            let ctx = format!("StochRsi(14,14,3,3) at bar {ref_idx} (t={})", bar.open_time);

            assert_near(
                value.k,
                reference[ref_idx].k,
                TOLERANCE,
                &format!("{ctx} %K"),
            );

            if let Some(d) = value.d {
                assert_near(d, reference[ref_idx].d, TOLERANCE, &format!("{ctx} %D"));
                d_checked += 1;
            }
            ref_idx += 1;
        }
    }

    assert_eq!(
        ref_idx,
        reference.len(),
        "not all reference values checked: {ref_idx}/{}",
        reference.len()
    );
    assert!(
        d_checked > reference.len() / 2,
        "too few %D values checked: {d_checked}/{}",
        reference.len()
    );
}

#[test]
fn stoch_rsi_14_14_3_3_repaint_matches_closed() {
    let bars = load_reference_ohlcvs();

    let config = stoch_rsi_config();
    let mut closed = StochRsi::new(config);
    let mut repainted = StochRsi::new(config);

    for (i, bar) in bars.iter().enumerate() {
        closed.compute(bar);

        for tick in repaint_sequence(bar) {
            repainted.compute(&tick);
        }

        assert_stoch_rsi_values_match(i, closed.value(), repainted.value(), TOLERANCE);
    }
}
