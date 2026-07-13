use bevy::prelude::*;
use template_core::DevScenePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DevScenePlugin)
        .run();
}
