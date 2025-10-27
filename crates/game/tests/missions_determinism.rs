use game::systems::director::config::MissionCfg;
use game::systems::director::missions::{
    AnchorAudit, BreakTheChain, Mission, MissionResult, RainFlag, Sourvault, Wayleave,
};

fn sample_cfg() -> MissionCfg {
    MissionCfg {
        pp_success: -5,
        pp_fail: 7,
        basis_bp_success: -10,
        basis_bp_fail: 5,
    }
}

fn run_mission<M: Mission + Default>(seed: u64, cfg: &MissionCfg) -> MissionResult {
    let mut mission = M::default();
    mission.init(seed, cfg);
    for _ in 0..500 {
        if let Some(result) = mission.tick(1) {
            return result;
        }
    }
    panic!("mission did not resolve within window");
}

#[test]
fn missions_produce_consistent_results() {
    let cfg = sample_cfg();
    let seeds = [1_u64, 42, 1337];
    for &seed in &seeds {
        let a = run_mission::<RainFlag>(seed, &cfg);
        let b = run_mission::<RainFlag>(seed, &cfg);
        assert_eq!(a, b);
        let a = run_mission::<Sourvault>(seed, &cfg);
        let b = run_mission::<Sourvault>(seed, &cfg);
        assert_eq!(a, b);
        let a = run_mission::<BreakTheChain>(seed, &cfg);
        let b = run_mission::<BreakTheChain>(seed, &cfg);
        assert_eq!(a, b);
        let a = run_mission::<Wayleave>(seed, &cfg);
        let b = run_mission::<Wayleave>(seed, &cfg);
        assert_eq!(a, b);
        let a = run_mission::<AnchorAudit>(seed, &cfg);
        let b = run_mission::<AnchorAudit>(seed, &cfg);
        assert_eq!(a, b);
    }
}
