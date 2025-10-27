use std::fs;
use std::path::PathBuf;

use game::{
    runtime::{self, HeadlessConfig},
    systems::director::LegParameters,
};

const DEFAULT_DT: f64 = 0.033_333_333_3;
const DEFAULT_MAX_TICKS: u32 = 200_000;

#[test]
#[ignore]
fn regenerate_golden_records() {
    let base: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "repro", "records"]
        .into_iter()
        .collect();
    let cases = runtime::goldens::load_manifest().expect("manifest loads");
    assert!(
        !cases.is_empty(),
        "golden manifest must list at least one record"
    );
    for case in cases {
        let mut params = LegParameters::default();
        runtime::goldens::apply_case(&mut params, &case).expect("apply manifest case");

        let mut cfg = HeadlessConfig::default();
        cfg.dt = case.fixed_dt.unwrap_or(DEFAULT_DT);
        cfg.max_ticks = case.max_ticks.unwrap_or(DEFAULT_MAX_TICKS);
        cfg.logs_enabled = false;

        let record = runtime::record_leg(&params, &cfg).expect("headless run succeeds");
        let canonical = record
            .canonical_json()
            .expect("canonical serialization succeeds");
        let path = base.join(&case.file);
        fs::write(&path, canonical).expect("write record json");
        let hash_path = path.with_extension("hash");
        let hash = record.hash_hex().expect("hash computed");
        fs::write(hash_path, format!("{hash}\n")).expect("write hash file");
    }
}
