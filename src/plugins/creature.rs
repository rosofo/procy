use crate::prelude::*;

use super::pathfinding::DMap;

pub fn creature_plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, (flap, (lift, pathfind).chain()));
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

)]
pub struct Bat;

#[derive(Component)]
pub struct Wing;

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
                Wing,
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
                Wing,
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
    bats: Query<&Children, With<Bat>>,
    mut wings: Query<&mut ImpulseJoint, With<Parent>>,
    time: Res<Time>,
) {
    for children in bats.iter() {
        let mut sign = 1.0;
        let mut iter = wings.iter_many_mut(children);
        while let Some(mut joint) = iter.fetch_next() {
            let TypedJoint::RevoluteJoint(ref mut joint) = joint.data else {
                continue;
            };

            let target_pos = (-0.6 + (time.elapsed_secs() * 5.0).sin()) * 0.5;
            joint.set_motor_position(sign * target_pos, 2000.0, 30.0);
            sign *= -1.0;
        }
    }
}

fn lift(
    wings: Query<(&Velocity, &Parent), With<Wing>>,
    bats: Query<&Velocity, With<Bat>>,
    mut commands: Commands,
) {
    for (wing, parent) in wings.iter() {
        let relvel = wing.linvel - bats.get(parent.get()).unwrap().linvel;
        commands.entity(parent.get()).insert(ExternalImpulse {
            impulse: Vec2::Y * relvel.dot(Vec2::Y) * 100.0,
            ..Default::default()
        });
    }
}

fn pathfind(
    bats: Query<(Entity, &Transform), With<Bat>>,
    mut commands: Commands,
    dmap: Single<&DMap>,
) {
    for (bat, trans) in bats.iter() {
        let coord = TilePos::from_world_pos(
            &trans.translation.truncate(),
            &TilemapSize::new(256, 256),
            &TilemapTileSize::new(12.0, 12.0).into(),
            &TilemapType::default(),
        )
        .unwrap();
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
        let min = min - coord;
        let force = Vec2::new(min.x as f32, min.y as f32) * 300.0;
        debug!("pathfinding force: {:?}", force);
        commands.entity(bat).insert(ExternalImpulse {
            impulse: force,
            ..Default::default()
        });
    }
}
