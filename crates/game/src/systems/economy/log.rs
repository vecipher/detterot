#[cfg(feature = "econ_logs")]
use std::collections::HashMap;
#[cfg(feature = "econ_logs")]
use std::env;
#[cfg(feature = "econ_logs")]
use std::fs::{self, OpenOptions};
#[cfg(feature = "econ_logs")]
use std::io::{Error as IoError, ErrorKind, Write};
#[cfg(feature = "econ_logs")]
use std::path::{Path, PathBuf};

#[cfg(feature = "econ_logs")]
use serde_json::json;

use super::state::EconDelta;

#[cfg(feature = "econ_logs")]
use super::{compute_price, BasisBp, CommodityId, MoneyCents, Weather};

#[cfg(feature = "econ_logs")]
const LOG_BASE_PRICE: MoneyCents = MoneyCents(10_000);

#[cfg(feature = "econ_logs")]
pub fn log_econ_tick(delta: &EconDelta) {
    if delta.di.is_empty() {
        return;
    }

    if let Err(err) = append_entries(delta) {
        eprintln!("econ log error: {err}");
    }
}

#[cfg(feature = "econ_logs")]
fn append_entries(delta: &EconDelta) -> std::io::Result<()> {
    let mut basis_lookup: HashMap<CommodityId, BasisBp> = HashMap::new();
    for entry in &delta.basis {
        basis_lookup.insert(entry.commodity, entry.value);
    }

    let path = resolve_log_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    for entry in &delta.di {
        let basis_bp = basis_lookup
            .get(&entry.commodity)
            .copied()
            .unwrap_or(BasisBp(0));
        let price = compute_price(LOG_BASE_PRICE, entry.value, basis_bp);
        let record = json!({
            "day": delta.day.0,
            "hub": delta.hub.0,
            "com": entry.commodity.0,
            "di_bp": entry.value.0,
            "basis_bp": basis_bp.0,
            "drivers": {
                "pp": delta.pp_after.0,
                "weather": format!("{:?}", Weather::Clear),
                "routes": 0,
                "stock": 0
            },
            "clamps_applied": delta.clamps_hit.clone(),
            "price_cents": price.as_i64()
        });
        let line = serde_json::to_string(&record).map_err(json_err)?;
        writeln!(file, "{line}")?;
    }

    Ok(())
}

#[cfg(feature = "econ_logs")]
fn resolve_log_path() -> PathBuf {
    if let Ok(dir) = env::var("DETTEROT_ECON_LOG_DIR") {
        PathBuf::from(dir).join("econ_tick.jsonl")
    } else {
        Path::new("logs/econ/econ_tick.jsonl").to_path_buf()
    }
}

#[cfg(feature = "econ_logs")]
fn json_err(err: serde_json::Error) -> IoError {
    IoError::new(ErrorKind::Other, err)
}

#[cfg(not(feature = "econ_logs"))]
pub fn log_econ_tick(_delta: &EconDelta) {}

#[cfg(all(test, feature = "econ_logs"))]
mod tests {
    use super::*;
    use crate::systems::economy::state::CommodityDelta;
    use crate::systems::economy::{BasisBp, CommodityId, EconomyDay, HubId, MoneyCents, Pp};
    use tempfile::tempdir;

    #[test]
    fn writes_jsonl_entry() {
        let dir = tempdir().expect("dir");
        env::set_var("DETTEROT_ECON_LOG_DIR", dir.path());
        let delta = EconDelta {
            day: EconomyDay(0),
            hub: HubId(1),
            di: vec![CommodityDelta {
                commodity: CommodityId(1),
                value: BasisBp(10),
            }],
            basis: vec![CommodityDelta {
                commodity: CommodityId(1),
                value: BasisBp(5),
            }],
            pp_before: Pp(5000),
            pp_after: Pp(5000),
            rot_before: 0,
            rot_after: 0,
            debt_before: MoneyCents(0),
            debt_after: MoneyCents(0),
            clamps_hit: vec![],
            rng_cursors: vec![],
        };
        log_econ_tick(&delta);
        let log_path = dir.path().join("econ_tick.jsonl");
        let data = fs::read_to_string(log_path).expect("log file");
        assert!(data.contains("\"price_cents\""));
        env::remove_var("DETTEROT_ECON_LOG_DIR");
    }
}
