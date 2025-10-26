use std::path::{Path, PathBuf};

use crate::systems::economy::{
    load_rulepack, update_basis, BasisBp, BasisDrivers, DetRng, EconomyDay, HubId, Pp, Rulepack,
    Weather,
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
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

fn seeded_rng(tag: u32) -> DetRng {
    DetRng::from_seed(900, 1, HubId(2), EconomyDay(0), tag)
}

#[test]
fn pp_driver_is_monotone() {
    let rp = load_fixture_pack();
    let base = BasisBp(200);
    let drivers_low = BasisDrivers {
        pp: Pp(rp.pp.neutral_pp - 1500),
        weather: Weather::Clear,
        closed_routes: 0,
        stock_dev: 0,
    };
    let drivers_high = BasisDrivers {
        pp: Pp(rp.pp.neutral_pp + 1500),
        ..drivers_low
    };

    let mut rng_low = seeded_rng(1);
    let mut rng_high = seeded_rng(1);
    let low = update_basis(base, &drivers_low, &rp, &mut rng_low);
    let high = update_basis(base, &drivers_high, &rp, &mut rng_high);

    assert!(low.0 < base.0);
    assert!(high.0 > base.0);
    assert!(high.0 - base.0 > base.0 - low.0);
}

#[test]
fn stock_driver_is_monotone() {
    let rp = load_fixture_pack();
    let base = BasisBp(-100);
    let drivers_low = BasisDrivers {
        pp: Pp(rp.pp.neutral_pp),
        weather: Weather::Fog,
        closed_routes: 0,
        stock_dev: -500,
    };
    let drivers_high = BasisDrivers { stock_dev: 500, ..drivers_low };

    let mut rng_low = seeded_rng(2);
    let mut rng_high = seeded_rng(2);
    let low = update_basis(base, &drivers_low, &rp, &mut rng_low);
    let high = update_basis(base, &drivers_high, &rp, &mut rng_high);

    assert!(low.0 < high.0);
}

#[test]
fn basis_delta_hits_clamps() {
    let rp = load_fixture_pack();
    let clamp = rp.basis.per_day_clamp_bp;

    let drivers_push_up = BasisDrivers {
        pp: Pp(rp.pp.max_pp),
        weather: Weather::Windy,
        closed_routes: 12,
        stock_dev: 2_000,
    };
    let drivers_push_down = BasisDrivers {
        pp: Pp(rp.pp.min_pp),
        weather: Weather::Rains,
        closed_routes: 0,
        stock_dev: -2_000,
    };

    let mut rng_up = seeded_rng(3);
    let mut rng_down = seeded_rng(4);

    let up = update_basis(BasisBp(0), &drivers_push_up, &rp, &mut rng_up);
    let down = update_basis(BasisBp(0), &drivers_push_down, &rp, &mut rng_down);

    assert_eq!(up.0, clamp);
    assert_eq!(down.0, -clamp);
}
