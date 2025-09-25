use bevy::{ecs::change_detection, prelude::*};

use crate::nurbs::bezier_plane::{
    ToControlPoints2D, decrease_degree_surface, increase_degree_surface,
};

use super::{bezier_curve_renderer::SurfaceCreator, components::RenderPoint};

#[derive(Event)]
pub struct DecreaseDegreeEvent;

#[derive(Event)]
pub struct IncreaseDegreeEvent;

pub fn handle_degree_increase_event(
    mut reader: EventReader<IncreaseDegreeEvent>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut surface_creation: SurfaceCreator,
) {
    let mut change_curve = false;
    let mut points = control_points.to_control_points();

    for _ in reader.read() {
        points = increase_degree_surface(&points);
        change_curve = true;
    }

    if change_curve {
        let h = points.len();
        let w = points[0].len();

        let mut points_flattened = Vec::with_capacity(w * h + 1);

        for (y, points) in points.iter().enumerate() {
            for (x, p) in points.iter().enumerate() {
                points_flattened.push((y, x, Vec3::from(*p)));
            }
        }

        surface_creation.create_surface_from_points(points_flattened, w, h);
    }
}

pub fn handle_degree_reduction_event(
    mut reader: EventReader<DecreaseDegreeEvent>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut surface_creation: SurfaceCreator,
) {
    let mut change_curve = false;
    let mut points = control_points.to_control_points();

    for _ in reader.read() {
        points = decrease_degree_surface(&points);
        change_curve = true;
    }

    if change_curve {
        let h = points.len();
        let w = points[0].len();

        let mut points_flattened = Vec::with_capacity(w * h + 1);

        for (y, points) in points.iter().enumerate() {
            for (x, p) in points.iter().enumerate() {
                points_flattened.push((y, x, Vec3::from(*p)));
            }
        }

        surface_creation.create_surface_from_points(points_flattened, w, h);
    }
}
