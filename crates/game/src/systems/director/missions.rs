use std::cmp::Ordering;

use crate::systems::director::config::MissionCfg;

use super::spawn::DetRng;

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

#[derive(Debug, Clone)]
pub struct RainFlag {
    cfg: Option<MissionCfg>,
    rng: Option<DetRng>,
    state: RainFlagStage,
    elapsed: u32,
    signal_score: i32,
    counter_score: i32,
    pressure: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RainFlagStage {
    Boot,
    Deploy,
    Response,
    Handoff,
    Cleanup,
    Resolve,
    Done,
}

impl Default for RainFlag {
    fn default() -> Self {
        Self {
            cfg: None,
            rng: None,
            state: RainFlagStage::Boot,
            elapsed: 0,
            signal_score: 0,
            counter_score: 0,
            pressure: 0,
        }
    }
}

impl RainFlag {
    const BOOT_DURATION: u32 = 15;
    const DEPLOY_DURATION: u32 = 30;
    const RESPONSE_DURATION: u32 = 25;
    const HANDOFF_DURATION: u32 = 20;
    const CLEANUP_DURATION: u32 = 18;

    fn stage_duration(stage: RainFlagStage) -> Option<u32> {
        match stage {
            RainFlagStage::Boot => Some(Self::BOOT_DURATION),
            RainFlagStage::Deploy => Some(Self::DEPLOY_DURATION),
            RainFlagStage::Response => Some(Self::RESPONSE_DURATION),
            RainFlagStage::Handoff => Some(Self::HANDOFF_DURATION),
            RainFlagStage::Cleanup => Some(Self::CLEANUP_DURATION),
            RainFlagStage::Resolve | RainFlagStage::Done => None,
        }
    }

    fn advance_stage(&mut self) {
        self.state = match self.state {
            RainFlagStage::Boot => RainFlagStage::Deploy,
            RainFlagStage::Deploy => {
                self.signal_score += 4;
                RainFlagStage::Response
            }
            RainFlagStage::Response => {
                let early = self.pressure.min(2);
                self.counter_score += 3 + early;
                RainFlagStage::Handoff
            }
            RainFlagStage::Handoff => {
                self.signal_score += 5 - self.pressure;
                RainFlagStage::Cleanup
            }
            RainFlagStage::Cleanup => {
                let early = self.pressure.min(2);
                self.counter_score += 2 + (self.pressure - early);
                RainFlagStage::Resolve
            }
            RainFlagStage::Resolve | RainFlagStage::Done => RainFlagStage::Done,
        };
    }

    fn resolve(&mut self) -> MissionResult {
        let cfg = self.cfg.as_ref().expect("mission not initialised");
        let success = match self.signal_score.cmp(&self.counter_score) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                let rng = self.rng.as_mut().expect("mission rng not initialised");
                rng.next_u32() & 1 == 0
            }
        };
        if success {
            MissionResult::Success {
                pp_delta: cfg.pp_success,
                basis_bp_overlay: cfg.basis_bp_success,
            }
        } else {
            MissionResult::Fail {
                pp_delta: cfg.pp_fail,
                basis_bp_overlay: cfg.basis_bp_fail,
            }
        }
    }
}

impl Mission for RainFlag {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = Some(cfg.clone());
        self.rng = Some(DetRng::new(seed));
        self.state = RainFlagStage::Boot;
        self.elapsed = 0;
        self.signal_score = 0;
        self.counter_score = 0;
        self.pressure = (seed % 5) as i32;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if matches!(self.state, RainFlagStage::Done) {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        loop {
            match self.state {
                RainFlagStage::Resolve => {
                    let result = self.resolve();
                    self.state = RainFlagStage::Done;
                    self.elapsed = 0;
                    return Some(result);
                }
                RainFlagStage::Done => return None,
                _ => {
                    if let Some(duration) = Self::stage_duration(self.state) {
                        if self.elapsed < duration {
                            break;
                        }
                        self.elapsed -= duration;
                        self.advance_stage();
                        continue;
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Sourvault {
    cfg: Option<MissionCfg>,
    rng: Option<DetRng>,
    state: SourvaultStage,
    elapsed: u32,
    evac_score: i32,
    hazard_score: i32,
    evacuation_pressure: i32,
    stubborn_groups: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourvaultStage {
    Alert,
    Vent,
    Escort,
    Collapse,
    Extraction,
    Resolve,
    Done,
}

impl Default for Sourvault {
    fn default() -> Self {
        Self {
            cfg: None,
            rng: None,
            state: SourvaultStage::Alert,
            elapsed: 0,
            evac_score: 0,
            hazard_score: 0,
            evacuation_pressure: 0,
            stubborn_groups: 0,
        }
    }
}

impl Sourvault {
    const ALERT_DURATION: u32 = 24;
    const VENT_DURATION: u32 = 36;
    const ESCORT_DURATION: u32 = 28;
    const COLLAPSE_DURATION: u32 = 32;
    const EXTRACTION_DURATION: u32 = 30;

    fn stage_duration(stage: SourvaultStage) -> Option<u32> {
        match stage {
            SourvaultStage::Alert => Some(Self::ALERT_DURATION),
            SourvaultStage::Vent => Some(Self::VENT_DURATION),
            SourvaultStage::Escort => Some(Self::ESCORT_DURATION),
            SourvaultStage::Collapse => Some(Self::COLLAPSE_DURATION),
            SourvaultStage::Extraction => Some(Self::EXTRACTION_DURATION),
            SourvaultStage::Resolve | SourvaultStage::Done => None,
        }
    }

    fn advance_stage(&mut self) {
        self.state = match self.state {
            SourvaultStage::Alert => {
                self.evac_score += 3;
                SourvaultStage::Vent
            }
            SourvaultStage::Vent => {
                self.hazard_score += 3 + self.evacuation_pressure;
                SourvaultStage::Escort
            }
            SourvaultStage::Escort => {
                self.evac_score += 5 - self.stubborn_groups;
                SourvaultStage::Collapse
            }
            SourvaultStage::Collapse => {
                let stubborn = self.stubborn_groups;
                self.hazard_score += 2 + (self.evacuation_pressure - stubborn);
                SourvaultStage::Extraction
            }
            SourvaultStage::Extraction => {
                self.evac_score += 4 - (self.evacuation_pressure / 2);
                SourvaultStage::Resolve
            }
            SourvaultStage::Resolve | SourvaultStage::Done => SourvaultStage::Done,
        };
    }

    fn resolve(&mut self) -> MissionResult {
        let cfg = self.cfg.as_ref().expect("mission not initialised");
        let success = match self.evac_score.cmp(&self.hazard_score) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                let rng = self.rng.as_mut().expect("mission rng not initialised");
                rng.next_u32() & 1 == 0
            }
        };
        if success {
            MissionResult::Success {
                pp_delta: cfg.pp_success,
                basis_bp_overlay: cfg.basis_bp_success,
            }
        } else {
            MissionResult::Fail {
                pp_delta: cfg.pp_fail,
                basis_bp_overlay: cfg.basis_bp_fail,
            }
        }
    }
}

impl Mission for Sourvault {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = Some(cfg.clone());
        self.rng = Some(DetRng::new(seed));
        self.state = SourvaultStage::Alert;
        self.elapsed = 0;
        self.evac_score = 0;
        self.hazard_score = 0;
        self.evacuation_pressure = (seed % 6) as i32;
        self.stubborn_groups = self.evacuation_pressure.min(3);
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if matches!(self.state, SourvaultStage::Done) {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        loop {
            match self.state {
                SourvaultStage::Resolve => {
                    let result = self.resolve();
                    self.state = SourvaultStage::Done;
                    self.elapsed = 0;
                    return Some(result);
                }
                SourvaultStage::Done => return None,
                _ => {
                    if let Some(duration) = Self::stage_duration(self.state) {
                        if self.elapsed < duration {
                            break;
                        }
                        self.elapsed -= duration;
                        self.advance_stage();
                        continue;
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct BreakTheChain {
    cfg: Option<MissionCfg>,
    rng: Option<DetRng>,
    state: BreakTheChainStage,
    elapsed: u32,
    disruption_score: i32,
    crackdown_score: i32,
    network_density: i32,
    exposure: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BreakTheChainStage {
    Infiltration,
    Detection,
    Sabotage,
    Crackdown,
    Sweep,
    Resolve,
    Done,
}

impl Default for BreakTheChain {
    fn default() -> Self {
        Self {
            cfg: None,
            rng: None,
            state: BreakTheChainStage::Infiltration,
            elapsed: 0,
            disruption_score: 0,
            crackdown_score: 0,
            network_density: 0,
            exposure: 0,
        }
    }
}

impl BreakTheChain {
    const INFILTRATION_DURATION: u32 = 32;
    const DETECTION_DURATION: u32 = 38;
    const SABOTAGE_DURATION: u32 = 36;
    const CRACKDOWN_DURATION: u32 = 34;
    const SWEEP_DURATION: u32 = 30;

    fn stage_duration(stage: BreakTheChainStage) -> Option<u32> {
        match stage {
            BreakTheChainStage::Infiltration => Some(Self::INFILTRATION_DURATION),
            BreakTheChainStage::Detection => Some(Self::DETECTION_DURATION),
            BreakTheChainStage::Sabotage => Some(Self::SABOTAGE_DURATION),
            BreakTheChainStage::Crackdown => Some(Self::CRACKDOWN_DURATION),
            BreakTheChainStage::Sweep => Some(Self::SWEEP_DURATION),
            BreakTheChainStage::Resolve | BreakTheChainStage::Done => None,
        }
    }

    fn advance_stage(&mut self) {
        self.state = match self.state {
            BreakTheChainStage::Infiltration => {
                self.disruption_score += 4 + self.exposure / 2;
                BreakTheChainStage::Detection
            }
            BreakTheChainStage::Detection => {
                self.crackdown_score += 3 + self.network_density;
                BreakTheChainStage::Sabotage
            }
            BreakTheChainStage::Sabotage => {
                self.disruption_score += 5 - (self.network_density / 2);
                BreakTheChainStage::Crackdown
            }
            BreakTheChainStage::Crackdown => {
                self.crackdown_score += 3 + self.exposure;
                BreakTheChainStage::Sweep
            }
            BreakTheChainStage::Sweep => {
                self.disruption_score += 4 - (self.network_density / 3);
                BreakTheChainStage::Resolve
            }
            BreakTheChainStage::Resolve | BreakTheChainStage::Done => BreakTheChainStage::Done,
        };
    }

    fn resolve(&mut self) -> MissionResult {
        let cfg = self.cfg.as_ref().expect("mission not initialised");
        let success = match self.disruption_score.cmp(&self.crackdown_score) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                let rng = self.rng.as_mut().expect("mission rng not initialised");
                rng.next_u32() & 1 == 0
            }
        };
        if success {
            MissionResult::Success {
                pp_delta: cfg.pp_success,
                basis_bp_overlay: cfg.basis_bp_success,
            }
        } else {
            MissionResult::Fail {
                pp_delta: cfg.pp_fail,
                basis_bp_overlay: cfg.basis_bp_fail,
            }
        }
    }
}

impl Mission for BreakTheChain {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = Some(cfg.clone());
        self.rng = Some(DetRng::new(seed));
        self.state = BreakTheChainStage::Infiltration;
        self.elapsed = 0;
        self.disruption_score = 0;
        self.crackdown_score = 0;
        self.network_density = (seed % 8) as i32;
        self.exposure = self.network_density.min(4);
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if matches!(self.state, BreakTheChainStage::Done) {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        loop {
            match self.state {
                BreakTheChainStage::Resolve => {
                    let result = self.resolve();
                    self.state = BreakTheChainStage::Done;
                    self.elapsed = 0;
                    return Some(result);
                }
                BreakTheChainStage::Done => return None,
                _ => {
                    if let Some(duration) = Self::stage_duration(self.state) {
                        if self.elapsed < duration {
                            break;
                        }
                        self.elapsed -= duration;
                        self.advance_stage();
                        continue;
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Wayleave {
    cfg: Option<MissionCfg>,
    rng: Option<DetRng>,
    state: WayleaveStage,
    elapsed: u32,
    mandate_score: i32,
    opposition_score: i32,
    petition_weight: i32,
    success_stage1: i32,
    success_stage3: i32,
    success_stage5: i32,
    risk_stage2: i32,
    risk_stage4: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WayleaveStage {
    Survey,
    Lobby,
    Draft,
    Committee,
    Ratify,
    Resolve,
    Done,
}

impl Default for Wayleave {
    fn default() -> Self {
        Self {
            cfg: None,
            rng: None,
            state: WayleaveStage::Survey,
            elapsed: 0,
            mandate_score: 0,
            opposition_score: 0,
            petition_weight: 0,
            success_stage1: 0,
            success_stage3: 0,
            success_stage5: 0,
            risk_stage2: 0,
            risk_stage4: 0,
        }
    }
}

impl Wayleave {
    const SURVEY_DURATION: u32 = 26;
    const LOBBY_DURATION: u32 = 34;
    const DRAFT_DURATION: u32 = 36;
    const COMMITTEE_DURATION: u32 = 30;
    const RATIFY_DURATION: u32 = 32;

    fn stage_duration(stage: WayleaveStage) -> Option<u32> {
        match stage {
            WayleaveStage::Survey => Some(Self::SURVEY_DURATION),
            WayleaveStage::Lobby => Some(Self::LOBBY_DURATION),
            WayleaveStage::Draft => Some(Self::DRAFT_DURATION),
            WayleaveStage::Committee => Some(Self::COMMITTEE_DURATION),
            WayleaveStage::Ratify => Some(Self::RATIFY_DURATION),
            WayleaveStage::Resolve | WayleaveStage::Done => None,
        }
    }

    fn advance_stage(&mut self) {
        self.state = match self.state {
            WayleaveStage::Survey => {
                self.mandate_score += self.success_stage1;
                WayleaveStage::Lobby
            }
            WayleaveStage::Lobby => {
                self.opposition_score += self.risk_stage2;
                WayleaveStage::Draft
            }
            WayleaveStage::Draft => {
                self.mandate_score += self.success_stage3;
                WayleaveStage::Committee
            }
            WayleaveStage::Committee => {
                self.opposition_score += self.risk_stage4;
                WayleaveStage::Ratify
            }
            WayleaveStage::Ratify => {
                self.mandate_score += self.success_stage5;
                WayleaveStage::Resolve
            }
            WayleaveStage::Resolve | WayleaveStage::Done => WayleaveStage::Done,
        };
    }

    fn resolve(&mut self) -> MissionResult {
        let cfg = self.cfg.as_ref().expect("mission not initialised");
        let success = match self.mandate_score.cmp(&self.opposition_score) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                let rng = self.rng.as_mut().expect("mission rng not initialised");
                rng.next_u32() & 1 == 0
            }
        };
        if success {
            MissionResult::Success {
                pp_delta: cfg.pp_success,
                basis_bp_overlay: cfg.basis_bp_success,
            }
        } else {
            MissionResult::Fail {
                pp_delta: cfg.pp_fail,
                basis_bp_overlay: cfg.basis_bp_fail,
            }
        }
    }
}

impl Mission for Wayleave {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = Some(cfg.clone());
        self.rng = Some(DetRng::new(seed));
        self.state = WayleaveStage::Survey;
        self.elapsed = 0;
        self.mandate_score = 0;
        self.opposition_score = 0;
        self.petition_weight = (seed % 9) as i32;

        let target_success = 10 - (self.petition_weight / 2);
        let target_risk = 6 + (self.petition_weight / 2) + (self.petition_weight % 2);

        self.success_stage1 = 2 + (self.petition_weight % 2);
        self.success_stage3 = 2 + (self.petition_weight / 4);
        self.success_stage5 = target_success - (self.success_stage1 + self.success_stage3);
        self.risk_stage2 = 2 + (self.petition_weight % 2);
        self.risk_stage4 = target_risk - self.risk_stage2;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if matches!(self.state, WayleaveStage::Done) {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        loop {
            match self.state {
                WayleaveStage::Resolve => {
                    let result = self.resolve();
                    self.state = WayleaveStage::Done;
                    self.elapsed = 0;
                    return Some(result);
                }
                WayleaveStage::Done => return None,
                _ => {
                    if let Some(duration) = Self::stage_duration(self.state) {
                        if self.elapsed < duration {
                            break;
                        }
                        self.elapsed -= duration;
                        self.advance_stage();
                        continue;
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct AnchorAudit {
    cfg: Option<MissionCfg>,
    rng: Option<DetRng>,
    state: AnchorAuditStage,
    elapsed: u32,
    anchor_score: i32,
    drift_score: i32,
    drift: i32,
    success_stage1: i32,
    success_stage3: i32,
    success_stage5: i32,
    risk_stage2: i32,
    risk_stage4: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnchorAuditStage {
    Snapshot,
    Trace,
    Reconcile,
    Audit,
    Publish,
    Resolve,
    Done,
}

impl Default for AnchorAudit {
    fn default() -> Self {
        Self {
            cfg: None,
            rng: None,
            state: AnchorAuditStage::Snapshot,
            elapsed: 0,
            anchor_score: 0,
            drift_score: 0,
            drift: 0,
            success_stage1: 0,
            success_stage3: 0,
            success_stage5: 0,
            risk_stage2: 0,
            risk_stage4: 0,
        }
    }
}

impl AnchorAudit {
    const SNAPSHOT_DURATION: u32 = 18;
    const TRACE_DURATION: u32 = 20;
    const RECONCILE_DURATION: u32 = 22;
    const AUDIT_DURATION: u32 = 20;
    const PUBLISH_DURATION: u32 = 18;

    fn stage_duration(stage: AnchorAuditStage) -> Option<u32> {
        match stage {
            AnchorAuditStage::Snapshot => Some(Self::SNAPSHOT_DURATION),
            AnchorAuditStage::Trace => Some(Self::TRACE_DURATION),
            AnchorAuditStage::Reconcile => Some(Self::RECONCILE_DURATION),
            AnchorAuditStage::Audit => Some(Self::AUDIT_DURATION),
            AnchorAuditStage::Publish => Some(Self::PUBLISH_DURATION),
            AnchorAuditStage::Resolve | AnchorAuditStage::Done => None,
        }
    }

    fn advance_stage(&mut self) {
        self.state = match self.state {
            AnchorAuditStage::Snapshot => {
                self.anchor_score += self.success_stage1;
                AnchorAuditStage::Trace
            }
            AnchorAuditStage::Trace => {
                self.drift_score += self.risk_stage2;
                AnchorAuditStage::Reconcile
            }
            AnchorAuditStage::Reconcile => {
                self.anchor_score += self.success_stage3;
                AnchorAuditStage::Audit
            }
            AnchorAuditStage::Audit => {
                self.drift_score += self.risk_stage4;
                AnchorAuditStage::Publish
            }
            AnchorAuditStage::Publish => {
                self.anchor_score += self.success_stage5;
                AnchorAuditStage::Resolve
            }
            AnchorAuditStage::Resolve | AnchorAuditStage::Done => AnchorAuditStage::Done,
        };
    }

    fn resolve(&mut self) -> MissionResult {
        let cfg = self.cfg.as_ref().expect("mission not initialised");
        let success = match self.anchor_score.cmp(&self.drift_score) {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => {
                let rng = self.rng.as_mut().expect("mission rng not initialised");
                rng.next_u32() & 1 == 0
            }
        };
        if success {
            MissionResult::Success {
                pp_delta: cfg.pp_success,
                basis_bp_overlay: cfg.basis_bp_success,
            }
        } else {
            MissionResult::Fail {
                pp_delta: cfg.pp_fail,
                basis_bp_overlay: cfg.basis_bp_fail,
            }
        }
    }
}

impl Mission for AnchorAudit {
    fn init(&mut self, seed: u64, cfg: &MissionCfg) {
        self.cfg = Some(cfg.clone());
        self.rng = Some(DetRng::new(seed));
        self.state = AnchorAuditStage::Snapshot;
        self.elapsed = 0;
        self.anchor_score = 0;
        self.drift_score = 0;
        self.drift = (seed % 6) as i32;

        let target_success = 8 - (self.drift / 2);
        let target_risk = 4 + self.drift;

        self.success_stage1 = 3;
        self.success_stage3 = 3 - (self.drift / 3);
        self.success_stage5 = target_success - (self.success_stage1 + self.success_stage3);
        self.risk_stage2 = 2 + (self.drift % 2);
        self.risk_stage4 = target_risk - self.risk_stage2;
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if matches!(self.state, AnchorAuditStage::Done) {
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(dt_ticks);
        loop {
            match self.state {
                AnchorAuditStage::Resolve => {
                    let result = self.resolve();
                    self.state = AnchorAuditStage::Done;
                    self.elapsed = 0;
                    return Some(result);
                }
                AnchorAuditStage::Done => return None,
                _ => {
                    if let Some(duration) = Self::stage_duration(self.state) {
                        if self.elapsed < duration {
                            break;
                        }
                        self.elapsed -= duration;
                        self.advance_stage();
                        continue;
                    }
                }
            }
        }
        None
    }
}

#[derive(Default)]
pub struct MissionBank {
    pub rain_flag: RainFlag,
    pub sourvault: Sourvault,
    pub break_chain: BreakTheChain,
    pub wayleave: Wayleave,
    pub anchor_audit: AnchorAudit,
}

impl MissionBank {
    pub fn iter_mut(&mut self) -> [(&str, &mut dyn Mission); 5] {
        [
            ("rain_flag", &mut self.rain_flag),
            ("sourvault", &mut self.sourvault),
            ("break_chain", &mut self.break_chain),
            ("wayleave", &mut self.wayleave),
            ("anchor_audit", &mut self.anchor_audit),
        ]
    }
}
