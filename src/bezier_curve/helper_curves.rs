use std::collections::HashMap;

use bevy::{
    asset::RenderAssetUsages, color::palettes::tailwind::PURPLE_600, prelude::*,
    render::mesh::PrimitiveTopology,
};

use crate::{
    RootTransform,
    click_decider::LogTrace,
    nurbs::{bezier::horner_scheme, point::Point},
    picking3d::events::{self, Click, Pointer3d},
    translation_control::translation_controller::EnableTranslationControl,
};

use super::{
    EntityDeletedEvent, components::ControlState, render_info::RenderInformation,
    util::enable_gizmo3d,
};

#[derive(Component)]
pub struct ControlCurve;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurvePoint(usize);

#[derive(Component)]
pub struct TemporaryCurve;

#[derive(Component)]
#[require(Transform)]
pub struct TemporaryCurvePoint(usize);

#[derive(Resource, Default)]
pub struct CreateCurveState {
    counter: usize,
}

#[derive(Event)]
pub struct RedrawCurvesEvent;

#[cfg(feature = "vr_enable")]
#[allow(clippy::complexity)]
pub fn add_point_3d(
    mut commands: Commands,
    mut reader: EventReader<Pointer3d<Click>>,
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

    let sphere = meshes.add(Sphere::new(0.6 * render_info.scale));
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
            ));
        });
        state.counter += 1;
        redraw = true;
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
        cmd.spawn(TemporaryCurve);
    });
}

#[allow(clippy::complexity)]
fn handle_click_on_curve_point(
    trigger: Trigger<Pointer3d<events::Click>>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    trace_log_writer: EventWriter<LogTrace>,
    state: Res<State<ControlState>>,
    points: Query<(Entity, &ChildOf), With<ControlCurvePoint>>,
    children: Query<&Children>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
) {
    // when clicked on a point that belongs to a curve, delete the curve and all its control
    // points. Trigger deleted events
    if *state == ControlState::Delete {
        if let Ok(curve) = points.get(trigger.target()) {
            let parent = curve.1.parent();
            for child in children.iter_descendants(parent) {
                commands
                    .get_entity(child)
                    .unwrap()
                    .trigger(EntityDeletedEvent(child));
                delete_event.write(EntityDeletedEvent(child));
            }

            delete_event.write(EntityDeletedEvent(parent));
            commands
                .get_entity(parent)
                .unwrap()
                .trigger(EntityDeletedEvent(parent))
                .despawn();
        }
    } else if *state == ControlState::Main {
        enable_gizmo3d(trigger, commands, enabled, trace_log_writer);
    }
}

#[allow(clippy::complexity)]
/// Commit a curve after the mode for the creation ends
pub fn commit_curve(
    mut commands: Commands,
    root: Query<(Entity, &Transform), (With<RootTransform>, Without<TemporaryCurve>)>,
    tmp_curves: Query<Entity, With<TemporaryCurve>>,
    children: Query<&Children>,
    mut tmp_points: Query<(Entity, &mut Transform, &TemporaryCurvePoint), Without<RootTransform>>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    let root = root.single().unwrap();

    let root_transform = root.1.clone().compute_affine().inverse();

    // Iterate over possible (actually only one) temporary curves
    for curve in tmp_curves.iter() {
        let mut contains_points = false;
        for _ in children.iter_descendants(curve) {
            contains_points = true;
        }
        if contains_points {
            let parent = commands.spawn((ControlCurve, ChildOf(root.0))).id();

            for child_point_entity in children.iter_descendants(curve) {
                let (entity, mut transform, point) =
                    tmp_points.get_mut(child_point_entity).unwrap();

                transform.translation = root_transform.transform_point3(transform.translation);
                let idx = point.0;

                commands
                    .get_entity(entity)
                    .unwrap()
                    .insert(ChildOf(parent))
                    .remove::<TemporaryCurvePoint>()
                    .insert(ControlCurvePoint(idx))
                    .observe(handle_click_on_curve_point);
            }
        } else {
            info!("No new curve to spawn. there are no control points");
        }
        // Despawn temporary curve, that was replaced by a final curve
        commands.get_entity(curve).unwrap().despawn();
    }

    redraw_curves_writer.write(RedrawCurvesEvent);
}

#[allow(clippy::complexity)]
/// Collect only created curves
pub fn collect_curves(
    mut curves: Query<Entity, (With<ControlCurve>, Without<TemporaryCurve>)>,
    children: Query<&Children>,
    curves_points: Query<(&ControlCurvePoint, &Transform), Without<TemporaryCurvePoint>>,
) -> HashMap<Entity, Vec<Point>> {
    let mut result = HashMap::new();
    for curve in curves.iter_mut() {
        let mut points = Vec::new();
        for child in children.iter_descendants(curve) {
            if let Ok(point) = curves_points.get(child) {
                points.push((point.0.0, Point::from(point.1.translation)));
            }
        }
        points.sort_by_key(|p| p.0);
        result.insert(curve, points.iter().map(|p| p.1).collect());
    }

    result
}

#[allow(clippy::complexity)]
/// Collect only temporary curves
pub fn collect_temporary_curves(
    curves: Query<Entity, (With<TemporaryCurve>, Without<ControlCurve>)>,
    children: Query<&Children>,
    curves_points: Query<(&TemporaryCurvePoint, &Transform), Without<ControlCurvePoint>>,
) -> HashMap<Entity, Vec<Point>> {
    let mut result = HashMap::new();
    for curve in curves.iter() {
        let mut points = Vec::new();
        for child in children.iter_descendants(curve) {
            if let Ok(point) = curves_points.get(child) {
                points.push((point.0.0, Point::from(point.1.translation)));
            }
        }
        points.sort_by_key(|p| p.0);
        result.insert(curve, points.iter().map(|p| p.1).collect());
    }

    result
}

/// Collect both temporary and created curves
pub fn collect_all_curves(
    curves: Query<Entity, (With<ControlCurve>, Without<TemporaryCurve>)>,
    curves_points: Query<(&ControlCurvePoint, &Transform), Without<TemporaryCurvePoint>>,
    temporary_curves: Query<Entity, (With<TemporaryCurve>, Without<ControlCurve>)>,
    temporary_points: Query<(&TemporaryCurvePoint, &Transform), Without<ControlCurvePoint>>,
    children: Query<&Children>,
) -> HashMap<Entity, Vec<Point>> {
    let mut map = collect_curves(curves, children, curves_points);
    map.extend(collect_temporary_curves(
        temporary_curves,
        children,
        temporary_points,
    ));

    map
}

#[allow(clippy::complexity)]
pub fn render_curves(
    mut redraw_curves_writer: EventReader<RedrawCurvesEvent>,
    mut commands: Commands,
    mut meshes3d: Query<&mut Mesh3d>,
    // curves
    curves: Query<Entity, (With<ControlCurve>, Without<TemporaryCurve>)>,
    curves_points: Query<(&ControlCurvePoint, &Transform), Without<TemporaryCurvePoint>>,
    // temp_curves
    temporary_curves: Query<Entity, (With<TemporaryCurve>, Without<ControlCurve>)>,
    temporary_points: Query<(&TemporaryCurvePoint, &Transform), Without<ControlCurvePoint>>,
    //
    children: Query<&Children>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if redraw_curves_writer.is_empty() {
        return;
    }
    redraw_curves_writer.clear();

    let curves_collected = collect_all_curves(
        curves,
        curves_points,
        temporary_curves,
        temporary_points,
        children,
    );

    // after collecting, set meshes accordingly
    for (entity, points) in curves_collected {
        if points.len() >= 2 {
            let mut verticies = Vec::new();
            for u in 0..=100 {
                let point = horner_scheme(&points, (u as f64) / 100.0);
                verticies.push(Vec3::from(point));
            }

            let mut mesh = Mesh::new(
                PrimitiveTopology::LineStrip,
                RenderAssetUsages::RENDER_WORLD,
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);

            let mesh3d = meshes3d.get_mut(entity);
            // Either change the old mesh3d, or create a new one, if no old mesh3d existed
            // beforehand
            if let Ok(mut mesh3d) = mesh3d {
                mesh3d.0 = meshes.add(mesh);
            } else {
                let material = StandardMaterial::from_color(Color::BLACK);
                commands.get_entity(entity).unwrap().insert((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(material)),
                ));
            }
        }
    }
}

/// Commit a plane after the mode for the creation ends
pub fn commit_plane() {
    unimplemented!()
}
