use bevy::prelude::Resource;

use super::config::MissionCfg;
use super::econ_intent::EconIntent;
use super::rng::{hash_mission_name, mission_seed, DetRng};
use crate::logs::m2;
use crate::systems::command_queue::CommandQueue;
use crate::systems::economy::RouteId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionResult {
    Success {
        pp_delta: i16,
        basis_bp_overlay: i16,
    },
    Fail {
        pp_delta: i16,
        basis_bp_overlay: i16,
    },
}

pub trait Mission {
    fn init(&mut self, seed: u64, cfg: &MissionCfg);
    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult>;
}

fn success_result(cfg: &MissionCfg) -> MissionResult {
    MissionResult::Success {
        pp_delta: cfg.pp_success,
        basis_bp_overlay: cfg.basis_bp_success,
    }
}

fn fail_result(cfg: &MissionCfg) -> MissionResult {
    MissionResult::Fail {
        pp_delta: cfg.pp_fail,
        basis_bp_overlay: cfg.basis_bp_fail,
    }
}

#[derive(Default)]
pub struct RainFlagUplink {
    cfg: MissionCfg,
    resolve_at: u32,
    elapsed: u32,
    success: bool,
    done: bool,
}

impl Mission for RainFlagUplink {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = cfg.clone();
        let mut rng = DetRng::from_seed(seed);
        self.resolve_at = 90 + rng.range_u32(0, 30);
        self.elapsed = 0;
        self.success = rng.next_bool();
        self.done = false;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.done {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed >= self.resolve_at {
            self.done = true;
            if self.success {
                Some(success_result(&self.cfg))
            } else {
                Some(fail_result(&self.cfg))
            }
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct SourvaultEvac {
    cfg: MissionCfg,
    hazard_budget: u32,
    elapsed: u32,
    done: bool,
    success: bool,
}

impl Mission for SourvaultEvac {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = cfg.clone();
        let mut rng = DetRng::from_seed(seed);
        self.hazard_budget = 120 + rng.range_u32(0, 60);
        self.elapsed = 0;
        self.done = false;
        self.success = rng.next_bool();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.done {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed >= self.hazard_budget {
            self.done = true;
            if self.success {
                Some(success_result(&self.cfg))
            } else {
                Some(fail_result(&self.cfg))
            }
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct BreakTheChain {
    cfg: MissionCfg,
    targets: u32,
    destroyed: u32,
    done: bool,
}

impl Mission for BreakTheChain {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = cfg.clone();
        let mut rng = DetRng::from_seed(seed);
        self.targets = 3 + rng.range_u32(0, 4);
        self.destroyed = 0;
        self.done = false;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.done {
            return None;
        }
        self.destroyed = (self.destroyed + dt_ticks.min(1)).min(self.targets);
        if self.destroyed >= self.targets {
            self.done = true;
            Some(success_result(&self.cfg))
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct WayleaveDefault {
    cfg: MissionCfg,
    checkpoints: u32,
    reached: u32,
    deadline: u32,
    elapsed: u32,
    done: bool,
}

impl Mission for WayleaveDefault {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = cfg.clone();
        let mut rng = DetRng::from_seed(seed);
        self.checkpoints = 2 + rng.range_u32(0, 3);
        self.deadline = 150 + rng.range_u32(0, 50);
        self.reached = 0;
        self.elapsed = 0;
        self.done = false;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.done {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed.is_multiple_of(40) && self.reached < self.checkpoints {
            self.reached += 1;
        }
        if self.reached >= self.checkpoints {
            self.done = true;
            Some(success_result(&self.cfg))
        } else if self.elapsed >= self.deadline {
            self.done = true;
            Some(fail_result(&self.cfg))
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct AnchorAudit {
    cfg: MissionCfg,
    scan_ticks: u32,
    elapsed: u32,
    done: bool,
    success: bool,
}

impl Mission for AnchorAudit {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = cfg.clone();
        let mut rng = DetRng::from_seed(seed);
        self.scan_ticks = 100 + rng.range_u32(0, 25);
        self.elapsed = 0;
        self.done = false;
        self.success = rng.next_bool();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.done {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed >= self.scan_ticks {
            self.done = true;
            if self.success {
                Some(success_result(&self.cfg))
            } else {
                Some(fail_result(&self.cfg))
            }
        } else {
            None
        }
    }
}

#[derive(Resource, Default)]
pub struct MissionRuntime {
    pub rain_flag: RainFlagUplink,
    pub sourvault: SourvaultEvac,
    pub break_chain: BreakTheChain,
    pub wayleave: WayleaveDefault,
    pub anchor_audit: AnchorAudit,
}

impl MissionRuntime {
    pub fn init_all(
        &mut self,
        world_seed: u64,
        link_id: RouteId,
        day: u32,
        cfgs: &[(String, MissionCfg)],
    ) {
        for (name, cfg) in cfgs.iter() {
            let mission_id = hash_mission_name(name);
            let seed = mission_seed(world_seed, link_id, day, mission_id);
            match name.as_str() {
                "rain_flag" => self.rain_flag.init(seed, cfg),
                "sourvault" => self.sourvault.init(seed, cfg),
                "break_chain" => self.break_chain.init(seed, cfg),
                "wayleave" => self.wayleave.init(seed, cfg),
                "anchor_audit" => self.anchor_audit.init(seed, cfg),
                _ => {}
            }
        }
    }

    pub fn tick_all(
        &mut self,
        current_tick: u32,
        dt_ticks: u32,
        queue: &mut CommandQueue,
        econ: &mut EconIntent,
    ) {
        let missions = [
            ("rain_flag", self.rain_flag.tick(dt_ticks)),
            ("sourvault", self.sourvault.tick(dt_ticks)),
            ("break_chain", self.break_chain.tick(dt_ticks)),
            ("wayleave", self.wayleave.tick(dt_ticks)),
            ("anchor_audit", self.anchor_audit.tick(dt_ticks)),
        ];
        for (name, result) in missions {
            if let Some(outcome) = result {
                let mission_hash = hash_mission_name(name);
                let mission_key = (mission_hash & 0x7FFF_FFFF) as i32;
                let (pp_delta, basis_bp_overlay, success_flag) = match outcome {
                    MissionResult::Success {
                        pp_delta,
                        basis_bp_overlay,
                    } => (pp_delta, basis_bp_overlay, 1),
                    MissionResult::Fail {
                        pp_delta,
                        basis_bp_overlay,
                    } => (pp_delta, basis_bp_overlay, 0),
                };

                econ.pending_pp_delta += pp_delta;
                econ.pending_basis_overlay_bp += basis_bp_overlay;
                queue.meter("pp_delta", pp_delta as i32);
                queue.meter("basis_bp_overlay", basis_bp_overlay as i32);
                queue.meter("mission_result", success_flag);
                queue.meter("mission_id", mission_key);
                queue.meter("mission_resolve_tick", current_tick as i32);
                let outcome_label = if success_flag == 1 { "Success" } else { "Fail" };
                let _ = m2::log_mission_result(name, outcome_label, pp_delta, basis_bp_overlay);
            }
        }
    }
}
