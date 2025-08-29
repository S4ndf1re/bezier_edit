use crate::bezier_curve::bezier_curve_renderer::RedrawEvent;
use crate::bezier_curve::helper_curves::{CurveCollection, RedrawCurvesEvent};
use crate::bezier_curve::render_info::RenderInformation;
use crate::click_decider::LogTrace;
use crate::history::plugin::HistoryLogEvent;
use crate::nurbs::bezier::{de_casteljau, derive_after_de_casteljau};
use crate::nurbs::parametric::{Circle3D, MinDistanceToPoint, Parametric};
use crate::nurbs::point::Point;
use crate::picking3d::events::{HoveredBy, MoveIn, MoveOut, Pointer3d};
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::translation_control::control_storage::ControlStorage;
use crate::util::update_material_on;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use crate::{MainCamera, RootTransform};
use bevy::color::palettes::tailwind::{
    BLUE_600, BLUE_800, GRAY_500, PURPLE_900, RED_600, RED_800, RED_900, YELLOW_400, YELLOW_600,
    YELLOW_900,
};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::{Read, Write};
use bevy::input_focus::directional_navigation;
use bevy::prelude::*;
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;

use super::control_storage::{self, ControlDirection};

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

#[derive(Component)]
struct SnappedPoint {
    u: f64,
    curve: Entity,
}

#[derive(Component)]
pub struct ShadowMarker;

#[derive(Component, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum EnableTranslationControl {
    OnlyTranslation,
    WithRotation,
    OnlyOnPlane(Entity),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ArrowDirection {
    Up,
    Left,
}
#[derive(Component)]
struct OnPlaneMovableMarker {
    plane: Entity,
    arrow_direction: ArrowDirection,
}

impl EnableTranslationControl {
    pub fn new(with_rotation: bool) -> Self {
        if with_rotation {
            Self::WithRotation
        } else {
            Self::OnlyTranslation
        }
    }
}

impl Default for EnableTranslationControl {
    fn default() -> Self {
        Self::new(false)
    }
}

#[derive(Component, Clone, Copy)]
struct SnappedArrow;

#[derive(Component)]
struct ControlParent(Entity);

#[derive(Component)]
struct Control(Vec3);

#[derive(Component)]
struct ControlRotation {
    normal: Vec3,
    radius: f64,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnappingBehaviour {
    #[default]
    NoSnap,
    Snap,
}

#[derive(Resource, Default)]
struct TranslationControllerState {
    curve_snapping: SnappingBehaviour,
}

#[derive(Event)]
pub struct ToggleSnappingBehaviour;

#[derive(Component, Clone, Default)]
pub enum CantSnapToCurve {
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
) {
    for event in deleted.read() {
        for (entity, mut visibility, contrl) in controls.iter_mut() {
            if contrl.0 == event {
                *visibility = Visibility::Hidden;
                commands.entity(entity).insert(Pickable::IGNORE);
            }
        }
    }
}

fn draw_arrow(
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
        Picking3dInteractable,
    ));
    obj.observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer<Out>>(mat.clone()))
        .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
        .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()));
    if !is_shadow {
        obj.observe(
            |trigger: Trigger<Pointer3d<MoveIn>>,
             mut writer_left: EventWriter<VibrateLeftEvent>,
             mut writer_right: EventWriter<VibrateRightEvent>| {
                match trigger.controler {
                    HoveredBy::Left => {
                        writer_left.write(VibrateLeftEvent::new(Vibration::default()));
                    }
                    HoveredBy::Right => {
                        writer_right.write(VibrateRightEvent::new(Vibration::default()));
                    }
                };
            },
        );
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
                Picking3dInteractable,
            ))
            .observe(update_material_on::<Pointer<Over>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer<Out>>(mat.clone()))
            .observe(update_material_on::<Pointer3d<MoveIn>>(mat_hover.clone()))
            .observe(update_material_on::<Pointer3d<MoveOut>>(mat.clone()))
            .observe(
                |trigger: Trigger<Pointer3d<MoveIn>>,
                 mut writer_left: EventWriter<VibrateLeftEvent>,
                 mut writer_right: EventWriter<VibrateRightEvent>| {
                    match trigger.controler {
                        HoveredBy::Left => {
                            writer_left.write(VibrateLeftEvent::new(Vibration::default()));
                        }
                        HoveredBy::Right => {
                            writer_right.write(VibrateRightEvent::new(Vibration::default()));
                        }
                    };
                },
            );
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
) {
    let scale = scale.scale;

    for (entity, enabled_control) in to_enable.iter() {
        let mut already_created = false;
        for (mut visibility, parent) in already_existing.iter_mut() {
            if parent.0 == entity {
                *visibility = Visibility::Inherited;
                already_created = true;
                commands.entity(entity).remove::<Pickable>();
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
                        for arrow in arrows.as_ref().iter() {
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
                                    .observe(rotate_controller)
                                    .observe(rotate_controller3d)
                                    .observe(rotate_end_trigger_redraw)
                                    .observe(rotate_end_trigger_redraw3d);
                            }
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
    root: Query<Entity, With<RootTransform>>,
    mut commands: Commands,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    all_transforms: Query<&Transform, Without<ControlParent>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    arrows: Res<ControlStorage>,
    scale: Res<RenderInformation>,
    mut history: EventWriter<HistoryLogEvent>,
) {
    let mut root = commands.get_entity(root.single().unwrap()).unwrap();
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();

    let control_parent = control_parents.get(dragged_parent).unwrap();
    let mut start_transform = *all_transforms.get(control_parent.0).unwrap();
    start_transform.rotation = Quat::IDENTITY;

    let scale = scale.scale;

    root.with_children(|cmd| {
        cmd.spawn((ShadowMarker, start_transform, Visibility::default()))
            .with_children(|parent| {
                for arrow in arrows.as_ref().iter() {
                    parent
                        .spawn((
                            Transform::from_xyz(0.0, 0.0, 0.0)
                                .looking_to(arrow.normalized, Vec3::Y),
                            Control(arrow.normalized),
                            Picking3dInteractable,
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
    });

    history.write(HistoryLogEvent::Begin(
        control_parent.0,
        Some(start_transform),
    ));
}

#[allow(clippy::too_many_arguments)]
fn drag_start3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::DragStart>>,
    root: Query<Entity, With<RootTransform>>,
    mut commands: Commands,
    arrow_query: Query<(&ChildOf, Entity), With<Control>>,
    control_parents: Query<&ControlParent>,
    all_transforms: Query<&Transform, Without<ControlParent>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    arrows: Res<ControlStorage>,
    scale: Res<RenderInformation>,
    mut history: EventWriter<HistoryLogEvent>,
    mut trace_log_writer: EventWriter<LogTrace>,
) {
    let mut root = commands.get_entity(root.single().unwrap()).unwrap();
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();

    let control_parent = control_parents.get(dragged_parent).unwrap();
    let start_transform = all_transforms.get(control_parent.0).unwrap();

    let scale = scale.scale;

    root.with_children(|cmd| {
        cmd.spawn((ShadowMarker, *start_transform, Visibility::default()))
            .with_children(|parent| {
                for arrow in arrows.as_ref().iter() {
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
                                materials.add(arrow.shadow_color),
                                materials.add(arrow.shadow_color),
                                &mut meshes,
                                scale,
                                true,
                            );
                        });
                }
            });
    });

    history.write(HistoryLogEvent::Begin(
        control_parent.0,
        Some(*start_transform),
    ));

    trace_log_writer.write(LogTrace::default());
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
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    for entity in query {
        commands.get_entity(entity).unwrap().despawn();
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
    mut trace_log_writer: EventWriter<LogTrace>,
) {
    let dragged_entity = trigger.target();
    let (dragged_childof, _) = arrow_query.get(dragged_entity).unwrap();

    let dragged_parent = dragged_childof.parent();
    let control_parent = control_parents.get(dragged_parent).unwrap();

    redraw_writer.write(RedrawEvent::HighQuality);
    redraw_curves_writer.write(RedrawCurvesEvent);

    for entity in query {
        commands.get_entity(entity).unwrap().despawn();
    }

    history.write(HistoryLogEvent::End(control_parent.0, None));

    trace_log_writer.write(LogTrace::default());
}

#[allow(clippy::complexity)]
#[derive(SystemParam)]
struct ObligatoryDragParams<'w, 's> {
    commands: Commands<'w, 's>,
    transform_set: ParamSet<
        'w,
        's,
        (
            Query<'w, 's, Write<Transform>, (Without<Control>, Without<ControlParent>)>,
            CurveCollection<'w, 's>,
        ),
    >,
    redraw_writer: EventWriter<'w, RedrawEvent>,
    redraw_curves_writer: EventWriter<'w, RedrawCurvesEvent>,
    snapped: Query<'w, 's, (Entity, Write<SnappedPoint>)>,
    info: Res<'w, RenderInformation>,
    state: Res<'w, TranslationControllerState>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    cant_snap_to_curve: Query<'w, 's, Read<CantSnapToCurve>>,
    moved_entity_writer: EventWriter<'w, MovedEntityEvent>,
}

impl<'w, 's> ObligatoryDragParams<'w, 's> {
    fn update_position_drag_universal(
        &mut self,
        control_parent: (Entity, &ControlParent),
        translation: Vec3,
    ) {
        // Only adjust the control parent
        let is_already_snapped = self.snapped.get(control_parent.1.0).is_ok();
        let cant_snap_to_curve = self.cant_snap_to_curve.get(control_parent.1.0).ok();

        let changed_entity = control_parent.1.0;
        let control_point = self.transform_set.p0().get(control_parent.1.0).copied();
        if let Ok(t) = control_point {
            let started_translation = t.translation;
            let ending_translation = if self.state.curve_snapping == SnappingBehaviour::Snap {
                if !is_already_snapped {
                    let shortest = self.transform_set.p1().collect_shortest(
                        Point::from(t.translation),
                        cant_snap_to_curve.unwrap_or(&CantSnapToCurve::default()),
                    );

                    if let Some((curve, u, p, dist, points)) = shortest
                        && dist < 0.05 * self.info.scale as f64
                    {
                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();

                        t.translation = p.into();

                        let mat = self.materials.add(StandardMaterial::from_color(YELLOW_600));
                        let mat_hover =
                            self.materials.add(StandardMaterial::from_color(YELLOW_400));

                        let deriv = derive_after_de_casteljau(&de_casteljau(&points, u), 1);

                        self.commands
                            .get_entity(control_parent.1.0)
                            .unwrap()
                            .insert(SnappedPoint { u, curve });

                        self.commands
                            .get_entity(control_parent.0)
                            .unwrap()
                            .with_children(|cmd| {
                                cmd.spawn((
                                    Control(Vec3::from(deriv)),
                                    SnappedArrow,
                                    Transform::default().looking_to(Vec3::from(deriv), Vec3::Y),
                                    Visibility::Inherited,
                                ))
                                .with_children(|cmd| {
                                    draw_arrow(
                                        cmd,
                                        mat,
                                        mat_hover,
                                        &mut self.meshes,
                                        self.info.scale,
                                        false,
                                    );
                                })
                                .observe(drag_controller)
                                .observe(drag_controller3d);
                            });
                        t.translation
                    } else {
                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation += translation;
                        t.translation
                    }
                } else {
                    let point = Point::from(t.translation + translation);

                    let shortest = self.transform_set.p1().collect_shortest(
                        point,
                        cant_snap_to_curve.unwrap_or(&CantSnapToCurve::default()),
                    );
                    if let Some((curve, u, p, dist, _)) = shortest
                        && dist < 0.05 * self.info.scale as f64
                    {
                        let mut snap = self.snapped.get_mut(control_parent.1.0).unwrap().1;
                        snap.curve = curve;
                        snap.u = u;

                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation = p.into();
                        t.translation
                    } else {
                        let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                        self.commands
                            .get_entity(entity)
                            .unwrap()
                            .remove::<SnappedPoint>();

                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation = point.into();
                        t.translation
                    }
                }
            } else {
                let mut p0 = self.transform_set.p0();
                let mut t = p0.get_mut(control_parent.1.0).unwrap();
                t.translation += translation;
                t.translation
            };

            // Trigger the moved event, so that linked entities (TODO) may update the position of the
            // linked entity, correspondingly (using lokal transforms)
            if let Ok(mut entity) = self.commands.get_entity(changed_entity) {
                let delta = ending_translation - started_translation;
                entity.trigger(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
                self.moved_entity_writer.write(MovedEntityEvent {
                    entity: changed_entity,
                    delta,
                });
            }
        }

        self.redraw_writer.write(RedrawEvent::Fast);
        self.redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

fn handle_translate_by_delta_event(
    mut reader: EventReader<MoveEntityByDeltaEvent>,
    mut obligatory: ObligatoryDragParams,
    children: Query<&Children>,
    control_parents: Query<(Entity, &ControlParent)>,
) {
    for evt in reader.read() {
        if let Ok(children) = children.get(evt.entity) {
            for child in children {
                if let Ok(control_parent) = control_parents.get(*child) {
                    obligatory.update_position_drag_universal(control_parent, evt.delta);
                }
            }
        }
    }
}

#[allow(clippy::complexity)]
fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(&Control, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(ObligatoryDragParams, Query<&GlobalTransform>)>,
) {
    let (control, child_of) = control_query.get(trigger.target()).unwrap();

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

        params
            .p0()
            .update_position_drag_universal((parent, control_parent), translation);
    }
}

#[allow(clippy::complexity)]
fn drag_controller3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    control_query: Query<(&Control, &ChildOf)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ObligatoryDragParams,
) {
    // NOTE: Make sure that the draw event is triggered only once. Otherwise this difference adding happens multiple times for the same event........
    let (control, child_of) = control_query.get(trigger.target()).unwrap();

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

    params.update_position_drag_universal((parent, control_parent), translation);
}

#[allow(clippy::complexity)]
fn rotate_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(&ControlRotation, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform), (Without<RootTransform>, With<MainCamera>)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    global_transforms: Query<&GlobalTransform, (Without<RootTransform>, Without<Camera>)>,
    mut changable_transforms: Query<&mut Transform>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    control_storage: Res<ControlStorage>,
    state: Res<TranslationControllerState>,
) {
    let (control_rotation, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();
    let parent_transform = *changable_transforms.get(control_parent.0).unwrap();
    let root = root.single().unwrap();

    if let Ok((camera, camera_transform)) = camera.single() {
        let (start, diff) = {
            let dist = (global_transforms
                .get(control_parent.0)
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
            (start, root.affine().inverse().transform_point3(end - start))
        };

        let circle = Circle3D::new(
            parent_transform.translation.into(),
            control_rotation.radius,
            control_rotation.normal.into(),
        );

        let closest = circle.min_distance_to_point(start.into());

        let axis: Vec3 = circle.derive(&closest.params, 1).into();
        info!("axis: {axis}");

        let direction = (diff.dot(axis)) / (diff.length() * axis.length());
        info!("Direction {direction}");
        let angle = direction * diff.length();

        let mut parent_transform_mut = changable_transforms.get_mut(control_parent.0).unwrap();
        let (mut inverse, forward, up) = {
            parent_transform_mut.rotation = Quat::from_axis_angle(control_rotation.normal, angle)
                * parent_transform_mut.rotation;
            (
                parent_transform_mut.rotation.inverse(),
                parent_transform_mut.forward(),
                parent_transform_mut.up(),
            )
        };

        if state.curve_snapping == SnappingBehaviour::Snap {
            for axis in control_storage.iter() {
                // info!(
                //     "With axis {}, and forward {forward:?}, the diff is {}",
                //     axis.normalized,
                //     forward.normalize_or_zero().dot(axis.normalized)
                // );
                if axis.with_rotation
                    && forward.normalize_or_zero().dot(axis.normalized).abs() > 0.990
                {
                    parent_transform_mut.look_to(axis.normalized, up);
                    inverse = parent_transform_mut.rotation.inverse();
                    break;
                }

                if axis.with_rotation && up.normalize_or_zero().dot(axis.normalized).abs() > 0.990 {
                    parent_transform_mut.look_to(forward, axis.normalized);
                    inverse = parent_transform_mut.rotation.inverse();
                    break;
                }
            }
        }

        let mut arrow_transform = changable_transforms.get_mut(parent).unwrap();
        arrow_transform.rotation = inverse;

        redraw_writer.write(RedrawEvent::Fast);
        redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

#[allow(clippy::complexity)]
fn rotate_controller3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    control_query: Query<(&ControlRotation, &ChildOf)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut changable_transforms: Query<&mut Transform>,
    mut redraw_writer: EventWriter<RedrawEvent>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    let (control_rotation, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();
    let parent_transform = *changable_transforms.get(control_parent.0).unwrap();

    let (start, diff) = {
        let diff = trigger.event.delta;
        let start = trigger.event.current_entity_position - diff;

        (
            start,
            root.single()
                .unwrap()
                .affine()
                .inverse()
                .transform_point3(diff),
        )
    };

    let circle = Circle3D::new(
        parent_transform.translation.into(),
        control_rotation.radius,
        control_rotation.normal.into(),
    );

    let closest = circle.min_distance_to_point(start.into());
    let axis: Vec3 = circle.derive(&closest.params, 1).into();

    let direction = (diff.dot(axis)) / (diff.length() * axis.length());
    let angle = direction * diff.length();

    let inverse = {
        let mut parent_transform = changable_transforms.get_mut(control_parent.0).unwrap();
        parent_transform.rotation =
            Quat::from_axis_angle(control_rotation.normal, angle) * parent_transform.rotation;
        parent_transform.rotation.inverse()
    };

    let mut arrow_transform = changable_transforms.get_mut(parent).unwrap();
    arrow_transform.rotation = inverse;

    redraw_writer.write(RedrawEvent::Fast);
    redraw_curves_writer.write(RedrawCurvesEvent);
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
        if let Ok((mut transform, snap, entity)) = snapped.get_mut(control_parent.0) {
            if let Some(curve) = curves.get(&snap.curve) {
                let p = *de_casteljau(curve, snap.u).last().unwrap().last().unwrap();
                transform.translation = p.into();

                let deriv = derive_after_de_casteljau(&de_casteljau(curve, snap.u), 1);
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
            ),
        );
        app.init_resource::<ControlStorage>();

        app.init_resource::<TranslationControllerState>();
        app.add_event::<ToggleSnappingBehaviour>();
        app.add_event::<MovedEntityEvent>();
        app.add_event::<MoveEntityByDeltaEvent>();
    }
}
