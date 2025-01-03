use std::f32::consts::TAU;

use crate::prelude::*;

#[derive(Component)]
#[require(Transform)]
pub struct Raycaster {
    pub bounces: usize,
}

#[derive(Component)]
#[require(Transform)]
pub struct Raycasts(pub Vec<Vec<CastData>>);

pub struct CastData {
    pub origin: Vec2,
    pub point: Vec2,
    pub normal: Vec2,
    pub dir: Vec2,
}

#[derive(Component)]
#[require(Transform, InheritedVisibility)]
pub struct RayDebug(pub Entity);

pub(super) fn cast_rays(
    casters: Query<(Entity, &Transform, &Raycaster)>,
    rapier_context: Single<&RapierContext>,
    mut commands: Commands,
) {
    for (entity, trans, caster) in casters.iter() {
        let rays = radial_cast(trans.translation.truncate(), 10, &rapier_context);
        let bounced = rays.map(|(_entity, intersection, dir)| {
            iterate(
                Some((intersection, dir, trans.translation.truncate())),
                |prev| {
                    let (intersection, incoming, _origin) = prev.as_ref()?;
                    let result = bounce_ray(intersection, *incoming, &rapier_context)?;
                    Some((result.0, result.1, intersection.point))
                },
            )
            .while_some()
            .take(caster.bounces)
        });

        commands.entity(entity).insert(Raycasts(
            bounced
                .map(|rays| {
                    rays.map(|(intersection, dir, origin)| CastData {
                        dir,
                        origin,
                        point: intersection.point,
                        normal: intersection.normal,
                    })
                    .collect_vec()
                })
                .collect_vec(),
        ));
    }
}

fn radial_cast(
    pos: Vec2,
    n: usize,
    rapier_context: &RapierContext,
) -> impl Iterator<Item = (Entity, RayIntersection, Vec2)> + use<'_> {
    let angle = TAU / n as f32;
    (0..n).filter_map(move |i| {
        let dir = Vec2::from_angle(angle * i as f32);
        let (entity, intersection) = rapier_context.cast_ray_and_get_normal(
            pos,
            dir,
            1000.0,
            true,
            QueryFilter::default(),
        )?;
        Some((entity, intersection, dir))
    })
}

fn bounce_ray(
    intersection: &RayIntersection,
    incoming: Vec2,
    rapier_context: &RapierContext,
) -> Option<(RayIntersection, Vec2)> {
    let dir = incoming.normalize().reflect(intersection.normal);
    rapier_context
        .cast_ray_and_get_normal(
            intersection.point + dir * 0.1,
            dir,
            1000.0,
            false,
            QueryFilter::default(),
        )
        .map(|(_, intersection)| (intersection, dir))
}

pub(super) fn debug_rays(
    debugs: Query<(Entity, &RayDebug)>,
    casts: Query<(&Raycasts, &Transform)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mat = materials.add(ColorMaterial::from_color(BLUE_VIOLET));
    for (entity, debug) in debugs.iter() {
        commands.entity(entity).despawn_descendants();
        if let Ok((casts, trans)) = casts.get(debug.0) {
            commands.entity(entity).with_children(|parent| {
                for rays in casts.0.iter() {
                    for cast in rays.iter() {
                        let len = (cast.point - cast.origin).length();
                        let angle = Vec2::X.angle_to(cast.dir);
                        let mesh = Rectangle::from_size(Vec2::new(len, 1.0))
                            .mesh()
                            .build()
                            .translated_by(Vec2::new(len / 2.0, 0.0).extend(0.0))
                            .rotated_by(Quat::from_rotation_z(angle));
                        parent.spawn((
                            Transform::from_translation(cast.origin.extend(1.0)),
                            Mesh2d(meshes.add(mesh)),
                            MeshMaterial2d(mat.clone()),
                        ));
                    }
                }
            });
        }
    }
}
