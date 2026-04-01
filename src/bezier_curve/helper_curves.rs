use std::collections::HashMap;

use super::bezier_curve_renderer::EndModeEvent;
pub(crate) use super::bridges::BridgeSpawner;
pub(crate) use super::{
    EntityDeletedEvent, components::ControlState, render_info::RenderInformation,
};
use crate::bezier_curve::bezier_curve_renderer::hover_3d;
use crate::bezier_curve::bridges::BridgeDespawner;
use crate::nurbs::bezier::{de_casteljau, increase_degree};
use crate::nurbs::parametric::Parametric;
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::{AddBoundingEntityEvent, BoundingEntitiesManager, DisplayIn};
use crate::translation_control::translation_controller::{
    CantSnapToCurve, EnableTranslationControlType, SnappedPoint,
};
use crate::translation_control::{enable_gizmo, enable_gizmo3d};
#[cfg(feature = "vr_enable")]
use crate::vr_control::{GripLeft, GripRight};

use crate::vr_menu::VrMenuRoot;
use crate::{MainCamera, picking3d};
use crate::{
    RootTransform,
    nurbs::{bezier::shortest_distance_to_point, point::Point},
    picking3d::events::{self, Pointer3d},
    translation_control::translation_controller::EnableTranslationControl,
};
use bevy::render::view::RenderLayers;
use bevy::{
    color::palettes::tailwind::PURPLE_600,
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
};
use bevy_lunex::UiLayoutRoot;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use scopeguard::defer;

#[derive(Component)]
pub struct IgnoreCurve;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurve;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurvePoint(pub usize);

#[derive(Component)]
#[require(Transform)]
pub struct TemporaryCurve;

#[derive(Component)]
#[require(Transform)]
pub struct TemporaryCurvePoint(usize);

#[derive(Resource, Default)]
pub struct CreateCurveState {
    counter: usize,
}

#[derive(Component)]
#[relationship_target(relationship = CurveSegment, linked_spawn)]
pub struct CurveSegments(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = CurveSegments)]
pub struct CurveSegment {
    u: usize,
    #[relationship]
    curve: Entity,
}

#[derive(Component)]
#[require(Transform)]
pub struct CurveSupportPoint {
    u: f64,
}

#[derive(Event)]
pub struct RedrawCurvesEvent;

#[cfg(feature = "vr_enable")]
#[derive(Component)]
pub struct PreviewSphereMarker;

/// Spawn a new curve. Note that this does not add the curve and its points to the BoundingEntityManager using the AddBoundingEntity Event
pub fn spawn_new_curve(
    commands: &mut Commands,
    root: Entity,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    points: &[Vec3],
    render_info: &Res<RenderInformation>,
    hidden: bool,
) -> (Entity, Vec<(usize, Entity, Vec3)>) {
    let parent = if hidden {
        commands
            .spawn((
                ControlCurve,
                IgnoreCurve,
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                Name::new("Control Curve"),
                Transform::default(),
                Visibility::Inherited,
                ChildOf(root),
            ))
            .id()
    } else {
        commands
            .spawn((
                ControlCurve,
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                Name::new("Control Curve"),
                Transform::default(),
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_click_event_on_curve_level)
            .observe(handle_click_event_on_curve_level3d)
            .id()
    };

    let sphere = meshes.add(Sphere::new(0.08 * render_info.scale));
    let material = materials.add(StandardMaterial::from_color(PURPLE_600));

    let mut indexed_points = Vec::with_capacity(points.len());

    for (idx, p) in points.iter().enumerate() {
        let child = if hidden {
            commands
                .spawn((
                    ChildOf(parent),
                    Transform::from_translation(*p),
                    ControlCurvePoint(idx),
                    Picking3dInteractable::default(),
                    CantSnapToCurve::Single(parent),
                    Name::new("Curve Point"),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material.clone()),
                    Visibility::Hidden,
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .id()
        } else {
            commands
                .spawn((
                    ChildOf(parent),
                    Transform::from_translation(*p),
                    ControlCurvePoint(idx),
                    Picking3dInteractable::default(),
                    CantSnapToCurve::Single(parent),
                    Name::new("Curve Point"),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material.clone()),
                    Visibility::Inherited,
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .observe(hover_3d)
                .observe(handle_click_on_curve_point3d)
                .observe(handle_click_on_curve_point)
                .id()
        };

        indexed_points.push((idx, child, *p));
    }

    if points.len() >= 2 {
        let (sphere_mesh, purple) = {
            (
                meshes.add(Sphere::new(0.03 * render_info.scale)),
                materials.add(StandardMaterial::from_color(PURPLE_600)),
            )
        };

        for i in 0..=10 {
            let u = if hidden {
                i as f64 / 10.0
            } else {
                (i + 1) as f64 / 12.0
            };

            let points: Vec<Point> = points.iter().map(|p| Point::from(*p)).collect();

            let point = points.f(&[u]);

            commands.spawn((
                ChildOf(parent),
                CurveSupportPoint { u },
                Transform::from_translation(Vec3::from(point)),
                Mesh3d(sphere_mesh.clone()),
                MeshMaterial3d(purple.clone()),
            ));
        }
    }
    (parent, indexed_points)
}

#[cfg(feature = "vr_enable")]
#[allow(clippy::complexity)]
pub fn add_point_3d(
    mut commands: Commands,
    mut reader: EventReader<Pointer3d<events::Click>>,
    mut state: ResMut<CreateCurveState>,
    temp_curve: Query<Entity, With<TemporaryCurve>>,
    root: Query<&Transform, With<RootTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    render_info: Res<RenderInformation>,
) {
    let parent = temp_curve.single().unwrap();
    let root = root.single().unwrap();
    let mut redraw = false;

    let sphere = meshes.add(Sphere::new(0.08 * render_info.scale));
    let material = materials.add(StandardMaterial::from_color(PURPLE_600));

    for evt in reader.read() {
        let pos = root
            .compute_affine()
            .inverse()
            .transform_point3(evt.position);

        commands.get_entity(parent).unwrap().with_children(|cmd| {
            cmd.spawn((
                TemporaryCurvePoint(state.counter),
                Transform::from_translation(pos),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                Visibility::Inherited,
            ));
        });
        state.counter += 1;
        redraw = true;
    }

    if redraw {
        redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

#[allow(clippy::complexity)]
pub fn add_point(
    mut reader: EventReader<Pointer<Click>>,
    mut commands: Commands,
    mut state: ResMut<CreateCurveState>,
    camera: Query<(&GlobalTransform, &Camera), With<MainCamera>>,
    temp_curve: Query<Entity, With<TemporaryCurve>>,
    root: Query<&Transform, With<RootTransform>>,
    ui_root: Query<&UiLayoutRoot>,
    vr_ui_root: Query<&VrMenuRoot>,
    childof: Query<&ChildOf>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    render_info: Res<RenderInformation>,
) {
    let parent = temp_curve.single().unwrap();
    let root = root.single().unwrap();
    let mut redraw = false;

    let sphere = meshes.add(Sphere::new(0.08 * render_info.scale));
    let material = materials.add(StandardMaterial::from_color(PURPLE_600));

    for evt in reader.read() {
        let mut is_ui_element = false;
        for parent in childof.iter_ancestors(evt.target) {
            if ui_root.get(parent).is_ok() || vr_ui_root.get(parent).is_ok() {
                is_ui_element = true;
                break;
            }
        }

        if is_ui_element {
            continue;
        }

        if let Ok((camera_transform, camera)) = camera.single()
            && let Ok(position_ray) =
                camera.viewport_to_world(camera_transform, evt.pointer_location.position)
            && let Some(hit) = position_ray.intersect_plane(
                Vec3::ZERO,
                InfinitePlane3d::new(-camera_transform.forward()),
            )
        {
            let position = position_ray.get_point(hit);
            let pos = root.compute_affine().inverse().transform_point3(position);

            commands.get_entity(parent).unwrap().with_children(|cmd| {
                cmd.spawn((
                    TemporaryCurvePoint(state.counter),
                    Name::new(format!("Curve point {}", state.counter)),
                    Transform::from_translation(pos),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material.clone()),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ));
            });
            state.counter += 1;
            redraw = true;
        }
    }

    if redraw {
        redraw_curves_writer.write(RedrawCurvesEvent);
    }
}

pub fn enter_create_curve_mode(
    mut commands: Commands,
    mut state: ResMut<CreateCurveState>,
    root: Query<Entity, With<RootTransform>>,
) {
    state.counter = 0;

    let root = root.single().unwrap();
    commands.get_entity(root).unwrap().with_children(|cmd| {
        cmd.spawn((
            TemporaryCurve,
            Name::new("Temporary Curve"),
            Transform::default(),
            Visibility::Inherited,
            RenderLayers::from(DisplayIn::BothNormalAndOrtho),
        ));
    });

    commands.spawn((Name::new("Preview Left"), Visibility::Inherited));
}

#[cfg(feature = "vr_enable")]
pub fn enter_create_curve_mode_vr(
    mut commands: Commands,
    left_handle: Query<Entity, With<GripLeft>>,
    right_handle: Query<Entity, With<GripRight>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    info: Res<RenderInformation>,
) {
    let sphere = meshes.add(Sphere::new(0.1 * info.scale));
    let purple = materials.add(Color::from(PURPLE_600));

    let spawn_preview = |e: Entity, cmds: &mut Commands| {
        cmds.spawn((
            ChildOf(e),
            Name::new("Preview Sphere"),
            Visibility::Inherited,
            PreviewSphereMarker,
            MeshMaterial3d(purple.clone()),
            Mesh3d(sphere.clone()),
            Transform::default(),
        ));
    };

    left_handle
        .iter()
        .for_each(|e| spawn_preview(e, &mut commands));
    right_handle
        .iter()
        .for_each(|e| spawn_preview(e, &mut commands));
}

#[cfg(feature = "vr_enable")]
pub fn exit_create_curve_mode_vr(
    mut commands: Commands,
    previews: Query<Entity, With<PreviewSphereMarker>>,
) {
    for preview in previews {
        let _ = commands.get_entity(preview).map(|mut e| e.despawn());
    }
}

#[allow(clippy::complexity)]
fn handle_click_on_curve_point3d(
    trigger: Trigger<Pointer3d<events::Click>>,
    commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    points: Query<(Entity, &ChildOf, &ControlCurvePoint)>,
    mut remove_point_writer: EventWriter<RemovePointFromCurveEvent>,
    mut add_point_writer: EventWriter<AddPointToCurveEvent>,
) {
    if *state == ControlState::Main {
        enable_gizmo3d(EnableTranslationControl::new_with_root(
            EnableTranslationControlType::OnlyTranslation,
        ))(trigger, commands, enabled, state);
    } else if *state == ControlState::Minus
        && let Ok(point) = points.get(trigger.target())
    {
        let curve = point.1.parent();
        let idx = point.2.0;

        remove_point_writer.write(RemovePointFromCurveEvent { curve, point: idx });
    } else if *state == ControlState::Plus
        && let Ok(point) = points.get(trigger.target())
    {
        let curve = point.1.parent();

        add_point_writer.write(AddPointToCurveEvent { curve });
    }
}

#[allow(clippy::complexity)]
fn handle_click_on_curve_point(
    trigger: Trigger<Pointer<Click>>,
    commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    points: Query<(Entity, &ChildOf, &ControlCurvePoint)>,
    mut remove_point_writer: EventWriter<RemovePointFromCurveEvent>,
    mut add_point_writer: EventWriter<AddPointToCurveEvent>,
) {
    if *state == ControlState::Main {
        enable_gizmo(EnableTranslationControl::new_with_root(
            EnableTranslationControlType::OnlyTranslation,
        ))(trigger, commands, enabled, state);
    } else if *state == ControlState::Minus
        && let Ok(point) = points.get(trigger.target())
    {
        let curve = point.1.parent();
        let idx = point.2.0;

        remove_point_writer.write(RemovePointFromCurveEvent { curve, point: idx });
    } else if *state == ControlState::Plus
        && let Ok(point) = points.get(trigger.target())
    {
        let curve = point.1.parent();

        add_point_writer.write(AddPointToCurveEvent { curve });
    }
}

#[allow(clippy::complexity)]
fn handle_click_event_on_curve_level(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    state: Res<State<ControlState>>,
    curves: Query<Entity, With<ControlCurve>>,
    children: Query<&Children>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut bounding_entities: BoundingEntitiesManager,
) {
    if *state == ControlState::Delete
        && let Ok(curve) = curves.get(trigger.target())
    {
        for child in children.iter_descendants(curve) {
            let _ = commands.get_entity(child).map(|mut e| {
                e.trigger(EntityDeletedEvent(child));
            });

            delete_event.write(EntityDeletedEvent(child));

            bounding_entities.remove_entity(&child);
        }

        delete_event.write(EntityDeletedEvent(curve));
        let _ = commands.get_entity(curve).map(|mut e| {
            e.trigger(EntityDeletedEvent(curve)).despawn();
        });

        end_mode_writer.write(EndModeEvent);
    }
}

#[allow(clippy::complexity)]
fn handle_click_event_on_curve_level3d(
    trigger: Trigger<Pointer3d<picking3d::events::Click>>,
    mut commands: Commands,
    state: Res<State<ControlState>>,
    curves: Query<Entity, With<ControlCurve>>,
    children: Query<&Children>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut bounding_entities: BoundingEntitiesManager,
) {
    if *state == ControlState::Delete
        && let Ok(curve) = curves.get(trigger.target())
    {
        for child in children.iter_descendants(curve) {
            let _ = commands.get_entity(child).map(|mut e| {
                e.trigger(EntityDeletedEvent(child));
            });

            delete_event.write(EntityDeletedEvent(child));

            bounding_entities.remove_entity(&child);
        }

        delete_event.write(EntityDeletedEvent(curve));
        let _ = commands.get_entity(curve).map(|mut e| {
            e.trigger(EntityDeletedEvent(curve)).despawn();
        });

        end_mode_writer.write(EndModeEvent);
    }
}

#[derive(Event)]
pub struct RemovePointFromCurveEvent {
    curve: Entity,
    point: usize,
}

#[allow(clippy::complexity)]
pub fn remove_point_from_curve_handler(
    mut reader: EventReader<RemovePointFromCurveEvent>,
    children: Query<&Children>,
    mut duplicate_set: ParamSet<(Commands, BridgeSpawner, BridgeDespawner)>,
    mut points: Query<(Entity, &ChildOf, &mut ControlCurvePoint)>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut redraw_curves: EventWriter<RedrawCurvesEvent>,
    transforms: Query<&Transform>,
) {
    for evt in reader.read() {
        defer!({
            redraw_curves.write(RedrawCurvesEvent);
            end_mode_writer.write(EndModeEvent);
        });

        let curve = evt.curve;
        let selected_idx = evt.point;

        let Ok(curve_children) = children.get(curve) else {
            continue;
        };

        let mut points_collected = Vec::new();

        for child in curve_children {
            if let Ok(point) = points.get(*child) {
                points_collected.push((point.2.0, point.0));
            }
        }

        if points_collected.len() <= 2 {
            continue;
        }

        duplicate_set.p2().despawn_from_parent(curve);

        points_collected.sort_by_key(|p| p.0);

        if let Some(to_remove_idx) = points_collected.iter().position(|p| p.0 == selected_idx) {
            let removed = points_collected.remove(to_remove_idx);
            delete_event.write(EntityDeletedEvent(removed.1));
            let _ = duplicate_set
                .p0()
                .get_entity(removed.1)
                .map(|mut e| e.despawn());

            points_collected
                .iter_mut()
                .enumerate()
                .for_each(|(idx, p)| p.0 = idx);
        }

        for point in &points_collected {
            if let Ok(mut curve_point) = points.get_mut(point.1) {
                curve_point.2.0 = point.0;
            }
        }

        let l = points_collected.len();
        let mut points_with_position = Vec::new();
        let mut ids = Vec::new();

        for p in points_collected {
            ids.push(p.1);

            let position = transforms
                .get(p.1)
                .expect("Is required for curve points")
                .translation;
            points_with_position.push((p.0, position));
        }

        duplicate_set
            .p1()
            .spawn_bridges_1d(curve, l, &points_with_position, &ids);
    }
}

#[derive(Event)]
pub struct AddPointToCurveEvent {
    curve: Entity,
}

#[allow(clippy::complexity)]
pub fn add_point_to_curve_handler(
    mut reader: EventReader<AddPointToCurveEvent>,
    children: Query<&Children>,
    mut duplicate_set: ParamSet<(
        Commands,
        BridgeSpawner,
        BridgeDespawner,
        (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
        (
            Commands,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<StandardMaterial>>,
        ),
    )>,
    points: Query<(Entity, &ChildOf, &ControlCurvePoint)>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut redraw_curves: EventWriter<RedrawCurvesEvent>,
    mut transforms: Query<&mut Transform>,
    snapped: Query<&SnappedPoint>,
    info: Res<RenderInformation>,
) {
    for evt in reader.read() {
        defer!({
            redraw_curves.write(RedrawCurvesEvent);
            end_mode_writer.write(EndModeEvent);
        });

        let curve = evt.curve;

        let Ok(curve_children) = children.get(curve) else {
            continue;
        };

        let mut points_collected = Vec::new();

        for child in curve_children {
            if let Ok(point) = points.get(*child) {
                points_collected.push((point.2.0, point.0));
            }
        }

        duplicate_set.p2().despawn_from_parent(curve);

        points_collected.sort_by_key(|p| p.0);

        let mut control_points = Vec::new();
        for point in &points_collected {
            control_points.push(Point::from(
                transforms
                    .get(point.1)
                    .expect("must be present")
                    .translation,
            ));
        }

        let new_points = increase_degree(&control_points);
        let mut snapped_value = None;

        for (idx, (_, point_entity)) in points_collected.iter().enumerate() {
            if let Ok(mut transform) = transforms.get_mut(*point_entity) {
                transform.translation = Vec3::from(new_points[idx]);
            }

            // NOTE(jan): the end point is now the second last point, i.e. new_points[i-2].
            //            If the previous last point was snapped, make sure to apply the snapp to the
            //            new last point and unsnap the old last
            if idx == points_collected.len() - 1
                && let Ok(snap) = snapped.get(*point_entity)
            {
                let _ = duplicate_set.p0().get_entity(*point_entity).map(|mut e| {
                    e.remove::<SnappedPoint>();
                });
                snapped_value = Some(*snap);
            }
        }

        let mut l = points_collected.len();
        let mut points_with_position = Vec::new();
        let mut ids = Vec::new();

        for p in &points_collected {
            ids.push(p.1);

            let position = transforms
                .get(p.1)
                .expect("Is required for curve points")
                .translation;
            points_with_position.push((p.0, position));
        }

        let (sphere, material) = {
            let (mut meshes, mut materials) = duplicate_set.p3();
            let sphere = meshes.add(Sphere::new(0.08 * info.scale));
            let material = materials.add(StandardMaterial::from_color(PURPLE_600));
            (sphere, material)
        };
        let counter = points_with_position.last().unwrap().0 + 1;

        let new_id = {
            let mut commands = duplicate_set.p0();
            let mut new_entity = commands.spawn((
                ChildOf(curve),
                ControlCurvePoint(counter),
                Name::new(format!("Curve point {}", counter)),
                Transform::from_translation(Vec3::from(*new_points.last().unwrap())),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                Picking3dInteractable::default(),
                CantSnapToCurve::Single(curve),
            ));
            new_entity
                .observe(hover_3d)
                .observe(handle_click_on_curve_point3d)
                .observe(handle_click_on_curve_point);

            if let Some(snap) = snapped_value {
                new_entity.insert(snap);
            }

            new_entity.id()
        };

        // Finally, move all old pos children to new pos. Treat the old last point as the newly created point
        for (_, point_entity) in &points_collected {
            let Ok(children) = children.get(*point_entity) else {
                continue;
            };

            for child in children {
                let _ = duplicate_set.p0().get_entity(*child).map(|mut e| {
                    e.insert(ChildOf(new_id));
                });
            }
        }

        ids.push(new_id);
        points_with_position.push((counter, Vec3::from(*new_points.last().unwrap())));
        l += 1;

        duplicate_set
            .p1()
            .spawn_bridges_1d(curve, l, &points_with_position, &ids);
    }
}

#[allow(clippy::complexity)]
/// Commit a curve after the mode for the creation ends
pub fn commit_curve(
    root: Query<(Entity, &Transform), (With<RootTransform>, Without<TemporaryCurve>)>,
    tmp_curves: Query<Entity, With<TemporaryCurve>>,
    children: Query<&Children>,
    mut tmp_points: Query<(Entity, &TemporaryCurvePoint), Without<RootTransform>>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
    mut duplicate_set: ParamSet<(
        (Commands, EventWriter<AddBoundingEntityEvent>),
        BridgeSpawner,
        (ResMut<Assets<Mesh>>, ResMut<Assets<StandardMaterial>>),
    )>,
    transforms: Query<&Transform, (Without<RootTransform>, Without<TemporaryCurve>)>,
    info: Res<RenderInformation>,
) {
    let root = root.single().unwrap();
    let (sphere_mesh, purple) = {
        let (mut meshes, mut materials) = duplicate_set.p2();
        (
            meshes.add(Sphere::new(0.03 * info.scale)),
            materials.add(StandardMaterial::from_color(PURPLE_600)),
        )
    };

    // Iterate over possible (actually only one) temporary curves
    for curve in tmp_curves.iter() {
        let mut contains_points = false;
        if let Ok(children) = children.get(curve) {
            for _ in children {
                contains_points = true;
            }
        }

        if contains_points {
            let parent = duplicate_set
                .p0()
                .0
                .spawn((
                    ControlCurve,
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                    Name::new("Control Curve"),
                    Transform::default(),
                    Visibility::Inherited,
                    ChildOf(root.0),
                ))
                .observe(handle_click_event_on_curve_level)
                .observe(handle_click_event_on_curve_level3d)
                .id();

            let mut points = Vec::new();
            for child_point_entity in children
                .get(curve)
                .expect("This is checked by the flag contains_points")
            {
                let (entity, point) = tmp_points.get_mut(*child_point_entity).unwrap();

                let idx = point.0;

                duplicate_set
                    .p0()
                    .1
                    .write(AddBoundingEntityEvent(*child_point_entity));

                duplicate_set
                    .p0()
                    .0
                    .get_entity(entity)
                    .unwrap()
                    .remove::<TemporaryCurvePoint>()
                    .insert((
                        ChildOf(parent),
                        ControlCurvePoint(idx),
                        Picking3dInteractable::default(),
                        CantSnapToCurve::Single(parent),
                    ))
                    .observe(hover_3d)
                    .observe(handle_click_on_curve_point3d)
                    .observe(handle_click_on_curve_point);

                points.push((idx, entity, transforms.get(entity).unwrap().translation));
            }

            if points.len() >= 2 {
                points.sort_by_key(|v| v.0);
                let ids = points.iter().map(|p| p.1).collect::<Vec<_>>();
                let points = points.into_iter().map(|p| (p.0, p.2)).collect::<Vec<_>>();
                duplicate_set
                    .p1()
                    .spawn_bridges_1d(parent, points.len(), &points, &ids);

                // Spawn helper spheres
                let mut points = points;
                points.sort_by_key(|p| p.0);
                for i in 0..10 {
                    let u = (i + 1) as f64 / 11.0;

                    let points = points.iter().map(|p| Point::from(p.1)).collect::<Vec<_>>();

                    let point = *de_casteljau(&points, u).last().unwrap().last().unwrap();

                    duplicate_set.p0().0.spawn((
                        ChildOf(parent),
                        CurveSupportPoint { u },
                        Transform::from_translation(Vec3::from(point)),
                        Mesh3d(sphere_mesh.clone()),
                        MeshMaterial3d(purple.clone()),
                    ));
                }
            }
        }
        // Despawn temporary curve, that was replaced by a final curve
        let _ = duplicate_set
            .p0()
            .0
            .get_entity(curve)
            .map(|mut e| e.despawn());
    }

    redraw_curves_writer.write(RedrawCurvesEvent);
}

#[derive(SystemParam)]
#[allow(clippy::complexity)]
pub struct CurveCollection<'w, 's> {
    curves: Query<
        'w,
        's,
        (Entity, Option<Read<IgnoreCurve>>),
        (With<ControlCurve>, Without<TemporaryCurve>),
    >,
    children: Query<'w, 's, Read<Children>>,
    curves_points:
        Query<'w, 's, (Read<ControlCurvePoint>, Read<Transform>), Without<TemporaryCurvePoint>>,
}

impl<'w, 's> CurveCollection<'w, 's> {
    pub fn collect(&self, ignore_marked_curves: bool) -> HashMap<Entity, Vec<Point>> {
        let mut result = HashMap::new();
        for (curve, ignore_curve) in self.curves.iter() {
            // Note: Ignore if curve has ignore marker, and we actually want to ignore the curves
            if ignore_marked_curves && ignore_curve.is_some() {
                continue;
            }
            let mut points = Vec::new();
            if let Ok(children) = self.children.get(curve) {
                for child in children {
                    if let Ok(point) = self.curves_points.get(*child) {
                        points.push((point.0.0, Point::from(point.1.translation)));
                    }
                }
                points.sort_by_key(|p| p.0);
                result.insert(curve, points.iter().map(|p| p.1).collect());
            }
        }

        result
    }

    pub fn collect_shortest(
        &self,
        point: Point,
        ignore_curves: &CantSnapToCurve,
        ignore_marked_curves: bool,
    ) -> Option<(Entity, f64, Point, f64, Vec<Point>)> {
        let mut min = f64::MAX;
        let mut min_u = None;
        let mut min_curve = None;
        let mut min_point = None;
        let mut min_points = None;

        let collected = self.collect(ignore_marked_curves);
        for (curve, points) in collected {
            match ignore_curves {
                CantSnapToCurve::All => {
                    continue;
                }
                CantSnapToCurve::Single(ignore_curve) => {
                    if *ignore_curve == curve {
                        continue;
                    }
                }
                CantSnapToCurve::Multiple(ignore_curves) => {
                    if ignore_curves.contains(&curve) {
                        continue;
                    }
                }
                _ => {}
            }
            let (u, p, dist) = shortest_distance_to_point(&points, point);
            if dist < min {
                min = dist;
                min_u = Some(u);
                min_curve = Some(curve);
                min_point = Some(p);
                min_points = Some(points);
            }
        }

        if let Some(u) = min_u
            && let Some(curve) = min_curve
            && let Some(p) = min_point
            && let Some(points) = min_points
        {
            Some((curve, u, p, min, points))
        } else {
            None
        }
    }
}

#[derive(SystemParam)]
pub struct TemporaryCurveCollection<'w, 's> {
    curves: Query<'w, 's, Entity, (With<TemporaryCurve>, Without<ControlCurve>)>,
    children: Query<'w, 's, Read<Children>>,
    curves_points:
        Query<'w, 's, (Read<TemporaryCurvePoint>, Read<Transform>), Without<ControlCurvePoint>>,
}

impl<'w, 's> TemporaryCurveCollection<'w, 's> {
    pub fn collect(&self) -> HashMap<Entity, Vec<Point>> {
        let mut result = HashMap::new();
        for curve in self.curves.iter() {
            if let Ok(children) = self.children.get(curve) {
                let mut points = Vec::new();
                for child in children {
                    if let Ok(point) = self.curves_points.get(*child) {
                        points.push((point.0.0, Point::from(point.1.translation)));
                    }
                }
                points.sort_by_key(|p| p.0);
                result.insert(curve, points.iter().map(|p| p.1).collect());
            }
        }

        result
    }
}

#[derive(SystemParam)]
pub struct AllCurveCollection<'w, 's> {
    curves: CurveCollection<'w, 's>,
    temporary_curves: TemporaryCurveCollection<'w, 's>,
}

impl<'w, 's> AllCurveCollection<'w, 's> {
    pub fn collect(&self) -> HashMap<Entity, Vec<Point>> {
        let mut map = self.curves.collect(false);
        map.extend(self.temporary_curves.collect());
        map
    }
}

#[allow(clippy::complexity)]
pub fn render_curves(
    mut redraw_curves_writer: EventReader<RedrawCurvesEvent>,
    mut commands: Commands,
    mut meshes3d: Query<&mut Mesh3d>,
    mut transforms: Query<
        &mut Transform,
        (Without<TemporaryCurvePoint>, Without<ControlCurvePoint>),
    >,
    all_curves: AllCurveCollection,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    root: Query<Entity, With<RootTransform>>,
    curve_segments: Query<&CurveSegments>,
    curve_segment: Query<&CurveSegment>,
    children: Query<&Children>,
    info: Res<RenderInformation>,
) {
    if redraw_curves_writer.is_empty() {
        return;
    }
    redraw_curves_writer.clear();

    let curves_collected = all_curves.collect();
    let purple = materials.add(StandardMaterial::from_color(PURPLE_600));

    // after collecting, set meshes accordingly
    for (entity, points) in curves_collected {
        if points.len() >= 2 {
            let points = (0..=100)
                .into_par_iter()
                .map(|u| {
                    Vec3::from(
                        *de_casteljau(&points, (u as f64) / 100.0)
                            .last()
                            .unwrap()
                            .last()
                            .unwrap(),
                    )
                })
                .collect::<Vec<_>>();

            if let Ok(segments) = curve_segments.get(entity) {
                for segment in &segments.0 {
                    if let Ok(segment_params) = curve_segment.get(*segment) {
                        let point = points[segment_params.u];
                        let next_point = points[segment_params.u + 1];

                        if let Ok(mut transform) = transforms.get_mut(*segment) {
                            *transform =
                                Transform::from_translation(point).looking_at(next_point, Vec3::Y);
                        }

                        let diff = next_point - point;
                        let length = diff.length();

                        for child in children.get(*segment).unwrap() {
                            if let Ok(mut transform) = transforms.get_mut(*child) {
                                *transform =
                                    Transform::from_xyz(0.0, 0.0, -length / 2.0).with_rotation(
                                        Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians()),
                                    );
                            }

                            if let Ok(mut mesh3d) = meshes3d.get_mut(*child) {
                                mesh3d.0 = meshes.add(Cylinder::new(0.01 * info.scale, length));
                            }
                        }
                    }
                }
            } else {
                for u in 0..100 {
                    let point = points[u];
                    let next_point = points[u + 1];

                    let diff = next_point - point;
                    let length = diff.length();
                    let cylinder = meshes.add(Cylinder::new(0.01 * info.scale, length));
                    commands.entity(root.single().unwrap()).with_child((
                        Transform::from_translation(point).looking_at(next_point, Vec3::Y),
                        // This is the relationship
                        CurveSegment { u, curve: entity },
                        Name::new(format!("Curve Segment {entity}, u: {u}")),
                        Visibility::Inherited,
                        related!(
                            Children[(
                                Mesh3d(cylinder),
                                MeshMaterial3d(purple.clone()),
                                Transform::from_xyz(0.0, 0.0, -length / 2.0).with_rotation(
                                    Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())
                                ),
                                Visibility::Inherited,
                            )]
                        ),
                    ));
                }
            }
        }
    }
}

#[allow(clippy::complexity)]
pub fn update_sphere_positions(
    all_curves: AllCurveCollection,
    mut transforms: Query<
        (&ChildOf, &mut Transform, &CurveSupportPoint),
        (Without<TemporaryCurvePoint>, Without<ControlCurvePoint>),
    >,
) {
    let curves = all_curves.collect();

    for (child_of, mut support_transform, support) in &mut transforms {
        if let Some(points) = curves.get(&child_of.parent())
            && points.len() >= 2
        {
            let point = *de_casteljau(points, support.u)
                .last()
                .unwrap()
                .last()
                .unwrap();
            support_transform.translation = Vec3::from(point);
        }
    }
}
