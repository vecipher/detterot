use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context;
use repro::Command;
use serde::Serialize;

use crate::systems::director::SpawnBudget;

static LOGS_ENABLED: AtomicBool = AtomicBool::new(cfg!(feature = "m2_logs"));

pub fn set_enabled(enabled: bool) {
    LOGS_ENABLED.store(enabled, Ordering::Relaxed);
}

fn enabled() -> bool {
    LOGS_ENABLED.load(Ordering::Relaxed)
}

fn append_jsonl<T: Serialize>(file: &str, value: &T) -> anyhow::Result<()> {
    const DIR: &str = "logs/m2";
    create_dir_all(DIR).context("creating m2 log directory")?;
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

pub fn log_spawn_budget(
    tick: u32,
    link_id: u16,
    pp: u16,
    weather: &str,
    budget: &SpawnBudget,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct SpawnBudgetLog<'a> {
        tick: u32,
        link_id: u16,
        pp: u16,
        weather: &'a str,
        budget: &'a SpawnBudget,
    }

    let value = SpawnBudgetLog {
        tick,
        link_id,
        pp,
        weather,
        budget,
    };

    append_jsonl("spawn_budget.jsonl", &value)
}

pub fn log_post_leg_summary(
    danger_delta: i32,
    applied_pp_delta: i16,
    applied_basis_overlay: i16,
    di_bp_after: i32,
    basis_bp_after: i32,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct SummaryLog {
        danger_delta: i32,
        applied_pp_delta: i16,
        applied_basis_overlay: i16,
        di_bp_after: i32,
        basis_bp_after: i32,
    }

    let value = SummaryLog {
        danger_delta,
        applied_pp_delta,
        applied_basis_overlay,
        di_bp_after,
        basis_bp_after,
    };

    append_jsonl("post_leg_summary.jsonl", &value)
}

pub fn log_leg_duration_clamped(
    mission_minutes: u32,
    tolerance_ticks: u32,
    attempted_tick: u32,
    clamped_tick: u32,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct DurationClampLog {
        mission_minutes: u32,
        tolerance_ticks: u32,
        attempted_tick: u32,
        clamped_tick: u32,
    }

    let value = DurationClampLog {
        mission_minutes,
        tolerance_ticks,
        attempted_tick,
        clamped_tick,
    };

    append_jsonl("leg_duration_tolerance.jsonl", &value)
}

pub fn log_mission_result(
    name: &str,
    outcome: &str,
    pp_delta: i16,
    basis_bp_overlay: i16,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct MissionLog<'a> {
        name: &'a str,
        outcome: &'a str,
        pp_delta: i16,
        basis_bp_overlay: i16,
    }

    let value = MissionLog {
        name,
        outcome,
        pp_delta,
        basis_bp_overlay,
    };

    append_jsonl("mission_result.jsonl", &value)
}

pub fn log_replay_mismatch(
    tick: u32,
    expected: Option<&Command>,
    actual: Option<&Command>,
) -> anyhow::Result<()> {
    if !enabled() {
        return Ok(());
    }

    #[derive(Serialize)]
    struct ReplayMismatch<'a> {
        tick: u32,
        expected: Option<&'a Command>,
        actual: Option<&'a Command>,
    }

    let value = ReplayMismatch {
        tick,
        expected,
        actual,
    };

    append_jsonl("replay_mismatch.jsonl", &value)
}
