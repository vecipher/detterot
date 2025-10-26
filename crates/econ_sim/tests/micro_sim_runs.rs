use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn micro_sim_generates_golden_csv() {
    let dir = tempdir().expect("temp dir");
    let out_path = dir.path().join("econ_curves.csv");
    let status = Command::new(env!("CARGO_BIN_EXE_econ-sim"))
        .args([
            "--world-seed",
            "42",
            "--days",
            "15",
            "--hubs",
            "3",
            "--pp",
            "1500,5000,9000",
            "--debt",
            "0,500_000_00,5_000_000_00",
            "--out",
            out_path.to_str().expect("utf8 path"),
        ])
        .status()
        .expect("run econ-sim");
    assert!(status.success(), "econ-sim exited with {status:?}");

    let actual = fs::read_to_string(&out_path).expect("read csv");
    let golden = include_str!("goldens/econ_curves_seed42.csv");
    assert_eq!(actual, golden);
}
