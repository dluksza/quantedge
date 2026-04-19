mod fixtures;

fixtures::reference_test!(
    willr_14,
    WillR,
    WillRConfig::close(nz(14)),
    "tests/fixtures/data/willr-14.csv",
    1e-6
);
