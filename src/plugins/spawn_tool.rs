use std::time::Duration;

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
) {
    timer.tick(time.delta());
    if let Some(pos) = events.read().last().map(|e| e.position) {
        *cursor = pos;
    }

    // Each action has a button-like state of its own that you can check
    if action_state.pressed(&Action::SpawnAt) && timer.finished() {
        debug!("spawn at {:?}", cursor);
        timer.set_duration(Duration::from_millis(100));
        timer.reset();
    }
}
