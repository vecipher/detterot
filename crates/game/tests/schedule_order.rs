use bevy::prelude::*;
use bevy::MinimalPlugins;

#[derive(Resource, Default)]
struct Log(Vec<&'static str>);

#[test]
fn fixed_update_sets_run_in_order() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    game::scheduling::configure(&mut app);
    app.insert_resource(Log::default());

    app.add_systems(
        FixedUpdate,
        record("director").in_set(game::scheduling::sets::DETTEROT_Director),
    );
    app.add_systems(
        FixedUpdate,
        record("missions").in_set(game::scheduling::sets::DETTEROT_Missions),
    );
    app.add_systems(
        FixedUpdate,
        record("spawns").in_set(game::scheduling::sets::DETTEROT_Spawns),
    );
    app.add_systems(
        FixedUpdate,
        record("physics").in_set(game::scheduling::sets::DETTEROT_PhysicsStep),
    );
    app.add_systems(
        FixedUpdate,
        record("cleanup").in_set(game::scheduling::sets::DETTEROT_Cleanup),
    );

    app.world_mut().run_schedule(FixedUpdate);
    let log = app.world().resource::<Log>();
    assert_eq!(
        log.0.as_slice(),
        &["director", "missions", "spawns", "physics", "cleanup"]
    );
}

fn record(label: &'static str) -> impl FnMut(ResMut<Log>) + 'static {
    move |mut log: ResMut<Log>| {
        log.0.push(label);
    }
}
