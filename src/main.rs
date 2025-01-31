mod camera;
mod math;
mod plugins;
mod prelude;
use bevy::log::{Level, LogPlugin};
use iyes_perf_ui::prelude::PerfUiAllEntries;
use plugins::{
    caves::{caves_plugin, Caves},
    creature::creature_plugin,
    pathfinding::pathfinding_plugin,
    physics::physics_plugin,
    sound::sound_plugin,
    spawn_tool::spawn_tool_plugin,
    terrain::terrain_plugin,
};
use prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(LogPlugin {
            level: Level::INFO,
            filter: "wgpu=error,procy=debug".to_string(),
            custom_layer: |_| Some(Box::new(tracing_tracy::TracyLayer::default())),
        }))
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(iyes_perf_ui::PerfUiPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(24.0))
        .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(EguiPlugin)
        .add_plugins(TilemapPlugin)
        .add_plugins(terrain_plugin)
        .add_plugins(caves_plugin)
        .add_plugins(spawn_tool_plugin)
        .add_plugins(creature_plugin)
        .add_plugins(pathfinding_plugin)
        .add_plugins(physics_plugin)
        .add_plugins(sound_plugin)
        .add_systems(Startup, setup)
        .add_systems(FixedPostUpdate, || {
            tracy_client::secondary_frame_mark!("Fixed Frame");
        })
        .add_systems(PostUpdate, || {
            tracy_client::frame_mark();
        })
        .add_systems(Update, camera::movement)
        .run();
}

fn setup(mut cmd: Commands) {
    cmd.spawn((Camera2d, Msaa::Off));
    cmd.spawn(Caves {
        size: Vec2::new(256.0, 256.0),
        ..default()
    });
    cmd.spawn(PerfUiAllEntries::default());
}
