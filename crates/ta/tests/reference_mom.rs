mod fixtures;

fixtures::reference_test!(
    mom_10,
    Mom,
    MomConfig::close(nz(10)),
    "tests/fixtures/data/mom-10-close.csv",
    1e-6
);
