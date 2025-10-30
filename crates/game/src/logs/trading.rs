use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use serde::Serialize;

use crate::systems::economy::MoneyCents;
use crate::systems::trading::engine::{TradeKind, TradeResult, TradeTx};

static LOGS_ENABLED: AtomicBool = AtomicBool::new(cfg!(feature = "m3_logs"));

pub fn set_enabled(enabled: bool) {
    LOGS_ENABLED.store(enabled, Ordering::Relaxed);
}

fn enabled() -> bool {
    LOGS_ENABLED.load(Ordering::Relaxed)
}

fn append_jsonl<T: Serialize>(file: &str, value: &T) -> anyhow::Result<()> {
    const DIR: &str = "logs/m3";
    create_dir_all(DIR).context("creating m3 log directory")?;
    let path = Path::new(DIR).join(file);
    let mut handle = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening log file {}", path.display()))?;
    let line = repro::canonical_json_bytes(value)?;
    handle.write_all(&line)?;
    Ok(())
}

pub fn log_trade(
    tx: &TradeTx,
    result: &TradeResult,
    wallet_after: MoneyCents,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct TradeLog {
        hub: u16,
        commodity: u16,
        kind: &'static str,
        units: u32,
        unit_price_cents: i64,
        subtotal_cents: i64,
        fee_cents: i64,
        total_cents: i64,
        wallet_after_cents: i64,
    }

    let kind = match tx.kind {
        TradeKind::Buy => "buy",
        TradeKind::Sell => "sell",
    };

    let value = TradeLog {
        hub: tx.hub.0,
        commodity: tx.com.0,
        kind,
        units: tx.units,
        unit_price_cents: result.unit_price.as_i64(),
        subtotal_cents: result.subtotal.as_i64(),
        fee_cents: result.fee_cents.as_i64(),
        total_cents: result.total_cents.as_i64(),
        wallet_after_cents: wallet_after.as_i64(),
    };

    append_jsonl("trades.jsonl", &value)
}
