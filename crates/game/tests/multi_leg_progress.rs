use game::systems::director::{
    config::load_director_cfg,
    spawn::{compute_spawn_budget, danger_score, SpawnBudget},
    DirectorState, LegStatus, DIRECTOR_CFG_PATH,
};
use game::systems::economy::{Pp, RouteId, Weather};
use game::{build_headless_app, collect_commands, director_inputs_mut, request_new_leg};
use repro::CommandKind;

#[test]
fn director_handles_multiple_legs() {
    let mut app = build_headless_app();

    {
        let mut inputs = director_inputs_mut(&mut app);
        inputs.world_seed = 4242;
        inputs.pp = Pp(4000);
        inputs.link_id = RouteId(10);
    }
    {
        let mut state = app.world_mut().resource_mut::<DirectorState>();
        state.link_id = RouteId(10);
        state.weather = Weather::Clear;
    }

    let cfg = load_director_cfg(DIRECTOR_CFG_PATH).expect("director config");

    let first_commands = collect_commands(&mut app, 400);
    assert!(
        !first_commands.is_empty(),
        "first leg should emit deterministic commands"
    );

    let first_danger_diff = first_commands.iter().find_map(|command| match &command.kind {
        CommandKind::Meter { key, .. } if key == "danger_diff" => Some(()),
        _ => None,
    });
    assert!(
        first_danger_diff.is_none(),
        "first leg should not emit danger_diff meter"
    );

    let first_spawn_count = first_commands
        .iter()
        .filter(|command| matches!(command.kind, CommandKind::Spawn { .. }))
        .count();
    let expected_first = compute_spawn_budget(Pp(4000), Weather::Clear, None, &cfg).enemies;
    assert_eq!(
        first_spawn_count as u32, expected_first,
        "first leg should match spawn budget",
    );

    let first_danger = {
        let state = app.world().resource::<DirectorState>();
        assert!(matches!(state.status, LegStatus::Completed(_)));
        state.prior_danger_score
    };

    {
        let mut inputs = director_inputs_mut(&mut app);
        inputs.world_seed = 5252;
        inputs.pp = Pp(5200);
        inputs.link_id = RouteId(11);
    }
    {
        let mut state = app.world_mut().resource_mut::<DirectorState>();
        state.link_id = RouteId(11);
        state.weather = Weather::Fog;
    }
    request_new_leg(&mut app);

    let second_commands = collect_commands(&mut app, 400);
    assert!(
        !second_commands.is_empty(),
        "second leg should emit deterministic commands"
    );

    let second_spawn_count = second_commands
        .iter()
        .filter(|command| matches!(command.kind, CommandKind::Spawn { .. }))
        .count();
    let expected_second = compute_spawn_budget(Pp(5200), Weather::Fog, None, &cfg).enemies;
    let expected_danger = danger_score(
        &SpawnBudget {
            enemies: expected_second,
            obstacles: 0,
        },
        12,
        5,
        3,
        50,
    );
    assert_eq!(
        second_spawn_count as u32, expected_second,
        "new link should reset spawn growth to first-leg baseline",
    );

    {
        let state = app.world().resource::<DirectorState>();
        assert_eq!(state.link_id, RouteId(11));
        assert!(matches!(state.status, LegStatus::Completed(_)));
        assert_eq!(state.prior_danger_score, expected_danger);
    }

    let danger_score_meter = second_commands
        .iter()
        .find_map(|command| match &command.kind {
            CommandKind::Meter { key, value } if key == "danger_score" => Some(*value),
            _ => None,
        })
        .expect("danger_score meter emitted");
    assert_eq!(danger_score_meter, expected_danger);

    let danger_diff_meter = second_commands
        .iter()
        .find_map(|command| match &command.kind {
            CommandKind::Meter { key, value } if key == "danger_diff" => Some(*value),
            _ => None,
        })
        .expect("danger_diff meter emitted");
    assert_eq!(danger_diff_meter, (expected_danger - first_danger).signum());
}
