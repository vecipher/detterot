pub mod cli;
pub mod logs;
pub mod scheduling;
pub mod systems;

use bevy::app::App;
use bevy::prelude::*;
use bevy::time::{Fixed, Time, TimeUpdateStrategy};
use bevy::MinimalPlugins;

use std::time::Duration;

use systems::command_queue::CommandQueue;
use systems::director::{
    DirectorInputs, DirectorPlugin, DirectorRequests, DirectorState, LegStatus,
};

pub const DEFAULT_FIXED_DT: f64 = 1.0 / 30.0;
pub const MIN_FIXED_DT: f64 = 0.008_333_333_333_333_333;

pub fn build_headless_app() -> App {
    let mut app = App::new();
    #[cfg(feature = "deterministic")]
    configure_task_pools();
    app.add_plugins(MinimalPlugins);
    scheduling::configure(&mut app);
    configure_fixed_dt(&mut app, DEFAULT_FIXED_DT);
    app.add_plugins(DirectorPlugin);
    app.world_mut().run_schedule(Startup);
    app
}

pub fn collect_commands(app: &mut App, ticks: u32) -> Vec<repro::Command> {
    for _ in 0..ticks {
        app.world_mut().run_schedule(FixedUpdate);
    }
    app.world_mut().resource_mut::<CommandQueue>().drain()
}

pub fn director_inputs_mut(app: &mut App) -> Mut<'_, DirectorInputs> {
    app.world_mut().resource_mut::<DirectorInputs>()
}

pub fn request_new_leg(app: &mut App) {
    {
        let mut state = app.world_mut().resource_mut::<DirectorState>();
        state.status = LegStatus::Loading;
    }
    app.world_mut()
        .resource_mut::<DirectorRequests>()
        .queue_new_leg = true;
}

#[allow(clippy::float_arithmetic)]
pub fn configure_fixed_dt(app: &mut App, seconds: f64) {
    let clamped = if seconds < MIN_FIXED_DT {
        MIN_FIXED_DT
    } else {
        seconds
    };
    let duration = Duration::from_secs_f64(clamped);
    if let Some(mut strategy) = app.world_mut().get_resource_mut::<TimeUpdateStrategy>() {
        *strategy = TimeUpdateStrategy::ManualDuration(duration);
    } else {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(duration));
    }
    if let Some(mut fixed) = app.world_mut().get_resource_mut::<Time<Fixed>>() {
        fixed.set_timestep(duration);
    } else {
        let mut fixed = Time::<Fixed>::default();
        fixed.set_timestep(duration);
        app.insert_resource(fixed);
    }
}

#[cfg(feature = "deterministic")]
fn configure_task_pools() {
    use bevy::tasks::{AsyncComputeTaskPool, ComputeTaskPool, IoTaskPool, TaskPoolBuilder};

    fn pool(name: &str) -> bevy::tasks::TaskPool {
        TaskPoolBuilder::new()
            .num_threads(1)
            .thread_name(name.to_owned())
            .build()
    }

    let _ = ComputeTaskPool::get_or_init(|| pool("detterot-compute"));
    let _ = AsyncComputeTaskPool::get_or_init(|| pool("detterot-async"));
    let _ = IoTaskPool::get_or_init(|| pool("detterot-io"));
}
