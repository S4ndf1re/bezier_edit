use bevy::{
    asset::RenderAssetUsages, color::palettes::tailwind::PURPLE_600, prelude::*,
    render::mesh::PrimitiveTopology,
};

use crate::{
    RootTransform,
    nurbs::{bezier::horner_scheme, point::Point},
    picking3d::{self, events::Pointer3d},
};

use super::render_info::RenderInformation;

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

#[derive(Resource)]
pub struct CreateCurveState {
    counter: usize,
}

#[derive(Event)]
pub struct RedrawCurvesEvent;

#[allow(clippy::complexity)]
pub fn add_point_3d(
    mut commands: Commands,
    mut reader: EventReader<Pointer3d<picking3d::events::Click>>,
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

/// Commit a curve after the mode for the creation ends
pub fn commit_curve(
    mut commands: Commands,
    root: Query<(Entity, &Transform), With<RootTransform>>,
    tmp_curves: Query<Entity, With<TemporaryCurve>>,
    children: Query<&Children>,
    mut tmp_points: Query<(Entity, &mut Transform, &TemporaryCurvePoint)>,
    mut redraw_curves_writer: EventWriter<RedrawCurvesEvent>,
) {
    let root = root.single().unwrap();

    let root_transform = root.1.clone().compute_affine().inverse();

    // Iterate over possible (actually only one) temporary curves
    for curve in tmp_curves.iter() {
        let parent = commands.spawn((ControlCurve, ChildOf(root.0))).id();

        for child_point_entity in children.iter_descendants(curve) {
            let (entity, mut transform, point) = tmp_points.get_mut(child_point_entity).unwrap();

            transform.translation = root_transform.transform_point3(transform.translation);
            let mut cmd_entity = commands.get_entity(entity).unwrap();
            cmd_entity.insert(ChildOf(parent));
            let idx = point.0;
            cmd_entity.remove::<TemporaryCurvePoint>();
            cmd_entity.insert(ControlCurvePoint(idx));
        }

        // Despawn temporary curve, that was replaced by a final curve
        commands.get_entity(curve).unwrap().despawn();
    }

    redraw_curves_writer.write(RedrawCurvesEvent);
}

#[allow(clippy::complexity)]
pub fn render_curves(
    mut redraw_curves_writer: EventReader<RedrawCurvesEvent>,
    mut commands: Commands,
    mut curves: Query<
        (Entity, Option<&mut Mesh3d>),
        Or<(With<ControlCurve>, With<TemporaryCurve>)>,
    >,
    children: Query<&Children>,
    curves_points: Query<(&ControlCurvePoint, &Transform, Option<&ChildOf>)>,
    temporary_points: Query<(&ControlCurvePoint, &Transform, Option<&ChildOf>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if redraw_curves_writer.is_empty() {
        return;
    }
    redraw_curves_writer.clear();

    for curve in curves.iter_mut() {
        let mut points = Vec::new();
        for child in children.iter_descendants(curve.0) {
            if let Ok(point) = curves_points.get(child) {
                points.push((point.0.0, Point::from(point.1.translation)));
            } else if let Ok(point) = temporary_points.get(child) {
                points.push((point.0.0, Point::from(point.1.translation)));
            }
        }

        if points.len() >= 2 {
            points.sort_by_key(|p| p.0);
            let points = points.iter().map(|p| p.1).collect::<Vec<_>>();

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

            // Either change the old mesh3d, or create a new one, if no old mesh3d existed
            // beforehand
            if curve.1.is_some() {
                curve.1.unwrap().0 = meshes.add(mesh);
            } else {
                let material = StandardMaterial::from_color(Color::BLACK);
                commands.get_entity(curve.0).unwrap().insert((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(material)),
                ));
            }
        }
    }
}

/// Commit a plane after the mode for the creation ends
pub fn commit_plane() {}
