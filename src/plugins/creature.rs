use crate::prelude::*;

pub fn creature_plugin(app: &mut App) {
    app.add_systems(Startup, setup);
}

#[derive(Component)]
#[require(
    Transform,
    RigidBody(|| RigidBody::Dynamic),
    Collider(|| Collider::ball(1.5)),
    ColliderMassProperties(|| ColliderMassProperties::Density(1.2)),
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
                Transform::from_xyz(-15.0, 0.0, 0.0),
                ImpulseJoint::new(parent.parent_entity(), joint_a),
            ));
            parent.spawn((
                collider,
                RigidBody::Dynamic,
                Transform::from_xyz(15.0, 0.0, 0.0),
                ImpulseJoint::new(parent.parent_entity(), joint_b),
            ));
        });
    });
}
