use bevy::{input::mouse::MouseButtonInput, prelude::*};

use crate::{MainCamera, projection::EnableOrthoCamera};

use super::{
    bezier_curve_renderer::EndModeEvent, components::RenderPoint, helper_curves::ControlCurvePoint,
};

#[allow(clippy::complexity)]
/// Enable a camera in space
pub fn create_camera_on_click(
    window: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut writer: EventWriter<EnableOrthoCamera>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    render_points: Query<Entity, Or<(With<RenderPoint>, With<ControlCurvePoint>)>>,
    mut end_state_writer: EventWriter<EndModeEvent>,
) {
    let mut created_cam = false;
    if mouse.just_pressed(MouseButton::Left)
        && let Ok(window) = window.single()
        && let Ok((camera, camera_transform)) = camera.single()
        && let Some(cursor) = window.cursor_position()
        && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
    {
        let plane = InfinitePlane3d::new(camera_transform.forward());
        if let Some(dist) = ray.intersect_plane(Vec3::ZERO, plane) {
            let point_of_intersection = ray.get_point(dist);
            let diff = camera_transform.translation() - point_of_intersection;
            let distance = diff.length();
            let pos_of_surface = camera_transform.forward().normalize_or_zero() * distance * 2.0
                + camera_transform.translation();
            let transform = Transform::from_translation(pos_of_surface)
                .looking_at(-camera_transform.forward().as_vec3(), Vec3::Y);

            writer.write(EnableOrthoCamera::new(
                transform,
                render_points.iter().collect(),
            ));
            created_cam = true;
        }
    }

    if created_cam {
        end_state_writer.write(EndModeEvent);
    }
}
