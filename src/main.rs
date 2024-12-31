mod plugins;
mod prelude;
use plugins::caves::{caves_plugin, Caves};
use prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(Shape2dPlugin::new(ShapeConfig::default_2d()))
        .add_plugins(caves_plugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut cmd: Commands) {
    cmd.spawn((Camera2d, Msaa::Off));
    cmd.spawn(Caves {
        size: Vec2::new(256.0, 256.0),
    });
}
