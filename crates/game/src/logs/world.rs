use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use serde::Serialize;

static LOGS_ENABLED: AtomicBool = AtomicBool::new(cfg!(feature = "m2_logs"));

pub fn set_enabled(enabled: bool) {
    LOGS_ENABLED.store(enabled, Ordering::Relaxed);
}

fn enabled() -> bool {
    LOGS_ENABLED.load(Ordering::Relaxed)
}

fn append_jsonl<T: Serialize>(file: &str, value: &T) -> anyhow::Result<()> {
    const DIR: &str = "logs/world";
    create_dir_all(DIR).context("creating world log directory")?;
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

#[derive(Serialize)]
pub struct WeatherLogData {
    pub route_id: u16,
    pub weather: String,
    pub los_m: u32,
    pub drift_mm: u32,
    pub aggression_pct: i32,
    pub timestamp: u64,
}

pub fn log_weather_state(data: &WeatherLogData) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }
    append_jsonl("weather.jsonl", data)
}

#[derive(Serialize)]
pub struct BoardGenLogData {
    pub link_id: u16,
    pub style: String,
    pub board_hash: u64,
    pub timestamp: u64,
}

pub fn log_board_gen(data: &BoardGenLogData) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }
    append_jsonl("board_gen.jsonl", data)
}
