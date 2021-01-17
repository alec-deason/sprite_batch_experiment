use bevy::prelude::*;

mod sprite_batch;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(sprite_batch::BatchingPlugin)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("circle.png");
    commands
        .spawn(Camera2dBundle::default())
        .spawn(sprite_batch::BatchedSpriteBundle::new(materials.add(texture_handle.into()), Transform::default()));
}
