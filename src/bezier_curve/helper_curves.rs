use std::collections::HashMap;

use super::bezier_curve_renderer::EndModeEvent;
use super::{components::ControlState, render_info::RenderInformation, EntityDeletedEvent};
use crate::nurbs::bezier::de_casteljau;
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::DisplayIn;
use crate::translation_control::enable_gizmo3d;
use crate::translation_control::translation_controller::CantSnapToCurve;
use crate::{
    click_decider::LogTrace,
    nurbs::{bezier::shortest_distance_to_point, point::Point},
    picking3d::events::{self, Click, Pointer3d},
    translation_control::translation_controller::EnableTranslationControl,
    RootTransform,
};
use bevy::color::palettes::tailwind::PURPLE_900;
use bevy::render::view::RenderLayers;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::tailwind::PURPLE_600,
    ecs::system::{lifetimeless::Read, SystemParam},
    prelude::*,
    render::mesh::PrimitiveTopology,
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

    let sphere = meshes.add(Sphere::new(0.08 * render_info.scale));
    let material = materials.add(StandardMaterial::from_color(PURPLE_600));

    for evt in reader.read() {
        let pos = root
            .compute_affine()
            .inverse()
            .transform_point3(evt.position);

        info!("Adding point at {:?}", pos);

        commands.get_entity(parent).unwrap().with_children(|cmd| {
            cmd.spawn((
                TemporaryCurvePoint(state.counter),
                Transform::from_translation(pos),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
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
        cmd.spawn((
            TemporaryCurve,
            Transform::default(),
            Visibility::Inherited,
            RenderLayers::from(DisplayIn::BothNormalAndOrtho),
        ));
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
    mut end_mode_writer: EventWriter<EndModeEvent>,
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

            end_mode_writer.write(EndModeEvent);
        }
    } else if *state == ControlState::Main {
        enable_gizmo3d(EnableTranslationControl::OnlyTranslation)(
            trigger,
            commands,
            enabled,
            trace_log_writer,
            state,
        );
    }
}

#[allow(clippy::complexity)]
/// Commit a curve after the mode for the creation ends
pub fn commit_curve(
    mut commands: Commands,
    root: Query<(Entity, &Transform), (With<RootTransform>, Without<TemporaryCurve>)>,
    tmp_curves: Query<Entity, With<TemporaryCurve>>,
    children: Query<&Children>,
    mut tmp_points: Query<(Entity, &TemporaryCurvePoint), Without<RootTransform>>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    let root = root.single().unwrap();

    // Iterate over possible (actually only one) temporary curves
    for curve in tmp_curves.iter() {
        let mut contains_points = false;
        for _ in children.iter_descendants(curve) {
            contains_points = true;
        }
        if contains_points {
            let parent = commands
                .spawn((
                    ControlCurve,
                    ChildOf(root.0),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .id();

            for child_point_entity in children.iter_descendants(curve) {
                let (entity, point) = tmp_points.get_mut(child_point_entity).unwrap();

                let idx = point.0;

                info!("Consolidated curve point {entity:?}");
                commands
                    .get_entity(entity)
                    .unwrap()
                    .insert(ChildOf(parent))
                    .remove::<TemporaryCurvePoint>()
                    .insert((
                        ControlCurvePoint(idx),
                        Picking3dInteractable::default(),
                        CantSnapToCurve::Single(parent),
                    ))
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

#[derive(SystemParam)]
pub struct CurveCollection<'w, 's> {
    curves: Query<'w, 's, Entity, (With<ControlCurve>, Without<TemporaryCurve>)>,
    children: Query<'w, 's, Read<Children>>,
    curves_points:
        Query<'w, 's, (Read<ControlCurvePoint>, Read<Transform>), Without<TemporaryCurvePoint>>,
}

impl<'w, 's> CurveCollection<'w, 's> {
    pub fn collect(&self) -> HashMap<Entity, Vec<Point>> {
        let mut result = HashMap::new();
        for curve in self.curves.iter() {
            let mut points = Vec::new();
            for child in self.children.iter_descendants(curve) {
                if let Ok(point) = self.curves_points.get(child) {
                    points.push((point.0.0, Point::from(point.1.translation)));
                }
            }
            points.sort_by_key(|p| p.0);
            result.insert(curve, points.iter().map(|p| p.1).collect());
        }

        result
    }

    pub fn collect_shortest(
        &self,
        point: Point,
        ignore_curves: &CantSnapToCurve,
    ) -> Option<(Entity, f64, Point, f64, Vec<Point>)> {
        let mut min = f64::MAX;
        let mut min_u = None;
        let mut min_curve = None;
        let mut min_point = None;
        let mut min_points = None;

        let collected = self.collect();
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
            let mut points = Vec::new();
            for child in self.children.iter_descendants(curve) {
                if let Ok(point) = self.curves_points.get(child) {
                    points.push((point.0.0, Point::from(point.1.translation)));
                }
            }
            points.sort_by_key(|p| p.0);
            result.insert(curve, points.iter().map(|p| p.1).collect());
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
        let mut map = self.curves.collect();
        map.extend(self.temporary_curves.collect());
        map
    }
}

#[allow(clippy::complexity)]
pub fn render_curves(
    mut redraw_curves_writer: EventReader<RedrawCurvesEvent>,
    mut commands: Commands,
    mut meshes3d: Query<&mut Mesh3d>,
    all_curves: AllCurveCollection,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if redraw_curves_writer.is_empty() {
        return;
    }
    redraw_curves_writer.clear();

    let curves_collected = all_curves.collect();

    // after collecting, set meshes accordingly
    for (entity, points) in curves_collected {
        if points.len() >= 2 {
            let mut verticies = Vec::new();
            for u in 0..=100 {
                let point = *de_casteljau(&points, (u as f64) / 100.0)
                    .last()
                    .unwrap()
                    .last()
                    .unwrap();

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
                let material = StandardMaterial::from_color(PURPLE_900);
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
