use bevy::{asset::RenderAssetUsages, prelude::*, render::mesh::PrimitiveTopology};

use crate::{
    RootTransform,
    nurbs::{bezier::horner_scheme, point::Point},
    picking3d::{self, events::Pointer3d},
};

#[derive(Component)]
pub struct ControlCurve;

#[derive(Component)]
#[require(Transform)]
pub struct ControlCurvePoint(usize);

#[derive(Resource)]
pub struct CreateCurveState {
    counter: usize,
}

pub fn add_point_3d(
    mut commands: Commands,
    mut reader: EventReader<Pointer3d<picking3d::events::Click>>,
    mut state: ResMut<CreateCurveState>,
) {
    for evt in reader.read() {
        let pos = evt.position;

        // TODO: add mesh to display sphere at position
        commands.spawn((
            ControlCurvePoint(state.counter),
            Transform::from_translation(pos),
        ));
        state.counter += 1;
    }
}

pub fn enter_create_curve_mode(mut state: ResMut<CreateCurveState>) {
    state.counter = 0;
}

/// Commit a curve after the mode for the creation ends
pub fn commit_curve(
    mut commands: Commands,
    root: Query<(Entity, &Transform), With<RootTransform>>,
    mut no_parents: Query<(Entity, &mut Transform, &ControlCurvePoint), Without<ChildOf>>,
) {
    let root = root.single().unwrap();

    let root_transform = root.1.clone().compute_affine().inverse();

    let parent = commands.spawn((ControlCurve, ChildOf(root.0))).id();

    let mut points = Vec::new();
    for (entity, mut transform, point) in no_parents.iter_mut() {
        transform.translation = root_transform.transform_point3(transform.translation);
        commands.get_entity(entity).unwrap().insert(ChildOf(parent));
        points.push((point.0, transform.translation));
    }

    points.sort_by_key(|v| v.0);
    let points = points.iter().map(|p| Point::from(p.1)).collect::<Vec<_>>();

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

    // TODO: insert mesh as child
}

/// Commit a plane after the mode for the creation ends
pub fn commit_plane() {}
