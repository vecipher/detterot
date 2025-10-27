use std::{fs, path::PathBuf};

use game::{
    runtime::{self, HeadlessConfig},
    systems::director::LegParameters,
};

const DEFAULT_DT: f64 = 0.033_333_333_3;
const DEFAULT_MAX_TICKS: u32 = 200_000;

#[test]
fn golden_hashes_match() {
    let base: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "repro", "records"]
        .into_iter()
        .collect();
    let cases = runtime::goldens::load_manifest().expect("manifest loads");
    assert!(
        !cases.is_empty(),
        "golden manifest must list at least one record"
    );
    for case in cases {
        let json_path = base.join(&case.file);
        let json = fs::read_to_string(&json_path).expect("record json present");
        let expected: repro::Record = serde_json::from_str(&json).expect("valid record json");

        let mut params = LegParameters::default();
        runtime::goldens::apply_case(&mut params, &case).expect("apply manifest case");

        let mut cfg = HeadlessConfig::default();
        cfg.dt = case.fixed_dt.unwrap_or(DEFAULT_DT);
        cfg.max_ticks = case.max_ticks.unwrap_or(DEFAULT_MAX_TICKS);
        cfg.logs_enabled = false;

        let produced = runtime::record_leg(&params, &cfg).expect("headless run succeeds");
        let expected_hash_path = json_path.with_extension("hash");
        let expected_hash = fs::read_to_string(&expected_hash_path)
            .expect("hash file present")
            .trim()
            .to_string();
        let actual_hash = produced.hash_hex().expect("hash computed");
        assert_eq!(
            actual_hash, expected_hash,
            "hash mismatch for {:?}",
            json_path
        );

        let canonical = produced
            .canonical_json()
            .expect("canonical serialization succeeds");
        assert_eq!(
            canonical.trim_end(),
            json.trim_end(),
            "canonical output mismatch for {:?}",
            json_path
        );

        assert_eq!(
            produced.commands, expected.commands,
            "command stream mismatch for {:?}",
            json_path
        );
    }
}
