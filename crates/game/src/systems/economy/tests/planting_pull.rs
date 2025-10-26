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
