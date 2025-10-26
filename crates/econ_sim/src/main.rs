use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::num::ParseIntError;
use std::path::PathBuf;

use game::systems::economy::{
    compute_price, load_rulepack, step_economy_day, BasisBp, CommodityId, EconState, EconomyDay,
    HubId, MoneyCents, Pp, Rulepack,
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

    let mut states = seed_states(args, rp);
    for day in 0..args.days {
        for (idx, state) in states.iter_mut().enumerate() {
            let hub = HubId((idx as u16) + 1);
            let delta = step_economy_day(rp, args.world_seed, ECON_VERSION, hub, state);
            let interest = delta.interest_delta;
            let mut commodities: Vec<_> = state.di_bp.keys().copied().collect();
            commodities.sort_by_key(|c| c.0);
            for commodity in commodities {
                let di_bp = state.di_bp.get(&commodity).copied().unwrap_or(BasisBp(0));
                let basis_bp = state
                    .basis_bp
                    .get(&(hub, commodity))
                    .copied()
                    .unwrap_or(BasisBp(0));
                let price = compute_price(MoneyCents(BASE_PRICE_CENTS), di_bp, basis_bp);
                writeln!(
                    writer,
                    "{day},{},{},{},{},{},{},{},{},{}",
                    hub.0,
                    commodity.0,
                    di_bp.0,
                    basis_bp.0,
                    price.as_i64(),
                    state.debt_cents.as_i64(),
                    interest.as_i64(),
                    state.pp.0,
                    state.rot_u16
                )?;
            }
            assert!(delta.day.0 <= day, "delta day monotonic");
        }
    }

    writer.flush()
}

fn seed_states(args: &Args, rp: &Rulepack) -> Vec<EconState> {
    let mut states = Vec::with_capacity(args.hubs as usize);
    for idx in 0..args.hubs as usize {
        let mut di_bp = HashMap::new();
        di_bp.insert(CommodityId(1), BasisBp(0));
        di_bp.insert(CommodityId(2), BasisBp(0));
        let pp_value = pick_value(idx, &args.pp, rp.pp.neutral_pp);
        let debt_value = pick_value(idx, &args.debt, 0i64);
        states.push(EconState {
            day: EconomyDay(0),
            di_bp,
            di_overlay_bp: 0,
            basis_bp: HashMap::new(),
            pp: Pp(pp_value),
            rot_u16: 0,
            pending_planting: Vec::new(),
            debt_cents: MoneyCents(debt_value),
        });
    }
    states
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
