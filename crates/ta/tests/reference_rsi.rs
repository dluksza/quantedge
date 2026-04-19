mod fixtures;

fixtures::reference_test!(
    rsi_14,
    Rsi,
    RsiConfig::close(nz(14)),
    "tests/fixtures/data/rsi-14-close.csv",
    1e-6
);
