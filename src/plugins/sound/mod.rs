use raytrace::{RayDebug, Raycaster};
use waveform::{CpalState, Waveform};

use crate::prelude::*;
mod raytrace;
mod waveform;

pub fn sound_plugin(app: &mut App) {
    let (state, input) = CpalState::setup();
    app.insert_non_send_resource(state);
    app.insert_resource(input);
    app.register_type::<Waveform>();
    app.add_systems(
        Update,
        (
            (raytrace::cast_rays, raytrace::debug_rays).chain(),
            debug_from_cursor,
            waveform::trace_waves,
        ),
    );
    app.add_systems(Startup, setup);
}

fn setup(mut commands: Commands) {
    let id = commands.spawn(Raycaster { bounces: 3 }).id();
    commands.spawn(RayDebug(id));
}

fn debug_from_cursor(
    mut events: EventReader<CursorMoved>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut debug: Single<&mut Transform, With<Raycaster>>,
) {
    if let Some(moved) = events.read().last() {
        let cursor = moved.position;
        let pos = camera.0.viewport_to_world_2d(camera.1, cursor).unwrap();
        debug.translation = pos.extend(0.0);
    }
}
