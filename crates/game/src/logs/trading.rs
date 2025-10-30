#![allow(dead_code)]

#[cfg(feature = "econ_logs")]
use std::env;
#[cfg(feature = "econ_logs")]
use std::fs::{create_dir_all, OpenOptions};
#[cfg(feature = "econ_logs")]
use std::io::Write;
#[cfg(feature = "econ_logs")]
use std::path::{Path, PathBuf};

#[cfg(feature = "econ_logs")]
use anyhow::Context;
#[cfg(feature = "econ_logs")]
use repro::canonical_json_bytes;
#[cfg(feature = "econ_logs")]
use serde::Serialize;

#[cfg(feature = "econ_logs")]
use crate::systems::economy::MoneyCents;
#[cfg(feature = "econ_logs")]
use crate::systems::trading::engine::{TradeKind, TradeResult, TradeTx};

#[cfg(feature = "econ_logs")]
pub fn log_trade(tx: &TradeTx, result: &TradeResult, wallet_after: MoneyCents) {
    if let Err(err) = append_entry(tx, result, wallet_after) {
        eprintln!("trading log error: {err}");
    }
}

#[cfg(feature = "econ_logs")]
fn append_entry(
    tx: &TradeTx,
    result: &TradeResult,
    wallet_after: MoneyCents,
) -> anyhow::Result<()> {
    let entry = TradeLogEntry::new(tx, result, wallet_after);
    let path = resolve_log_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening trade log {}", path.display()))?;
    let bytes = canonical_json_bytes(&entry).context("serializing trade log entry")?;
    file.write_all(&bytes).context("writing trade log entry")?;
    Ok(())
}

#[cfg(feature = "econ_logs")]
fn resolve_log_path() -> PathBuf {
    if let Ok(dir) = env::var("DETTEROT_ECON_LOG_DIR") {
        Path::new(&dir).join("trading.jsonl")
    } else {
        PathBuf::from("logs/econ/trading.jsonl")
    }
}

#[cfg(feature = "econ_logs")]
#[derive(Serialize)]
struct TradeLogEntry {
    hub: u16,
    com: u16,
    kind: &'static str,
    units: u32,
    unit_price: i64,
    fee: i64,
    wallet_after: i64,
}

#[cfg(feature = "econ_logs")]
impl TradeLogEntry {
    fn new(tx: &TradeTx, result: &TradeResult, wallet_after: MoneyCents) -> Self {
        Self {
            hub: tx.hub.0,
            com: tx.commodity.0,
            kind: match tx.kind {
                TradeKind::Buy => "buy",
                TradeKind::Sell => "sell",
            },
            units: result.units_executed,
            unit_price: result.unit_price.as_i64(),
            fee: result.fee.as_i64(),
            wallet_after: wallet_after.as_i64(),
        }
    }
}

#[cfg(all(test, feature = "econ_logs"))]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn trade_log_is_canonical() {
        let dir = tempdir().expect("dir");
        env::set_var("DETTEROT_ECON_LOG_DIR", dir.path());

        let tx = TradeTx {
            kind: TradeKind::Buy,
            hub: crate::systems::economy::HubId(2),
            commodity: crate::systems::economy::CommodityId(1),
            units: 5,
        };
        let result = TradeResult {
            units_executed: 5,
            unit_price: MoneyCents(123),
            subtotal: MoneyCents(615),
            fee: MoneyCents(6),
            total_cents: MoneyCents(621),
            wallet_delta: MoneyCents(-621),
        };

        log_trade(&tx, &result, MoneyCents(10));

        let log_path = dir.path().join("trading.jsonl");
        let contents = fs::read_to_string(log_path).expect("log file");
        assert_eq!(
            contents,
            "{\"com\":1,\"fee\":6,\"hub\":2,\"kind\":\"buy\",\"unit_price\":123,\"units\":5,\"wallet_after\":10}\n"
        );

        env::remove_var("DETTEROT_ECON_LOG_DIR");
    }
}
