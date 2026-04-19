mod fixtures;

fixtures::reference_test!(
    ema_20,
    Ema,
    EmaConfig::close(nz(20)),
    "tests/fixtures/data/ema-20-close.csv",
    1e-6
);
