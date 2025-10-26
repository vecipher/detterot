use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::systems::economy::{
    load_rulepack, step_economy_day, BasisBp, CommodityId, EconState, EconStepScope, EconomyDay,
    HubId, MoneyCents, PendingPlanting, Pp,
};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

#[test]
fn state_step_matches_golden() {
    let rp = load_rulepack(
        workspace_path("assets/rulepacks/day_001.toml")
            .to_str()
            .unwrap(),
    )
    .expect("rulepack");

    let mut state = EconState {
        day: EconomyDay(0),
        di_bp: HashMap::from([(CommodityId(1), BasisBp(0)), (CommodityId(2), BasisBp(-50))]),
        di_overlay_bp: 120,
        basis_bp: HashMap::new(),
        pp: Pp(rp.pp.neutral_pp),
        rot_u16: 200,
        pending_planting: vec![PendingPlanting {
            hub: HubId(1),
            size: 5,
            age_days: 0,
        }],
        debt_cents: MoneyCents(10_000),
    };

    let mut history = Vec::new();
    for _ in 0..7 {
        history.push(step_economy_day(
            &rp,
            9,
            1,
            HubId(1),
            &mut state,
            EconStepScope::GlobalAndHub,
        ));
    }

    let actual = serde_json::to_string_pretty(&history).expect("serialize");
    let golden_path =
        workspace_path("crates/game/src/systems/economy/tests/state_step_golden.json");
    maybe_update_state_golden(&golden_path, &actual);
    let golden_contents = fs::read_to_string(&golden_path).expect("read golden");
    let golden = golden_contents.trim();
    assert_eq!(actual, golden);
}

#[test]
fn hub_only_scope_skips_global_progression() {
    let rp = load_rulepack(
        workspace_path("assets/rulepacks/day_001.toml")
            .to_str()
            .unwrap(),
    )
    .expect("rulepack");

    let mut state = EconState {
        day: EconomyDay(0),
        di_bp: HashMap::from([(CommodityId(1), BasisBp(0)), (CommodityId(2), BasisBp(0))]),
        di_overlay_bp: 0,
        basis_bp: HashMap::new(),
        pp: Pp(rp.pp.neutral_pp),
        rot_u16: 0,
        pending_planting: Vec::new(),
        debt_cents: MoneyCents(1_000),
    };

    let first_delta =
        step_economy_day(&rp, 9, 1, HubId(1), &mut state, EconStepScope::GlobalAndHub);
    assert!(
        !first_delta.di.is_empty(),
        "global step should populate di deltas"
    );
    let day_after_first = state.day;
    let pp_after_first = state.pp;
    let debt_after_first = state.debt_cents;

    let second_delta = step_economy_day(&rp, 9, 1, HubId(2), &mut state, EconStepScope::HubOnly);

    assert_eq!(state.day, day_after_first, "day advanced unexpectedly");
    assert_eq!(state.pp, pp_after_first, "pp mutated during hub-only step");
    assert_eq!(
        state.debt_cents, debt_after_first,
        "debt mutated during hub-only step"
    );
    assert!(
        second_delta.di.is_empty(),
        "hub-only step should not log di deltas"
    );
    assert_eq!(second_delta.pp_before, state.pp);
    assert_eq!(second_delta.pp_after, state.pp);
    assert_eq!(second_delta.interest_delta, MoneyCents::ZERO);
}

fn maybe_update_state_golden(path: &Path, contents: &str) {
    if std::env::var_os("UPDATE_ECON_GOLDENS").is_none() {
        return;
    }
    fs::write(path, format!("{contents}\n")).expect("write golden");
}
