use game::cli::{CliOptions, Mode};
use repro::{from_canonical_json_bytes, hash_record, Record};
use std::fs;
use std::path::Path;

const GOLDENS: [&str; 5] = [
    "repro/records/leg_seed_01.json",
    "repro/records/leg_seed_02.json",
    "repro/records/leg_seed_03.json",
    "repro/records/leg_seed_04.json",
    "repro/records/leg_seed_05.json",
];

#[test]
fn golden_records_replay_cleanly() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");

    for path in GOLDENS {
        let record_path = base.join(path);
        let bytes = fs::read(&record_path).expect("read record");
        let record: Record = from_canonical_json_bytes(&bytes).expect("parse record");
        let hash = hash_record(&record).expect("hash record");
        let hash_path = record_path.with_extension("hash");
        let expected = fs::read_to_string(&hash_path).expect("read hash");
        assert_eq!(hash, expected.trim(), "hash mismatch for {path}");

        let mut opts = CliOptions::for_mode(Mode::Replay);
        opts.io = Some(record_path.to_str().expect("record path").to_string());
        game::run_with_options(opts).expect("replay matches");
    }
}
