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

#[test]
fn missions_resolve_deterministically() {
    let cfg_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/director/m2.toml");
    let cfg = load_director_cfg(cfg_path.to_str().expect("cfg path")).expect("load config");
    let rain_cfg = cfg.missions.get("rain_flag").expect("rain flag cfg");
    let sourvault_cfg = cfg.missions.get("sourvault").expect("sourvault cfg");
    let break_cfg = cfg.missions.get("break_chain").expect("break cfg");
    let wayleave_cfg = cfg.missions.get("wayleave").expect("wayleave cfg");
    let anchor_cfg = cfg.missions.get("anchor_audit").expect("anchor cfg");

    let rain_a = resolve::<RainFlagUplink>(42, rain_cfg);
    let rain_b = resolve::<RainFlagUplink>(42, rain_cfg);
    assert_eq!(rain_a, rain_b);

    let sour_a = resolve::<SourvaultEvac>(99, sourvault_cfg);
    let sour_b = resolve::<SourvaultEvac>(99, sourvault_cfg);
    assert_eq!(sour_a, sour_b);

    let chain_a = resolve::<BreakTheChain>(17, break_cfg);
    let chain_b = resolve::<BreakTheChain>(17, break_cfg);
    assert_eq!(chain_a, chain_b);

    let way_a = resolve::<WayleaveDefault>(123, wayleave_cfg);
    let way_b = resolve::<WayleaveDefault>(123, wayleave_cfg);
    assert_eq!(way_a, way_b);

    let anchor_a = resolve::<AnchorAudit>(88, anchor_cfg);
    let anchor_b = resolve::<AnchorAudit>(88, anchor_cfg);
    assert_eq!(anchor_a, anchor_b);
}
