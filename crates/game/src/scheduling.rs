#![allow(non_camel_case_types)]

#[allow(unused_imports)]
use bevy::ecs::schedule::SystemSet;
use bevy::prelude::IntoScheduleConfigs;

pub mod sets {
    use bevy::ecs::schedule::SystemSet;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct DETTEROT_Director;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct DETTEROT_Missions;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct DETTEROT_Spawns;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct DETTEROT_PhysicsStep;

    #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    pub struct DETTEROT_Cleanup;
}

pub fn configure_fixed_update(app: &mut bevy::prelude::App) {
    use bevy::prelude::FixedUpdate;
    use sets::*;

    app.configure_sets(
        FixedUpdate,
        (
            DETTEROT_Director,
            DETTEROT_Missions.after(DETTEROT_Director),
            DETTEROT_Spawns.after(DETTEROT_Missions),
            DETTEROT_PhysicsStep.after(DETTEROT_Spawns),
            DETTEROT_Cleanup.after(DETTEROT_PhysicsStep),
        ),
    );
}
