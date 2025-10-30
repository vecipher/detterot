use std::path::PathBuf;

use bevy::prelude::*;

use crate::scheduling;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Pp, Rulepack};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::types::{load_commodities, Commodities};
use crate::ui::hub_trade::{
    ActiveHub, HubTradeCatalog, HubTradeUiPlugin, HubTradeViewModel, SelectedCommodity,
    WalletBalance,
};

fn load_rulepack() -> Rulepack {
    crate::systems::economy::load_rulepack(&asset_path("assets/rulepacks/day_001.toml")).unwrap()
}

fn load_specs() -> Commodities {
    load_commodities(&asset_path("assets/trading/commodities.toml")).unwrap()
}

fn asset_path(relative: &str) -> String {
    let direct = PathBuf::from(relative);
    if direct.exists() {
        return direct
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../{relative}"))
        .to_string_lossy()
        .into_owned()
}

#[test]
fn view_models_reflect_resources() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.add_plugins(HubTradeUiPlugin);

    let mut econ = EconState {
        pp: Pp(6200),
        ..Default::default()
    };
    econ.di_bp.insert(CommodityId(1), BasisBp(120));
    econ.di_bp.insert(CommodityId(2), BasisBp(-45));
    econ.basis_bp
        .insert((HubId(1), CommodityId(1)), BasisBp(30));
    econ.basis_bp
        .insert((HubId(1), CommodityId(2)), BasisBp(-15));
    app.insert_resource(econ);

    app.insert_resource(load_rulepack());
    let specs = load_specs();
    app.insert_resource(specs.clone());

    let mut catalog = HubTradeCatalog::default();
    catalog.rebuild_from_specs(&specs);
    app.insert_resource(catalog);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 120;
    cargo.capacity_used = 20;
    cargo.mass_capacity_total = 150;
    cargo.mass_capacity_used = 10;
    cargo.set_units(CommodityId(1), 4);
    cargo.set_units(CommodityId(2), 8);
    app.insert_resource(cargo);

    app.insert_resource(ActiveHub(HubId(1)));
    app.insert_resource(SelectedCommodity(Some(CommodityId(1))));
    app.insert_resource(WalletBalance(MoneyCents(25_000)));

    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    let vm = app.world().resource::<HubTradeViewModel>().clone();

    assert!(!vm.di_ticker.entries.is_empty());
    let grain = vm
        .di_ticker
        .entries
        .iter()
        .find(|entry| entry.commodity == CommodityId(1))
        .unwrap();
    assert_eq!(grain.di_bp, 120);

    let list = &vm.commodity_list;
    assert!(list.rows.len() >= 2);
    let selected_index = list.selected.expect("selected index");
    let selected_row = &list.rows[selected_index];
    assert_eq!(selected_row.commodity, CommodityId(1));

    let grain_row = list
        .rows
        .iter()
        .find(|row| row.commodity == CommodityId(1))
        .expect("grain row present");
    assert!(grain_row.can_buy);
    assert!(grain_row.can_sell);
    assert!(grain_row.max_buy >= 1);
    assert_eq!(grain_row.held_units, 4);

    let textiles_row = list
        .rows
        .iter()
        .find(|row| row.commodity == CommodityId(2))
        .expect("textiles row present");
    assert!(textiles_row.can_sell);

    assert_eq!(vm.cargo_panel.capacity_total, 120);
    assert_eq!(vm.wallet_panel.balance, MoneyCents(25_000));

    assert_eq!(vm.buy_stepper.step, 1);
    assert!(vm.buy_stepper.max >= 1);
    assert_eq!(vm.sell_stepper.step, 1);
    assert!(vm.sell_stepper.max >= 4);

    assert_eq!(vm.driver_chips.chips.len(), 4);
    let pp_chip = vm
        .driver_chips
        .chips
        .iter()
        .find(|chip| chip.label == "PP")
        .unwrap();
    assert_eq!(pp_chip.value, "6200");
}
