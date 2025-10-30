#![allow(dead_code)]

use repro::{canonical_json_bytes, from_canonical_json_bytes, hash_record, Record};
use std::fs;
use std::path::{Path, PathBuf};

const UPDATE_ENV: &str = "DETTEROT_UPDATE_GOLDENS";

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../")
}

pub fn repo_path(relative: &str) -> PathBuf {
    repo_root().join(relative)
}

pub fn read_record(path: &Path) -> Record {
    let bytes = fs::read(path)
        .unwrap_or_else(|err| panic!("failed to read record {}: {err}", path.display()));
    from_canonical_json_bytes(&bytes)
        .unwrap_or_else(|err| panic!("failed to parse record {}: {err}", path.display()))
}

pub fn read_record_from_disk(relative: &str) -> Record {
    let path = repo_path(relative);
    read_record(&path)
}

pub fn read_hash(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read hash {}: {err}", path.display()))
        .trim()
        .to_string()
}

pub fn hash_path(record_path: &Path) -> PathBuf {
    record_path.with_extension("hash")
}

pub fn assert_hash_matches(record: &Record, record_path: &Path) {
    let expected = read_hash(&hash_path(record_path));
    let actual = hash_record(record).expect("hash record");
    assert_eq!(
        actual,
        expected,
        "hash mismatch for {}",
        record_path.display()
    );
}

fn should_update_goldens() -> bool {
    match std::env::var(UPDATE_ENV) {
        Ok(value) => matches!(value.as_str(), "1" | "true" | "TRUE"),
        Err(_) => false,
    }
}

pub fn assert_golden_record(record: &Record, relative_path: &str) {
    let record_path = repo_path(relative_path);
    let hash_path = hash_path(&record_path);
    let bytes = canonical_json_bytes(record).expect("canonical json");
    let hash = hash_record(record).expect("hash record");

    if should_update_goldens() {
        if let Some(parent) = record_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .unwrap_or_else(|err| panic!("failed to create {}: {err}", parent.display()));
            }
        }
        fs::write(&record_path, &bytes).unwrap_or_else(|err| {
            panic!("failed to write record {}: {err}", record_path.display())
        });
        fs::write(&hash_path, format!("{}\n", hash))
            .unwrap_or_else(|err| panic!("failed to write hash {}: {err}", hash_path.display()));
        println!("updated golden {relative_path}");
        return;
    }

    let expected_bytes = fs::read(&record_path).unwrap_or_else(|err| {
        panic!(
            "missing golden {} (set {UPDATE_ENV}=1 to refresh): {err}",
            record_path.display()
        )
    });
    assert_eq!(
        expected_bytes, bytes,
        "record content mismatch for {relative_path}; set {UPDATE_ENV}=1 to refresh"
    );

    let expected_hash = read_hash(&hash_path);
    assert_eq!(
        hash, expected_hash,
        "hash mismatch for {relative_path}; set {UPDATE_ENV}=1 to refresh"
    );
}
