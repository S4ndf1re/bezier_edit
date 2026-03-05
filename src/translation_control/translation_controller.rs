use crate::bezier_curve::bezier_curve_renderer::{RedrawEvent, hover_3d};
use crate::bezier_curve::helper_curves::{CurveCollection, RedrawCurvesEvent};
use crate::bezier_curve::render_info::RenderInformation;
use crate::history::plugin::HistoryLogEvent;
use crate::nurbs::parametric::Parametric;
use crate::picking3d::events::{MoveIn, MoveOut, Pointer3d};
use crate::picking3d::picking_3d::{CustomPicking3dHitbox, Picking3dInteractable};
use crate::translation_control::control_storage::ControlStorage;
use crate::translation_control::proximity_detector::Snappable;
use crate::translation_control::shadow_boxes::{
    AssignedShadowBoxes, generate_shadow_box_bundle, update_boxes,
};
use crate::util::update_material_on;
use crate::{MainCamera, RootTransform};
use bevy::color::palettes::tailwind::{
    BLUE_600, BLUE_800, GRAY_400, GRAY_500, PURPLE_600, PURPLE_800, RED_600, RED_800,
};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::prelude::*;
use bevy_lunex::prelude::{Text3d, Text3dStyling, TextAlign, TextAtlas, Weight};
use bevy_xr_utils::tracking_utils::XrTrackedView;
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

use super::accumulated::AccumulatedMovementStore;
use super::control_storage::{ControlDirection, ControlPlane};
use super::obligatory_drag_params::ObligatoryDragParams;

#[derive(Event)]
pub struct ToggleRobotVisibilityEvent;

#[derive(Component)]
pub struct TemporaryInvisible;

#[derive(Component)]
pub struct CoordinateTextMarker {
    use_root: bool,
}

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
    ToEntity,
}

#[derive(Component)]
#[relationship(relationship_target = AssignedShadowMarkers)]
pub struct ShadowMarker(Entity);

#[derive(Component)]
#[relationship_target(relationship = ShadowMarker, linked_spawn)]
pub struct AssignedShadowMarkers(Vec<Entity>);

impl AssignedShadowMarkers {
    pub fn entities(&self) -> &Vec<Entity> {
        &self.0
    }
}

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub struct EnableTranslationControl {
    type_of: EnableTranslationControlType,
    use_root: bool,
    use_parent_translation: bool,
    invert: bool,
    hide_lines: bool,
    no_shadow: bool,
    no_text: bool,
    custom_root: Option<Entity>,
}

impl EnableTranslationControl {
    pub fn new_without_root(type_of: EnableTranslationControlType) -> Self {
        Self {
            type_of,
            use_root: false,
            use_parent_translation: false,
            invert: false,
            hide_lines: false,
            no_shadow: false,
            no_text: false,
            custom_root: None,
        }
    }

    pub fn new_with_root(type_of: EnableTranslationControlType) -> Self {
        Self {
            type_of,
            use_root: true,
            use_parent_translation: true,
            invert: false,
            hide_lines: false,
            no_shadow: false,
            no_text: false,
            custom_root: None,
        }
    }

    pub fn use_parent_translation(mut self) -> Self {
        self.use_parent_translation = true;
        self
    }

    pub fn invert(mut self) -> Self {
        self.invert = true;
        self
    }

    pub fn hide_lines(mut self) -> Self {
        self.hide_lines = true;
        self
    }

    pub fn no_shadow(mut self) -> Self {
        self.no_shadow = true;
        self
    }

    pub fn no_text(mut self) -> Self {
        self.no_text = true;
        self
    }

    pub fn with_custom_root(mut self, root: Entity) -> Self {
        self.custom_root = Some(root);
        self
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Default)]
pub enum EnableTranslationControlType {
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
pub struct ControlParent {
    pub entity: Entity,
    use_root: bool,
    no_shadow: bool,
    no_text: bool,
    custom_root: Option<Entity>,
}

#[derive(Component)]
pub struct ControlSphere;

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

#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
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

#[derive(Default, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
pub enum PrismMode {
    #[default]
    None,
    Prism,
}

impl PrismMode {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Prism,
            Self::Prism => Self::None,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct TranslationControllerState {
    pub curve_snapping: SnappingBehaviour,
    pub step_mode: StepMode,
    pub prism_mode: PrismMode,
    pub invisible_robots: bool,
}

#[derive(Event)]
pub struct ToggleSnappingBehaviour;

#[derive(Event)]
pub struct SetPrismMode(pub PrismMode);

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

#[derive(Component)]
pub struct TemporaryCurveSnappingBlocker;

pub fn handle_toggle_snapping(
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

pub fn handle_set_prism_mode(
    mut reader: EventReader<SetPrismMode>,
    mut state: ResMut<TranslationControllerState>,
) {
    for evt in reader.read() {
        state.prism_mode = evt.0;
    }
}

fn register_deletes(
    mut commands: Commands,
    mut deleted: RemovedComponents<EnableTranslationControl>,
    mut controls: Query<(Entity, &mut Visibility, &ControlParent)>,
    children: Query<&Children>,
    mut picking3d_interactable: Query<&mut Picking3dInteractable>,
    mut pickables: Query<&mut Pickable>,
) {
    for event in deleted.read() {
        for (entity, mut visibility, contrl) in controls.iter_mut() {
            if contrl.entity == event {
                *visibility = Visibility::Hidden;
                commands.entity(entity).insert(Pickable::IGNORE);
                commands.entity(entity).remove::<TemporaryInvisible>();

                for child in children.iter_descendants(event) {
                    if let Ok(mut pickable) = picking3d_interactable.get_mut(child) {
                        *pickable = Picking3dInteractable::Ignore;
                    }

                    if let Ok(mut pickable) = pickables.get_mut(child) {
                        *pickable = Pickable::IGNORE;
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
    picking3d: Picking3dInteractable,
    picking: Pickable,
) {
    let plane = meshes.add(Cuboid::new(0.24 * scale, 0.24 * scale, 0.01 * scale));

    let mut obj = child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        MeshMaterial3d(mat.clone()),
        Mesh3d(plane.clone()),
        picking3d,
        picking,
        Visibility::Inherited,
    ));
    obj.observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()))
        .observe(hover_3d);
}

#[allow(clippy::complexity)]
pub fn draw_arrow(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
    is_shadow: bool,
    picking3d: Picking3dInteractable,
    picking: Pickable,
    hide_lines: bool,
) {
    let cuboid = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.4 * scale));
    let line = meshes.add(Cuboid::new(0.02 * scale, 0.02 * scale, 0.8 * scale));
    let arrow = meshes.add(Cone::new(0.035 * scale, 0.2 * scale));
    let thin_line = meshes.add(Cylinder::new(0.005 * scale, 2000.0));

    let mut obj = child_builder.spawn((
        Transform::from_xyz(0.0, 0.0, -0.4 * scale),
        MeshMaterial3d(mat.clone()),
        Mesh3d(cuboid.clone()),
        picking3d,
        picking.clone(),
        Visibility::Inherited,
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
            Visibility::Inherited,
            picking.clone(),
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
            Visibility::Inherited,
            picking.clone(),
        ))
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()));

    if !is_shadow && !hide_lines {
        child_builder.spawn((
            Transform::default()
                .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
            MeshMaterial3d(mat.clone()),
            Visibility::Inherited,
            Mesh3d(thin_line.clone()),
            picking,
        ));
    }
}

fn draw_ring(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
    picking3d: Picking3dInteractable,
    picking: Pickable,
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
        .spawn((
            transform,
            Mesh3d(torus),
            MeshMaterial3d(mat.clone()),
            Visibility::Inherited,
            picking.clone(),
        ))
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
                picking3d,
                picking.clone(),
                Visibility::Inherited,
            ))
            .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer<Out>>(mat.clone()))
            .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()))
            .observe(hover_3d);
    }
}

fn draw_sphere(
    child_builder: &mut RelatedSpawnerCommands<ChildOf>,
    mat: Handle<StandardMaterial>,
    mat_hover: Handle<StandardMaterial>,
    meshes: &mut ResMut<Assets<Mesh>>,
    scale: f32,
    picking3d: Picking3dInteractable,
    picking: Pickable,
) {
    // NOTE: This should be slightly larger than the original
    let sphere = meshes.add(Sphere::new(0.105 * scale));

    child_builder
        .spawn((
            Transform::default(),
            Visibility::Inherited,
            picking3d,
            picking,
            Mesh3d(sphere),
            MeshMaterial3d(mat.clone()),
        ))
        .observe(hover_3d)
        .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()));
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
    mut already_existing: Query<(Entity, &mut Visibility, &ControlParent)>,
    children: Query<&Children>,
    mut picking3d_interactable: Query<&mut Picking3dInteractable>,
    mut pickables: Query<&mut Pickable>,
    state: Res<TranslationControllerState>,
) {
    let scale = scale.scale;

    for (entity, enabled_control) in to_enable.iter() {
        let multiplier: f32 = if enabled_control.invert { -1.0 } else { 1.0 };
        let mut already_created = false;
        for (parent_entity, mut visibility, parent) in already_existing.iter_mut() {
            if parent.entity == entity {
                already_created = true;
                // Match behaviour with toggle_visibility system
                if state.invisible_robots {
                    *visibility = Visibility::Hidden;
                    commands
                        .entity(parent_entity)
                        .insert((Pickable::IGNORE, TemporaryInvisible));
                    for child in children.iter_descendants(parent_entity) {
                        if let Ok(mut pickable3d) = picking3d_interactable.get_mut(child) {
                            *pickable3d = Picking3dInteractable::Ignore;
                        }

                        if let Ok(mut pickable) = pickables.get_mut(child) {
                            *pickable = Pickable::IGNORE;
                        }
                    }
                } else {
                    *visibility = Visibility::Inherited;
                    commands.entity(parent_entity).remove::<Pickable>();
                    for child in children.iter_descendants(parent_entity) {
                        if let Ok(mut pickable3d) = picking3d_interactable.get_mut(child) {
                            *pickable3d = Picking3dInteractable::Default;
                        }

                        if let Ok(mut pickable) = pickables.get_mut(child) {
                            *pickable = Pickable::default();
                        }
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
        let (pickable3d, pickable) = if state.invisible_robots {
            (Picking3dInteractable::Ignore, Pickable::IGNORE)
        } else {
            (Picking3dInteractable::Default, Pickable::default())
        };

        commands.get_entity(entity).unwrap().with_children(|cmd| {
            let mut parent = cmd.spawn((
                ControlParent {
                    entity,
                    use_root: enabled_control.use_root,
                    no_shadow: enabled_control.no_shadow,
                    no_text: enabled_control.no_text,
                    custom_root: enabled_control.custom_root,
                },
                Name::new("Control Parent"),
                transform,
                if state.invisible_robots {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                },
            ));
            parent.with_children(|parent| {
                if enabled_control.type_of == EnableTranslationControlType::OnlyTranslation
                    || enabled_control.type_of == EnableTranslationControlType::WithRotation
                {
                    parent
                        .spawn((
                            Transform::default(),
                            ControlSphere,
                            Visibility::Inherited,
                            Control(Vec3::ONE),
                        ))
                        .with_children(|parent| {
                            draw_sphere(
                                parent,
                                materials.add(Color::from(GRAY_400)),
                                materials.add(Color::from(GRAY_500)),
                                &mut meshes,
                                scale,
                                pickable3d,
                                pickable.clone(),
                            );
                        })
                        .observe(drag_sphere_controller)
                        .observe(drag_sphere_controller3d)
                        .observe(drag_start)
                        .observe(drag_start3d)
                        .observe(drag_end_trigger_redraw)
                        .observe(drag_end3d_trigger_redraw);

                    for arrow in arrows.as_ref().iter_arrows() {
                        parent
                            .spawn((
                                Transform::default()
                                    .looking_to(arrow.normalized * multiplier, Vec3::Y),
                                Control(arrow.normalized * multiplier),
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
                                    pickable3d,
                                    pickable.clone(),
                                    enabled_control.hide_lines,
                                );
                            })
                            .observe(drag_controller)
                            .observe(drag_controller3d)
                            .observe(drag_start)
                            .observe(drag_start3d)
                            .observe(drag_end_trigger_redraw)
                            .observe(drag_end3d_trigger_redraw);

                        if enabled_control.type_of == EnableTranslationControlType::WithRotation {
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
                                        pickable3d,
                                        pickable.clone(),
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
                                Transform::from_translation(
                                    (plane.axis / 3.0) * scale * multiplier,
                                )
                                .looking_to(plane.normal * multiplier, Vec3::Y),
                                Control(plane.axis * multiplier),
                                Visibility::default(),
                            ))
                            .with_children(|parent| {
                                draw_plane(
                                    parent,
                                    materials.add(plane.color),
                                    materials.add(plane.hover_color),
                                    &mut meshes,
                                    scale,
                                    pickable3d,
                                    pickable.clone(),
                                );
                            })
                            .observe(drag_plane)
                            .observe(drag_plane3d)
                            .observe(drag_start)
                            .observe(drag_start3d)
                            .observe(drag_end_trigger_redraw)
                            .observe(drag_end3d_trigger_redraw);
                    }
                } else if let EnableTranslationControlType::OnlyOnPlane(plane_entity) =
                    enabled_control.type_of
                    && let Ok(plane_transform) = transforms.get(plane_entity)
                {
                    let up_direction = ControlDirection::new(
                        plane_transform.up().as_vec3().normalize_or_zero(),
                        Color::from(BLUE_600),
                        Color::from(BLUE_800),
                        Color::from(GRAY_500),
                        false,
                    );

                    let left_direction = ControlDirection::new(
                        plane_transform.left().as_vec3().normalize_or_zero(),
                        Color::from(RED_600),
                        Color::from(RED_800),
                        Color::from(GRAY_500),
                        false,
                    );

                    let plane = ControlPlane::new(
                        plane_transform.up().as_vec3().normalize_or_zero()
                            + plane_transform.left().as_vec3().normalize_or_zero(),
                        plane_transform.forward().normalize_or_zero(),
                        PURPLE_600.into(),
                        PURPLE_800.into(),
                    );

                    for (arrow, direction) in [
                        (up_direction, ArrowDirection::Up),
                        (left_direction, ArrowDirection::Left),
                    ] {
                        parent
                            .spawn((
                                Transform::from_xyz(0.0, 0.0, 0.0)
                                    .looking_to(arrow.normalized * multiplier, Vec3::Y),
                                Control(arrow.normalized * multiplier),
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
                                    pickable3d,
                                    pickable.clone(),
                                    enabled_control.hide_lines,
                                );
                            })
                            .observe(drag_controller)
                            .observe(drag_controller3d)
                            .observe(drag_start)
                            .observe(drag_start3d)
                            .observe(drag_end_trigger_redraw)
                            .observe(drag_end3d_trigger_redraw);
                    }
                    parent
                        .spawn((
                            Transform::from_translation((plane.axis / 3.0) * multiplier * scale)
                                .looking_to(plane.normal * multiplier, Vec3::Y),
                            Control(plane.axis * multiplier),
                            Visibility::default(),
                        ))
                        .with_children(|parent| {
                            draw_plane(
                                parent,
                                materials.add(plane.color),
                                materials.add(plane.hover_color),
                                &mut meshes,
                                scale,
                                pickable3d,
                                pickable.clone(),
                            );
                        })
                        .observe(drag_plane)
                        .observe(drag_plane3d)
                        .observe(drag_start)
                        .observe(drag_start3d)
                        .observe(drag_end_trigger_redraw)
                        .observe(drag_end3d_trigger_redraw);
                }
            });
            if state.invisible_robots {
                parent.insert((TemporaryInvisible, Pickable::IGNORE));
            }
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
    let mut start_transform = *all_transforms.get(control_parent.entity).unwrap();
    start_transform.rotation = Quat::IDENTITY;

    accumulated_movement.start_movement_entity(control_parent.entity, start_transform.translation);

    let scale = scale.scale;

    // When no_shadow = true, don't spawn shadows and helper lines
    if !control_parent.no_shadow {
        let mut entity_commands = commands.spawn((
            ShadowMarker(control_parent.entity),
            start_transform,
            Visibility::default(),
            Snappable,
        ));

        entity_commands.with_children(|parent| {
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
                            Picking3dInteractable::Ignore,
                            Pickable::IGNORE,
                            true,
                        );
                    });
            }
        });

        if control_parent.use_root {
            entity_commands.insert(ChildOf(root));
        }

        // Spawn box
        let diff = Vec3::new(0.0, 0.0, 0.0);
        commands.spawn(generate_shadow_box_bundle(
            diff,
            &mut materials,
            &mut meshes,
            control_parent.entity,
            scale,
        ));
    }

    if !control_parent.no_text {
        // Spawn texts
        let start_transform = *all_transforms.get(control_parent.entity).unwrap();
        let camera_transform = camera.single().unwrap();
        let camera_forward = root_transform
            .compute_affine()
            .inverse()
            .transform_vector3(camera_transform.forward().normalize_or_zero());

        commands.spawn((
            ChildOf(control_parent.entity),
            Transform::from_rotation(start_transform.rotation.inverse()),
            CoordinateTextMarker {
                use_root: control_parent.use_root,
            },
            Visibility::Inherited,
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
                Visibility::Inherited,
            )],
        ));
    }

    history.write(HistoryLogEvent::Begin(
        control_parent.entity,
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
    let mut start_transform = *all_transforms.get(control_parent.entity).unwrap();
    start_transform.rotation = Quat::IDENTITY;

    accumulated_movement.start_movement_entity(control_parent.entity, start_transform.translation);

    let scale = scale.scale;

    if !control_parent.no_shadow {
        let mut entity_commands = commands.spawn((
            ShadowMarker(control_parent.entity),
            start_transform,
            Visibility::default(),
            ChildOf(root),
            Snappable,
        ));

        entity_commands.with_children(|parent| {
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
                            Picking3dInteractable::Ignore,
                            Pickable::IGNORE,
                            true,
                        );
                    });
            }
        });

        if control_parent.use_root {
            entity_commands.insert(ChildOf(root));
        }

        // Spawn box
        let diff = Vec3::new(0.0, 0.0, 0.0);
        commands.spawn(generate_shadow_box_bundle(
            diff,
            &mut materials,
            &mut meshes,
            control_parent.entity,
            scale,
        ));
    }

    if !control_parent.no_text {
        // Spawn texts
        let start_transform = *all_transforms.get(control_parent.entity).unwrap();
        let camera_transform = camera.single().unwrap();
        let camera_forward = root_transform
            .compute_affine()
            .inverse()
            .transform_vector3(camera_transform.forward().normalize_or_zero());

        commands.spawn((
            ChildOf(control_parent.entity),
            Transform::from_rotation(start_transform.rotation.inverse()),
            CoordinateTextMarker {
                use_root: control_parent.use_root,
            },
            Visibility::Inherited,
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
                Visibility::Inherited,
            )],
        ));
    }

    history.write(HistoryLogEvent::Begin(
        control_parent.entity,
        Some(start_transform),
    ));
}

#[allow(clippy::complexity)]
fn drag_end_trigger_redraw(
    trigger: Trigger<Pointer<DragEnd>>,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    assigned_markers: Query<&AssignedShadowMarkers>,
    assigned_boxes: Query<&AssignedShadowBoxes>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    mut commands: Commands,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
    children: Query<&Children>,
    text_marker: Query<Entity, With<CoordinateTextMarker>>,
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    accumulated_movement.end_movement(&control_parent.entity);

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    if let Ok(markers) = assigned_markers.get(control_parent.entity) {
        for marker in markers.entities() {
            let _ = commands.get_entity(*marker).map(|mut e| e.despawn());
        }
    }

    if let Ok(boxes) = assigned_boxes.get(control_parent.entity) {
        for r#box in boxes.entities() {
            let _ = commands.get_entity(*r#box).map(|mut e| e.despawn());
        }
    }

    // Despawn text
    for child in children.get(control_parent.entity).unwrap() {
        if let Ok(text_entity) = text_marker.get(*child) {
            let _ = commands.get_entity(text_entity).map(|mut e| e.despawn());
        }
    }

    let _ = commands.get_entity(control_parent.entity).map(|mut e| {
        e.remove::<TemporaryCurveSnappingBlocker>();
    });

    history.write(HistoryLogEvent::End(control_parent.entity, None));
}

#[allow(clippy::complexity)]
fn drag_end3d_trigger_redraw(
    trigger: Trigger<Pointer3d<crate::picking3d::events::DragEnd>>,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    assigned_markers: Query<&AssignedShadowMarkers>,
    assigned_boxes: Query<&AssignedShadowBoxes>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    mut commands: Commands,
    mut history: EventWriter<HistoryLogEvent>,
    mut accumulated_movement: ResMut<AccumulatedMovementStore>,
    children: Query<&Children>,
    text_marker: Query<Entity, With<CoordinateTextMarker>>,
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    accumulated_movement.end_movement(&control_parent.entity);

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    if let Ok(markers) = assigned_markers.get(control_parent.entity) {
        for marker in markers.iter() {
            let _ = commands.get_entity(marker).map(|mut e| e.despawn());
        }
    }

    if let Ok(boxes) = assigned_boxes.get(control_parent.entity) {
        for r#box in boxes.entities() {
            let _ = commands.get_entity(*r#box).map(|mut e| e.despawn());
        }
    }

    // Despawn text
    for child in children.get(control_parent.entity).unwrap() {
        if let Ok(text_entity) = text_marker.get(*child) {
            let _ = commands.get_entity(text_entity).map(|mut e| e.despawn());
        }
    }

    let _ = commands.get_entity(control_parent.entity).map(|mut e| {
        e.remove::<TemporaryCurveSnappingBlocker>();
    });

    history.write(HistoryLogEvent::End(control_parent.entity, None));
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
    mut params: ParamSet<(
        ObligatoryDragParams,
        Query<&GlobalTransform>,
        Query<&Transform>,
    )>,
) {
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let diff = {
            let dist = (params
                .p1()
                .get(control_parent.entity)
                .unwrap()
                .translation()
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
            let diff = end - start;

            if control_parent.use_root {
                root.single()
                    .unwrap()
                    .affine()
                    .inverse()
                    .transform_vector3(diff)
            } else if let Some(custom_root) = control_parent.custom_root {
                let custom_query = params.p2();
                let custom_transform = custom_query
                    .get(custom_root)
                    .expect("critical error, cannot recover");

                custom_transform
                    .compute_affine()
                    .inverse()
                    .transform_vector3(diff)
            } else {
                diff
            }
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
    state: Res<TranslationControllerState>,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let control_parent = control_parents.get_mut(parent).unwrap();

    let diff = if state.prism_mode == PrismMode::Prism {
        trigger.event.delta
    } else {
        trigger.event.real_delta
    };

    let diff = if control_parent.use_root {
        root.single()
            .unwrap()
            .affine()
            .inverse()
            .transform_vector3(diff)
    } else {
        diff
    };

    let axis = control.0;
    let translation = axis * diff;

    params.update_position_drag_universal((parent, control_parent), translation, control_entity);
}

#[allow(clippy::complexity)]
pub fn drag_sphere_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(Entity, &ControlSphere, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(ObligatoryDragParams, Query<&GlobalTransform>)>,
) {
    let (control_entity, _, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let diff = {
            let dist = (params
                .p1()
                .get(control_parent.entity)
                .unwrap()
                .translation()
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
            if control_parent.use_root {
                root.single()
                    .unwrap()
                    .affine()
                    .inverse()
                    .transform_vector3(end - start)
            } else {
                end - start
            }
        };

        params
            .p0()
            .update_position_drag_universal((parent, control_parent), diff, control_entity);
    }
}

#[allow(clippy::complexity)]
pub fn drag_sphere_controller3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ObligatoryDragParams,
    state: Res<TranslationControllerState>,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control_entity, _, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let control_parent = control_parents.get_mut(parent).unwrap();

    let diff = if state.prism_mode == PrismMode::Prism {
        trigger.event.delta
    } else {
        trigger.event.real_delta
    };
    let diff = if control_parent.use_root {
        root.single()
            .unwrap()
            .affine()
            .inverse()
            .transform_vector3(diff)
    } else {
        diff
    };

    params.update_position_drag_universal((parent, control_parent), diff, control_entity);
}

#[allow(clippy::complexity)]
pub fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(Entity, &Control, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(
        ObligatoryDragParams,
        Query<&GlobalTransform>,
        Query<&Transform>,
    )>,
) {
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let diff = {
            let dist = (params
                .p1()
                .get(control_parent.entity)
                .unwrap()
                .translation()
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
            let diff = end - start;

            // let diff = if control_parent.use_parent_translation {
            //     let query = params.p2();
            //     let parent_transform = query
            //         .get(control_parent.entity)
            //         .expect("must be child of parent");
            //
            //     let translation = parent_transform.translation;
            //
            //     diff - translation
            // } else {
            //     diff
            // };

            if control_parent.use_root {
                root.single()
                    .unwrap()
                    .affine()
                    .inverse()
                    .transform_vector3(diff)
            } else if let Some(custom_root) = control_parent.custom_root {
                let custom_query = params.p2();
                let custom_transform = custom_query
                    .get(custom_root)
                    .expect("critical error, cannot recover");

                custom_transform
                    .compute_affine()
                    .inverse()
                    .transform_vector3(diff)
            } else {
                diff
            }
        };

        let axis = control.0;
        let direction = axis.dot(diff.normalize_or_zero());
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
    state: Res<TranslationControllerState>,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control_entity, control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();

    let control_parent = control_parents.get_mut(parent).unwrap();

    let diff = if state.prism_mode == PrismMode::Prism {
        trigger.event.delta
    } else {
        trigger.event.real_delta
    };
    let diff = if control_parent.use_root {
        root.single()
            .unwrap()
            .affine()
            .inverse()
            .transform_vector3(diff)
    } else {
        diff
    };

    let axis = control.0.normalize_or_zero();
    let direction = axis.dot(diff.normalize_or_zero());
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
    let parent_transform = *transforms.get(control_parent.entity).unwrap();

    let root = root.single().unwrap();

    if let Ok((camera, camera_transform)) = camera.single()
        && let Ok(ray) =
            camera.viewport_to_world(camera_transform, trigger.pointer_location.position)
    {
        let (new_origin, new_direction) = if control_parent.use_root {
            let inverse = root.compute_affine().inverse();
            (
                inverse.transform_point3(ray.origin),
                inverse.transform_vector3(ray.direction.as_vec3()),
            )
        } else if let Some(custom_root) = control_parent.custom_root {
            let custom_transform = transforms
                .get(custom_root)
                .expect("Critical error, can't recover");
            let inverse = custom_transform.compute_affine();

            (
                inverse.transform_point3(ray.origin),
                inverse.transform_vector3(ray.direction.as_vec3()),
            )
        } else {
            (ray.origin, ray.direction.as_vec3())
        };

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
    let parent_transform = *transforms.get(control_parent.entity).unwrap();

    let root = root.single().unwrap();

    let origin = trigger.event().position;
    let inverse = root.compute_affine().inverse();
    let new_origin = if control_parent.use_root {
        inverse.transform_point3(origin)
    } else {
        origin
    };
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

/// Snap transform.forward() into a plane by rotating about `axis`,
/// but only if the correction angle is less than `max_angle_deg`.
pub fn snap_forward_to_plane(
    transform: &mut Transform,
    plane_normal: Vec3,
    axis: Vec3,
    max_angle_deg: f32,
) {
    let forward = transform.forward().as_vec3();

    // Project forward and plane_normal onto the plane orthogonal to axis
    let f_on_plane = (forward - forward.dot(axis) * axis).normalize_or_zero();
    let n_on_plane = (plane_normal - plane_normal.dot(axis) * axis).normalize_or_zero();

    if f_on_plane == Vec3::ZERO || n_on_plane == Vec3::ZERO {
        return;
    }

    let angle = f_on_plane.angle_between(n_on_plane);

    // Sign of the rotation around the axis
    let sign = f_on_plane.cross(n_on_plane).dot(axis).signum();
    let signed_angle = sign * angle;

    if signed_angle.abs() > max_angle_deg.to_radians() {
        return;
    }

    // Apply rotation
    let delta = Quat::from_axis_angle(axis, signed_angle);
    transform.rotation = delta * transform.rotation;
}

/// Snap transform.up() into a plane by rotating about `axis`,
/// but only if the correction angle is less than `max_angle_deg`.
pub fn snap_up_to_plane(
    transform: &mut Transform,
    plane_normal: Vec3,
    axis: Vec3,
    max_angle_deg: f32,
) {
    let up = transform.up().as_vec3();

    // Project forward and plane_normal onto the plane orthogonal to axis
    let f_on_plane = (up - up.dot(axis) * axis).normalize_or_zero();
    let n_on_plane = (plane_normal - plane_normal.dot(axis) * axis).normalize_or_zero();

    if f_on_plane == Vec3::ZERO || n_on_plane == Vec3::ZERO {
        return;
    }

    // Angle between them in the rotation plane
    let angle = f_on_plane.angle_between(n_on_plane);

    // Sign of the rotation around the axis
    let sign = f_on_plane.cross(n_on_plane).dot(axis).signum();
    let signed_angle = sign * angle;

    // Clamp against maximum allowed correction
    if signed_angle.abs() > max_angle_deg.to_radians() {
        return;
    }

    // Apply rotation
    let delta = Quat::from_axis_angle(axis, signed_angle);
    transform.rotation = delta * transform.rotation;
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
    let parent_transform = *changable_transforms.get(control_parent.entity).unwrap();
    let root = root.single().unwrap();

    if let Ok((camera, camera_transform)) = camera.single()
        && let Ok(ray) =
            camera.viewport_to_world(camera_transform, trigger.pointer_location.position)
    {
        let (new_origin, new_direction) = if control_parent.use_root {
            let inverse = root.compute_affine().inverse();
            (
                inverse.transform_point3(ray.origin),
                inverse.transform_vector3(ray.direction.as_vec3()),
            )
        } else {
            (ray.origin, ray.direction.as_vec3())
        };

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

            let angle = last_diff.angle_between(diff);
            let sign = (last_diff.cross(diff).dot(control_rotation.normal)).signum();

            let mut parent_transform_mut =
                changable_transforms.get_mut(control_parent.entity).unwrap();
            let mut inverse = {
                parent_transform_mut.rotation =
                    Quat::from_axis_angle(control_rotation.normal, angle * sign)
                        * parent_transform_mut.rotation;

                parent_transform_mut.rotation.inverse()
            };

            if state.curve_snapping == SnappingBehaviour::Snap {
                for plane in control_storage.iter_arrows() {
                    if plane.with_rotation {
                        snap_forward_to_plane(
                            &mut parent_transform_mut,
                            plane.normalized,
                            control_rotation.normal,
                            2.0,
                        );
                        snap_forward_to_plane(
                            &mut parent_transform_mut,
                            -plane.normalized,
                            control_rotation.normal,
                            2.0,
                        );
                        snap_up_to_plane(
                            &mut parent_transform_mut,
                            plane.normalized,
                            control_rotation.normal,
                            1.5,
                        );
                        snap_up_to_plane(
                            &mut parent_transform_mut,
                            -plane.normalized,
                            control_rotation.normal,
                            1.5,
                        );
                    }
                }

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
    let parent_transform = *changeable_transforms.get(control_parent.entity).unwrap();

    let root = root.single().unwrap();

    let origin = if state.prism_mode == PrismMode::Prism {
        trigger.event().event.current_entity_position
    } else {
        trigger.event().event.real_current_entity_position
    };
    let inverse = root.compute_affine().inverse();
    let new_origin = if control_parent.use_root {
        inverse.transform_point3(origin)
    } else {
        origin
    };
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

        let angle = last_diff.angle_between(diff);
        let sign = (last_diff.cross(diff).dot(control_rotation.normal)).signum();

        let mut parent_transform_mut = changeable_transforms
            .get_mut(control_parent.entity)
            .unwrap();
        let mut inverse = {
            parent_transform_mut.rotation =
                Quat::from_axis_angle(control_rotation.normal, angle * sign)
                    * parent_transform_mut.rotation;

            parent_transform_mut.rotation.inverse()
        };

        if state.curve_snapping == SnappingBehaviour::Snap {
            for plane in control_storage.iter_arrows() {
                if plane.with_rotation {
                    snap_forward_to_plane(
                        &mut parent_transform_mut,
                        plane.normalized,
                        control_rotation.normal,
                        2.0,
                    );
                    snap_forward_to_plane(
                        &mut parent_transform_mut,
                        -plane.normalized,
                        control_rotation.normal,
                        2.0,
                    );
                    snap_up_to_plane(
                        &mut parent_transform_mut,
                        plane.normalized,
                        control_rotation.normal,
                        1.5,
                    );
                    snap_up_to_plane(
                        &mut parent_transform_mut,
                        -plane.normalized,
                        control_rotation.normal,
                        1.5,
                    );
                }
            }

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
    let curves = set.p1().collect(false);

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
        )) = snapped.get_mut(control_parent.entity)
        {
            if let Some(curve) = curves.get(snap_curve) {
                let p = curve.f(&[*snap_u]);
                transform.translation = p.into();

                let deriv = curve.derive(&[*snap_u], 1)[0];

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
#[allow(clippy::complexity)]
fn update_texts(
    mut transforms: Query<&mut Transform>,
    root: Query<Entity, With<RootTransform>>,
    coordinate_texts: Query<(Entity, &ChildOf, &Children, &CoordinateTextMarker)>,
    mut text3d: Query<&mut Text3d>,
    markers: Query<&AssignedShadowMarkers>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    info: Res<RenderInformation>,
) {
    let root = root.single().unwrap();
    let root_transform = *transforms.get(root).unwrap();
    let camera_transform = camera.single().unwrap();

    for (text_entity, &ChildOf(parent), children, marker) in coordinate_texts {
        let start_transform = *transforms.get(parent).unwrap();

        let camera_forward = if marker.use_root {
            root_transform
                .compute_affine()
                .inverse()
                .transform_vector3(camera_transform.forward().normalize_or_zero())
        } else {
            camera_transform.forward().as_vec3()
        };

        {
            let mut transform = transforms.get_mut(text_entity).unwrap();
            transform.rotation = start_transform.rotation.inverse();
        }

        let (start, diff, current) = if let Ok(markers) = markers.get(parent) {
            let marker = *markers
                .0
                .first()
                .expect("Only one marker can exists at a time");

            let shadow_position = *transforms
                .get(marker)
                .expect("A shadow must always exist when the text exists");

            (
                shadow_position.translation,
                start_transform.translation - shadow_position.translation,
                start_transform.translation,
            )
        } else {
            (
                start_transform.translation,
                Vec3::ZERO,
                start_transform.translation,
            )
        };

        let signs = (
            if diff.x >= 0.0 { "+" } else { "" },
            if diff.y >= 0.0 { "+" } else { "" },
            if diff.z >= 0.0 { "+" } else { "" },
        );

        for child in children {
            let mut transform = transforms.get_mut(*child).unwrap();
            transform.translation = -camera_forward * 0.3 * info.scale + Vec3::Y * info.scale;
            transform.look_to(camera_forward, Vec3::Y);
            if let Ok(mut text3d) = text3d.get_mut(*child) {
                *text3d = Text3d::new(format!(
                    "X {:.3}, \t{}{:.3}, \t{:.3}\nY {:.3}, \t{}{:.3}, \t{:.3}\nZ {:.3}, \t{}{:.3}, \t{:.3}",
                    start.x * 1000.0,
                    signs.0,
                    diff.x * 1000.0,
                    current.x * 1000.0,
                    start.y * 1000.0,
                    signs.1,
                    diff.y * 1000.0,
                    current.y * 1000.0,
                    start.z * 1000.0,
                    signs.2,
                    diff.z * 1000.0,
                    current.z * 1000.0,
                ));
            }
        }
    }
}

#[cfg(feature = "vr_enable")]
#[allow(clippy::complexity)]
fn update_texts(
    mut transforms: Query<&mut Transform>,
    root: Query<Entity, With<RootTransform>>,
    coordinate_texts: Query<
        (Entity, &ChildOf, &Children, &CoordinateTextMarker),
        Without<DistanceTextMarker>,
    >,
    distance_texts: Query<
        (Entity, &ChildOf, &Children, &DistanceTextMarker),
        Without<CoordinateTextMarker>,
    >,
    mut text3d: Query<&mut Text3d>,
    markers: Query<&AssignedShadowMarkers>,
    camera: Query<&GlobalTransform, With<XrTrackedView>>,
    info: Res<RenderInformation>,
) {
    let root = root.single().unwrap();
    let root_transform = *transforms.get(root).unwrap();
    let camera_transform = camera.single().unwrap();

    for (text_entity, &ChildOf(parent), children, marker) in coordinate_texts {
        let start_transform = *transforms.get(parent).unwrap();

        let camera_forward = if marker.use_root {
            root_transform
                .compute_affine()
                .inverse()
                .transform_vector3(camera_transform.forward().normalize_or_zero())
        } else {
            camera_transform.forward().as_vec3()
        };

        {
            let mut transform = transforms.get_mut(text_entity).unwrap();
            transform.rotation = start_transform.rotation.inverse();
        }

        let (start, diff, current) = if let Ok(markers) = markers.get(parent) {
            let marker = *markers
                .0
                .first()
                .expect("Only one marker can exists at a time");

            let shadow_position = *transforms
                .get(marker)
                .expect("A shadow must always exist when the text exists");

            (
                shadow_position.translation,
                start_transform.translation - shadow_position.translation,
                start_transform.translation,
            )
        } else {
            (
                start_transform.translation,
                Vec3::ZERO,
                start_transform.translation,
            )
        };

        for child in children {
            let mut transform = transforms.get_mut(*child).unwrap();
            transform.translation = -camera_forward * 0.3 * info.scale + Vec3::Y * info.scale;
            transform.look_to(camera_forward, Vec3::Y);
            if let Ok(mut text3d) = text3d.get_mut(*child) {
                *text3d = Text3d::new(format!(
                    "X {:.3}, {:.3}, {:.3}\nY {:.3}, {:.3}, {:.3}\nZ {:.3}, {:.3}, {:.3}",
                    start.x,
                    diff.x,
                    current.x,
                    start.y,
                    diff.y,
                    current.y,
                    start.z,
                    diff.z,
                    current.z,
                ));
            }
        }
    }
}

#[allow(clippy::complexity)]
fn toggle_visibility(
    mut reader: EventReader<ToggleRobotVisibilityEvent>,
    mut state: ResMut<TranslationControllerState>,
    mut control_parents: Query<
        (Entity, &mut Visibility),
        (With<ControlParent>, Without<TemporaryInvisible>),
    >,
    mut temporary_invisible: Query<
        (Entity, &mut Visibility),
        (With<ControlParent>, With<TemporaryInvisible>),
    >,
    mut commands: Commands,
    children: Query<&Children>,
    mut picking3d_interactable: Query<&mut Picking3dInteractable>,
    mut pickables: Query<&mut Pickable>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    state.invisible_robots = !state.invisible_robots;

    if state.invisible_robots {
        for (parent_entity, mut visibility) in &mut control_parents {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;

                let _ = commands.get_entity(parent_entity).map(|mut e| {
                    e.insert(TemporaryInvisible);
                    e.insert(Pickable::IGNORE);
                });

                if let Ok(children) = children.get(parent_entity) {
                    for child in children {
                        if let Ok(mut picking3d) = picking3d_interactable.get_mut(*child) {
                            *picking3d = Picking3dInteractable::Ignore;
                        }

                        if let Ok(mut pickable) = pickables.get_mut(*child) {
                            *pickable = Pickable::IGNORE;
                        }
                    }
                }
            }
        }
    } else {
        for (parent_entity, mut visibility) in &mut temporary_invisible {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;

                let _ = commands.get_entity(parent_entity).map(|mut e| {
                    e.remove::<TemporaryInvisible>();
                    e.remove::<Pickable>();
                });

                if let Ok(children) = children.get(parent_entity) {
                    for child in children {
                        if let Ok(mut picking3d) = picking3d_interactable.get_mut(*child) {
                            *picking3d = Picking3dInteractable::Default;
                        }

                        if let Ok(mut pickable) = pickables.get_mut(*child) {
                            *pickable = Pickable::default();
                        }
                    }
                }
            }
        }
    }
}

pub struct TranslationController;

impl Plugin for TranslationController {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                show_transitional_controls,
                register_deletes.before(show_transitional_controls),
                handle_toggle_snapping.run_if(on_event::<ToggleSnappingBehaviour>),
                handle_set_prism_mode.run_if(on_event::<SetPrismMode>),
                update_snapped_points,
                update_plane_directions,
                handle_translate_by_delta_event,
                update_texts,
                update_boxes,
                toggle_visibility.run_if(on_event::<ToggleRobotVisibilityEvent>),
            ),
        );
        app.init_resource::<ControlStorage>();

        app.init_resource::<TranslationControllerState>();
        app.init_resource::<AccumulatedMovementStore>();
        app.add_event::<ToggleSnappingBehaviour>();
        app.add_event::<MovedEntityEvent>();
        app.add_event::<MoveEntityByDeltaEvent>();
        app.add_event::<SetPrismMode>();
        app.add_event::<ToggleRobotVisibilityEvent>();
    }
}
