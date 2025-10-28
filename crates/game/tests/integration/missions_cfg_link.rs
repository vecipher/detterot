use game::systems::director::config::load_director_cfg;
use game::systems::director::missions::{
    AnchorAudit, BreakTheChain, Mission, MissionResult, RainFlagUplink, SourvaultEvac,
    WayleaveDefault,
};
use std::path::Path;

fn resolve<M: Mission + Default>(
    seed: u64,
    cfg: &game::systems::director::config::MissionCfg,
) -> MissionResult {
    let mut mission = M::default();
    mission.init(seed, cfg);
    for _ in 0..512 {
        if let Some(result) = mission.tick(1) {
            return result;
        }
    }
    panic!("mission did not resolve");
}

fn assert_matches(
    cfg: &game::systems::director::config::MissionCfg,
    result: MissionResult,
    expect_success: bool,
) {
    match result {
        MissionResult::Success {
            pp_delta,
            basis_bp_overlay,
        } => {
            assert!(expect_success, "expected failure but mission succeeded");
            assert_eq!(pp_delta, cfg.pp_success);
            assert_eq!(basis_bp_overlay, cfg.basis_bp_success);
        }
        MissionResult::Fail {
            pp_delta,
            basis_bp_overlay,
        } => {
            assert!(!expect_success, "expected success but mission failed");
            assert_eq!(pp_delta, cfg.pp_fail);
            assert_eq!(basis_bp_overlay, cfg.basis_bp_fail);
        }
    }
}

#[test]
fn mission_results_follow_config_values() {
    let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/director/m2.toml");
    let cfg = load_director_cfg(cfg_path.to_str().expect("cfg path")).expect("load config");

    let rain_cfg = cfg.missions.get("rain_flag").expect("rain flag cfg");
    assert_matches(rain_cfg, resolve::<RainFlagUplink>(42, rain_cfg), true);
    assert_matches(rain_cfg, resolve::<RainFlagUplink>(41, rain_cfg), false);

    let sour_cfg = cfg.missions.get("sourvault").expect("sour cfg");
    assert_matches(sour_cfg, resolve::<SourvaultEvac>(60, sour_cfg), true);
    assert_matches(sour_cfg, resolve::<SourvaultEvac>(99, sour_cfg), false);

    let break_cfg = cfg.missions.get("break_chain").expect("break cfg");
    assert_matches(break_cfg, resolve::<BreakTheChain>(11, break_cfg), true);

    let way_cfg = cfg.missions.get("wayleave").expect("way cfg");
    assert_matches(way_cfg, resolve::<WayleaveDefault>(20, way_cfg), true);

    let anchor_cfg = cfg.missions.get("anchor_audit").expect("anchor cfg");
    assert_matches(anchor_cfg, resolve::<AnchorAudit>(88, anchor_cfg), true);
    assert_matches(anchor_cfg, resolve::<AnchorAudit>(2, anchor_cfg), false);
}
