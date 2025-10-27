use std::sync::atomic::{AtomicBool, Ordering};

use crate::systems::director::spawn::SpawnBudget;
use crate::systems::economy::{Pp, RouteId, Weather};

static DEBUG_LOGS: AtomicBool = AtomicBool::new(false);

fn logs_enabled() -> bool {
    cfg!(feature = "m2_logs") || DEBUG_LOGS.load(Ordering::Relaxed)
}

pub fn enable_debug_logs() {
    DEBUG_LOGS.store(true, Ordering::Relaxed);
}

fn append_line(path: &str, value: serde_json::Value) {
    if !logs_enabled() {
        return;
    }
    if std::fs::create_dir_all("logs/m2").is_err() {
        return;
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{}", value);
    }
}

pub fn log_spawn_budget(
    tick: u32,
    link_id: RouteId,
    pp: Pp,
    weather: Weather,
    budget: SpawnBudget,
) {
    append_line(
        "logs/m2/spawn_budget.jsonl",
        serde_json::json!({
            "tick": tick,
            "link_id": link_id.0,
            "pp": pp.0,
            "weather": format!("{:?}", weather),
            "budget": {"enemies": budget.enemies, "obstacles": budget.obstacles},
        }),
    );
}

pub fn log_mission_result(name: &str, outcome: &str, pp_delta: i16, basis: i16, tick: u32) {
    append_line(
        "logs/m2/mission_result.jsonl",
        serde_json::json!({
            "tick": tick,
            "name": name,
            "outcome": outcome,
            "pp_delta": pp_delta,
            "basis_bp_overlay": basis,
        }),
    );
}

pub fn log_post_leg_summary(
    tick: u32,
    danger_delta: i32,
    applied_pp_delta: i16,
    applied_basis_overlay: i16,
    di_bp_after: i32,
    basis_bp_after: i32,
) {
    append_line(
        "logs/m2/post_leg_summary.jsonl",
        serde_json::json!({
            "tick": tick,
            "danger_delta": danger_delta,
            "applied_pp_delta": applied_pp_delta,
            "applied_basis_overlay": applied_basis_overlay,
            "di_bp_after": di_bp_after,
            "basis_bp_after": basis_bp_after,
        }),
    );
}

pub fn log_replay_mismatch(index: usize, expected: &repro::Command, actual: &repro::Command) {
    append_line(
        "logs/m2/replay_mismatch.jsonl",
        serde_json::json!({
            "index": index,
            "expected": command_value(expected),
            "actual": command_value(actual),
        }),
    );
}

fn command_value(command: &repro::Command) -> serde_json::Value {
    match &command.kind {
        repro::CommandKind::Spawn {
            kind,
            x_mm,
            y_mm,
            z_mm,
        } => serde_json::json!({
            "t": command.t,
            "Spawn": {
                "kind": kind,
                "x_mm": x_mm,
                "y_mm": y_mm,
                "z_mm": z_mm,
            }
        }),
        repro::CommandKind::Meter { key, value } => serde_json::json!({
            "t": command.t,
            "Meter": {"key": key, "value": value},
        }),
    }
}
