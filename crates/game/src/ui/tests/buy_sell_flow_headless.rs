use std::path::PathBuf;

use bevy::prelude::*;

use crate::scheduling;
use crate::systems::economy::{BasisBp, CommodityId, EconState, HubId, MoneyCents, Rulepack};
use crate::systems::trading::inventory::Cargo;
use crate::systems::trading::types::load_commodities;
use crate::ui::hub_trade::{
    ActiveHub, BuyUnitsEvent, HubTradeCatalog, HubTradeUiPlugin, HubTradeViewModel,
    SelectedCommodity, SellUnitsEvent, WalletBalance,
};

fn load_rulepack() -> Rulepack {
    crate::systems::economy::load_rulepack(&asset_path("assets/rulepacks/day_001.toml")).unwrap()
}

fn load_specs_path() -> String {
    asset_path("assets/trading/commodities.toml")
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
fn buying_and_selling_updates_inventory_and_wallet() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.add_plugins(HubTradeUiPlugin);

    let mut econ = EconState::default();
    econ.di_bp.insert(CommodityId(1), BasisBp(0));
    econ.basis_bp.insert((HubId(1), CommodityId(1)), BasisBp(0));
    app.insert_resource(econ);
    app.insert_resource(load_rulepack());
    app.insert_resource(load_commodities(&load_specs_path()).unwrap());

    let mut catalog = HubTradeCatalog::default();
    catalog.insert(CommodityId(1), MoneyCents(250), 4, 2);
    app.insert_resource(catalog);

    let mut cargo = Cargo::default();
    cargo.capacity_total = 64;
    cargo.mass_capacity_total = 64;
    app.insert_resource(cargo);

    app.insert_resource(ActiveHub(HubId(1)));
    app.insert_resource(SelectedCommodity(Some(CommodityId(1))));
    app.insert_resource(WalletBalance(MoneyCents(10_000)));

    app.update();

    {
        let mut messages = app.world_mut().resource_mut::<Messages<BuyUnitsEvent>>();
        messages.write(BuyUnitsEvent {
            commodity: CommodityId(1),
            units: 3,
        });
        messages.update();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    {
        let cargo = app.world().resource::<Cargo>();
        assert_eq!(cargo.units(CommodityId(1)), 3);
        assert_eq!(cargo.capacity_used, 12);
        assert_eq!(cargo.mass_capacity_used, 6);
    }
    {
        let wallet = app.world().resource::<WalletBalance>();
        assert_eq!(wallet.0, MoneyCents(9_245));
    }
    {
        let vm = app.world().resource::<HubTradeViewModel>();
        assert_eq!(vm.buy_stepper.last_units, 3);
    }

    {
        let mut messages = app.world_mut().resource_mut::<Messages<SellUnitsEvent>>();
        messages.write(SellUnitsEvent {
            commodity: CommodityId(1),
            units: 2,
        });
        messages.update();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    {
        let cargo = app.world().resource::<Cargo>();
        assert_eq!(cargo.units(CommodityId(1)), 1);
        assert_eq!(cargo.capacity_used, 4);
        assert_eq!(cargo.mass_capacity_used, 2);
    }
    {
        let wallet = app.world().resource::<WalletBalance>();
        assert_eq!(wallet.0, MoneyCents(9_742));
    }
    {
        let vm = app.world().resource::<HubTradeViewModel>();
        assert_eq!(vm.sell_stepper.last_units, 2);
    }
}
