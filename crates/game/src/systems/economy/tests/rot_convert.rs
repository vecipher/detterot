use std::path::{Path, PathBuf};

use crate::systems::economy::{convert_rot_to_debt, load_rulepack, MoneyCents};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

fn cfg() -> crate::systems::economy::RotCfg {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path"))
        .expect("rulepack")
        .rot
}

#[test]
fn convert_respects_floor_and_chunks() {
    let cfg = cfg();
    let (rot_after, delta) = convert_rot_to_debt(cfg.rot_floor.saturating_sub(1), &cfg);
    assert_eq!(rot_after, cfg.rot_floor);
    assert_eq!(delta, MoneyCents::ZERO);

    let start = cfg.rot_floor + cfg.rot_decay_per_day + cfg.conversion_chunk * 3;
    let (rot_after, delta) = convert_rot_to_debt(start, &cfg);
    assert_eq!(rot_after, cfg.rot_floor);
    assert_eq!(delta.as_i64(), i64::from(cfg.debt_per_chunk_cents) * 3);
}

#[test]
fn conversion_is_idempotent_once_drained() {
    let cfg = cfg();
    let start = cfg.rot_ceiling;
    let (rot_after, delta) = convert_rot_to_debt(start, &cfg);
    assert!(delta.as_i64() > 0);
    let (rot_again, delta_again) = convert_rot_to_debt(rot_after, &cfg);
    assert_eq!(rot_again, rot_after);
    assert_eq!(delta_again, MoneyCents::ZERO);
}

#[test]
fn decay_applies_before_conversion() {
    let cfg = cfg();
    let start = cfg.rot_floor + cfg.rot_decay_per_day.saturating_sub(1);
    let (rot_after, delta) = convert_rot_to_debt(start, &cfg);
    assert_eq!(rot_after, cfg.rot_floor);
    assert_eq!(delta, MoneyCents::ZERO);
}
