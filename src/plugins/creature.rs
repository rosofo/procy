use bevy::sprite::Material2d;

use crate::prelude::*;

use super::{pathfinding::DMap, physics::AddForces, terrain::MapConfig};

pub fn creature_plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(
        FixedUpdate,
        (flap, ((lift, pathfind).in_set(AddForces)).chain()),
    );
}

#[derive(Component)]
#[require(
    Transform,
    RigidBody(|| RigidBody::Dynamic),
    LockedAxes(|| LockedAxes::ROTATION_LOCKED_Z),
    Collider(|| Collider::ball(1.5)),
    ColliderMassProperties(|| ColliderMassProperties::Density(6.2)),
    Restitution(|| Restitution {coefficient: 0.7, ..default()}),
    GravityScale(|| GravityScale(1.5)),
    InheritedVisibility,
    ExternalForce


)]
pub struct Bat;

#[derive(Component)]
pub enum Wing {
    Left,
    Right,
}

fn setup(world: &mut World) {
    let hooks = world.register_component_hooks::<Bat>();
    hooks.on_add(|mut world, entity, _| {
        world.commands().entity(entity).with_children(|parent| {
            let collider = Collider::cuboid(6.0, 1.0);
            let joint_a = RevoluteJointBuilder::new()
                .local_anchor2(Vec2::new(12.0, 0.0))
                .limits([-1.5, 1.5])
                .build();
            let joint_b = RevoluteJointBuilder::new()
                .local_anchor2(Vec2::new(-12.0, 0.0))
                .limits([-1.5, 1.5])
                .build();
            parent.spawn((
                collider.clone(),
                Wing::Left,
                RigidBody::Dynamic,
                ColliderMassProperties::Density(0.3),
                GravityScale(1.0),
                Transform::from_xyz(-15.0, 0.0, 0.0),
                Ccd::enabled(),
                Friction::coefficient(0.9),
                Velocity::default(),
                Sleeping::disabled(),
                ImpulseJoint::new(parent.parent_entity(), joint_a),
            ));
            parent.spawn((
                collider,
                Wing::Right,
                RigidBody::Dynamic,
                Sleeping::disabled(),
                GravityScale(1.0),
                ColliderMassProperties::Density(0.3),
                Transform::from_xyz(15.0, 0.0, 0.0),
                Ccd::enabled(),
                Velocity::default(),
                Friction::coefficient(0.9),
                ImpulseJoint::new(parent.parent_entity(), joint_b),
            ));
        });
    });
}

fn flap(
    bats: Query<(&Children, &PathfindDir), With<Bat>>,
    mut wings: Query<&mut ImpulseJoint, With<Parent>>,
    time: Res<Time>,
) {
    for (children, dir) in bats.iter() {
        let mut sign = 1.0;
        let mut iter = wings.iter_many_mut(children);
        while let Some(mut joint) = iter.fetch_next() {
            let TypedJoint::RevoluteJoint(ref mut joint) = joint.data else {
                continue;
            };
            let pathfind_down = if dir.0.y < 0.0 { 0.5 } else { 1.0 };

            let target_pos = (-0.6 + (time.elapsed_secs() * 14.0).sin()) * 0.5;
            joint.set_motor_position(sign * target_pos, 3000.0 * pathfind_down, 30.0);
            sign *= -1.0;
        }
    }
}

fn lift(
    wings: Query<(&Velocity, &Parent, &Wing)>,
    mut bats: Query<(&Velocity, &mut ExternalForce), With<Bat>>,
) {
    for (vel, parent, wing) in wings.iter() {
        let (bat, mut ext) = bats.get_mut(parent.get()).unwrap();
        let sign = match wing {
            Wing::Left => -1.0,
            Wing::Right => 1.0,
        };
        let mut force = Vec2::Y * vel.angvel * 2000.0 * sign;
        force.y = force.y.max(0.0);
        ext.force += force;
    }
}

fn pathfind(
    mut bats: Query<(Entity, &Transform, &mut ExternalForce), With<Bat>>,
    mut commands: Commands,
    dmap: Single<&DMap>,
    config: Res<MapConfig>,
) {
    for (bat, trans, mut ext) in bats.iter_mut() {
        let Some(coord) = config.world_to_tile(trans.translation.truncate()) else {
            warn!("bat out of bounds");
            continue;
        };
        let coord = IVec2::new(coord.x as i32, coord.y as i32);
        debug!("bat at {:?}", coord);
        let window = [
            coord + IVec2::new(1, 0),
            coord + IVec2::new(-1, 0),
            coord + IVec2::new(0, 1),
            coord + IVec2::new(0, -1),
            coord,
            coord + IVec2::new(1, 1),
            coord + IVec2::new(-1, 1),
            coord + IVec2::new(1, -1),
            coord + IVec2::new(-1, -1),
        ];
        let vals = window.iter().map(|c| dmap.get(*c));
        let Some((min, min_val)) = window
            .iter()
            .zip(vals)
            .filter_map(|(c, v)| v.map(|v| (*c, v)))
            .min_by_key(|(_, v)| *v)
        else {
            continue;
        };
        let min = config.tile_to_world(TilePos::new(min.x as u32, min.y as u32));
        let dir = (min - trans.translation.truncate()).normalize();
        let force = Vec2::new(dir.x, dir.y)
            * 2000.0
            * (min_val as f32).clamp(0.0, 50.0).remap(0.0, 50.0, 0.0, 1.0);
        debug!("pathfinding force: {:?}", force);
        commands.entity(bat).insert(PathfindDir(dir));
        ext.force += force;
    }
}

#[derive(Component)]
pub struct PathfindDir(pub Vec2);

#[derive(Component)]
pub struct DebugArrow;
fn pathfind_debug_arrow(
    pathfind_dirs: Query<(Entity, &Children, &PathfindDir)>,
    mut arrows: Query<&mut Transform, With<DebugArrow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, children, dir) in pathfind_dirs.iter() {
        if let Some(mut arrow) = arrows.iter_many_mut(children.iter()).fetch_next() {
            arrow.rotation = Quat::from_rotation_z(Vec2::X.angle_to(dir.0));
        } else {
            let mesh = Mesh2d(
                meshes.add(
                    Rectangle::from_size(Vec2::new(80.0, 5.0))
                        .mesh()
                        .build()
                        .translated_by(Vec3::new(40.0, 0.0, 0.0)),
                ),
            );
            commands.entity(entity).with_child((
                DebugArrow,
                mesh,
                MeshMaterial2d(materials.add(ColorMaterial::from_color(WHITE))),
                Transform::from_xyz(0.0, 0.0, 2.0),
            ));
        }
    }
}
