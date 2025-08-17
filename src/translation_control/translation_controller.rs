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
use bevy::color::palettes::tailwind::{YELLOW_400, YELLOW_600};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::{Read, Write};
use bevy::prelude::*;
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;

#[derive(Event)]
pub struct MovedEvent {
    delta: Vec3,
}

#[derive(Component)]
struct SnappedPoint {
    u: f64,
    curve: Entity,
}

#[derive(Component)]
pub struct ShadowMarker;

#[derive(Component)]
pub struct EnableTranslationControl {
    with_rotation: bool,
}

impl EnableTranslationControl {
    pub fn new(with_rotation: bool) -> Self {
        Self { with_rotation }
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
    NoSnap,
    #[default]
    Snap,
}
#[derive(Resource, Default)]
struct TranslationControllerState {
    curve_snapping: SnappingBehaviour,
}

#[derive(Event)]
pub struct ToggleSnappingBehaviour;

#[derive(Component)]
pub struct CantSnapToCurve(pub Entity);

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
            cmd.spawn((
                ControlParent(entity),
                Transform::default(),
                Visibility::default(),
            ))
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

                    if enabled_control.with_rotation {
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
                            .observe(rotate_controller);
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
        Some(*start_transform),
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
    gizmos: Gizmos<'w, 's>,
    cant_snap_to_curve: Query<'w, 's, Read<CantSnapToCurve>>,
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
        let mut ignore_curves = HashSet::new();
        if let Some(curve) = cant_snap_to_curve {
            ignore_curves.insert(curve.0);
        }

        let changed_entity = control_parent.1.0;
        let control_point = self.transform_set.p0().get(control_parent.1.0).copied();
        if let Ok(t) = control_point {
            let started_translation = t.translation;
            if self.state.curve_snapping == SnappingBehaviour::Snap {
                if !is_already_snapped {
                    let shortest = self
                        .transform_set
                        .p1()
                        .collect_shortest(Point::from(t.translation), ignore_curves);

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
                    } else {
                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation += translation;
                    }
                } else {
                    let point = Point::from(t.translation + translation);

                    let shortest = self
                        .transform_set
                        .p1()
                        .collect_shortest(point, ignore_curves);
                    if let Some((curve, u, p, dist, _)) = shortest
                        && dist < 0.05 * self.info.scale as f64
                    {
                        let mut snap = self.snapped.get_mut(control_parent.1.0).unwrap().1;
                        snap.curve = curve;
                        snap.u = u;

                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation = p.into();
                    } else {
                        let entity = self.snapped.get(control_parent.1.0).unwrap().0;
                        self.commands
                            .get_entity(entity)
                            .unwrap()
                            .remove::<SnappedPoint>();

                        let mut p0 = self.transform_set.p0();
                        let mut t = p0.get_mut(control_parent.1.0).unwrap();
                        t.translation = point.into();
                    }
                }
            } else {
                let mut p0 = self.transform_set.p0();
                let mut t = p0.get_mut(control_parent.1.0).unwrap();
                t.translation += translation;
            }

            // Trigger the moved event, so that linked entities (TODO) may update the position of the
            // linked entity, correspondingly (using lokal transforms)
            if let Ok(mut entity) = self.commands.get_entity(changed_entity) {
                let delta = t.translation - started_translation;
                entity.trigger(MovedEvent { delta });
            }
        }

        self.redraw_writer.write(RedrawEvent::Fast);
        self.redraw_curves_writer.write(RedrawCurvesEvent);
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
) {
    let (control_rotation, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();
    let parent_transform = *changable_transforms.get(control_parent.0).unwrap();

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
            (
                start,
                root.single()
                    .unwrap()
                    .affine()
                    .inverse()
                    .transform_point3(end - start),
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
    }
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
                arrow.0 = Vec3::from(deriv);
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
            ),
        );
        app.init_resource::<ControlStorage>();

        app.init_resource::<TranslationControllerState>();
        app.add_event::<ToggleSnappingBehaviour>();
        app.add_event::<MovedEvent>();
    }
}
