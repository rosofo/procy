use std::time::Duration;

use crate::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    ecs::world::CommandQueue,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use image::{buffer::ConvertBuffer, DynamicImage, ImageBuffer, Luma, Rgb};
use imageproc::contours::{find_contours, BorderType, Contour};
use rand::{thread_rng, Rng};

pub fn caves_plugin(app: &mut App) {
    app.add_systems(Update, add_image);
    app.add_systems(FixedUpdate, (seed, compute_contours, grow_contours));
}

#[derive(Component)]
#[require(Transform(|| Transform::from_scale(Vec3::splat(2.0))))]
pub struct Caves {
    pub size: Vec2,
}

#[derive(Component)]
pub struct CavesImage(Handle<Image>);

pub fn add_image(
    caves: Query<(Entity, &Caves), Without<CavesImage>>,
    mut images: ResMut<Assets<Image>>,
    mut cmd: Commands,
) {
    for (entity, caves) in caves.iter() {
        let image = Image::from_dynamic(
            image::DynamicImage::new_rgb8(caves.size.x as u32, caves.size.y as u32),
            true,
            RenderAssetUsages::all(),
        );
        let cells = images.add(image);
        cmd.entity(entity)
            .insert(CavesImage(cells.clone()))
            .insert(Sprite::from_image(cells));
    }
}

fn seed(
    caves: Query<&CavesImage>,
    mut images: ResMut<Assets<Image>>,
    mut timer: Local<Timer>,
    time: Res<Time>,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
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

        timer.set_duration(Duration::from_secs_f32(0.5));
        timer.reset();
    }
}

fn compute_contours(
    caves: Query<(Entity, &CavesImage)>,
    images: ResMut<Assets<Image>>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    mut cmd: Commands,
) {
    timer.tick(time.delta());
    if timer.just_finished() {
        let thread_pool = AsyncComputeTaskPool::get();

        for (entity, caves) in caves.iter() {
            let image = images.get(&caves.0).unwrap().clone();
            let task = thread_pool.spawn(async move {
                find_contours(&image.try_into_dynamic().unwrap().into_luma8())
            });

            cmd.entity(entity).insert(ComputeContours(task));
        }

        timer.set_duration(Duration::from_secs_f32(0.0));
        timer.reset();
    }
}

fn grow_contours(
    mut contours: Query<(Entity, &CavesImage, &mut ComputeContours)>,
    mut images: ResMut<Assets<Image>>,
    mut cmd: Commands,
) {
    let mut rng = thread_rng();
    for (entity, handle, mut contours) in contours.iter_mut() {
        if let Some(contours) = block_on(future::poll_once(&mut contours.0)) {
            let image = images.get_mut(&handle.0).unwrap();
            // append the returned command queue to have it execute later
            for contour in contours {
                if contour.border_type == BorderType::Hole {
                    continue;
                }
                for point in contour.points.iter() {
                    let x = rng.gen_range(-1..=1);
                    let y = rng.gen_range(-1..=1);
                    if rng.gen_bool(0.05) {
                        image
                            .set_color_at(
                                ((point.x as i32) + x) as u32 % image.width(),
                                ((point.y as i32) + y) as u32 % image.height(),
                                RED.into(),
                            )
                            .unwrap();
                    }
                }
            }
            cmd.entity(entity).remove::<ComputeContours>();
        }
    }
}
#[derive(Component)]
struct ComputeContours(Task<Vec<Contour<u8>>>);
