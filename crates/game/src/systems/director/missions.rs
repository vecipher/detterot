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
struct MissionTimer {
    cfg: MissionCfg,
    ticks: u32,
    resolve_tick: u32,
    fail_tick: u32,
    success: bool,
    resolved: bool,
}

impl MissionTimer {
    fn new(cfg: MissionCfg, mut rng: DetRng, base: u32, window: u32, fail_offset: u32) -> Self {
        let window = window.max(1);
        let resolve_tick = base + (rng.next_u32() % window);
        let success = rng.next_u32() & 1 == 0;
        let fail_tick = resolve_tick + fail_offset.max(1);
        Self {
            cfg,
            ticks: 0,
            resolve_tick,
            fail_tick,
            success,
            resolved: false,
        }
    }

    fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
        if self.resolved {
            return None;
        }
        self.ticks = self.ticks.saturating_add(dt_ticks);
        if self.success && self.ticks >= self.resolve_tick {
            self.resolved = true;
            Some(MissionResult::Success {
                pp_delta: self.cfg.pp_success,
                basis_bp_overlay: self.cfg.basis_bp_success,
            })
        } else if !self.success && self.ticks >= self.fail_tick {
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

macro_rules! mission_struct {
    ($name:ident, $base:expr, $window:expr, $fail_offset:expr) => {
        #[derive(Default, Debug, Clone)]
        pub struct $name {
            timer: Option<MissionTimer>,
        }

        impl Mission for $name {
            fn init(&mut self, seed: u64, cfg: &MissionCfg) {
                self.timer = Some(MissionTimer::new(
                    cfg.clone(),
                    DetRng::new(seed),
                    $base,
                    $window,
                    $fail_offset,
                ));
            }

            fn tick(&mut self, dt_ticks: u32) -> Option<MissionResult> {
                self.timer.as_mut().and_then(|timer| timer.tick(dt_ticks))
            }
        }
    };
}

mission_struct!(RainFlagUplink, 90, 30, 45);
mission_struct!(SourvaultEvac, 110, 40, 60);
mission_struct!(BreakTheChain, 140, 35, 55);
mission_struct!(WayleaveDefault, 150, 30, 50);
mission_struct!(AnchorAudit, 80, 20, 40);

#[derive(Default)]
pub struct MissionBank {
    pub rain_flag: RainFlagUplink,
    pub sourvault: SourvaultEvac,
    pub break_chain: BreakTheChain,
    pub wayleave: WayleaveDefault,
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
