use std::collections::HashMap;
use std::path::PathBuf;

use game::app_state::AppState;
use game::systems::command_queue::CommandQueue;
use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::state::RngCursor;
use game::systems::economy::{
    BasisBp, CommodityId, EconState, HubId, MoneyCents, PendingPlanting, Pp,
};
use game::systems::save::{load_app_state, save_app_state, snapshot_from_app_state, InventorySlot};
use game::systems::trading::engine::{TradeKind, TradeTx};
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::{build_view, persist_on_exit, HubTradeActions, HubTradeUiState};
use tempfile::tempdir;

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

fn sample_app_state() -> AppState {
    let mut econ = EconState::default();
    econ.day = game::systems::economy::EconomyDay(3);
    econ.di_bp = HashMap::from([
        (CommodityId(1), BasisBp(120)),
        (CommodityId(2), BasisBp(-80)),
    ]);
    econ.di_overlay_bp = 90;
    econ.basis_bp = HashMap::from([
        ((HubId(1), CommodityId(1)), BasisBp(45)),
        ((HubId(1), CommodityId(2)), BasisBp(-30)),
    ]);
    econ.pp = Pp(5_100);
    econ.rot_u16 = 12;
    econ.pending_planting = vec![PendingPlanting {
        hub: HubId(1),
        size: 4,
        age_days: 2,
    }];
    econ.debt_cents = MoneyCents(4_200);

    AppState {
        econ_version: 7,
        world_seed: 0xDEADBEEF,
        econ,
        last_hub: HubId(1),
        inventory: vec![InventorySlot {
            commodity: CommodityId(9),
            amount: 33,
        }],
        cargo: Cargo {
            capacity_mass_kg: 2_000,
            capacity_volume_l: 1_500,
            items: HashMap::new(),
        },
        rng_cursors: vec![RngCursor {
            label: "di".to_string(),
            draws: 24,
        }],
        wallet: MoneyCents(100_000),
    }
}

#[test]
fn save_load_roundtrip_preserves_state() {
    install_globals();
    let rp = load_rulepack_fixture();
    let mut app_state = sample_app_state();
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);

    let mut ui_state = HubTradeUiState::default();

    let buy_spice = TradeTx {
        hub: HubId(1),
        com: CommodityId(3),
        units: 4,
        kind: TradeKind::Buy,
    };
    HubTradeActions::buy(
        &mut queue,
        buy_spice,
        &app_state.econ,
        &mut app_state.cargo,
        &mut app_state.wallet,
        &rp,
    )
    .expect("buy spice");

    let buy_grain = TradeTx {
        hub: HubId(1),
        com: CommodityId(1),
        units: 2,
        kind: TradeKind::Buy,
    };
    HubTradeActions::buy(
        &mut queue,
        buy_grain,
        &app_state.econ,
        &mut app_state.cargo,
        &mut app_state.wallet,
        &rp,
    )
    .expect("buy grain");

    let sell_spice = TradeTx {
        hub: HubId(1),
        com: CommodityId(3),
        units: 1,
        kind: TradeKind::Sell,
    };
    HubTradeActions::sell(
        &mut queue,
        sell_spice,
        &app_state.econ,
        &mut app_state.cargo,
        &mut app_state.wallet,
        &rp,
    )
    .expect("sell spice");

    let view = build_view(
        HubId(1),
        &app_state.econ,
        &rp,
        &app_state.cargo,
        app_state.wallet,
    );
    ui_state.remember(view);
    persist_on_exit(&ui_state, &mut app_state);

    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("state.json");
    save_app_state(&path, &app_state).expect("save state");

    let loaded = load_app_state(&path).expect("load state");
    assert_eq!(loaded, app_state);

    let snapshot = snapshot_from_app_state(&loaded);
    assert_eq!(snapshot.day, app_state.econ.day);
}
