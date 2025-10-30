#[path = "../support/repro.rs"]
mod repro_support;

use game::cli::{CliOptions, Mode};
use repro::Record;
use repro_support::{assert_hash_matches, read_record, repo_path};

const GOLDENS: [&str; 5] = [
    "repro/records/leg_seed_01.json",
    "repro/records/leg_seed_02.json",
    "repro/records/leg_seed_03.json",
    "repro/records/leg_seed_04.json",
    "repro/records/leg_seed_05.json",
];

#[test]
fn golden_records_replay_cleanly() {
    for path in GOLDENS {
        let record_path = repo_path(path);
        let record: Record = read_record(&record_path);
        assert_hash_matches(&record, &record_path);

        let mut opts = CliOptions::for_mode(Mode::Replay);
        opts.continue_after_mismatch = false;
        opts.io = Some(record_path.to_str().expect("record path").to_string());
        game::run_with_options(opts).expect("replay matches");
    }
}
