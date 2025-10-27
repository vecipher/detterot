use game::systems::director::config::MissionCfg;
use game::systems::director::missions::{
    AnchorAudit, BreakTheChain, Mission, MissionResult, RainFlag, Sourvault, Wayleave,
};
use game::systems::director::spawn::DetRng;

fn cfg() -> MissionCfg {
    MissionCfg {
        pp_success: -5,
        pp_fail: 7,
        basis_bp_success: -2,
        basis_bp_fail: 4,
    }
}

fn resolve_after<M: Mission>(mission: &mut M, total_ticks: u32) -> MissionResult {
    assert!(total_ticks > 0, "missions must advance");
    for _ in 0..(total_ticks - 1) {
        assert!(mission.tick(1).is_none(), "mission resolved too early");
    }
    mission
        .tick(1)
        .expect("mission should resolve on the scripted boundary")
}

fn is_success(result: &MissionResult) -> bool {
    matches!(result, MissionResult::Success { .. })
}

#[test]
fn rain_flag_pressure_script() {
    let cfg = cfg();
    let mut mission = RainFlag::default();
    mission.init(0, &cfg);
    assert!(is_success(&resolve_after(&mut mission, 108)));

    let mut mission = RainFlag::default();
    mission.init(3, &cfg);
    assert!(!is_success(&resolve_after(&mut mission, 108)));

    let mut mission = RainFlag::default();
    mission.init(7, &cfg); // seed mod 5 == 2 triggers the tie path
    let mut rng = DetRng::new(7);
    let expected_success = rng.next_u32() & 1 == 0;
    assert_eq!(
        is_success(&resolve_after(&mut mission, 108)),
        expected_success
    );
}

#[test]
fn sourvault_evacuation_pressure_script() {
    let cfg = cfg();
    let mut mission = Sourvault::default();
    mission.init(1, &cfg);
    assert!(is_success(&resolve_after(&mut mission, 150)));

    let mut mission = Sourvault::default();
    mission.init(11, &cfg);
    assert!(!is_success(&resolve_after(&mut mission, 150)));

    let mut mission = Sourvault::default();
    mission.init(9, &cfg); // seed mod 6 == 3, scripted tie
    let mut rng = DetRng::new(9);
    let expected_success = rng.next_u32() & 1 == 0;
    assert_eq!(
        is_success(&resolve_after(&mut mission, 150)),
        expected_success
    );
}

#[test]
fn break_the_chain_density_script() {
    let cfg = cfg();
    let mut mission = BreakTheChain::default();
    mission.init(0, &cfg);
    assert!(is_success(&resolve_after(&mut mission, 170)));

    let mut mission = BreakTheChain::default();
    mission.init(15, &cfg);
    assert!(!is_success(&resolve_after(&mut mission, 170)));

    let mut mission = BreakTheChain::default();
    mission.init(11, &cfg); // seed mod 8 == 3, scripted tie
    let mut rng = DetRng::new(11);
    let expected_success = rng.next_u32() & 1 == 0;
    assert_eq!(
        is_success(&resolve_after(&mut mission, 170)),
        expected_success
    );
}

#[test]
fn wayleave_petition_weight_script() {
    let cfg = cfg();
    let mut mission = Wayleave::default();
    mission.init(10, &cfg);
    assert!(is_success(&resolve_after(&mut mission, 158)));

    let mut mission = Wayleave::default();
    mission.init(26, &cfg);
    assert!(!is_success(&resolve_after(&mut mission, 158)));

    let mut mission = Wayleave::default();
    mission.init(22, &cfg); // seed mod 9 == 4, scripted tie
    let mut rng = DetRng::new(22);
    let expected_success = rng.next_u32() & 1 == 0;
    assert_eq!(
        is_success(&resolve_after(&mut mission, 158)),
        expected_success
    );
}

#[test]
fn anchor_audit_drift_script() {
    let cfg = cfg();
    let mut mission = AnchorAudit::default();
    mission.init(0, &cfg);
    assert!(is_success(&resolve_after(&mut mission, 98)));

    let mut mission = AnchorAudit::default();
    mission.init(17, &cfg);
    assert!(!is_success(&resolve_after(&mut mission, 98)));

    let mut mission = AnchorAudit::default();
    mission.init(27, &cfg); // seed mod 6 == 3, scripted tie
    let mut rng = DetRng::new(27);
    let expected_success = rng.next_u32() & 1 == 0;
    assert_eq!(
        is_success(&resolve_after(&mut mission, 98)),
        expected_success
    );
}
