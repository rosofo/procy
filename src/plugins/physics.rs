use crate::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AddForces;

pub fn physics_plugin(app: &mut App) {
    app.add_systems(FixedUpdate, reset_forces.before(AddForces));
    app.configure_sets(FixedUpdate, AddForces);
}

#[derive(Component)]
pub struct KeepForces;

#[allow(clippy::type_complexity)]
fn reset_forces(
    mut entities: Query<
        (Option<&mut ExternalForce>, Option<&mut ExternalImpulse>),
        (Without<KeepForces>,),
    >,
) {
    for entity in entities.iter_mut() {
        if let Some(mut force) = entity.0 {
            force.force = Vec2::ZERO;
        }

        if let Some(mut impulse) = entity.1 {
            impulse.impulse = Vec2::ZERO;
        }
    }
}
