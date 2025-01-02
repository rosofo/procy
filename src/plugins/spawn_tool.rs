use std::time::Duration;

use bevy::window::PrimaryWindow;

use crate::prelude::*;

pub fn spawn_tool_plugin(app: &mut App) {
    app.add_systems(Startup, spawn_player);
    app.add_systems(Update, spawn_at);
    app.add_plugins(InputManagerPlugin::<Action>::default());
}

#[derive(Actionlike, Debug, Clone, Reflect, Hash, PartialEq, PartialOrd, Ord, Eq)]
pub enum Action {
    SpawnAt,
}

#[derive(Component)]
struct Player;

fn spawn_player(mut commands: Commands) {
    // Describes how to convert from player inputs into those actions
    let input_map = InputMap::new([(Action::SpawnAt, MouseButton::Left)]);
    commands
        .spawn(InputManagerBundle::with_map(input_map))
        .insert(Player);
}

// Query for the `ActionState` component in your game logic systems!
fn spawn_at(
    action_state: Single<&ActionState<Action>, With<Player>>,
    mut events: EventReader<CursorMoved>,
    mut cursor: Local<Vec2>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform)>,
) {
    timer.tick(time.delta());
    let mut vel = Vec2::ZERO;
    if let Some(pos) = events.read().last().map(|e| e.position) {
        vel = pos - *cursor;
        *cursor = pos;
    }

    // Each action has a button-like state of its own that you can check
    if action_state.pressed(&Action::SpawnAt) && timer.finished() {
        let pos = cursor_to_world(*cursor, camera.0, camera.1);
        debug!("spawn at {:?}", pos);

        commands.spawn((
            Ball,
            Transform::from_translation(pos),
            Velocity::linear(vel.reflect(Vec2::Y) * 100.0),
        ));
        timer.set_duration(Duration::from_millis(100));
        timer.reset();
    }
}

#[derive(Component)]
#[require(
    Transform,
    Collider(|| Collider::ball(10.0)),
    RigidBody(|| RigidBody::Dynamic),
    ColliderMassProperties(|| ColliderMassProperties::Density(1.2)),
    Restitution(|| Restitution {coefficient: 0.7, ..default()}),
    GravityScale(|| GravityScale(1.5)),

)]
pub struct Ball;

fn cursor_to_world(cursor: Vec2, camera: &Camera, trans: &GlobalTransform) -> Vec3 {
    camera
        .viewport_to_world_2d(trans, cursor)
        .unwrap()
        .extend(0.0)
}
