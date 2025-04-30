use bevy::asset::Handle;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{Query, Trigger};

pub fn update_material_on<E>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(Trigger<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    move |trigger, mut query| {
        if let Ok(mut material) = query.get_mut(trigger.entity()) {
            material.0 = new_material.clone();
        }
    }
}
