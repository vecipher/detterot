use std::path::{Path, PathBuf};

use crate::systems::economy::{accrue_interest_per_leg, load_rulepack, InterestCfg, MoneyCents};

fn workspace_path(relative: &str) -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");
    root.join(relative)
}

fn fixture_cfg() -> crate::systems::economy::InterestCfg {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path"))
        .expect("rulepack")
        .interest
}

#[test]
fn interest_is_piecewise_and_caps() {
    let cfg = fixture_cfg();
    let cases = [
        MoneyCents(10_000),    // $100
        MoneyCents(500_000),   // $5,000
        MoneyCents(5_000_000), // $50,000
    ];

    let mut deltas = Vec::new();
    for debt in cases.iter().copied() {
        let (delta, next) = accrue_interest_per_leg(debt, &cfg);
        deltas.push((debt.as_i64(), delta.as_i64(), next.as_i64()));
    }

    let expected = vec![
        (10000, 150, 10150),
        (500000, 7500, 507500),
        (5000000, 82000, 5082000),
    ];
    assert_eq!(deltas, expected);

    let high_debt = MoneyCents(2_000_000_000);
    let (delta, _) = accrue_interest_per_leg(high_debt, &cfg);
    let observed_bp = (delta.as_i64() * 10_000) / high_debt.as_i64();
    assert_eq!(observed_bp, cfg.per_leg_cap_bp as i64);
}

#[test]
fn interest_rounds_half_cent_boundaries() {
    let mut cfg = InterestCfg {
        base_leg_bp: 0,
        linear_leg_bp: 0,
        linear_scale_cents: 1,
        convex_leg_bp: 0,
        convex_gamma_q16: 0,
        per_leg_cap_bp: i32::MAX,
    };

    cfg.base_leg_bp = 51;
    let (delta_high, _) = accrue_interest_per_leg(MoneyCents(100), &cfg);
    assert_eq!(delta_high, MoneyCents(1));

    cfg.base_leg_bp = 49;
    let (delta_low, _) = accrue_interest_per_leg(MoneyCents(100), &cfg);
    assert_eq!(delta_low, MoneyCents(0));
}
