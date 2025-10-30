use bevy::prelude::*;
use std::path::PathBuf;
use tempfile::tempdir;

use game::scheduling;
use game::systems::app_state::AppSaveState;
use game::systems::economy::{
    load_rulepack, BasisBp, CommodityId, EconState, EconomyDay, HubId, MoneyCents, Pp,
};
use game::systems::save::{
    load, save, BasisSave, CargoSave, CargoSlot, CommoditySave, SaveSchema, SaveV1_1,
};
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::load_commodities;
use game::systems::trading::{EnterRoutePlannerViewEvent, TradingPlugin};
use game::ui::hub_trade::{ActiveHub, BuyUnitsEvent, SellUnitsEvent, WalletBalance};

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
fn trade_save_load_roundtrip() {
    let initial_snapshot = SaveV1_1 {
        schema: SaveSchema::v1_1(),
        econ_version: 1,
        world_seed: 777,
        day: EconomyDay(0),
        di: vec![CommoditySave {
            commodity: CommodityId(1),
            value: BasisBp(0),
        }],
        di_overlay_bp: 0,
        basis: vec![BasisSave {
            hub: HubId(1),
            commodity: CommodityId(1),
            value: BasisBp(0),
        }],
        pp: Pp(0),
        rot: 0,
        debt_cents: MoneyCents(0),
        wallet_cents: MoneyCents(12_000),
        inventory: Vec::new(),
        pending_planting: Vec::new(),
        rng_cursors: Vec::new(),
        last_hub: Some(HubId(1)),
        cargo: CargoSave {
            capacity_total: 64,
            capacity_used: 0,
            mass_capacity_total: 64,
            mass_capacity_used: 0,
            manifest: vec![CargoSlot {
                commodity: CommodityId(1),
                units: 1,
            }],
        },
    };

    let mut app = App::new();
    app.insert_resource(AppSaveState::from_snapshot(initial_snapshot.clone()));
    app.insert_resource(load_rulepack(&asset_path("assets/rulepacks/day_001.toml")).unwrap());
    app.insert_resource(load_commodities(&asset_path("assets/trading/commodities.toml")).unwrap());
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.add_plugins(TradingPlugin);

    {
        let mut econ = app.world_mut().resource_mut::<EconState>();
        econ.di_bp.insert(CommodityId(1), BasisBp(0));
        econ.basis_bp.insert((HubId(1), CommodityId(1)), BasisBp(0));
    }
    app.update();

    {
        let cargo = app.world().resource::<Cargo>();
        assert_eq!(cargo.capacity_total, 64);
        assert_eq!(cargo.units(CommodityId(1)), 1);
    }
    {
        let wallet = app.world().resource::<WalletBalance>();
        assert_eq!(wallet.0, MoneyCents(12_000));
    }
    {
        let hub = app.world().resource::<ActiveHub>();
        assert_eq!(hub.0, HubId(1));
    }

    {
        let mut messages = app.world_mut().resource_mut::<Messages<BuyUnitsEvent>>();
        messages.write(BuyUnitsEvent {
            commodity: CommodityId(1),
            units: 2,
        });
        messages.update();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    {
        let mut messages = app.world_mut().resource_mut::<Messages<SellUnitsEvent>>();
        messages.write(SellUnitsEvent {
            commodity: CommodityId(1),
            units: 1,
        });
        messages.update();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    app.world_mut().resource_mut::<ActiveHub>().0 = HubId(3);

    {
        let mut messages = app
            .world_mut()
            .resource_mut::<Messages<EnterRoutePlannerViewEvent>>();
        messages.write(EnterRoutePlannerViewEvent);
        messages.update();
    }
    app.update();
    app.world_mut().run_schedule(FixedUpdate);

    let wallet_after_trade = app.world().resource::<WalletBalance>().0;
    let cargo_after_trade = app.world().resource::<Cargo>().manifest_snapshot();

    let snapshot_after_trade = app.world().resource::<AppSaveState>().clone_snapshot();
    assert_eq!(snapshot_after_trade.wallet_balance(), wallet_after_trade);
    assert_eq!(snapshot_after_trade.last_hub, Some(HubId(3)));
    let manifest: Vec<_> = snapshot_after_trade
        .cargo
        .manifest
        .iter()
        .map(|slot| (slot.commodity, slot.units))
        .collect();
    assert_eq!(manifest, cargo_after_trade);

    let dir = tempdir().expect("temp dir");
    let path = dir.path().join("roundtrip_save.json");
    save(&path, &snapshot_after_trade).expect("write save");
    let loaded_snapshot = load(&path).expect("load save");
    assert_eq!(loaded_snapshot, snapshot_after_trade);

    drop(app);

    let mut app = App::new();
    app.insert_resource(AppSaveState::from_snapshot(loaded_snapshot.clone()));
    app.insert_resource(load_rulepack(&asset_path("assets/rulepacks/day_001.toml")).unwrap());
    app.insert_resource(load_commodities(&asset_path("assets/trading/commodities.toml")).unwrap());
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    app.add_plugins(TradingPlugin);

    {
        let mut econ = app.world_mut().resource_mut::<EconState>();
        econ.di_bp.insert(CommodityId(1), BasisBp(0));
        econ.basis_bp.insert((HubId(1), CommodityId(1)), BasisBp(0));
    }
    app.update();

    {
        let cargo = app.world().resource::<Cargo>();
        assert_eq!(cargo.manifest_snapshot(), cargo_after_trade);
        assert_eq!(cargo.capacity_total, 64);
    }
    {
        let wallet = app.world().resource::<WalletBalance>();
        assert_eq!(wallet.0, wallet_after_trade);
    }
    {
        let hub = app.world().resource::<ActiveHub>();
        assert_eq!(hub.0, HubId(3));
    }
}
