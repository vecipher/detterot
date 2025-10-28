use bevy::prelude::*;

#[derive(Resource, Default)]
struct OrderLog(Vec<&'static str>);

fn record(tag: &'static str) -> impl FnMut(ResMut<OrderLog>) {
    move |mut log: ResMut<OrderLog>| {
        log.0.push(tag);
    }
}

#[test]
fn fixed_update_sets_chain_in_order() {
    let mut app = App::new();
    game::scheduling::configure(&mut app);
    app.insert_resource(OrderLog::default());
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

    app.finish();
    app.world_mut().run_schedule(FixedUpdate);

    let log = app.world().resource::<OrderLog>();
    assert_eq!(
        log.0,
        vec!["director", "missions", "spawns", "physics", "cleanup"]
    );
}
