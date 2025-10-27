use bevy::prelude::*;

pub mod sets {
    #![allow(non_camel_case_types)]
    use bevy::prelude::*;

    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct DETTEROT_Director;

    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct DETTEROT_Missions;

    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct DETTEROT_Spawns;

    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct DETTEROT_PhysicsStep;

    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
    pub struct DETTEROT_Cleanup;
}

pub fn configure(app: &mut App) {
    app.configure_sets(
        FixedUpdate,
        (
            sets::DETTEROT_Director,
            sets::DETTEROT_Missions.after(sets::DETTEROT_Director),
            sets::DETTEROT_Spawns.after(sets::DETTEROT_Missions),
            sets::DETTEROT_PhysicsStep.after(sets::DETTEROT_Spawns),
            sets::DETTEROT_Cleanup.after(sets::DETTEROT_PhysicsStep),
        ),
    );
}
