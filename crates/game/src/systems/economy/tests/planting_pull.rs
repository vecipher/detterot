use std::path::{Path, PathBuf};

use crate::systems::economy::{
    apply_planting_pull, load_rulepack, schedule_planting, EconState, HubId, PendingPlanting, Pp,
};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

fn cfg() -> crate::systems::economy::PpCfg {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path"))
        .expect("rulepack")
        .pp
}

#[test]
fn planting_pull_decays_over_time() {
    let cfg = cfg();
    let mut state = EconState {
        pp: Pp(cfg.neutral_pp),
        ..Default::default()
    };
    schedule_planting(
        PendingPlanting {
            hub: HubId(1),
            size: 10,
            age_days: 0,
        },
        &mut state,
    );

    let mut pp = state.pp;
    let mut series = Vec::new();
    for _ in 0..cfg.planting_max_age_days + 5 {
        pp = apply_planting_pull(pp, &mut state, &cfg);
        series.push(pp.0);
    }

    assert!(series[0] > cfg.neutral_pp);
    assert!(series.iter().all(|v| *v <= cfg.max_pp));
    assert!(series.windows(2).any(|w| w[1] <= w[0]));
}

#[test]
fn neutral_pp_without_plantings_is_stable() {
    let cfg = cfg();
    let mut state = EconState {
        pp: Pp(cfg.neutral_pp),
        ..Default::default()
    };

    let result = apply_planting_pull(state.pp, &mut state, &cfg);

    assert_eq!(result.0, cfg.neutral_pp);
    assert!(state.pending_planting.is_empty());
}

#[test]
fn passive_decay_and_pull_decay_follow_basis_points() {
    let cfg = cfg();
    let mut state = EconState {
        pp: Pp(cfg.neutral_pp + 1_000),
        ..Default::default()
    };
    schedule_planting(
        PendingPlanting {
            hub: HubId(42),
            size: 10,
            age_days: 0,
        },
        &mut state,
    );

    let result = apply_planting_pull(state.pp, &mut state, &cfg);

    let total_pull = i64::from(cfg.planting_size_to_pp_bp) * 10;
    let pull_decay_bp = i64::from(cfg.pull_decay_bp).clamp(-10_000, 10_000);
    let effective_pull = total_pull * (10_000 - pull_decay_bp) / 10_000;
    let pull_delta = effective_pull * i64::from(cfg.pull_strength_bp) / 10_000;
    let gap = i64::from(state.pp.0) - i64::from(cfg.neutral_pp);
    let passive_decay = gap * i64::from(cfg.decay_per_day_bp) / 10_000;
    let expected = (i64::from(state.pp.0) - passive_decay + pull_delta)
        .clamp(i64::from(cfg.min_pp), i64::from(cfg.max_pp));

    assert_eq!(result.0 as i64, expected);
}

#[test]
fn multiple_plantings_clamp_and_clear() {
    let cfg = cfg();
    let mut state = EconState {
        pp: Pp(cfg.max_pp - 50),
        ..Default::default()
    };
    for size in [5, 8, 12] {
        schedule_planting(
            PendingPlanting {
                hub: HubId(2),
                size,
                age_days: 0,
            },
            &mut state,
        );
    }

    let mut pp = state.pp;
    for _ in 0..cfg.planting_max_age_days + 5 {
        pp = apply_planting_pull(pp, &mut state, &cfg);
    }

    assert!(pp.0 <= cfg.max_pp);
    assert!(state.pending_planting.is_empty());
}
