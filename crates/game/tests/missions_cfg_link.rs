use game::systems::director::config::MissionCfg;
use game::systems::director::missions::{Mission, MissionResult, RainFlag};

fn mission_cfg() -> MissionCfg {
    MissionCfg {
        pp_success: -11,
        pp_fail: 9,
        basis_bp_success: -7,
        basis_bp_fail: 5,
    }
}

#[test]
fn mission_result_reflects_config() {
    let cfg = mission_cfg();
    let mut mission = RainFlag::default();
    mission.init(123, &cfg);
    let mut result = None;
    for _ in 0..400 {
        if let Some(res) = mission.tick(1) {
            result = Some(res);
            break;
        }
    }
    let result = result.expect("mission should resolve");
    match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        } => {
            assert_eq!(pp_delta, cfg.pp_success);
            assert_eq!(basis_bp_overlay, cfg.basis_bp_success);
        }
        MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => {
            assert_eq!(pp_delta, cfg.pp_fail);
            assert_eq!(basis_bp_overlay, cfg.basis_bp_fail);
        }
    }
}
