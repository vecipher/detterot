use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::num::ParseIntError;
use std::path::PathBuf;

use game::systems::economy::{
    compute_price, load_rulepack, step_economy_day, BasisBp, CommodityId, EconState, EconStepScope,
    EconomyDay, HubId, MoneyCents, Pp, Rulepack,
};

const ECON_VERSION: u32 = 1;
const RULEPACK_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/rulepacks/day_001.toml"
);
const BASE_PRICE_CENTS: i64 = 10_000;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), String> {
    let args = Args::parse()?;
    let rulepack = load_rulepack(RULEPACK_PATH).map_err(|err| err.to_string())?;
    run_sim(&args, &rulepack).map_err(|err| err.to_string())
}

fn run_sim(args: &Args, rp: &Rulepack) -> Result<(), std::io::Error> {
    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file = File::create(&args.out)?;
    let mut writer = BufWriter::new(file);
    writeln!(
        writer,
        "day,hub,com,di_bp,basis_bp,price_cents,debt_cents,interest_cents,pp,rot_u16"
    )?;

    let (mut state, hubs) = seed_state(args, rp);
    for day in 0..args.days {
        let mut interest_by_hub = Vec::with_capacity(hubs.len());
        let mut global_snapshot = None;
        for (idx, hub) in hubs.iter().enumerate() {
            let scope = if idx == 0 {
                EconStepScope::GlobalAndHub
            } else {
                EconStepScope::HubOnly
            };
            let delta =
                step_economy_day(rp, args.world_seed, ECON_VERSION, hub.id, &mut state, scope);
            if idx == 0 {
                global_snapshot = Some(GlobalSnapshot {
                    debt_cents: state.debt_cents,
                    pp: state.pp,
                    rot_u16: state.rot_u16,
                });
            }
            interest_by_hub.push(delta.interest_delta);
            assert!(delta.day.0 <= day, "delta day monotonic");
        }

        let global_snapshot = global_snapshot.expect("at least one hub");
        let mut commodities: Vec<_> = state.di_bp.keys().copied().collect();
        commodities.sort_by_key(|c| c.0);

        for (interest, hub_metadata) in interest_by_hub.into_iter().zip(hubs.iter()) {
            let hub_id = hub_metadata.id;
            for commodity in &commodities {
                let di_bp = state.di_bp.get(commodity).copied().unwrap_or(BasisBp(0));
                let basis_bp = state
                    .basis_bp
                    .get(&(hub_id, *commodity))
                    .copied()
                    .unwrap_or(BasisBp(0));
                let price =
                    compute_price(MoneyCents(BASE_PRICE_CENTS), di_bp, basis_bp, &rp.pricing);
                writeln!(
                    writer,
                    "{day},{},{},{},{},{},{},{},{},{}",
                    hub_id.0,
                    commodity.0,
                    di_bp.0,
                    basis_bp.0,
                    price.as_i64(),
                    global_snapshot.debt_cents.as_i64(),
                    interest.as_i64(),
                    global_snapshot.pp.0,
                    global_snapshot.rot_u16
                )?;
            }
        }
    }

    writer.flush()
}

fn seed_state(args: &Args, rp: &Rulepack) -> (EconState, Vec<HubMetadata>) {
    let mut di_bp = HashMap::new();
    di_bp.insert(CommodityId(1), BasisBp(0));
    di_bp.insert(CommodityId(2), BasisBp(0));
    let pp_value = pick_value(0, &args.pp, rp.pp.neutral_pp);
    let debt_value = pick_value(0, &args.debt, 0i64);
    let state = EconState {
        day: EconomyDay(0),
        di_bp,
        di_overlay_bp: 0,
        basis_bp: HashMap::new(),
        pp: Pp(pp_value),
        rot_u16: 0,
        pending_planting: Vec::new(),
        debt_cents: MoneyCents(debt_value),
    };
    let hubs = (0..args.hubs)
        .map(|idx| HubMetadata { id: HubId(idx + 1) })
        .collect();
    (state, hubs)
}

struct HubMetadata {
    id: HubId,
}

struct GlobalSnapshot {
    debt_cents: MoneyCents,
    pp: Pp,
    rot_u16: u16,
}

fn pick_value<T: Copy>(idx: usize, values: &[T], default: T) -> T {
    values.get(idx).copied().unwrap_or(default)
}

struct Args {
    world_seed: u64,
    days: u32,
    hubs: u16,
    pp: Vec<u16>,
    debt: Vec<i64>,
    out: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut world_seed = None;
        let mut days = None;
        let mut hubs = None;
        let mut pp = Vec::new();
        let mut debt = Vec::new();
        let mut out = PathBuf::from("target/econ_curves.csv");
        let mut iter = env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--world-seed" => {
                    world_seed = Some(parse_u64(next_value(&mut iter, "--world-seed")?)?)
                }
                "--days" => days = Some(parse_u32(next_value(&mut iter, "--days")?)?),
                "--hubs" => hubs = Some(parse_u16(next_value(&mut iter, "--hubs")?)?),
                "--pp" => pp = parse_list_u16(next_value(&mut iter, "--pp")?)?,
                "--debt" => debt = parse_list_i64(next_value(&mut iter, "--debt")?)?,
                "--out" => out = PathBuf::from(next_value(&mut iter, "--out")?),
                flag => return Err(format!("unknown argument {flag}")),
            }
        }

        Ok(Self {
            world_seed: world_seed.ok_or("--world-seed missing")?,
            days: days.ok_or("--days missing")?,
            hubs: hubs.ok_or("--hubs missing")?,
            pp,
            debt,
            out,
        })
    }
}

fn next_value(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    iter.next().ok_or_else(|| format!("{flag} expects a value"))
}

fn parse_u64(value: String) -> Result<u64, String> {
    value.parse().map_err(|err: ParseIntError| err.to_string())
}

fn parse_u32(value: String) -> Result<u32, String> {
    value.parse().map_err(|err: ParseIntError| err.to_string())
}

fn parse_u16(value: String) -> Result<u16, String> {
    value.parse().map_err(|err: ParseIntError| err.to_string())
}

fn parse_list_u16(raw: String) -> Result<Vec<u16>, String> {
    raw.split(',')
        .filter(|s| !s.is_empty())
        .map(|part| {
            part.replace('_', "")
                .parse::<u16>()
                .map_err(|err| err.to_string())
        })
        .collect()
}

fn parse_list_i64(raw: String) -> Result<Vec<i64>, String> {
    raw.split(',')
        .filter(|s| !s.is_empty())
        .map(|part| {
            part.replace('_', "")
                .parse::<i64>()
                .map_err(|err| err.to_string())
        })
        .collect()
}
