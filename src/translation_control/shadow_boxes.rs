use bevy::{
    color::palettes::tailwind::{BLUE_600, GREEN_600, RED_600},
    prelude::*,
};

use crate::{
    bezier_curve::render_info::RenderInformation,
    translation_control::translation_controller::{AssignedShadowMarkers, ShadowMarker},
};

#[derive(Component)]
#[relationship(relationship_target = AssignedShadowBoxes)]
pub struct ShadowBox(Entity);

#[derive(Component)]
#[relationship_target(relationship = ShadowBox, linked_spawn)]
pub struct AssignedShadowBoxes(Vec<Entity>);

impl AssignedShadowBoxes {
    pub fn entities(&self) -> &Vec<Entity> {
        &self.0
    }
}

pub enum ShadowBoxCylinderAlignment {
    X,
    Y,
    Z,
}

#[derive(Component)]
pub struct ShadowBoxCylinder(ShadowBoxCylinderAlignment);

pub fn generate_shadow_box_bundle(
    diff: Vec3,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    meshes: &mut ResMut<Assets<Mesh>>,
    parent: Entity,
    scale: f32,
) -> impl Bundle {
    let red = materials.add(Color::from(RED_600));
    let green = materials.add(Color::from(GREEN_600));
    let blue = materials.add(Color::from(BLUE_600));
    let x_cylinder = meshes.add(Cylinder::new(0.005 * scale, diff.x));
    let y_cylinder = meshes.add(Cylinder::new(0.005 * scale, diff.y));
    let z_cylinder = meshes.add(Cylinder::new(0.005 * scale, diff.z));

    let x_cylinder_bundle = |diff: Vec3| {
        (
            Mesh3d(x_cylinder.clone()),
            MeshMaterial3d(red.clone()),
            Transform::from_xyz(0.0, 0.0, -diff.x.abs() / 2.0)
                .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
            ShadowBoxCylinder(ShadowBoxCylinderAlignment::X),
        )
    };

    let y_cylinder_bundle = |diff: Vec3| {
        (
            Mesh3d(y_cylinder.clone()),
            MeshMaterial3d(green.clone()),
            Transform::from_xyz(0.0, 0.0, -diff.y.abs() / 2.0)
                .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
            ShadowBoxCylinder(ShadowBoxCylinderAlignment::Y),
        )
    };

    let z_cylinder_bundle = |diff: Vec3| {
        (
            Mesh3d(z_cylinder.clone()),
            MeshMaterial3d(blue.clone()),
            Transform::from_xyz(0.0, 0.0, -diff.z.abs() / 2.0)
                .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
            ShadowBoxCylinder(ShadowBoxCylinderAlignment::Z),
        )
    };
    (
        ShadowBox(parent),
        ChildOf(parent),
        Visibility::Inherited,
        Transform::default(),
        children![
            // X lines
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::new(diff.x, 0.0, 0.0), Vec3::Y),
                children![x_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, 0.0, diff.z)
                    .looking_to(Vec3::new(diff.x, 0.0, 0.0), Vec3::Y),
                children![x_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, diff.y, 0.0)
                    .looking_to(Vec3::new(diff.x, 0.0, 0.0), Vec3::Y),
                children![x_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, diff.y, diff.z)
                    .looking_to(Vec3::new(diff.x, 0.0, 0.0), Vec3::Y),
                children![x_cylinder_bundle(diff)]
            ),
            // Y lines
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::new(0.0, diff.y, 0.0), Vec3::X),
                children![y_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, 0.0, diff.z)
                    .looking_to(Vec3::new(0.0, diff.y, 0.0), Vec3::X),
                children![y_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(diff.x, 0.0, 0.0)
                    .looking_to(Vec3::new(0.0, diff.y, 0.0), Vec3::X),
                children![y_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(diff.x, 0.0, diff.z)
                    .looking_to(Vec3::new(0.0, diff.y, 0.0), Vec3::X),
                children![y_cylinder_bundle(diff)]
            ),
            // Z lines
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, 0.0, 0.0).looking_to(Vec3::new(0.0, 0.0, diff.z), Vec3::Y),
                children![z_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(diff.x, 0.0, 0.0)
                    .looking_to(Vec3::new(0.0, 0.0, diff.z), Vec3::Y),
                children![z_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(0.0, diff.y, 0.0)
                    .looking_to(Vec3::new(0.0, 0.0, diff.z), Vec3::Y),
                children![z_cylinder_bundle(diff)]
            ),
            (
                Visibility::Inherited,
                Transform::from_xyz(diff.x, diff.y, 0.0)
                    .looking_to(Vec3::new(0.0, 0.0, diff.z), Vec3::Y),
                children![z_cylinder_bundle(diff)]
            )
        ],
    )
}

pub fn update_boxes(
    mut commands: Commands,
    boxes: Query<(Entity, &ChildOf), With<ShadowBox>>,
    assigned_shadows: Query<&AssignedShadowMarkers>,
    transforms: Query<&Transform>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    info: Res<RenderInformation>,
) {
    for (box_entity, child_of) in boxes {
        if let Ok(assigned_shadows) = assigned_shadows.get(child_of.parent())
            && let Some(shadow_entity) = assigned_shadows.entities().first().copied()
        {
            let parent_translation = transforms.get(child_of.parent()).unwrap().translation;
            let shadow_translation = transforms.get(shadow_entity).unwrap().translation;
            let diff = shadow_translation - parent_translation;

            commands.entity(box_entity).despawn();

            commands.spawn(generate_shadow_box_bundle(
                diff,
                &mut materials,
                &mut meshes,
                child_of.parent(),
                info.scale,
            ));
        }
    }
}
