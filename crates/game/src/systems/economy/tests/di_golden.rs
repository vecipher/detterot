use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::systems::economy::{
    load_rulepack, step_di, BasisBp, CommodityId, DetRng, DiState, EconomyDay, HubId, Rulepack,
};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

fn load_fixture_pack() -> Rulepack {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack load")
}

#[test]
fn di_sequence_matches_golden() {
    let rp = load_fixture_pack();
    let mut per_com = HashMap::new();
    per_com.insert(CommodityId(1), BasisBp(0));
    per_com.insert(CommodityId(2), BasisBp(-50));
    let mut state = DiState {
        per_com,
        overlay_bp: 120,
    };
    let mut rng = DetRng::from_seed(777, 1, HubId(3), EconomyDay(0), 0);

    let mut samples = Vec::new();
    for day in 0..30u32 {
        step_di(EconomyDay(day), &mut state, &rp, &mut rng);
        let c1 = state.per_com.get(&CommodityId(1)).copied().unwrap().0;
        let c2 = state.per_com.get(&CommodityId(2)).copied().unwrap().0;
        samples.push((day, c1, c2, state.overlay_bp));
    }

    let expected: Vec<(u32, i32, i32, i32)> = vec![
        (0, 240, 28, 100),
        (1, 237, 162, 80),
        (2, 264, 151, 60),
        (3, 340, 276, 40),
        (4, 314, 333, 20),
        (5, 299, 311, 0),
        (6, 318, 339, 0),
        (7, 409, 265, 0),
        (8, 416, 255, 0),
        (9, 459, 179, 0),
        (10, 436, 218, 0),
        (11, 438, 201, 0),
        (12, 373, 225, 0),
        (13, 339, 147, 0),
        (14, 264, 30, 0),
        (15, 245, 20, 0),
        (16, 191, -18, 0),
        (17, 234, -118, 0),
        (18, 181, -127, 0),
        (19, 113, -79, 0),
        (20, 131, -60, 0),
        (21, 128, -27, 0),
        (22, 102, 55, 0),
        (23, 192, 58, 0),
        (24, 136, -44, 0),
        (25, 93, -16, 0),
        (26, 91, -29, 0),
        (27, -2, -9, 0),
        (28, 34, 28, 0),
        (29, 34, -122, 0),
    ];
    assert_eq!(samples, expected);
}
