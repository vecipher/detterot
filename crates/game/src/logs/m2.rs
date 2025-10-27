use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use repro::Command;
use serde::Serialize;

#[derive(Serialize)]
pub struct SpawnBudgetLog<'a> {
    pub tick: u32,
    pub link_id: &'a str,
    pub pp: i32,
    pub weather: &'a str,
    pub budget: Budget<'a>,
}

#[derive(Serialize)]
pub struct Budget<'a> {
    pub enemies: u32,
    pub obstacles: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<&'a str>,
}

#[derive(Serialize)]
pub struct MissionResultLog<'a> {
    pub name: &'a str,
    pub outcome: &'a str,
    pub pp_delta: i16,
    pub basis_bp_overlay: i16,
}

#[derive(Serialize)]
pub struct PostLegSummaryLog {
    pub danger_delta: i32,
    pub applied_pp_delta: i16,
    pub applied_basis_overlay: i16,
    pub di_bp_after: i16,
    pub basis_bp_after: i16,
}

#[derive(Serialize)]
pub struct ReplayMismatchLog {
    pub tick: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<Command>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub got: Option<Command>,
}

fn ensure_log_dir() -> Result<PathBuf> {
    let path = PathBuf::from("logs/m2");
    if !path.exists() {
        fs::create_dir_all(&path).context("failed to create logs/m2 directory")?;
    }
    Ok(path)
}

fn append_json_line<T: Serialize>(file: &str, payload: &T) -> Result<()> {
    let dir = ensure_log_dir()?;
    let path = dir.join(file);
    let mut handle = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("unable to open log {}", path.display()))?;
    let line = serde_json::to_string(payload)?;
    handle
        .write_all(line.as_bytes())
        .and_then(|_| handle.write_all(b"\n"))
        .with_context(|| format!("unable to write log {}", path.display()))?;
    Ok(())
}

pub fn write_spawn_budget(log: SpawnBudgetLog<'_>) -> Result<()> {
    append_json_line("spawn_budget.jsonl", &log)
}

pub fn write_mission_result(log: MissionResultLog<'_>) -> Result<()> {
    append_json_line("mission_result.jsonl", &log)
}

pub fn write_post_leg_summary(log: PostLegSummaryLog) -> Result<()> {
    append_json_line("post_leg_summary.jsonl", &log)
}

pub fn write_replay_mismatch(log: ReplayMismatchLog) -> Result<()> {
    append_json_line("replay_mismatch.jsonl", &log)
}
