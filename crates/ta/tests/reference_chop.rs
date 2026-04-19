mod fixtures;

fixtures::reference_test!(
    chop_14,
    Chop,
    ChopConfig::builder().length(nz(14)).build(),
    "tests/fixtures/data/chop-14.csv",
    1e-6
);
