#[path = "support/repro.rs"]
mod repro_support;

use game::systems::command_queue::CommandQueue;
use game::systems::economy::{
    load_rulepack, BasisBp, CommodityId, EconState, HubId, MoneyCents, Pp, Rulepack,
};
use game::systems::trading::types::CommoditySpec;
use game::systems::trading::{execute_trade, inventory::Cargo, meters, types, TradeKind, TradeTx};
use repro::{Record, RecordMeta};

#[test]
fn scripted_trading_sequences_match_goldens() {
    let rulepack = load_fixture_rulepack();
    for seed in trading_seeds() {
        let record = run_seed(&seed, &rulepack);
        repro_support::assert_golden_record(&record, seed.record_rel_path);
    }
}

struct TradingSeed {
    record_rel_path: &'static str,
    meta: RecordMeta,
    hub: HubId,
    commodity: CommodityId,
    base_price: MoneyCents,
    di_bp: BasisBp,
    basis_bp: BasisBp,
    volume_per_unit: u32,
    mass_per_unit: u32,
    initial_wallet: MoneyCents,
    cargo: CargoSetup,
    actions: Vec<TradeAction>,
}

struct CargoSetup {
    capacity_total: u32,
    capacity_used: u32,
    mass_capacity_total: u32,
    mass_capacity_used: u32,
    units: Vec<(CommodityId, u32)>,
}

impl CargoSetup {
    fn build(&self) -> Cargo {
        let mut cargo = Cargo::default();
        cargo.capacity_total = self.capacity_total;
        cargo.capacity_used = self.capacity_used;
        cargo.mass_capacity_total = self.mass_capacity_total;
        cargo.mass_capacity_used = self.mass_capacity_used;
        for (commodity, units) in &self.units {
            cargo.set_units(*commodity, *units);
        }
        cargo
    }
}

struct TradeAction {
    tick: u32,
    kind: TradeKind,
    units: u32,
    base_price: Option<MoneyCents>,
}

fn trading_seeds() -> Vec<TradingSeed> {
    vec![
        TradingSeed {
            record_rel_path: "repro/trading/trade_seed_01.json",
            meta: RecordMeta {
                schema: 1,
                world_seed: "0x00000000000000A1".into(),
                link_id: "401".into(),
                rulepack: "assets/rulepacks/day_001.toml".into(),
                weather: "Clear".into(),
                rng_salt: "0xA1A2A3A4".into(),
                day: 2,
                pp: 280,
                density_per_10k: 5,
                cadence_per_min: 4,
                mission_minutes: 9,
                player_rating: 43,
                prior_danger_score: None,
            },
            hub: HubId(1),
            commodity: CommodityId(3),
            base_price: MoneyCents(275),
            di_bp: BasisBp(0),
            basis_bp: BasisBp(50),
            volume_per_unit: 3,
            mass_per_unit: 2,
            initial_wallet: MoneyCents(9_500),
            cargo: CargoSetup {
                capacity_total: 48,
                capacity_used: 0,
                mass_capacity_total: 36,
                mass_capacity_used: 0,
                units: Vec::new(),
            },
            actions: vec![
                TradeAction {
                    tick: 0,
                    kind: TradeKind::Buy,
                    units: 4,
                    base_price: None,
                },
                TradeAction {
                    tick: 1,
                    kind: TradeKind::Buy,
                    units: 8,
                    base_price: Some(MoneyCents(290)),
                },
                TradeAction {
                    tick: 2,
                    kind: TradeKind::Sell,
                    units: 3,
                    base_price: None,
                },
            ],
        },
        TradingSeed {
            record_rel_path: "repro/trading/trade_seed_02.json",
            meta: RecordMeta {
                schema: 1,
                world_seed: "0x0000000000001F4B".into(),
                link_id: "0x0420".into(),
                rulepack: "assets/rulepacks/day_001.toml".into(),
                weather: "Rains".into(),
                rng_salt: "0x1F4B0ACE".into(),
                day: 5,
                pp: 315,
                density_per_10k: 8,
                cadence_per_min: 6,
                mission_minutes: 12,
                player_rating: 58,
                prior_danger_score: Some(-3),
            },
            hub: HubId(4),
            commodity: CommodityId(7),
            base_price: MoneyCents(320),
            di_bp: BasisBp(5_000),
            basis_bp: BasisBp(2_000),
            volume_per_unit: 5,
            mass_per_unit: 3,
            initial_wallet: MoneyCents(40_000),
            cargo: CargoSetup {
                capacity_total: 100,
                capacity_used: 50,
                mass_capacity_total: 60,
                mass_capacity_used: 30,
                units: vec![(CommodityId(7), 10)],
            },
            actions: vec![
                TradeAction {
                    tick: 0,
                    kind: TradeKind::Sell,
                    units: 8,
                    base_price: None,
                },
                TradeAction {
                    tick: 1,
                    kind: TradeKind::Sell,
                    units: 4,
                    base_price: Some(MoneyCents(315)),
                },
                TradeAction {
                    tick: 3,
                    kind: TradeKind::Buy,
                    units: 6,
                    base_price: None,
                },
                TradeAction {
                    tick: 4,
                    kind: TradeKind::Buy,
                    units: 20,
                    base_price: Some(MoneyCents(305)),
                },
            ],
        },
        TradingSeed {
            record_rel_path: "repro/trading/trade_seed_03.json",
            meta: RecordMeta {
                schema: 1,
                world_seed: "0x000000000000C0DE".into(),
                link_id: "733".into(),
                rulepack: "assets/rulepacks/day_001.toml".into(),
                weather: "Clear".into(),
                rng_salt: "0xC0DEFACE".into(),
                day: 9,
                pp: 260,
                density_per_10k: 6,
                cadence_per_min: 5,
                mission_minutes: 15,
                player_rating: 47,
                prior_danger_score: Some(2),
            },
            hub: HubId(2),
            commodity: CommodityId(5),
            base_price: MoneyCents(410),
            di_bp: BasisBp(-1_200),
            basis_bp: BasisBp(-800),
            volume_per_unit: 4,
            mass_per_unit: 6,
            initial_wallet: MoneyCents(2_400),
            cargo: CargoSetup {
                capacity_total: 24,
                capacity_used: 0,
                mass_capacity_total: 36,
                mass_capacity_used: 0,
                units: Vec::new(),
            },
            actions: vec![
                TradeAction {
                    tick: 0,
                    kind: TradeKind::Buy,
                    units: 20,
                    base_price: None,
                },
                TradeAction {
                    tick: 1,
                    kind: TradeKind::Sell,
                    units: 2,
                    base_price: Some(MoneyCents(405)),
                },
                TradeAction {
                    tick: 2,
                    kind: TradeKind::Buy,
                    units: 3,
                    base_price: None,
                },
            ],
        },
    ]
}

fn run_seed(seed: &TradingSeed, rulepack: &Rulepack) -> Record {
    let _guard = types::global_commodities_guard();
    let mut state = EconState::default();
    state.di_bp.insert(seed.commodity, seed.di_bp);
    state
        .basis_bp
        .insert((seed.hub, seed.commodity), seed.basis_bp);
    state.pp = Pp(seed.meta.pp);

    let mut cargo = seed.cargo.build();
    let mut wallet = seed.initial_wallet;
    let mut queue = CommandQueue::default();

    for action in &seed.actions {
        queue.begin_tick(action.tick);
        let base_price = action.base_price.unwrap_or(seed.base_price);
        register_metadata(
            seed.commodity,
            base_price,
            seed.volume_per_unit,
            seed.mass_per_unit,
        );
        let tx = TradeTx {
            kind: action.kind,
            hub: seed.hub,
            commodity: seed.commodity,
            units: action.units,
        };
        let result =
            execute_trade(&tx, &state, &mut cargo, &mut wallet, rulepack).expect("trade execution");
        meters::record_trade(&mut queue, action.kind, &result);
    }

    Record {
        meta: seed.meta.clone(),
        commands: queue.drain(),
        inputs: Vec::new(),
    }
}

fn register_metadata(
    commodity: CommodityId,
    base_price: MoneyCents,
    volume_per_unit: u32,
    mass_per_unit: u32,
) {
    types::clear_global_commodities();
    let spec = CommoditySpec {
        id: commodity,
        slug: format!("seed-{0}", commodity.0),
        display_name: "Seed".to_string(),
        base_price_cents: base_price.as_i64(),
        mass_per_unit_kg: mass_per_unit,
        volume_per_unit_l: volume_per_unit,
    };
    types::set_global_commodities(types::commodities_from_specs(vec![spec]));
}

fn load_fixture_rulepack() -> Rulepack {
    let path = workspace_path("assets/rulepacks/day_001.toml");
    load_rulepack(path.to_str().expect("utf-8 path")).expect("load rulepack")
}

fn workspace_path(relative: &str) -> std::path::PathBuf {
    let direct = std::path::PathBuf::from(relative);
    if direct.exists() {
        return direct
            .canonicalize()
            .unwrap_or_else(|err| panic!("failed to canonicalize {}: {err}", relative));
    }

    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../")
        .join(relative)
}
