use std::time::Duration;

use bevy::window::PrimaryWindow;

use crate::{
    plugins::{creature::Bat, pathfinding::Goal},
    prelude::*,
};

pub fn spawn_tool_plugin(app: &mut App) {
    app.init_state::<Tool>();
    app.add_systems(Startup, spawn_player);
    app.add_systems(Update, (spawn_at, ui));
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
#[allow(clippy::too_many_arguments)]
fn spawn_at(
    tool: Res<State<Tool>>,
    action_state: Single<&ActionState<Action>, With<Player>>,
    mut events: EventReader<CursorMoved>,
    mut cursor: Local<Vec2>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform)>,
    tiles: Query<Entity, With<TilePos>>,
) {
    timer.tick(time.delta());
    let mut vel = Vec2::ZERO;
    if let Some(pos) = events.read().last().map(|e| e.position) {
        vel = pos - *cursor;
        *cursor = pos;
    }

    // Each action has a button-like state of its own that you can check
    let pressed = if **tool == Tool::Goal {
        action_state.just_pressed(&Action::SpawnAt)
    } else {
        action_state.pressed(&Action::SpawnAt)
    };
    if pressed && timer.finished() {
        let pos = cursor_to_world(*cursor, camera.0, camera.1);
        debug!("spawn at {:?}", pos);

        match **tool {
            Tool::Ball => {
                commands.spawn((
                    Ball,
                    Transform::from_translation(pos),
                    Velocity::linear(vel.reflect(Vec2::Y) * 100.0),
                ));
            }
            Tool::Bat => {
                commands.spawn((
                    Bat,
                    Transform::from_translation(pos),
                    Velocity::linear(vel.reflect(Vec2::Y) * 100.0),
                ));
            }
            Tool::Goal => {
                let tile = tiles.iter().next().unwrap();
                commands.entity(tile).insert(Goal);
            }
        };
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

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tool {
    #[default]
    Ball,
    Bat,
    Goal,
}

fn ui(
    mut contexts: EguiContexts,
    state: Res<State<Tool>>,
    mut next_state: ResMut<NextState<Tool>>,
) {
    egui::Window::new("Spawn Tool").show(contexts.ctx_mut(), |ui| {
        let mut state = **state;
        ui.radio_value(&mut state, Tool::Ball, "Ball");
        ui.radio_value(&mut state, Tool::Bat, "Bat");
        ui.radio_value(&mut state, Tool::Goal, "Goal");
        next_state.set(state);
    });
}
