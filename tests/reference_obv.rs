mod fixtures;

fixtures::reference_test!(
    obv,
    Obv,
    ObvConfig::default(),
    "tests/fixtures/data/obv-close.csv",
    1e-6
);
