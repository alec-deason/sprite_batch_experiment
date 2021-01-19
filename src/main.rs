use bevy::prelude::*;
use rand::prelude::*;

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
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("circle.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(32.0, 32.0), 1, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands.spawn(Camera2dBundle::default());

    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        let x = rng.gen_range(-500.0..500.0);
        let y = rng.gen_range(-500.0..500.0);
        commands.spawn(sprite_batch::BatchedSpriteBundle::new(
            texture_atlas_handle.clone(),
            0,
            Transform::from_translation(Vec3::new(x, y, 0.0)),
        ));
    }
}
