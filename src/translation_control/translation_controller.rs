use crate::RootTransform;
use crate::bezier_curve::bezier_curve_renderer::RedrawEvent;
use crate::bezier_curve::helper_curves::{
    ControlCurve, ControlCurvePoint, CurveCollection, RedrawCurvesEvent, TemporaryCurve,
    TemporaryCurvePoint,
};
use crate::bezier_curve::render_info::RenderInformation;
use crate::click_decider::LogTrace;
use crate::history::plugin::HistoryLogEvent;
use crate::nurbs::bezier::{
    de_casteljau, derive_after_de_casteljau, horner_scheme, shortest_distance_to_point,
};
use crate::nurbs::point::Point;
use crate::picking3d::events::{HoveredBy, MoveIn, MoveOut, Pointer3d};
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::translation_control::control_storage::ControlStorage;
use crate::util::update_material_on;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use bevy::color::palettes::tailwind::{YELLOW_400, YELLOW_600};
use bevy::ecs::relationship::RelatedSpawnerCommands;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::{Read, Write};
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

#[derive(Component)]
struct SnappedPoint {
    u: f64,
    curve: Entity,
}

#[derive(Component)]
pub struct ShadowMarker;

#[derive(Component)]
pub struct EnableTranslationControl;

#[derive(Component)]
struct SnappedArrow(Entity);

#[derive(Component)]
struct ControlParent(Entity);

#[derive(Component)]
struct Control(Vec3);

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
    controls: Query<(Entity, &ControlParent)>,
) {
    for event in deleted.read() {
        for (entity, contrl) in controls.iter() {
            if contrl.0 == event {
                commands.entity(entity).despawn();
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
) {
    let cuboid = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.4 * scale));
    let line = meshes.add(Cuboid::new(0.02 * scale, 0.02 * scale, 0.8 * scale));
    let arrow = meshes.add(Cone::new(0.035 * scale, 0.2 * scale));

    child_builder
        .spawn((
            Transform::from_xyz(0.0, 0.0, -0.4 * scale),
            MeshMaterial3d(mat.clone()),
            Mesh3d(cuboid.clone()),
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

fn show_transitional_controls(
    mut commands: Commands,
    to_enable: Query<(&Transform, Entity), Added<EnableTranslationControl>>,
    arrows: Res<ControlStorage>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    scale: Res<RenderInformation>,
) {
    let scale = scale.scale;

    for (t, entity) in to_enable.iter() {
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
                            Picking3dInteractable,
                            Visibility::default(),
                        ))
                        .with_children(|parent| {
                            draw_arrow(
                                parent,
                                materials.add(arrow.color),
                                materials.add(arrow.hover_color),
                                &mut meshes,
                                scale,
                            );
                        })
                        .observe(drag_controller)
                        .observe(drag_controller3d)
                        .observe(drag_start)
                        .observe(drag_start3d)
                        .observe(drag_end_trigger_redraw)
                        .observe(drag_end3d_trigger_redraw);
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
    all_other_transforms: Query<
        'w,
        's,
        Write<Transform>,
        (
            Without<Control>,
            Without<ControlParent>,
            Without<ControlCurve>,
            Without<ControlCurvePoint>,
        ),
    >,
    redraw_writer: EventWriter<'w, RedrawEvent>,
    redraw_curves_writer: EventWriter<'w, RedrawCurvesEvent>,
    curves: CurveCollection<'w, 's>,
    snapped: Query<'w, 's, (Entity, Write<SnappedPoint>)>,
    info: Res<'w, RenderInformation>,
    state: Res<'w, TranslationControllerState>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
}

impl<'w, 's> ObligatoryDragParams<'w, 's> {
    fn update_position_drag_universal(
        &mut self,
        control_parent: &ControlParent,
        translation: Vec3,
    ) {
        // Only adjust the control parent
        let is_already_snapped = self.snapped.get(control_parent.0).is_ok();
        let control_point = self.all_other_transforms.get_mut(control_parent.0);
        if let Ok(mut t) = control_point {
            if self.state.curve_snapping == SnappingBehaviour::Snap {
                if !is_already_snapped {
                    let shortest = self.curves.collect_shortest(Point::from(t.translation));

                    if let Some((curve, u, p, dist, points)) = shortest
                        && dist < 0.001 * self.info.scale as f64
                    {
                        t.translation = p.into();

                        let mat = self.materials.add(StandardMaterial::from_color(YELLOW_600));
                        let mat_hover =
                            self.materials.add(StandardMaterial::from_color(YELLOW_400));

                        let deriv = derive_after_de_casteljau(&de_casteljau(&points, u), 1);

                        self.commands
                            .get_entity(control_parent.0)
                            .unwrap()
                            .insert(SnappedPoint { u, curve })
                            .with_children(|cmd| {
                                cmd.spawn((
                                    SnappedArrow(control_parent.0),
                                    Transform::default().looking_to(Vec3::from(deriv), Vec3::Y),
                                ))
                                .with_children(|cmd| {
                                    draw_arrow(
                                        cmd,
                                        mat,
                                        mat_hover,
                                        &mut self.meshes,
                                        self.info.scale,
                                    );
                                })
                                .observe(drag_snap_arrow)
                                .observe(drag_snap_arrow3d);
                            });
                        // TODO: Add arrow, that moves along the curvature of bezier curve that was snapped
                        // to
                    } else {
                        t.translation += translation;
                    }
                } else {
                    let point = Point::from(t.translation + translation);

                    let shortest = self.curves.collect_shortest(point);
                    if let Some((curve, u, p, dist, _)) = shortest
                        && dist < 0.001 * self.info.scale as f64
                    {
                        let mut snap = self.snapped.get_mut(control_parent.0).unwrap().1;
                        snap.curve = curve;
                        snap.u = u;

                        t.translation = p.into();
                    } else {
                        let entity = self.snapped.get(control_parent.0).unwrap().0;
                        self.commands
                            .get_entity(entity)
                            .unwrap()
                            .remove::<SnappedPoint>();
                        t.translation = point.into();
                    }
                }
            } else {
                t.translation += translation;
            }
        };

        self.redraw_writer.write(RedrawEvent::Fast);
        self.redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

#[allow(clippy::complexity)]
fn drag_controller(
    trigger: Trigger<Pointer<Drag>>,
    control_query: Query<(&Control, &ChildOf)>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut control_parents: Query<&ControlParent>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut params: ParamSet<(ObligatoryDragParams, Query<&GlobalTransform>)>,
) {
    let (control, child_of) = control_query.get(trigger.target()).unwrap();

    let parent = child_of.parent();
    let control_parent = control_parents.get_mut(parent).unwrap();

    let (camera, camera_transform) = camera.single().unwrap();

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
        .update_position_drag_universal(control_parent, translation);
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

    let control_parent = control_parents.get_mut(parent).unwrap();

    params.update_position_drag_universal(control_parent, translation);
}

#[allow(clippy::complexity)]
fn drag_snap_arrow(
    trigger: Trigger<Pointer<Drag>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    mut snapped: Query<&mut SnappedPoint, Without<SnappedArrow>>,
    mut params: ParamSet<(
        Query<(&Transform, &SnappedArrow), Without<SnappedPoint>>,
        Query<&GlobalTransform>,
    )>,
) {
    let (arrow_transform, arrow_entity) = {
        let p0 = params.p0();
        let arrow = p0.get(trigger.target()).unwrap();
        (*arrow.0, arrow.1.0)
    };

    let (camera, camera_transform) = camera.single().unwrap();

    let diff = {
        let dist = (params.p1().get(arrow_entity).unwrap().translation()
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

    let axis = arrow_transform.translation;
    let direction = (diff.dot(axis)) / (diff.length() * axis.length());
    let change = direction * trigger.delta.length() * 0.01;

    let mut snap = snapped.get_mut(arrow_entity).unwrap();
    snap.u += change as f64;
}

fn drag_snap_arrow3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::Drag>>,
    root: Query<&GlobalTransform, With<RootTransform>>,
    arrows: Query<(&Transform, &SnappedArrow), Without<SnappedPoint>>,
    mut snapped: Query<&mut SnappedPoint, Without<SnappedArrow>>,
) {
    let arrow = arrows.get(trigger.target()).unwrap();

    let diff = trigger.event.delta;
    let diff = root
        .single()
        .unwrap()
        .affine()
        .inverse()
        .transform_point3(diff);

    let axis = arrow.0.translation;
    let direction = (axis.dot(diff)) / (axis.length() * diff.length());
    let change = direction * diff.length();

    let mut snap = snapped.get_mut(arrow.1.0).unwrap();
    snap.u += change as f64;
}

#[allow(clippy::complexity)]
/// Update both arrows and snaps, despawning / removing them if invalid
fn update_snapped_points(
    mut commands: Commands,
    mut set: ParamSet<(
        (
            Query<(&mut Transform, &SnappedArrow, Entity), Without<SnappedPoint>>, // arrows
            Query<(&mut Transform, &SnappedPoint, Entity), Without<SnappedArrow>>, // snapped
        ),
        CurveCollection,
    )>,
) {
    let curves = set.p1().collect();

    let (mut arrows, mut snapped) = set.p0();

    for (mut t, arrow, arrow_entity) in &mut arrows {
        if let Ok((mut transform, snap, entity)) = snapped.get_mut(arrow.0) {
            if let Some(curve) = curves.get(&snap.curve) {
                let p = horner_scheme(curve, snap.u);
                transform.translation = p.into();

                let deriv = derive_after_de_casteljau(&de_casteljau(curve, snap.u), 1);
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
    }
}
