use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::systems::economy::{
    load_rulepack, step_economy_day, BasisBp, CommodityId, EconState, EconomyDay, HubId,
    MoneyCents, PendingPlanting, Pp,
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
        history.push(step_economy_day(&rp, 9, 1, HubId(1), &mut state));
    }

    let actual = serde_json::to_string_pretty(&history).expect("serialize");
    let golden = include_str!("state_step_golden.json").trim();
    assert_eq!(actual, golden);
}
