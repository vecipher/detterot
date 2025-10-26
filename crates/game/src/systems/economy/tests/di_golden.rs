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
        (0, 240, 28, 119),
        (1, 256, 181, 118),
        (2, 319, 206, 117),
        (3, 448, 384, 116),
        (4, 490, 509, 115),
        (5, 556, 568, 114),
        (6, 668, 689, 113),
        (7, 844, 700, 112),
        (8, 928, 768, 111),
        (9, 1041, 762, 110),
        (10, 1081, 865, 109),
        (11, 1140, 905, 108),
        (12, 1127, 981, 107),
        (13, 1139, 949, 106),
        (14, 1106, 874, 105),
        (15, 1125, 901, 104),
        (16, 1105, 895, 103),
        (17, 1178, 824, 102),
        (18, 1151, 841, 101),
        (19, 1106, 911, 100),
        (20, 1145, 950, 99),
        (21, 1160, 1001, 98),
        (22, 1150, 1097, 97),
        (23, 1254, 1113, 96),
        (24, 1209, 1021, 95),
        (25, 1175, 1058, 94),
        (26, 1181, 1052, 93),
        (27, 1094, 1077, 92),
        (28, 1133, 1118, 91),
        (29, 1135, 971, 90),
    ];
    assert_eq!(samples, expected);
}
