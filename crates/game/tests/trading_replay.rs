use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use blake3::Hasher;
use game::app_state::AppState;
use game::systems::command_queue::CommandQueue;
use game::systems::economy::rulepack::load_rulepack;
use game::systems::economy::{
    step_economy_day, BasisBp, CommodityId, EconState, EconStepScope, HubId, MoneyCents, Pp,
};
use game::systems::trading::engine::{TradeKind, TradeResult, TradeTx};
use game::systems::trading::inventory::Cargo;
use game::systems::trading::types::{CommodityCatalog, TradingConfig};
use game::ui::hub_trade::{build_view, persist_on_exit, HubTradeActions, HubTradeUiState};
use repro::CommandKind;
use serde::Serialize;

const HUB: HubId = HubId(1);

#[test]
fn trading_replay_matches_goldens() {
    install_globals();
    let rp = load_rulepack_fixture();
    let update = std::env::var_os("UPDATE_TRADING_GOLDENS").is_some();

    for seed in 1..=3_u64 {
        let (json, hash) = scripted_snapshot(seed, &rp);
        let json_path = golden_path(seed, "json");
        let hash_path = golden_path(seed, "hash");
        if update {
            fs::write(&json_path, &json).expect("write json");
            fs::write(&hash_path, format!("{hash}\n")).expect("write hash");
        } else {
            let golden_json = fs::read_to_string(&json_path).expect("read json");
            assert_eq!(json, golden_json, "snapshot mismatch for seed {seed:02}");

            let golden_hash = fs::read_to_string(&hash_path).expect("read hash");
            assert_eq!(
                hash,
                golden_hash.trim_end(),
                "hash mismatch for seed {seed:02}"
            );
        }
    }
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

fn scripted_snapshot(seed: u64, rp: &game::systems::economy::Rulepack) -> (String, String) {
    let mut app_state = seeded_app_state(seed);
    let starting_wallet = app_state.wallet.as_i64();
    let mut queue = CommandQueue::default();
    queue.begin_tick(0);
    let mut ui_state = HubTradeUiState::default();

    let buys = scripted_buys(seed);
    let mut buy_subtotals = 0_i64;
    let mut buy_fees = 0_i64;
    let mut buy_totals = 0_i64;

    for tx in &buys {
        let result = HubTradeActions::buy(
            &mut queue,
            *tx,
            &app_state.econ,
            &mut app_state.cargo,
            &mut app_state.wallet,
            rp,
        )
        .expect("buy commodity");
        record_buy(&result, &mut buy_subtotals, &mut buy_fees, &mut buy_totals);
    }

    let delta = step_economy_day(
        rp,
        app_state.world_seed,
        app_state.econ_version,
        HUB,
        &mut app_state.econ,
        EconStepScope::GlobalAndHub,
    );
    let clamp_hit = !delta.clamps_hit.is_empty();

    let sell_tx = TradeTx {
        hub: HUB,
        com: CommodityId(3),
        units: 1,
        kind: TradeKind::Sell,
    };
    let sell_result = HubTradeActions::sell(
        &mut queue,
        sell_tx,
        &app_state.econ,
        &mut app_state.cargo,
        &mut app_state.wallet,
        rp,
    )
    .expect("sell commodity");

    let view = build_view(HUB, &app_state.econ, rp, &app_state.cargo, app_state.wallet);
    ui_state.remember(view);
    persist_on_exit(&ui_state, &mut app_state);

    let wallet_delta = app_state.wallet.as_i64() - starting_wallet;
    let identity_rhs = -buy_subtotals + sell_result.subtotal.as_i64()
        - (buy_fees + sell_result.fee_cents.as_i64());
    assert_eq!(wallet_delta, identity_rhs, "wallet identity must hold");

    let meters = meters_from_queue(&mut queue);

    let snapshot = Snapshot {
        seed: seed as u32,
        world_seed: app_state.world_seed,
        hub: HUB.0,
        day: app_state.econ.day.0,
        clamp_hit: clamp_hit as u8,
        wallet_cents: app_state.wallet.as_i64(),
        cargo_units: cargo_units(&app_state.cargo),
        di_bp: basis_list(&app_state.econ.di_bp),
        basis_bp: basis_for_hub(&app_state.econ.basis_bp),
        buy_total_cents: buy_totals,
        sell_total_cents: sell_result.total_cents.as_i64(),
        fee_cents: buy_fees + sell_result.fee_cents.as_i64(),
        meter_buy: meters.0,
        meter_sell: meters.1,
    };

    let mut json = serde_json::to_string_pretty(&snapshot).expect("serialize snapshot");
    if !json.ends_with('\n') {
        json.push('\n');
    }
    let mut hasher = Hasher::new();
    hasher.update(json.as_bytes());
    let hash = hasher.finalize().to_hex().to_string();
    (json, hash)
}

fn record_buy(
    result: &TradeResult,
    subtotal_sum: &mut i64,
    fee_sum: &mut i64,
    total_sum: &mut i64,
) {
    *subtotal_sum += result.subtotal.as_i64();
    *fee_sum += result.fee_cents.as_i64();
    *total_sum += result.total_cents.as_i64();
}

fn seeded_app_state(seed: u64) -> AppState {
    use game::systems::economy::state::RngCursor;
    use game::systems::economy::EconomyDay;

    let econ = EconState {
        day: EconomyDay(2 + seed as u32),
        di_bp: HashMap::from([
            (CommodityId(1), BasisBp(100 + (seed as i32) * 10)),
            (CommodityId(2), BasisBp(-60 + (seed as i32) * 5)),
            (CommodityId(3), BasisBp(25 - (seed as i32) * 3)),
        ]),
        di_overlay_bp: 45 + seed as i32,
        basis_bp: HashMap::from([
            ((HUB, CommodityId(1)), BasisBp(30 + (seed as i32) * 4)),
            ((HUB, CommodityId(2)), BasisBp(-20 + (seed as i32) * 2)),
            ((HUB, CommodityId(3)), BasisBp(10 - (seed as i32))),
        ]),
        pp: Pp(5_000 + (seed as u16) * 150),
        rot_u16: 8 + seed as u16,
        pending_planting: vec![],
        debt_cents: MoneyCents(10_000 + (seed as i64) * 500),
        ..Default::default()
    };

    AppState {
        econ_version: 7,
        world_seed: 0x5000_0000 + seed,
        econ,
        last_hub: HUB,
        inventory: Vec::new(),
        cargo: Cargo {
            capacity_mass_kg: 600,
            capacity_volume_l: 400,
            items: HashMap::from([(CommodityId(2), 1 + seed as u32)]),
        },
        rng_cursors: vec![RngCursor {
            label: "di".to_string(),
            draws: 12 + seed as u32,
        }],
        wallet: MoneyCents(200_000 + (seed as i64) * 1_000),
        last_board_hash: 0,
        visited_links: Vec::new(),
    }
}

fn scripted_buys(seed: u64) -> Vec<TradeTx> {
    let grain_units = 1 + seed as u32;
    let spice_units = 2 + seed as u32;
    vec![
        TradeTx {
            hub: HUB,
            com: CommodityId(1),
            units: grain_units,
            kind: TradeKind::Buy,
        },
        TradeTx {
            hub: HUB,
            com: CommodityId(3),
            units: spice_units,
            kind: TradeKind::Buy,
        },
    ]
}

fn meters_from_queue(queue: &mut CommandQueue) -> (i32, i32) {
    let mut buy = 0;
    let mut sell = 0;
    for cmd in queue.drain() {
        if let CommandKind::Meter(m) = cmd.kind {
            if m.key == "ui_click_buy" {
                buy += m.value;
            } else if m.key == "ui_click_sell" {
                sell += m.value;
            }
        }
    }
    (buy, sell)
}

fn cargo_units(cargo: &Cargo) -> Vec<[u32; 2]> {
    let mut items: Vec<[u32; 2]> = cargo
        .items
        .iter()
        .map(|(commodity, units)| [commodity.0 as u32, *units])
        .collect();
    items.sort_by_key(|entry| entry[0]);
    items
}

fn basis_list(di: &HashMap<CommodityId, BasisBp>) -> Vec<[i32; 2]> {
    let mut items: Vec<[i32; 2]> = di
        .iter()
        .map(|(commodity, value)| [commodity.0 as i32, value.0])
        .collect();
    items.sort_by_key(|entry| entry[0]);
    items
}

fn basis_for_hub(basis: &HashMap<(HubId, CommodityId), BasisBp>) -> Vec<[i32; 2]> {
    let mut items: Vec<[i32; 2]> = basis
        .iter()
        .filter_map(|((hub, commodity), value)| {
            if *hub == HUB {
                Some([commodity.0 as i32, value.0])
            } else {
                None
            }
        })
        .collect();
    items.sort_by_key(|entry| entry[0]);
    items
}

fn asset_path(relative: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("..").join("..").join(relative)
}

fn golden_path(seed: u64, ext: &str) -> PathBuf {
    asset_path(&format!("repro/trading/trade_seed_{seed:02}.{ext}"))
}

#[derive(Serialize)]
struct Snapshot {
    seed: u32,
    world_seed: u64,
    hub: u16,
    day: u32,
    clamp_hit: u8,
    wallet_cents: i64,
    cargo_units: Vec<[u32; 2]>,
    di_bp: Vec<[i32; 2]>,
    basis_bp: Vec<[i32; 2]>,
    buy_total_cents: i64,
    sell_total_cents: i64,
    fee_cents: i64,
    meter_buy: i32,
    meter_sell: i32,
}
