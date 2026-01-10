use crate::{
    RootTransform,
    bezier_curve::render_info::RenderInformation,
    picking3d::{self, events::Pointer3d, picking_3d::Picking3dInteractable},
    translation_control::translation_controller::{
        CantSnapToCurve, CantSnapToEntities, EnableTranslationControl,
        EnableTranslationControlType, MovedEntityEvent,
    },
};
use bevy::prelude::*;

#[derive(Component)]
pub struct AlignmentCenterMarker;

#[derive(Component)]
pub enum EnabledAlignmentMode {
    Move,
    Rotate,
}

#[derive(Event)]
pub struct RecreateAlignmentChildren;

#[derive(Component)]
pub struct CrossManipulatorMarker;

#[derive(Component)]
pub struct CrossOriginMarker {
    actual_origin: Entity,
}

#[derive(Component)]
pub struct CrossBridge(Entity, Entity);

pub fn create_alignment_sphere(
    mut commands: Commands,
    root_transform: Query<Entity, With<RootTransform>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    render_info: Res<RenderInformation>,
) {
    if let Ok(root_entity) = root_transform.single() {
        let mut mat: StandardMaterial = Color::Srgba(Srgba {
            red: 206.0 / 255.0,
            green: 132.0 / 255.0,
            blue: 62.0 / 255.0,
            alpha: 1.0,
        })
        .into();

        mat.metallic = 1.0;

        let mat_handle = materials.add(mat);
        let mesh_handle = meshes.add(Sphere::new(0.1 * render_info.scale));

        commands
            .spawn((
                Transform::default(),
                // ChildOf(root_entity),
                AlignmentCenterMarker,
                MeshMaterial3d(mat_handle),
                Mesh3d(mesh_handle),
                Picking3dInteractable::default(),
                EnabledAlignmentMode::Move,
                EnableTranslationControl::new_without_root(
                    EnableTranslationControlType::OnlyTranslation,
                ),
                CantSnapToEntities::All,
                CantSnapToCurve::All,
            ))
            .observe(handle_click_on_alignment)
            .observe(handle_click_on_alignment3d)
            .observe(handle_rebuild);
    }
}

#[allow(clippy::complexity)]
pub fn delete_alignment_sphere(
    mut commands: Commands,
    mut root_transform: Query<
        &mut Transform,
        (With<RootTransform>, Without<AlignmentCenterMarker>),
    >,
    spheres: Query<
        (Entity, &GlobalTransform),
        (With<AlignmentCenterMarker>, Without<RootTransform>),
    >,
) {
    let Ok(mut root) = root_transform.single_mut() else {
        return;
    };

    for sphere in &spheres {
        let _ = commands
            .get_entity(sphere.0)
            .map(|mut entity| entity.despawn());

        *root = sphere.1.compute_transform();
    }
}

fn handle_click_on_alignment(
    trigger: Trigger<Pointer<Click>>,
    mut alignments: Query<&mut EnabledAlignmentMode, With<AlignmentCenterMarker>>,
    mut commands: Commands,
) {
    if let Ok(mut alignment) = alignments.get_mut(trigger.target()) {
        *alignment = match *alignment {
            EnabledAlignmentMode::Move => EnabledAlignmentMode::Rotate,
            EnabledAlignmentMode::Rotate => EnabledAlignmentMode::Move,
        };

        let _ = commands.get_entity(trigger.target()).map(|mut e| {
            e.trigger(RecreateAlignmentChildren);
        });
    }
}

fn handle_click_on_alignment3d(
    trigger: Trigger<Pointer3d<picking3d::events::Click>>,
    mut alignments: Query<&mut EnabledAlignmentMode, With<AlignmentCenterMarker>>,
    mut commands: Commands,
) {
    if let Ok(mut alignment) = alignments.get_mut(trigger.target()) {
        *alignment = match *alignment {
            EnabledAlignmentMode::Move => EnabledAlignmentMode::Rotate,
            EnabledAlignmentMode::Rotate => EnabledAlignmentMode::Move,
        };

        let _ = commands.get_entity(trigger.target()).map(|mut e| {
            e.trigger(RecreateAlignmentChildren);
        });
    }
}

fn handle_rebuild(
    trigger: Trigger<RecreateAlignmentChildren>,
    mut commands: Commands,
    alignments: Query<(&EnabledAlignmentMode, &Children), With<AlignmentCenterMarker>>,
    rotate_controller: Query<Entity, With<CrossManipulatorMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    render_info: Res<RenderInformation>,
) {
    if let Ok((alignment, children)) = alignments.get(trigger.target()) {
        // First, respawn the rotate controls. Since those are dependant on the children,
        // and use a different entity as a EntityCommands source, we need to split the code here
        for child in children {
            if rotate_controller.get(*child).is_ok() {
                let _ = commands.get_entity(*child).map(|mut e| e.despawn());
            }
        }
        // When we no longer need the entity_cmds from the previous loc's, we can get the actual entity commands
        if let Ok(mut entity_cmds) = commands.get_entity(trigger.target()) {
            match alignment {
                EnabledAlignmentMode::Move => {
                    entity_cmds.insert(EnableTranslationControl::new_without_root(
                        EnableTranslationControlType::OnlyTranslation,
                    ));
                }
                EnabledAlignmentMode::Rotate => {
                    entity_cmds.remove::<EnableTranslationControl>();
                    entity_cmds.with_children(|spawner| {
                        spawner
                            .spawn((
                                Name::new("CrossManipulatorMarker"),
                                CrossManipulatorMarker,
                                Transform::default(),
                                Visibility::Inherited,
                            ))
                            .with_children(|spawner| {
                                let actual_origin = spawner.spawn(Transform::default()).id();
                                let cross_origin = spawner
                                    .spawn((
                                        Name::new("CrossOriginMarker"),
                                        Transform::from_xyz(0.0, 0.0, 1.0),
                                        CrossOriginMarker { actual_origin },
                                        Visibility::Inherited,
                                        CantSnapToEntities::All,
                                        CantSnapToCurve::All,
                                        EnableTranslationControl::new_without_root(
                                            EnableTranslationControlType::OnlyTranslation,
                                        )
                                        .hide_lines(),
                                    ))
                                    .observe(handle_moved_trigger)
                                    .id();

                                let black = materials.add(Color::BLACK);
                                let cylinder =
                                    meshes.add(Cylinder::new(0.05 * render_info.scale, 1.0));
                                spawner.spawn((
                                    Transform::default(),
                                    CrossBridge(actual_origin, cross_origin),
                                    Mesh3d(cylinder),
                                    MeshMaterial3d(black),
                                ));

                                // Inverse direction
                                let actual_origin = spawner.spawn(Transform::default()).id();
                                let cross_origin = spawner
                                    .spawn((
                                        Name::new("CrossOriginMarker"),
                                        Transform::from_xyz(0.0, 0.0, -1.0),
                                        CrossOriginMarker { actual_origin },
                                        Visibility::Inherited,
                                        CantSnapToEntities::All,
                                        CantSnapToCurve::All,
                                        EnableTranslationControl::new_without_root(
                                            EnableTranslationControlType::OnlyTranslation,
                                        )
                                        .invert()
                                        .hide_lines(),
                                    ))
                                    .observe(handle_moved_trigger)
                                    .id();

                                let black = materials.add(Color::BLACK);
                                let cylinder =
                                    meshes.add(Cylinder::new(0.05 * render_info.scale, 1.0));
                                spawner.spawn((
                                    Transform::default(),
                                    CrossBridge(actual_origin, cross_origin),
                                    Mesh3d(cylinder),
                                    MeshMaterial3d(black),
                                ));

                                // Inverse direction
                                let actual_origin = spawner.spawn(Transform::default()).id();
                                let cross_origin = spawner
                                    .spawn((
                                        Name::new("CrossOriginMarker"),
                                        Transform::from_xyz(1.0, 0.0, 0.0),
                                        CrossOriginMarker { actual_origin },
                                        Visibility::Inherited,
                                        CantSnapToEntities::All,
                                        CantSnapToCurve::All,
                                        EnableTranslationControl::new_without_root(
                                            EnableTranslationControlType::OnlyTranslation,
                                        )
                                        .hide_lines(),
                                    ))
                                    .observe(handle_moved_trigger)
                                    .id();

                                let black = materials.add(Color::BLACK);
                                let cylinder =
                                    meshes.add(Cylinder::new(0.05 * render_info.scale, 1.0));
                                spawner.spawn((
                                    Transform::default(),
                                    CrossBridge(actual_origin, cross_origin),
                                    Mesh3d(cylinder),
                                    MeshMaterial3d(black),
                                ));

                                // Inverse direction
                                let actual_origin = spawner.spawn(Transform::default()).id();
                                let cross_origin = spawner
                                    .spawn((
                                        Name::new("CrossOriginMarker"),
                                        Transform::from_xyz(-1.0, 0.0, 0.0),
                                        CrossOriginMarker { actual_origin },
                                        Visibility::Inherited,
                                        CantSnapToEntities::All,
                                        CantSnapToCurve::All,
                                        EnableTranslationControl::new_without_root(
                                            EnableTranslationControlType::OnlyTranslation,
                                        )
                                        .invert()
                                        .hide_lines(),
                                    ))
                                    .observe(handle_moved_trigger)
                                    .id();

                                let black = materials.add(Color::BLACK);
                                let cylinder =
                                    meshes.add(Cylinder::new(0.05 * render_info.scale, 1.0));
                                spawner.spawn((
                                    Transform::default(),
                                    CrossBridge(actual_origin, cross_origin),
                                    Mesh3d(cylinder),
                                    MeshMaterial3d(black),
                                ));
                            });
                    });
                }
            }
        }
    }
}

pub fn update_cross_bridge(
    transforms: Query<&Transform, Without<CrossBridge>>,
    cross_bridges: Query<(&CrossBridge, &mut Mesh3d, &mut Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    render_info: Res<RenderInformation>,
) {
    for (bridge, mut mesh3d, mut transform) in cross_bridges {
        let (a, b) = (bridge.0, bridge.1);

        if let Ok(a_transform) = transforms.get(a)
            && let Ok(b_transform) = transforms.get(b)
        {
            let a_transform = a_transform.translation;
            let b_transform = b_transform.translation;
            let diff = a_transform - b_transform;
            let diff_length = diff.length();

            let mid = a_transform.midpoint(b_transform);

            transform.translation = mid;
            transform.align(Vec3::Y, diff, Vec3::NEG_Z, Vec3::Y);

            let mesh = meshes.add(Cylinder::new(0.05 * render_info.scale, diff_length));
            mesh3d.0 = mesh;
        }
    }
}

pub fn handle_moved_trigger(
    trigger: Trigger<MovedEntityEvent>,
    child_of: Query<&ChildOf>,
    mut transforms: Query<&mut Transform>,
    cross_origin_markers: Query<&CrossOriginMarker>,
) {
    let cross_origin_maker_entity = trigger.target();

    let Ok(cross_origin_marker) = cross_origin_markers.get(trigger.target()) else {
        return;
    };
    let Ok(cross_manipulator_marker) = child_of.get(cross_origin_maker_entity).map(|e| e.parent())
    else {
        return;
    };
    let Ok(alignment_manipulator) = child_of.get(cross_manipulator_marker).map(|e| e.parent())
    else {
        return;
    };

    let delta = trigger.delta;
    let mut origin = Vec3::ZERO;
    let mut old_pos = Vec3::ZERO;
    let mut new_pos = Vec3::ZERO;

    let _ = transforms
        .get_mut(cross_origin_maker_entity)
        .map(|mut trans| {
            trans.translation -= delta; // Reset transform
            old_pos = trans.translation;
            new_pos = trans.translation + delta;
            trans.translation.z += delta.z;
        });

    let _ = transforms
        .get(cross_origin_marker.actual_origin)
        .map(|trans| {
            origin = trans.translation;
        });

    let _ = transforms.get_mut(alignment_manipulator).map(|mut trans| {
        let diff_old_pos = old_pos - origin;
        let diff_new_pos = new_pos - origin;

        let angle = diff_old_pos.angle_between(diff_new_pos);
        let normal = diff_old_pos.cross(diff_new_pos).normalize_or_zero();
        info!("old: {origin}, new: {new_pos}");
        info!("diff_old: {diff_old_pos}, diff_new: {diff_new_pos}");
        info!("angle: {angle}, normal: {normal}");
        if !angle.is_nan() {
            trans.rotation *= Quat::from_axis_angle(normal, angle);
        }
    });
}
