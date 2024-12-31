use crate::prelude::*;
use bevy::{asset::RenderAssetUsages, color::palettes::css::GREY};
use rand::{thread_rng, Rng};

pub fn caves_plugin(app: &mut App) {
    app.add_systems(Update, add_image);
    app.add_systems(FixedUpdate, grow);
}

#[derive(Component)]
#[require(Transform)]
pub struct Caves {
    pub size: Vec2,
}

#[derive(Component)]
pub struct CavesImage(Handle<Image>);

pub fn add_image(
    caves: Query<(Entity, &Caves), Without<CavesImage>>,
    mut images: ResMut<Assets<Image>>,
    shapes: ShapeCommands,
    mut cmd: Commands,
) {
    for (entity, caves) in caves.iter() {
        let image = Image::from_dynamic(
            image::DynamicImage::new_luma8(caves.size.x as u32, caves.size.y as u32),
            false,
            RenderAssetUsages::all(),
        );
        let cells = images.add(image);
        let mut config = shapes.config().clone();
        config.color = Color::NONE;
        cmd.entity(entity)
            .insert(CavesImage(cells.clone()))
            .insert(Sprite::from_image(cells));
    }
}

fn grow(caves: Query<&CavesImage>, mut images: ResMut<Assets<Image>>) {
    let mut rng = thread_rng();
    for caves in caves.iter() {
        let image = images.get_mut(&caves.0).unwrap();
        let mut seed: Vec2 = rng.gen();
        seed.x *= image.width() as f32;
        seed.y *= image.height() as f32;
        info!("seeding caves {}", seed);
        let pixel = image
            .pixel_bytes_mut(UVec3::new(seed.x as u32, seed.y as u32, 0))
            .unwrap();
        pixel[0] = 255;
    }
}
