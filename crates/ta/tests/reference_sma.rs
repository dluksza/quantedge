mod fixtures;

fixtures::reference_test!(
    sma_20,
    Sma,
    SmaConfig::close(nz(20)),
    "tests/fixtures/data/sma-20-close.csv",
    1e-6
);
