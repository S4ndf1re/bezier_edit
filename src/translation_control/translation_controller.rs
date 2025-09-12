use crate::bezier_curve::bezier_curve_renderer::{RedrawEvent, hover_3d};
use crate::bezier_curve::helper_curves::{CurveCollection, RedrawCurvesEvent};
use crate::bezier_curve::render_info::RenderInformation;
use crate::history::plugin::HistoryLogEvent;
use crate::nurbs::bezier::{de_casteljau, derive_after_de_casteljau};
use crate::nurbs::parametric::{Circle3D, MinDistanceToPoint, Parametric};
use crate::picking3d::events::{HoveredBy, MoveIn, MoveOut, Pointer3d};
use crate::picking3d::picking_3d::{CustomPicking3dHitbox, Picking3dInteractable};
use crate::translation_control::control_storage::ControlStorage;
use crate::util::update_material_on;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use crate::{MainCamera, RootTransform};
use bevy::color::palettes::tailwind::{BLUE_600, BLUE_800, GRAY_500, RED_600, RED_800};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::gizmos::start_gizmo_context;
use bevy::math::bounding::BoundingSphere;
use bevy::math::ops::atan2;
use bevy::prelude::*;
use bevy_lunex::prelude::{Text3d, Text3dStyling, TextAlign, TextAtlas, Weight};
use bevy_xr_utils::tracking_utils::XrTrackedView;
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use super::accumulated::AccumulatedMovementStore;
use super::control_storage::ControlDirection;
use super::obligatory_drag_params::ObligatoryDragParams;

#[derive(Component)]
pub struct CoordinateTextMarker;

#[derive(Event)]
pub struct MovedEntityEvent {
    pub entity: Entity,
    pub delta: Vec3,
}

#[derive(Event)]
pub struct MoveEntityByDeltaEvent {
    pub delta: Vec3,
    pub entity: Entity,
}

#[derive(Component, Clone, Copy)]
pub enum SnappedPoint {
    ToCurve { u: f64, curve: Entity },
    ToProjection,
}

#[derive(Component)]
pub struct ShadowMarker;

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum EnableTranslationControl {
    #[default]
    OnlyTranslation,
    WithRotation,
    OnlyOnPlane(Entity),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArrowDirection {
    Up,
    Left,
}
#[derive(Component)]
pub struct OnPlaneMovableMarker {
    pub plane: Entity,
    pub arrow_direction: ArrowDirection,
}

#[derive(Component, Clone, Copy)]
pub struct SnappedArrow;

#[derive(Component)]
pub struct ControlParent(pub Entity);

#[derive(Component)]
pub struct Control(pub Vec3);

#[derive(Component)]
pub struct ControlRotation {
    pub normal: Vec3,
    pub radius: f64,
    pub last_vector: Vec3,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnappingBehaviour {
    NoSnap,
    #[default]
    Snap,
}

#[derive(Default, Clone, Copy)]
pub enum StepMode {
    #[default]
    None,
    MM10,
    MM5,
    MM1,
}

impl StepMode {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::MM10,
            Self::MM10 => Self::MM5,
            Self::MM5 => Self::MM1,
            Self::MM1 => Self::None,
        }
    }
}

#[derive(Resource, Default)]
pub struct TranslationControllerState {
    pub curve_snapping: SnappingBehaviour,
    pub step_mode: StepMode,
}

#[derive(Event)]
pub struct ToggleSnappingBehaviour;

#[derive(Component, Clone, Default)]
pub enum CantSnapToCurve {
    #[default]
    None,
    All,
    Single(Entity),
    #[allow(unused)]
    Multiple(HashSet<Entity>),
}

#[derive(Component, Clone, Default)]
pub enum CantSnapToEntities {
    #[default]
    None,
    All,
    Single(Entity),
    Multiple(HashSet<Entity>),
}

fn handle_toggle_snapping(
    mut reader: EventReader<ToggleSnappingBehaviour>,
    mut state: ResMut<TranslationControllerState>,
    mut commands: Commands,
    snapped: Query<Entity, With<SnappedPoint>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    state.curve_snapping = match state.curve_snapping {
        SnappingBehaviour::NoSnap => SnappingBehaviour::Snap,
        SnappingBehaviour::Snap => {
            for snap in snapped {
                commands.get_entity(snap).unwrap().remove::<SnappedPoint>();
            }
            SnappingBehaviour::NoSnap
        }
    };
}

fn register_deletes(
    mut commands: Commands,
    mut deleted: RemovedComponents<EnableTranslationControl>,
    mut controls: Query<(Entity, &mut Visibility, &ControlParent)>,
    children: Query<&Children>,
    mut picking3d_interactable: Query<&mut Picking3dInteractable>,
) {
    for event in deleted.read() {
        for (entity, mut visibility, contrl) in controls.iter_mut() {
            if contrl.0 == event {
                *visibility = Visibility::Hidden;
                commands.entity(entity).insert(Pickable::IGNORE);

                for child in children.iter_descendants(event) {
                    if let Ok(mut pickable) = picking3d_interactable.get_mut(child) {
                        *pickable = Picking3dInteractable::Ignore;
                    }
                }
            }
        }
    }
}

pub fn draw_plane(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
) {
    let plane = meshes.add(Cuboid::new(0.24 * scale, 0.24 * scale, 0.01 * scale));

    let mut obj = child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        MeshMaterial3d(mat.clone()),
        Mesh3d(plane.clone()),
        Picking3dInteractable::Default,
    ));
    obj.observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()))
        .observe(hover_3d);
}

pub fn draw_arrow(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
    is_shadow: bool,
) {
    let cuboid = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.4 * scale));
    let line = meshes.add(Cuboid::new(0.02 * scale, 0.02 * scale, 0.8 * scale));
    let arrow = meshes.add(Cone::new(0.035 * scale, 0.2 * scale));

    let mut obj = child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, -0.4 * scale),
        MeshMaterial3d(mat.clone()),
        Mesh3d(cuboid.clone()),
        Picking3dInteractable::Default,
    ));
    obj.observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()));
    if !is_shadow {
        obj.observe(hover_3d);
    }

    child_builder
        .spawn((
            Transform::from_xyz(0.0, 0.0, -0.4 * scale),
            MeshMaterial3d(mat.clone()),
            Mesh3d(line.clone()),
        ))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()));

    let mut transform = Transform::from_xyz(0.0, 0.0, -0.9 * scale);
    transform.rotate_x(-FRAC_PI_2);
    child_builder
        .spawn((
            transform,
            MeshMaterial3d(mat.clone()),
            Mesh3d(arrow.clone()),
        ))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()));
}

fn draw_ring(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
) {
    // this is a little smaller then the arrow
    let torus = meshes.add(Torus::new(0.38 * scale, 0.42 * scale));
    let ball = meshes.add(Sphere::new(0.035 * scale));

    let ball_positions = [
        Vec3::new(-0.4, 0.0, 0.0),
        Vec3::new(0.0, -0.4, 0.0),
        Vec3::new(0.4, 0.0, 0.0),
        Vec3::new(0.0, 0.4, 0.0),
    ];

    let angle: f32 = 90.0;
    let angle = angle.to_radians();
    let mut transform = Transform::default();
    // NOTE: For some reason, this is oriented in Vec3::Y Direction, instead of Vec3::NEG_Z
    transform.rotate(Quat::from_axis_angle(Vec3::X, angle));

    child_builder
        .spawn((transform, Mesh3d(torus), MeshMaterial3d(mat.clone())))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()));

    let angle: f32 = 45.0;
    let angle = angle.to_radians();
    for pos in ball_positions {
        let mut transform = Transform::from_translation(pos * scale);
        transform.rotate_around(Vec3::default(), Quat::from_axis_angle(Vec3::NEG_Z, angle));
        child_builder
            .spawn((
                transform,
                Mesh3d(ball.clone()),
                MeshMaterial3d(mat.clone()),
                CustomPicking3dHitbox::Sphere(0.035 * scale),
                Picking3dInteractable::Default,
            ))
            .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer<Out>>(mat.clone()))
            .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()))
            .observe(hover_3d);
    }
}

#[allow(clippy::complexity)]
fn show_transitional_controls(
    mut commands: Commands,
    to_enable: Query<(Entity, &EnableTranslationControl), Added<EnableTranslationControl>>,
    transforms: Query<&Transform>,
    arrows: Res<ControlStorage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    scale: Res<RenderInformation>,
    mut already_existing: Query<(&mut Visibility, &ControlParent)>,
    children: Query<&Children>,
    mut picking3d_interactable: Query<&mut Picking3dInteractable>,
) {
    let scale = scale.scale;

    for (entity, enabled_control) in to_enable.iter() {
        let mut already_created = false;
        for (mut visibility, parent) in already_existing.iter_mut() {
            if parent.0 == entity {
                *visibility = Visibility::Inherited;
                already_created = true;
                commands.entity(entity).remove::<Pickable>();
                for child in children.iter_descendants(entity) {
                    if let Ok(mut pickable3d) = picking3d_interactable.get_mut(child) {
                        *pickable3d = Picking3dInteractable::Default;
                    }
                }
                break;
            }
        }
        if already_created {
            continue;
        }

        let parent_transform = transforms.get(entity).unwrap();
        let rotation_inverse = parent_transform.rotation.inverse();

        let transform = Transform::from_xyz(0.0, 0.0, 0.0).with_rotation(rotation_inverse);

        commands.get_entity(entity).unwrap().with_children(|cmd| {
            cmd.spawn((ControlParent(entity), transform, Visibility::default()))
                .with_children(|parent| {
                    if *enabled_control == EnableTranslationControl::OnlyTranslation
                        || *enabled_control == EnableTranslationControl::WithRotation
                    {
                        for arrow in arrows.as_ref().iter_arrows() {
                            parent
                                .spawn((
                                    Transform::from_xyz(0.0, 0.0, 0.0)
                                        .looking_to(arrow.normalized, Vec3::Y),
                                    Control(arrow.normalized),
                                    Visibility::default(),
                                ))
                                .with_children(|parent| {
                                    draw_arrow(
                                        parent,
                                        materials.add(arrow.color),
                                        materials.add(arrow.hover_color),
                                        &mut meshes,
                                        scale,
                                        false,
                                    );
                                })
                                .observe(drag_controller)
                                .observe(drag_controller3d)
                                .observe(drag_start)
                                .observe(drag_start3d)
                                .observe(drag_end_trigger_redraw)
                                .observe(drag_end3d_trigger_redraw);

                            if *enabled_control == EnableTranslationControl::WithRotation {
                                parent
                                    .spawn((
                                        Transform::from_xyz(0.0, 0.0, 0.0)
                                            .looking_to(arrow.normalized, Vec3::Y),
                                        ControlRotation {
                                            normal: arrow.normalized,
                                            radius: 0.4 * scale as f64,
                                            last_vector: Vec3::ZERO,
                                        },
                                        Visibility::default(),
                                    ))
                                    .with_children(|parent| {
                                        draw_ring(
                                            parent,
                                            materials.add(arrow.color),
                                            materials.add(arrow.hover_color),
                                            &mut meshes,
                                            scale,
                                        );
                                    })
                                    .observe(rotate_start)
                                    .observe(rotate_start3d)
                                    .observe(rotate_controller)
                                    .observe(rotate_controller3d)
                                    .observe(rotate_end_trigger_redraw)
                                    .observe(rotate_end_trigger_redraw3d);
                            }
                        }

                        for plane in arrows.iter_planes() {
                            parent
                                .spawn((
                                    Transform::from_translation((plane.axis / 3.0) * scale)
                                        .looking_to(plane.normal, Vec3::Y),
                                    Control(plane.axis),
                                    Visibility::default(),
                                ))
                                .with_children(|parent| {
                                    draw_plane(
                                        parent,
                                        materials.add(plane.color),
                                        materials.add(plane.hover_color),
                                        &mut meshes,
                                        scale,
                                    );
                                })
                                .observe(drag_plane)
                                .observe(drag_plane3d)
                                .observe(drag_start)
                                .observe(drag_start3d)
                                .observe(drag_end_trigger_redraw)
                                .observe(drag_end3d_trigger_redraw);
                        }
                    } else if let EnableTranslationControl::OnlyOnPlane(plane_entity) =
                        *enabled_control
                        && let Ok(plane_transform) = transforms.get(plane_entity)
                    {
                        let up_direction = ControlDirection::new(
                            plane_transform.up().as_vec3(),
                            Color::from(BLUE_600),
                            Color::from(BLUE_800),
                            Color::from(GRAY_500),
                            false,
                        );

                        let left_direction = ControlDirection::new(
                            plane_transform.up().as_vec3(),
                            Color::from(RED_600),
                            Color::from(RED_800),
                            Color::from(GRAY_500),
                            false,
                        );

                        for (arrow, direction) in [
                            (up_direction, ArrowDirection::Up),
                            (left_direction, ArrowDirection::Left),
                        ] {
                            parent
                                .spawn((
                                    Transform::from_xyz(0.0, 0.0, 0.0)
                                        .looking_to(arrow.normalized, Vec3::Y),
                                    Control(arrow.normalized),
                                    Visibility::default(),
                                    OnPlaneMovableMarker {
                                        plane: plane_entity,
                                        arrow_direction: direction,
                                    },
                                ))
                                .with_children(|parent| {
                                    draw_arrow(
                                        parent,
                                        materials.add(arrow.color),
                                        materials.add(arrow.hover_color),
                                        &mut meshes,
                                        scale,
                                        false,
                                    );
                                })
                                .observe(drag_controller)
                                .observe(drag_controller3d)
                                .observe(drag_start)
                                .observe(drag_start3d)
                                .observe(drag_end_trigger_redraw)
                                .observe(drag_end3d_trigger_redraw);
                        }
                    }
                });
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn drag_start(
    trigger: Trigger<Pointer<DragStart>>,
    root: Query<(Entity, &Transform), With<RootTransform>>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    mut commands: Commands,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    all_transforms: Query<&Transform, (Without<ControlParent>, Without<RootTransform>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    arrows: Res<ControlStorage>,
    scale: Res<RenderInformation>,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
) {
    let (root, root_transform) = root.single().unwrap();
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();

    let control_parent = control_parents.get(dragged_parent).unwrap();
    let mut start_transform = *all_transforms.get(control_parent.0).unwrap();
    start_transform.rotation = Quat::IDENTITY;

    accumulated_movement.start_movement_entity(control_parent.0, start_transform.translation);

    let scale = scale.scale;

    commands
        .spawn((
            ShadowMarker,
            start_transform,
            Visibility::default(),
            ChildOf(root),
        ))
        .with_children(|parent| {
            for arrow in arrows.as_ref().iter_arrows() {
                parent
                    .spawn((
                        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(arrow.normalized, Vec3::Y),
                        Control(arrow.normalized),
                        Picking3dInteractable::default(),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        draw_arrow(
                            parent,
                            materials.add(arrow.shadow_color),
                            materials.add(arrow.shadow_color),
                            &mut meshes,
                            scale,
                            true,
                        );
                    });
            }
        });

    let start_transform = *all_transforms.get(control_parent.0).unwrap();
    let camera_transform = camera.single().unwrap();
    let camera_forward = root_transform
        .compute_affine()
        .inverse()
        .transform_vector3(camera_transform.forward().normalize_or_zero());

    commands.spawn((
        ChildOf(control_parent.0),
        Transform::from_rotation(start_transform.rotation.inverse()),
        CoordinateTextMarker,
        children![(
            Transform::from_translation(-camera_forward * 1.0 * scale + Vec3::Y * scale)
                .looking_to(camera_forward, Vec3::Y)
                .with_scale(Vec3::ONE * 0.0025 * scale),
            Text3d::new(format!(
                "({:.3}, {:.3}, {:.3})",
                start_transform.translation.x / scale,
                start_transform.translation.y / scale,
                start_transform.translation.z / scale
            )),
            Text3dStyling {
                size: 64.0,
                color: Srgba::new(0., 0., 0., 1.),
                align: TextAlign::Center,
                font: Arc::from("Rajdhani"),
                weight: Weight::BOLD,
                ..Default::default()
            },
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..Default::default()
            })),
            Mesh3d::default(),
        )],
    ));

    history.write(HistoryLogEvent::Begin(
        control_parent.0,
        Some(start_transform),
    ));
}

#[allow(clippy::too_many_arguments)]
fn drag_start3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::DragStart>>,
    root: Query<(Entity, &Transform), With<RootTransform>>,
    mut commands: Commands,
    camera: Query<&GlobalTransform, With<XrTrackedView>>,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    all_transforms: Query<&Transform, (Without<ControlParent>, Without<RootTransform>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    arrows: Res<ControlStorage>,
    scale: Res<RenderInformation>,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
) {
    let (root, root_transform) = root.single().unwrap();
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();

    let control_parent = control_parents.get(dragged_parent).unwrap();
    let mut start_transform = *all_transforms.get(control_parent.0).unwrap();
    start_transform.rotation = Quat::IDENTITY;

    accumulated_movement.start_movement_entity(control_parent.0, start_transform.translation);

    let scale = scale.scale;

    commands
        .spawn((
            ShadowMarker,
            start_transform,
            Visibility::default(),
            ChildOf(root),
        ))
        .with_children(|parent| {
            for arrow in arrows.as_ref().iter_arrows() {
                parent
                    .spawn((
                        Transform::from_xyz(0.0, 0.0, 0.0).looking_to(arrow.normalized, Vec3::Y),
                        Control(arrow.normalized),
                        Visibility::default(),
                    ))
                    .with_children(|parent| {
                        draw_arrow(
                            parent,
                            materials.add(arrow.shadow_color),
                            materials.add(arrow.shadow_color),
                            &mut meshes,
                            scale,
                            true,
                        );
                    });
            }
        });

    let start_transform = *all_transforms.get(control_parent.0).unwrap();
    let camera_transform = camera.single().unwrap();
    let camera_forward = root_transform
        .compute_affine()
        .inverse()
        .transform_vector3(camera_transform.forward().normalize_or_zero());

    commands.spawn((
        ChildOf(control_parent.0),
        Transform::from_rotation(start_transform.rotation.inverse()),
        CoordinateTextMarker,
        children![(
            Transform::from_translation(-camera_forward * 1.0 * scale + Vec3::Y * scale)
                .looking_to(camera_forward, Vec3::Y)
                .with_scale(Vec3::ONE * 0.0025 * scale),
            Text3d::new(format!(
                "({:.3}, {:.3}, {:.3})",
                start_transform.translation.x,
                start_transform.translation.y,
                start_transform.translation.z
            )),
            Text3dStyling {
                size: 64.0,
                color: Srgba::new(0., 0., 0., 1.),
                align: TextAlign::Center,
                font: Arc::from("Rajdhani"),
                weight: Weight::BOLD,
                ..Default::default()
            },
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..Default::default()
            })),
            Mesh3d::default(),
        )],
    ));

    history.write(HistoryLogEvent::Begin(
        control_parent.0,
        Some(start_transform),
    ));
}

#[allow(clippy::complexity)]
fn drag_end_trigger_redraw(
    trigger: Trigger<Pointer<DragEnd>>,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    mut commands: Commands,
    query: Query<Entity, With<ShadowMarker>>,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
    children: Query<&Children>,
    text_marker: Query<Entity, With<CoordinateTextMarker>>,
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    accumulated_movement.end_movement(&control_parent.0);

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    for entity in query {
        commands.get_entity(entity).unwrap().despawn();
    }

    // Despawn text
    for child in children.get(control_parent.0).unwrap() {
        if let Ok(text_entity) = text_marker.get(*child) {
            commands.entity(text_entity).despawn();
        }
    }

    history.write(HistoryLogEvent::End(control_parent.0, None));
}

#[allow(clippy::complexity)]
fn drag_end3d_trigger_redraw(
    trigger: Trigger<Pointer3d<crate::picking3d::events::DragEnd>>,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    mut commands: Commands,
    query: Query<Entity, With<ShadowMarker>>,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
    children: Query<&Children>,
    text_marker: Query<Entity, With<CoordinateTextMarker>>,
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    accumulated_movement.end_movement(&control_parent.0);

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    for entity in query {
        commands.get_entity(entity).unwrap().despawn();
    }

    // Despawn text
    for child in children.get(control_parent.0).unwrap() {
        if let Ok(text_entity) = text_marker.get(*child) {
            commands.entity(text_entity).despawn();
        }
    }

    history.write(HistoryLogEvent::End(control_parent.0, None));
}

pub fn handle_translate_by_delta_event(
    mut reader: EventReader<MoveEntityByDeltaEvent>,
    mut obligatory: ObligatoryDragParams,
    children: Query<&Children>,
    control_parents: Query<(Entity, &ControlParent)>,
) {
    for evt in reader.read() {
        if let Ok(children) = children.get(evt.entity) {
            for child in children {
                if let Ok(control_parent) = control_parents.get(*child) {
                    obligatory.update_position_drag_universal(
                        control_parent,
                        evt.delta,
                        Entity::PLACEHOLDER,
                    );
                }
            }
        }
    }
}

#[allow(clippy::complexity)]
pub fn drag_plane(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(ObligatoryDragParams, Query<&GlobalTransform>)>,
) {
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let diff = {
            let dist = (params.p1().get(control_parent.0).unwrap().translation()
                - camera_transform.translation())
            .length();

            let mouse_start = camera
                .viewport_to_world(
                    camera_transform,
                    trigger.pointer_location.position - trigger.delta,
                )
                .unwrap();

            let mouse_end = camera
                .viewport_to_world(camera_transform, trigger.pointer_location.position)
                .unwrap();

            let start = mouse_start.get_point(dist);
            let end = mouse_end.get_point(dist);
            root.single()
                .unwrap()
                .affine()
                .inverse()
                .transform_point3(end - start)
        };

        let axis = control.0;
        let translation = diff * axis;

        params.p0().update_position_drag_universal(
            (parent, control_parent),
            translation,
            control_entity,
        );
    }
}

#[allow(clippy::complexity)]
pub fn drag_plane3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ObligatoryDragParams,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let control_parent = control_parents.get_mut(parent).unwrap();

    let diff = trigger.event.delta;
    let diff = root
        .single()
        .unwrap()
        .affine()
        .inverse()
        .transform_point3(diff);

    let axis = control.0;
    let translation = axis * diff;

    params.update_position_drag_universal((parent, control_parent), translation, control_entity);
}

#[allow(clippy::complexity)]
pub fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(ObligatoryDragParams, Query<&GlobalTransform>)>,
) {
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let diff = {
            let dist = (params.p1().get(control_parent.0).unwrap().translation()
                - camera_transform.translation())
            .length();

            let mouse_start = camera
                .viewport_to_world(
                    camera_transform,
                    trigger.pointer_location.position - trigger.delta,
                )
                .unwrap();

            let mouse_end = camera
                .viewport_to_world(camera_transform, trigger.pointer_location.position)
                .unwrap();

            let start = mouse_start.get_point(dist);
            let end = mouse_end.get_point(dist);
            root.single()
                .unwrap()
                .affine()
                .inverse()
                .transform_point3(end - start)
        };

        let axis = control.0;
        let direction = (diff.dot(axis)) / (diff.length() * axis.length());
        let translation = axis * direction * diff.length();

        params.p0().update_position_drag_universal(
            (parent, control_parent),
            translation,
            control_entity,
        );
    }
}

#[allow(clippy::complexity)]
pub fn drag_controller3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ObligatoryDragParams,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let control_parent = control_parents.get_mut(parent).unwrap();

    let diff = trigger.event.delta;
    let diff = root
        .single()
        .unwrap()
        .affine()
        .inverse()
        .transform_point3(diff);

    let axis = control.0;
    let direction = (axis.dot(diff)) / (axis.length() * diff.length());
    let translation = axis * diff.length() * direction;

    params.update_position_drag_universal((parent, control_parent), translation, control_entity);
}

#[allow(clippy::complexity)]
fn rotate_start(
    trigger: Trigger<Pointer<DragStart>>,
    camera: Query<(&Camera, &GlobalTransform), (Without<RootTransform>, With<MainCamera>)>,
    mut control_query: Query<(&mut ControlRotation, &ChildOf)>,
    control_parents: Query<&ControlParent>,
    transforms: Query<&Transform, Without<RootTransform>>,
    root: Query<&Transform, With<RootTransform>>,
) {
    let (mut control_rotation, child_of) = control_query.get_mut(trigger.target()).unwrap();
    let parent = child_of.parent();
    let control_parent = control_parents.get(parent).unwrap();
    let parent_transform = *transforms.get(control_parent.0).unwrap();

    let root = root.single().unwrap();

    if let Ok((camera, camera_transform)) = camera.single()
        && let Ok(ray) =
            camera.viewport_to_world(camera_transform, trigger.pointer_location.position)
    {
        let inverse = root.compute_affine().inverse();
        let new_origin = inverse.transform_point3(ray.origin);
        let new_direction = inverse.transform_vector3(ray.direction.as_vec3());

        // Use custom ray implementation
        let ray = crate::nurbs::plane::Ray3d::new(new_origin.into(), new_direction.into());

        let plane = crate::nurbs::plane::Plane3d::new_unchecked(
            parent_transform.translation.into(),
            control_rotation.normal.into(),
            Vec3::ZERO.into(),
            Vec3::ZERO.into(),
        );

        if let Some(hit) = ray.plane_intersection_both_ends(&plane) {
            let point: Vec3 = ray.f(&[hit]).into();
            let diff = (point - parent_transform.translation).normalize_or_zero()
                * control_rotation.radius as f32;

            control_rotation.last_vector = diff;
        }
    }
}

#[allow(clippy::complexity)]
fn rotate_start3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::DragStart>>,
    mut control_query: Query<(&mut ControlRotation, &ChildOf)>,
    control_parents: Query<&ControlParent>,
    transforms: Query<&Transform, Without<RootTransform>>,
    root: Query<&Transform, With<RootTransform>>,
) {
    let (mut control_rotation, child_of) = control_query.get_mut(trigger.target()).unwrap();
    let parent = child_of.parent();
    let control_parent = control_parents.get(parent).unwrap();
    let parent_transform = *transforms.get(control_parent.0).unwrap();

    let root = root.single().unwrap();

    let origin = trigger.event().position;
    let inverse = root.compute_affine().inverse();
    let new_origin = inverse.transform_point3(origin);
    let new_direction = -control_rotation.normal;

    // Use custom ray implementation
    let ray = crate::nurbs::plane::Ray3d::new(new_origin.into(), new_direction.into());

    let plane = crate::nurbs::plane::Plane3d::new_unchecked(
        parent_transform.translation.into(),
        control_rotation.normal.into(),
        Vec3::ZERO.into(),
        Vec3::ZERO.into(),
    );

    if let Some(hit) = ray.plane_intersection_both_ends(&plane) {
        let point: Vec3 = ray.f(&[hit]).into();
        let diff = (point - parent_transform.translation).normalize_or_zero()
            * control_rotation.radius as f32;

        control_rotation.last_vector = diff;
    }
}

#[allow(clippy::complexity)]
fn rotate_controller(
    trigger: Trigger<Pointer<Drag>>,
    mut control_query: Query<(&mut ControlRotation, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), (Without<RootTransform>, With<MainCamera>)>,
    control_parents: Query<&ControlParent>,
    root: Query<&Transform, With<RootTransform>>,
    mut changable_transforms: Query<&mut Transform, Without<RootTransform>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    control_storage: Res<ControlStorage>,
    state: Res<TranslationControllerState>,
) {
    let (mut control_rotation, child_of) = control_query.get_mut(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get(parent).unwrap();
    let parent_transform = *changable_transforms.get(control_parent.0).unwrap();
    let root = root.single().unwrap();

    if let Ok((camera, camera_transform)) = camera.single()
        && let Ok(ray) =
            camera.viewport_to_world(camera_transform, trigger.pointer_location.position)
    {
        let inverse = root.compute_affine().inverse();
        let new_origin = inverse.transform_point3(ray.origin);
        let new_direction = inverse.transform_vector3(ray.direction.as_vec3());

        // Use custom ray implementation
        let ray = crate::nurbs::plane::Ray3d::new(new_origin.into(), new_direction.into());

        let plane = crate::nurbs::plane::Plane3d::new_unchecked(
            parent_transform.translation.into(),
            control_rotation.normal.into(),
            Vec3::ZERO.into(),
            Vec3::ZERO.into(),
        );

        if let Some(hit) = ray.plane_intersection_both_ends(&plane) {
            let point: Vec3 = ray.f(&[hit]).into();
            let diff = (point - parent_transform.translation).normalize_or_zero()
                * control_rotation.radius as f32;

            let last_diff = control_rotation.last_vector;
            control_rotation.last_vector = diff;

            let angle = atan2(last_diff.cross(diff).length(), last_diff.dot(diff));
            let sign = (last_diff.cross(diff).dot(control_rotation.normal)).signum();

            let mut parent_transform_mut = changable_transforms.get_mut(control_parent.0).unwrap();
            let (mut inverse, mut forward, mut up) = {
                parent_transform_mut.rotation =
                    Quat::from_axis_angle(control_rotation.normal, angle * sign)
                        * parent_transform_mut.rotation;
                (
                    parent_transform_mut.rotation.inverse(),
                    parent_transform_mut.forward().as_vec3(),
                    parent_transform_mut.up().as_vec3(),
                )
            };

            if state.curve_snapping == SnappingBehaviour::Snap {
                for axis in control_storage.iter_arrows() {
                    if axis.with_rotation {
                        let cos_score = forward.normalize_or_zero().dot(axis.normalized);
                        if cos_score.abs() > 0.999 {
                            let multiplier = cos_score.signum();
                            forward = axis.normalized * multiplier;
                        }

                        let cos_score = up.normalize_or_zero().dot(axis.normalized);
                        if cos_score.abs() > 0.999 {
                            let multiplier = cos_score.signum();
                            up = axis.normalized * multiplier;
                        }
                    }
                }

                parent_transform_mut.look_to(forward, up);
                inverse = parent_transform_mut.rotation.inverse();
            }

            let mut arrow_transform = changable_transforms.get_mut(parent).unwrap();
            arrow_transform.rotation = inverse;
        }

        redraw_writer.write(RedrawEvent::Fast);
        redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

#[allow(clippy::complexity)]
fn rotate_controller3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    mut control_query: Query<(&mut ControlRotation, &ChildOf)>,
    control_parents: Query<&ControlParent>,
    root: Query<&Transform, With<RootTransform>>,
    mut changeable_transforms: Query<&mut Transform, Without<RootTransform>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    control_storage: Res<ControlStorage>,
    state: Res<TranslationControllerState>,
) {
    let (mut control_rotation, child_of) = control_query.get_mut(trigger.target()).unwrap();
    let parent = child_of.parent();
    let control_parent = control_parents.get(parent).unwrap();
    let parent_transform = *changeable_transforms.get(control_parent.0).unwrap();

    let root = root.single().unwrap();

    let origin = trigger.event().event.current_entity_position;
    let inverse = root.compute_affine().inverse();
    let new_origin = inverse.transform_point3(origin);
    let new_direction = -control_rotation.normal;

    // Use custom ray implementation
    let ray = crate::nurbs::plane::Ray3d::new(new_origin.into(), new_direction.into());

    let plane = crate::nurbs::plane::Plane3d::new_unchecked(
        parent_transform.translation.into(),
        control_rotation.normal.into(),
        Vec3::ZERO.into(),
        Vec3::ZERO.into(),
    );

    if let Some(hit) = ray.plane_intersection_both_ends(&plane) {
        let point: Vec3 = ray.f(&[hit]).into();
        let diff = (point - parent_transform.translation).normalize_or_zero()
            * control_rotation.radius as f32;

        let last_diff = control_rotation.last_vector;
        control_rotation.last_vector = diff;

        let angle = atan2(last_diff.cross(diff).length(), last_diff.dot(diff));
        let sign = (last_diff.cross(diff).dot(control_rotation.normal)).signum();

        let mut parent_transform_mut = changeable_transforms.get_mut(control_parent.0).unwrap();
        let (mut inverse, mut forward, mut up) = {
            parent_transform_mut.rotation =
                Quat::from_axis_angle(control_rotation.normal, angle * sign)
                    * parent_transform_mut.rotation;
            (
                parent_transform_mut.rotation.inverse(),
                parent_transform_mut.forward().as_vec3(),
                parent_transform_mut.up().as_vec3(),
            )
        };

        if state.curve_snapping == SnappingBehaviour::Snap {
            for axis in control_storage.iter_arrows() {
                if axis.with_rotation {
                    let cos_score = forward.normalize_or_zero().dot(axis.normalized);
                    if cos_score.abs() > 0.999 {
                        let multiplier = cos_score.signum();
                        forward = axis.normalized * multiplier;
                    }

                    let cos_score = up.normalize_or_zero().dot(axis.normalized);
                    if cos_score.abs() > 0.999 {
                        let multiplier = cos_score.signum();
                        up = axis.normalized * multiplier;
                    }
                }
            }

            parent_transform_mut.look_to(forward, up);
            inverse = parent_transform_mut.rotation.inverse();
        }

        let mut arrow_transform = changeable_transforms.get_mut(parent).unwrap();
        arrow_transform.rotation = inverse;

        redraw_writer.write(RedrawEvent::Fast);
        redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

#[allow(clippy::complexity)]
fn rotate_end_trigger_redraw(
    _: Trigger<Pointer<DragEnd>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);
}

#[allow(clippy::complexity)]
fn rotate_end_trigger_redraw3d(
    _: Trigger<Pointer3d<crate::picking3d::events::DragEnd>>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);
}

#[allow(clippy::complexity)]
/// Update both arrows and snaps, despawning / removing them if invalid
fn update_snapped_points(
    mut commands: Commands,
    mut set: ParamSet<(
        (
            Query<
                (&mut Transform, &mut Control, Entity, &ChildOf),
                (Without<SnappedPoint>, With<SnappedArrow>),
            >, // arrows
            Query<(&mut Transform, &SnappedPoint, Entity), Without<SnappedArrow>>, // snapped
        ),
        CurveCollection,
    )>,
    control_parents: Query<&ControlParent>,
) {
    let curves = set.p1().collect();

    let (mut arrows, mut snapped) = set.p0();

    for (mut t, mut arrow, arrow_entity, relation) in &mut arrows {
        let parent = relation.parent();
        let control_parent = control_parents.get(parent).unwrap();
        // Only use the SnappedPoint::ToCurve snapping mode to update. the other mode may not snap
        // permanently
        if let Ok((
            mut transform,
            SnappedPoint::ToCurve {
                u: snap_u,
                curve: snap_curve,
            },
            entity,
        )) = snapped.get_mut(control_parent.0)
        {
            if let Some(curve) = curves.get(snap_curve) {
                let p = *de_casteljau(curve, *snap_u).last().unwrap().last().unwrap();
                transform.translation = p.into();

                let deriv = derive_after_de_casteljau(&de_casteljau(curve, *snap_u), 1);
                arrow.0 = Vec3::from(deriv).normalize();
                t.look_to(Vec3::from(deriv), Vec3::Y);
            } else {
                commands
                    .get_entity(entity)
                    .unwrap()
                    .remove::<SnappedPoint>();
                commands.get_entity(arrow_entity).unwrap().despawn();
            }
        } else {
            commands.get_entity(arrow_entity).unwrap().despawn();
        }
    }
}

/// Update the arrow directions for the arrows that are snapped to planes.
/// Up and left direction will get updated according to the current transform of the plane entity
fn update_plane_directions(
    mut commands: Commands,
    transforms: Query<&Transform, Without<OnPlaneMovableMarker>>,
    mut arrows: Query<(
        &ChildOf,
        &mut Transform,
        &OnPlaneMovableMarker,
        &mut Control,
    )>,
) {
    for (arrow_childof, mut arrow_transform, plane_info, mut control) in &mut arrows {
        if let Ok(transform) = transforms.get(plane_info.plane) {
            match plane_info.arrow_direction {
                ArrowDirection::Up => {
                    arrow_transform.look_to(transform.up(), Vec3::Y);
                    control.0 = transform.up().normalize();
                }
                ArrowDirection::Left => {
                    arrow_transform.look_to(transform.left(), Vec3::Y);
                    control.0 = transform.left().normalize();
                }
            }
        } else {
            // Despawn, since the plane entity does not exist any longer. Meaning the controls
            // should despawn
            let _ = commands
                .get_entity(arrow_childof.parent())
                .map(|mut entity| entity.despawn());
        }
    }
}

#[cfg(not(feature = "vr_enable"))]
fn update_texts(
    mut transforms: Query<&mut Transform>,
    root: Query<Entity, With<RootTransform>>,
    texts: Query<(Entity, &ChildOf, &Children), With<CoordinateTextMarker>>,
    mut text3d: Query<&mut Text3d>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    info: Res<RenderInformation>,
) {
    let root = root.single().unwrap();
    let root_transform = *transforms.get(root).unwrap();
    let camera_transform = camera.single().unwrap();
    let scale = info.scale;

    for (text_entity, &ChildOf(parent), children) in texts {
        let start_transform = *transforms.get(parent).unwrap();

        let camera_forward = root_transform
            .compute_affine()
            .inverse()
            .transform_vector3(camera_transform.forward().normalize_or_zero());

        {
            let mut transform = transforms.get_mut(text_entity).unwrap();
            transform.rotation = start_transform.rotation.inverse();
        }

        for child in children {
            let mut transform = transforms.get_mut(*child).unwrap();
            transform.translation = -camera_forward * 0.3 * info.scale + Vec3::Y * info.scale;
            transform.look_to(camera_forward, Vec3::Y);
            if let Ok(mut text3d) = text3d.get_mut(*child) {
                *text3d = Text3d::new(format!(
                    "({:.3}, {:.3}, {:.3})",
                    start_transform.translation.x / scale,
                    start_transform.translation.y / scale,
                    start_transform.translation.z / scale
                ));
            }
        }
    }
}

#[cfg(feature = "vr_enable")]
fn update_texts(
    mut transforms: Query<&mut Transform>,
    root: Query<Entity, With<RootTransform>>,
    texts: Query<(Entity, &ChildOf, &Children), With<CoordinateTextMarker>>,
    mut text3d: Query<&mut Text3d>,
    camera: Query<&GlobalTransform, With<XrTrackedView>>,
    info: Res<RenderInformation>,
) {
    let root = root.single().unwrap();
    let root_transform = *transforms.get(root).unwrap();

    let Ok(camera_transform) = camera.single() else {
        return;
    };
    let scale = info.scale;

    for (text_entity, &ChildOf(parent), children) in texts {
        let start_transform = *transforms.get(parent).unwrap();
        let direction = start_transform.translation - camera_transform.translation();

        let camera_forward = root_transform
            .compute_affine()
            .inverse()
            .transform_vector3(direction.normalize_or_zero());

        {
            let mut transform = transforms.get_mut(text_entity).unwrap();
            transform.rotation = start_transform.rotation.inverse();
        }

        for child in children {
            let mut transform = transforms.get_mut(*child).unwrap();
            transform.translation = -camera_forward * 0.3 * info.scale + Vec3::Y * info.scale;
            transform.look_to(camera_forward, Vec3::Y);
            if let Ok(mut text3d) = text3d.get_mut(*child) {
                *text3d = Text3d::new(format!(
                    "({:.3}, {:.3}, {:.3})",
                    start_transform.translation.x / scale,
                    start_transform.translation.y / scale,
                    start_transform.translation.z / scale
                ));
            }
        }
    }
}

pub struct TranslationController;

impl Plugin for TranslationController {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, show_transitional_controls);
        app.add_systems(
            PostUpdate,
            (
                register_deletes,
                handle_toggle_snapping.run_if(on_event::<ToggleSnappingBehaviour>),
                update_snapped_points,
                update_plane_directions,
                handle_translate_by_delta_event,
                update_texts,
            ),
        );
        app.init_resource::<ControlStorage>();

        app.init_resource::<TranslationControllerState>();
        app.init_resource::<AccumulatedMovementStore>();
        app.add_event::<ToggleSnappingBehaviour>();
        app.add_event::<MovedEntityEvent>();
        app.add_event::<MoveEntityByDeltaEvent>();
    }
}
