use crate::prelude::*;

pub fn creature_plugin(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(FixedUpdate, flap);
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
