mod fixtures;

fixtures::reference_test!(
    atr_14,
    Atr,
    AtrConfig::builder().length(nz(14)).build(),
    "tests/fixtures/data/atr-14.csv",
    1e-6
);
