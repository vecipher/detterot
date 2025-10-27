use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::prelude::*;
use game::scheduling::{self, sets};

#[derive(Resource, Default)]
struct OrderLog(Vec<&'static str>);

fn director_system(mut log: ResMut<OrderLog>) {
    log.0.push("director");
}

fn missions_system(mut log: ResMut<OrderLog>) {
    log.0.push("missions");
}

fn spawns_system(mut log: ResMut<OrderLog>) {
    log.0.push("spawns");
}

fn physics_system(mut log: ResMut<OrderLog>) {
    log.0.push("physics");
}

fn cleanup_system(mut log: ResMut<OrderLog>) {
    log.0.push("cleanup");
}

#[test]
fn schedule_sets_are_ordered() {
    let mut app = App::new();
    scheduling::configure_fixed_update(&mut app);

    app.world_mut().insert_resource(OrderLog::default());

    app.add_systems(FixedUpdate, director_system.in_set(sets::DETTEROT_Director));
    app.add_systems(FixedUpdate, missions_system.in_set(sets::DETTEROT_Missions));
    app.add_systems(FixedUpdate, spawns_system.in_set(sets::DETTEROT_Spawns));
    app.add_systems(
        FixedUpdate,
        physics_system.in_set(sets::DETTEROT_PhysicsStep),
    );
    app.add_systems(FixedUpdate, cleanup_system.in_set(sets::DETTEROT_Cleanup));

    app.world_mut().run_schedule(FixedUpdate);

    let order = app
        .world()
        .get_resource::<OrderLog>()
        .expect("order log present")
        .0
        .clone();
    assert_eq!(
        order,
        vec!["director", "missions", "spawns", "physics", "cleanup"]
    );
}
