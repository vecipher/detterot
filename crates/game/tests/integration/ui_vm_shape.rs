use std::path::PathBuf;

use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents};
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::build_view;

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn install_globals() {
    let catalog_path = asset_path("assets/trading/commodities.toml");
    let catalog = CommodityCatalog::load_from_path(catalog_path.as_path()).expect("catalog");
    CommodityCatalog::install_global(catalog);
    TradingConfig::install_global(TradingConfig { fee_bp: 75 });
}

fn load_rulepack_fixture() -> game::systems::economy::Rulepack {
    let path = asset_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("rulepack")
}

#[test]
fn view_contains_expected_rows() {
    install_globals();
    let rp = load_rulepack_fixture();
    let mut econ = EconState::default();
    econ.di_bp.insert(CommodityId(1), BasisBp(120));
    econ.di_bp.insert(CommodityId(2), BasisBp(-80));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(45));
    econ.basis_bp
        .insert((HubId(1), CommodityId(2)), BasisBp(-30));

    let cargo = Cargo {
        capacity_mass_kg: 2_000,
        capacity_volume_l: 1_500,
        items: Default::default(),
    };

    let wallet = MoneyCents(12_345);

    let view = build_view(HubId(1), &econ, &rp, &cargo, wallet);

    assert_eq!(view.hub, HubId(1));
    assert_eq!(view.wallet_cents, wallet);
    assert_eq!(view.fee_bp, 75);
    let expected_di = {
        let catalog = CommodityCatalog::global();
        let total: i64 = catalog
            .list()
            .iter()
            .map(|spec| econ.di_bp.get(&spec.id).copied().unwrap_or(BasisBp(0)).0 as i64)
            .sum();
        let count = catalog.list().len().max(1) as i64;
        BasisBp((total / count) as i32)
    };
    assert_eq!(view.di_bp, expected_di);
    assert!(!view.clamp_hit);
    assert_eq!(view.cargo.capacity_mass_kg, 2_000);
    assert_eq!(view.cargo.capacity_volume_l, 1_500);

    let catalog = CommodityCatalog::global();
    assert_eq!(view.commodities.len(), catalog.list().len());
    if let Some(first) = view.commodities.first() {
        assert!(catalog.get(first.id).is_some());
    }
}

#[test]
fn clamp_flag_trips_at_limits() {
    install_globals();
    let rp = load_rulepack_fixture();
    let mut econ = EconState::default();
    econ.di_bp
        .insert(CommodityId(1), BasisBp(rp.di.absolute_max_bp));
    econ.basis_bp.insert(
        (HubId(1), CommodityId(1)),
        BasisBp(rp.basis.absolute_max_bp),
    );

    let cargo = Cargo::default();

    let view = build_view(HubId(1), &econ, &rp, &cargo, MoneyCents::ZERO);
    assert!(view.clamp_hit);
}
