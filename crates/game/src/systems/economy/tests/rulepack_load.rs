use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;

use crate::systems::economy::load_rulepack;

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

#[test]
fn loads_day_001_rulepack() {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    let pack = load_rulepack(path.to_str().expect("utf-8 path")).expect("load rulepack");
    assert_eq!(pack.di.long_run_mean_bp, 75);
    assert_eq!(pack.basis.weather.rains_bp, -35);
    assert_eq!(pack.interest.per_leg_cap_bp, 1_200);
}

#[test]
fn rejects_unknown_keys() {
    let source_path = workspace_path("assets/rulepacks/day_001.toml");
    let base = fs::read_to_string(source_path).expect("fixture");
    let marker = "[di]\n";
    let mut mutated = String::with_capacity(base.len() + 64);
    mutated.push_str(marker);
    mutated.push_str("unknown_field = 1\n");
    mutated.push_str(&base[marker.len()..]);

    let mut tmp = NamedTempFile::new().expect("tmp file");
    tmp.write_all(mutated.as_bytes()).expect("write tmp");

    let err = load_rulepack(tmp.path().to_str().unwrap()).expect_err("should fail");
    let msg = err.to_string();
    assert!(msg.contains("unknown"), "unexpected error: {}", msg);
}

#[test]
fn rejects_missing_sections() {
    let mut tmp = NamedTempFile::new().expect("tmp file");
    write!(
        tmp,
        "[di]\nlong_run_mean_bp = 0\nretention_bp = 0\nnoise_sigma_bp = 0\nnoise_clamp_bp = 0\nper_day_clamp_bp = 0\nabsolute_min_bp = 0\nabsolute_max_bp = 0\noverlay_decay_bp = 0\noverlay_min_bp = 0\noverlay_max_bp = 0\n"
    )
    .expect("write tmp");

    let err = load_rulepack(tmp.path().to_str().unwrap()).expect_err("missing keys");
    let msg = err.to_string();
    assert!(msg.contains("missing field"), "unexpected error: {}", msg);
}
