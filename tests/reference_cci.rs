mod fixtures;

fixtures::reference_test!(
    cci_20,
    Cci,
    CciConfig::hlc3(nz(20)),
    "tests/fixtures/data/cci-20.csv",
    1e-6
);
