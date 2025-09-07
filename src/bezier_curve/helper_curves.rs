use std::collections::HashMap;

use super::bezier_curve_renderer::EndModeEvent;
use super::{EntityDeletedEvent, components::ControlState, render_info::RenderInformation};
use crate::MainCamera;
use crate::nurbs::bezier::de_casteljau;
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::{BoundingEntitiesManager, DisplayIn};
use crate::translation_control::translation_controller::CantSnapToCurve;
use crate::translation_control::{enable_gizmo, enable_gizmo3d};
use crate::{
    RootTransform,
    nurbs::{bezier::shortest_distance_to_point, point::Point},
    picking3d::events::{self, Pointer3d},
    translation_control::translation_controller::EnableTranslationControl,
};
use bevy::color::palettes::css::BLACK;
use bevy::render::view::RenderLayers;
use bevy::{
    color::palettes::tailwind::PURPLE_600,
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
};
use bevy_lunex::UiLayoutRoot;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurve;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurvePoint(usize);

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

#[derive(Event)]
pub struct RedrawCurvesEvent;

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
            if ui_root.get(parent).is_ok() {
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
}

#[allow(clippy::complexity)]
fn handle_click_on_curve_point3d(
    trigger: Trigger<Pointer3d<events::Click>>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    points: Query<(Entity, &ChildOf), With<ControlCurvePoint>>,
    children: Query<&Children>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut bounding_entities: BoundingEntitiesManager,
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

                bounding_entities.remove_entity(&child);
            }

            delete_event.write(EntityDeletedEvent(parent));
            commands
                .get_entity(parent)
                .unwrap()
                .trigger(EntityDeletedEvent(parent))
                .despawn();

            bounding_entities.remove_entity(&parent);

            end_mode_writer.write(EndModeEvent);
        }
    } else if *state == ControlState::Main {
        enable_gizmo3d(EnableTranslationControl::OnlyTranslation)(
            trigger, commands, enabled, state,
        );
    }
}

#[allow(clippy::complexity)]
fn handle_click_on_curve_point(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    points: Query<(Entity, &ChildOf), With<ControlCurvePoint>>,
    children: Query<&Children>,
    mut delete_event: EventWriter<EntityDeletedEvent>,
    mut end_mode_writer: EventWriter<EndModeEvent>,
    mut bounding_entities: BoundingEntitiesManager,
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

                bounding_entities.remove_entity(&child);
            }

            delete_event.write(EntityDeletedEvent(parent));
            commands
                .get_entity(parent)
                .unwrap()
                .trigger(EntityDeletedEvent(parent))
                .despawn();

            bounding_entities.remove_entity(&parent);

            end_mode_writer.write(EndModeEvent);
        }
    } else if *state == ControlState::Main {
        enable_gizmo(EnableTranslationControl::OnlyTranslation)(trigger, commands, enabled, state);
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
    mut bounding_entities: BoundingEntitiesManager,
) {
    let root = root.single().unwrap();

    // Iterate over possible (actually only one) temporary curves
    for curve in tmp_curves.iter() {
        let mut contains_points = false;
        if let Ok(children) = children.get(curve) {
            for _ in children {
                contains_points = true;
            }
        }

        if contains_points {
            let parent = commands
                .spawn((
                    ControlCurve,
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                    Name::new("Control Curve"),
                    Transform::default(),
                    ChildOf(root.0),
                ))
                .id();

            for child_point_entity in children
                .get(curve)
                .expect("This is checked by the flag contains_points")
            {
                let (entity, point) = tmp_points.get_mut(*child_point_entity).unwrap();

                let idx = point.0;

                bounding_entities.add_bounding_entity(*child_point_entity);

                info!("Consolidated curve point {entity:?}");
                commands
                    .get_entity(entity)
                    .unwrap()
                    .remove::<TemporaryCurvePoint>()
                    .remove::<CurveSegments>()
                    .insert((
                        ChildOf(parent),
                        ControlCurvePoint(idx),
                        Picking3dInteractable::default(),
                        CantSnapToCurve::Single(parent),
                    ))
                    .observe(handle_click_on_curve_point3d)
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
    let black = materials.add(StandardMaterial::from_color(BLACK));

    // after collecting, set meshes accordingly
    for (entity, points) in curves_collected {
        if points.len() >= 2 {
            if let Ok(segments) = curve_segments.get(entity) {
                for segment in &segments.0 {
                    if let Ok(segment_params) = curve_segment.get(*segment) {
                        let point = Vec3::from(
                            *de_casteljau(&points, (segment_params.u as f64) / 100.0)
                                .last()
                                .unwrap()
                                .last()
                                .unwrap(),
                        );

                        let next_point = Vec3::from(
                            *de_casteljau(&points, ((segment_params.u + 1) as f64) / 100.0)
                                .last()
                                .unwrap()
                                .last()
                                .unwrap(),
                        );

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
                    let point = Vec3::from(
                        *de_casteljau(&points, (u as f64) / 100.0)
                            .last()
                            .unwrap()
                            .last()
                            .unwrap(),
                    );

                    let next_point = Vec3::from(
                        *de_casteljau(&points, ((u + 1) as f64) / 100.0)
                            .last()
                            .unwrap()
                            .last()
                            .unwrap(),
                    );

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
                                MeshMaterial3d(black.clone()),
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

/// Commit a plane after the mode for the creation ends
pub fn commit_plane() {
    unimplemented!()
}
