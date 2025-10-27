use repro::DetRng;
use std::fmt;

use crate::systems::command_queue::CommandQueue;

use super::{config::MissionCfg, econ_intent::EconIntent};

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub enum MissionKind {
    RainFlag(RainFlagUplink),
    Sourvault(SourvaultEvac),
    BreakChain(BreakTheChain),
    Wayleave(WayleaveDefault),
    AnchorAudit(AnchorAudit),
}

impl MissionKind {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "rain_flag" | "rain_flag_uplink" => Some(Self::RainFlag(RainFlagUplink::default())),
            "sourvault" | "sourvault_evac" => Some(Self::Sourvault(SourvaultEvac::default())),
            "break_chain" | "break_the_chain" => Some(Self::BreakChain(BreakTheChain::default())),
            "wayleave" | "wayleave_default" => Some(Self::Wayleave(WayleaveDefault::default())),
            "anchor_audit" => Some(Self::AnchorAudit(AnchorAudit::default())),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            MissionKind::RainFlag(_) => "rain_flag",
            MissionKind::Sourvault(_) => "sourvault",
            MissionKind::BreakChain(_) => "break_chain",
            MissionKind::Wayleave(_) => "wayleave",
            MissionKind::AnchorAudit(_) => "anchor_audit",
        }
    }

    pub fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        match self {
            MissionKind::RainFlag(m) => m.init(seed, cfg),
            MissionKind::Sourvault(m) => m.init(seed, cfg),
            MissionKind::BreakChain(m) => m.init(seed, cfg),
            MissionKind::Wayleave(m) => m.init(seed, cfg),
            MissionKind::AnchorAudit(m) => m.init(seed, cfg),
        }
    }

    pub fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        match self {
            MissionKind::RainFlag(m) => m.tick(dt_ticks),
            MissionKind::Sourvault(m) => m.tick(dt_ticks),
            MissionKind::BreakChain(m) => m.tick(dt_ticks),
            MissionKind::Wayleave(m) => m.tick(dt_ticks),
            MissionKind::AnchorAudit(m) => m.tick(dt_ticks),
        }
    }
}

pub fn resolve_result(result: MissionResult, queue: &mut CommandQueue, econ: &mut EconIntent) {
    match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        }
        | MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => {
            econ.apply(pp_delta, basis_bp_overlay);
            queue.meter("pp_delta", pp_delta as i32);
            queue.meter("basis_bp_overlay", basis_bp_overlay as i32);
        }
    }
}

#[derive(Clone)]
pub struct RainFlagUplink {
    segments: Vec<RainSegment>,
    current: usize,
    elapsed: u32,
    fail_limit: u32,
    resolved: bool,
    cfg: MissionCfg,
}

impl Default for RainFlagUplink {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            current: 0,
            elapsed: 0,
            fail_limit: 0,
            resolved: false,
            cfg: MissionCfg::default(),
        }
    }
}

impl fmt::Debug for RainFlagUplink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RainFlagUplink")
            .field("segments", &self.segments.len())
            .field("current", &self.current)
            .field("elapsed", &self.elapsed)
            .field("fail_limit", &self.fail_limit)
            .field("resolved", &self.resolved)
            .finish()
    }
}

#[derive(Debug, Clone)]
struct RainSegment {
    hold_total: u32,
    hold_remaining: u32,
    cooldown_total: u32,
    cooldown_remaining: u32,
}

impl Mission for RainFlagUplink {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        let mut rng = DetRng::from_seed(seed);
        self.segments.clear();
        for _ in 0..3 {
            let hold = 40 + rng.next_u32() % 25;
            let cooldown = 8 + rng.next_u32() % 10;
            self.segments.push(RainSegment {
                hold_total: hold,
                hold_remaining: hold,
                cooldown_total: cooldown,
                cooldown_remaining: 0,
            });
        }
        self.current = 0;
        self.elapsed = 0;
        self.fail_limit = self
            .segments
            .iter()
            .map(|s| s.hold_total + s.cooldown_total)
            .sum::<u32>()
            + 60;
        self.resolved = false;
        self.cfg = cfg.clone();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        if self.current >= self.segments.len() {
            self.resolved = true;
            return Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            });
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed > self.fail_limit {
            self.resolved = true;
            return Some(MissionResult::Fail {
                pp_delta: self.cfg.pp_fail,
                basis_bp_overlay: self.cfg.basis_bp_fail,
            });
        }
        let segment = &mut self.segments[self.current];
        if segment.hold_remaining > 0 {
            segment.hold_remaining = segment.hold_remaining.saturating_sub(dt_ticks);
            if segment.hold_remaining == 0 {
                segment.cooldown_remaining = segment.cooldown_total;
            }
            return None;
        }
        if segment.cooldown_remaining > 0 {
            segment.cooldown_remaining = segment.cooldown_remaining.saturating_sub(dt_ticks);
            if segment.cooldown_remaining == 0 {
                self.current += 1;
            }
            return None;
        }
        None
    }
}

#[derive(Clone)]
pub struct SourvaultEvac {
    remaining: u32,
    hazard_budget: u32,
    hazard_hits: u32,
    hazard_cooldown: u32,
    hazard_timer: u32,
    resolved: bool,
    rng: DetRng,
    cfg: MissionCfg,
}

impl Default for SourvaultEvac {
    fn default() -> Self {
        Self {
            remaining: 0,
            hazard_budget: 0,
            hazard_hits: 0,
            hazard_cooldown: 0,
            hazard_timer: 0,
            resolved: false,
            rng: DetRng::from_seed(0),
            cfg: MissionCfg::default(),
        }
    }
}

impl fmt::Debug for SourvaultEvac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SourvaultEvac")
            .field("remaining", &self.remaining)
            .field("hazard_budget", &self.hazard_budget)
            .field("hazard_hits", &self.hazard_hits)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl Mission for SourvaultEvac {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.rng = DetRng::from_seed(seed);
        self.remaining = 180 + self.rng.next_u32() % 80;
        self.hazard_budget = 2 + self.rng.next_u32() % 3;
        self.hazard_hits = 0;
        self.hazard_cooldown = 12 + self.rng.next_u32() % 10;
        self.hazard_timer = 0;
        self.resolved = false;
        self.cfg = cfg.clone();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        if self.remaining == 0 {
            self.resolved = true;
            return Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            });
        }
        self.remaining = self.remaining.saturating_sub(dt_ticks);
        self.hazard_timer = self.hazard_timer.saturating_add(dt_ticks);
        if self.hazard_timer >= self.hazard_cooldown {
            self.hazard_timer = 0;
            let roll = self.rng.next_u32() % 100;
            if roll < 35 {
                self.hazard_hits += 1;
            }
            if self.hazard_hits > self.hazard_budget {
                self.resolved = true;
                return Some(MissionResult::Fail {
                    pp_delta: self.cfg.pp_fail,
                    basis_bp_overlay: self.cfg.basis_bp_fail,
                });
            }
        }
        if self.remaining == 0 {
            self.resolved = true;
            Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            })
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct BreakTheChain {
    targets: u32,
    destroyed: u32,
    elapsed: u32,
    limit: u32,
    rng: DetRng,
    resolved: bool,
    cfg: MissionCfg,
}

impl Default for BreakTheChain {
    fn default() -> Self {
        Self {
            targets: 0,
            destroyed: 0,
            elapsed: 0,
            limit: 0,
            rng: DetRng::from_seed(0),
            resolved: false,
            cfg: MissionCfg::default(),
        }
    }
}

impl fmt::Debug for BreakTheChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BreakTheChain")
            .field("targets", &self.targets)
            .field("destroyed", &self.destroyed)
            .field("elapsed", &self.elapsed)
            .field("limit", &self.limit)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl Mission for BreakTheChain {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.rng = DetRng::from_seed(seed);
        self.targets = 4 + self.rng.next_u32() % 4;
        self.destroyed = 0;
        self.elapsed = 0;
        self.limit = (20 + self.rng.next_u32() % 15) * self.targets;
        self.resolved = false;
        self.cfg = cfg.clone();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        let roll = self.rng.next_u32() % 100;
        if roll < 45 {
            self.destroyed += 1;
        }
        if roll % 23 == 0 {
            self.destroyed += 1;
        }
        if self.destroyed >= self.targets {
            self.resolved = true;
            return Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            });
        }
        if self.elapsed >= self.limit {
            self.resolved = true;
            Some(MissionResult::Fail {
                pp_delta: self.cfg.pp_fail,
                basis_bp_overlay: self.cfg.basis_bp_fail,
            })
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct WayleaveDefault {
    checkpoints: Vec<u32>,
    current: usize,
    progress: u32,
    elapsed: u32,
    deadline: u32,
    resolved: bool,
    rng: DetRng,
    cfg: MissionCfg,
}

impl Default for WayleaveDefault {
    fn default() -> Self {
        Self {
            checkpoints: Vec::new(),
            current: 0,
            progress: 0,
            elapsed: 0,
            deadline: 0,
            resolved: false,
            rng: DetRng::from_seed(0),
            cfg: MissionCfg::default(),
        }
    }
}

impl fmt::Debug for WayleaveDefault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WayleaveDefault")
            .field("checkpoints", &self.checkpoints.len())
            .field("current", &self.current)
            .field("elapsed", &self.elapsed)
            .field("deadline", &self.deadline)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl Mission for WayleaveDefault {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.rng = DetRng::from_seed(seed);
        self.checkpoints.clear();
        let count = 3 + (self.rng.next_u32() % 2) as usize;
        for _ in 0..count {
            self.checkpoints.push(45 + self.rng.next_u32() % 25);
        }
        self.current = 0;
        self.progress = 0;
        self.elapsed = 0;
        self.deadline = self.checkpoints.iter().sum::<u32>() + 90;
        self.resolved = false;
        self.cfg = cfg.clone();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed > self.deadline {
            self.resolved = true;
            return Some(MissionResult::Fail {
                pp_delta: self.cfg.pp_fail,
                basis_bp_overlay: self.cfg.basis_bp_fail,
            });
        }
        if self.current >= self.checkpoints.len() {
            self.resolved = true;
            return Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            });
        }
        self.progress = self.progress.saturating_add(dt_ticks);
        if self.progress >= self.checkpoints[self.current] {
            self.progress = 0;
            self.current += 1;
            if self.current >= self.checkpoints.len() {
                self.resolved = true;
                return Some(MissionResult::Success {
                    pp_delta: self.cfg.pp_success,
                    basis_bp_overlay: self.cfg.basis_bp_success,
                });
            }
        }
        None
    }
}

#[derive(Clone)]
pub struct AnchorAudit {
    hold_required: u32,
    elapsed: u32,
    hazard_spacing: u32,
    hazard_timer: u32,
    hazard_budget: u32,
    hazard_hits: u32,
    resolved: bool,
    rng: DetRng,
    cfg: MissionCfg,
}

impl Default for AnchorAudit {
    fn default() -> Self {
        Self {
            hold_required: 0,
            elapsed: 0,
            hazard_spacing: 0,
            hazard_timer: 0,
            hazard_budget: 0,
            hazard_hits: 0,
            resolved: false,
            rng: DetRng::from_seed(0),
            cfg: MissionCfg::default(),
        }
    }
}

impl fmt::Debug for AnchorAudit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnchorAudit")
            .field("hold_required", &self.hold_required)
            .field("elapsed", &self.elapsed)
            .field("hazard_spacing", &self.hazard_spacing)
            .field("hazard_budget", &self.hazard_budget)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl Mission for AnchorAudit {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.rng = DetRng::from_seed(seed);
        self.hold_required = 110 + self.rng.next_u32() % 50;
        self.elapsed = 0;
        self.hazard_spacing = 18 + self.rng.next_u32() % 10;
        self.hazard_timer = 0;
        self.hazard_budget = 1 + self.rng.next_u32() % 2;
        self.hazard_hits = 0;
        self.resolved = false;
        self.cfg = cfg.clone();
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        if self.elapsed >= self.hold_required {
            self.resolved = true;
            return Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            });
        }
        self.hazard_timer = self.hazard_timer.saturating_add(dt_ticks);
        if self.hazard_timer >= self.hazard_spacing {
            self.hazard_timer = 0;
            if self.rng.next_u32() % 5 == 0 {
                self.hazard_hits += 1;
            }
            if self.hazard_hits > self.hazard_budget {
                self.resolved = true;
                return Some(MissionResult::Fail {
                    pp_delta: self.cfg.pp_fail,
                    basis_bp_overlay: self.cfg.basis_bp_fail,
                });
            }
        }
        if self.elapsed > self.hold_required + 60 {
            self.resolved = true;
            Some(MissionResult::Fail {
                pp_delta: self.cfg.pp_fail,
                basis_bp_overlay: self.cfg.basis_bp_fail,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systems::director::config::MissionCfg;

    fn cfg() -> MissionCfg {
        MissionCfg {
            pp_success: -3,
            pp_fail: 5,
            basis_bp_success: -10,
            basis_bp_fail: 8,
        }
    }

    #[test]
    fn rain_flag_deterministic() {
        let mut mission = RainFlagUplink::default();
        mission.init(42, &cfg());
        let mut ticks = 0;
        let mut result = None;
        while ticks < 400 {
            if let Some(res) = mission.tick(1) {
                result = Some(res);
                break;
            }
            ticks += 1;
        }
        assert!(result.is_some());
    }

    #[test]
    fn factory_from_name() {
        assert!(matches!(
            MissionKind::from_name("rain_flag"),
            Some(MissionKind::RainFlag(_))
        ));
        assert!(MissionKind::from_name("unknown").is_none());
    }
}
