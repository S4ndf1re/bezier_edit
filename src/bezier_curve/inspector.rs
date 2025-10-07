use bevy::{color::palettes::tailwind::RED_400, prelude::*};
use struct_patch::Patch;

use crate::{
    MainCamera, RootTransform,
    nurbs::{
        bezier_plane::{ControlPoints2D, ToControlPoints2D, eval_2d_bezier_curves},
        parametric::{MinDistanceToPoint, Parametric},
        point::Point,
    },
    picking3d::{self, events::Pointer3d, picking_3d::Picking3dInteractable},
    ui::UiStateChangeset,
};

use super::{components::RenderPoint, render_info::RenderInformation};

#[derive(Component, Patch)]
#[patch(
    name = "SurfaceInspectorChangeset",
    attribute(derive(Event, Clone, Default))
)]
pub struct SurfaceInspector {
    pub u: f64,
    pub v: f64,
}

#[derive(Component)]
pub struct SurfaceInspectorMesh;

#[derive(Event)]
pub struct UpdateSurfaceInspectorEvent;

#[allow(clippy::complexity)]
pub fn update_surface_inspector(
    mut reader: EventReader<UpdateSurfaceInspectorEvent>,
    mut set: ParamSet<(
        Query<(&SurfaceInspector, &mut Transform, &Children), Without<SurfaceInspectorMesh>>,
        Query<(&Transform, &RenderPoint)>,
    )>,
    mut meshes_query: Query<
        (&mut Mesh3d, &mut Transform),
        (
            With<SurfaceInspectorMesh>,
            Without<SurfaceInspector>,
            Without<RenderPoint>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    scale_res: Res<RenderInformation>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    let scale = scale_res.scale;
    let points = set.p1().to_control_points();

    for (surface, mut transform, children) in set.p0() {
        let point = eval_2d_bezier_curves(&points, surface.u, surface.v);
        let [u_diff, v_diff] = points.derive(&[surface.u, surface.v], 1);

        let normal = u_diff.cross(&v_diff);

        transform.translation = Vec3::new(point.x as f32, point.y as f32, point.z as f32);
        transform.look_to(Into::<Vec3>::into(-1.0 * normal), Vec3::Y);

        for child in children {
            if let Ok((mut mesh, mut transform)) = meshes_query.get_mut(*child) {
                let normal_pointer = meshes.add(Cuboid::new(
                    0.07 * scale,
                    0.07 * scale,
                    0.5 * scale,
                    // (normal.magnitude() as f32) * scale,
                ));
                *mesh = Mesh3d(normal_pointer.clone());
                *transform = Transform::from_xyz(
                    0.0,
                    0.0,
                    -0.25 * scale,
                    //-(normal.magnitude() as f32) / 2.0 * scale
                );
            }
        }
    }
}

/// Check if clicks exist
pub fn inspectors_exist(inspectors: Query<&SurfaceInspector>) -> bool {
    !inspectors.is_empty()
}

/// Setup the surface inspector
pub fn setup_surface_inspector(
    mut commands: Commands,
    root: Query<Entity, With<RootTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scale_res: Res<RenderInformation>,
) {
    let scale = scale_res.scale;
    let u = 0.5;
    let v = 0.5;
    let surface_inspector = SurfaceInspector { u, v };

    let material = materials.add(Color::from(RED_400));
    let sphere = meshes.add(Sphere::new(0.07 * scale).mesh().ico(5).unwrap());
    let normal_pointer = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.5 * scale));

    let mut root = commands.get_entity(root.single().unwrap()).unwrap();

    root.with_children(|ui| {
        ui.spawn((
            surface_inspector,
            Name::new(format!("SurfaceClick({u}, {v})",)),
            MeshMaterial3d(material.clone()),
            Mesh3d(sphere.clone()),
            Transform::default(),
            Visibility::Inherited,
            children![(
                Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                MeshMaterial3d(material.clone()),
                Mesh3d(normal_pointer.clone()),
                Visibility::Inherited,
                Picking3dInteractable::default(),
            )],
        ))
        .observe(drag_surface_inspector)
        .observe(drag_surface_inspector3d);
    });
}

#[allow(clippy::complexity)]
pub fn handle_inspector_change_event(
    mut reader: EventReader<SurfaceInspectorChangeset>,
    mut clicks: Query<&mut SurfaceInspector>,
    mut update_writer: EventWriter<UpdateSurfaceInspectorEvent>,
) {
    let mut update = false;
    for evt in reader.read() {
        for mut click in clicks.iter_mut() {
            click.apply(evt.clone());
            update = true;
        }
    }

    if update {
        update_writer.write(UpdateSurfaceInspectorEvent);
    }
}

#[allow(clippy::complexity)]
pub fn drag_surface_inspector(
    trigger: Trigger<Pointer<Drag>>,
    mut surface_click: Query<
        (&mut SurfaceInspector, &Transform),
        (Without<RenderPoint>, Without<RootTransform>),
    >,
    control_points: Query<(&Transform, &RenderPoint)>,
    camera: Query<(&GlobalTransform, &Camera), With<MainCamera>>,
    root: Query<&Transform, (With<RootTransform>, Without<RenderPoint>)>,
    global_transforms: Query<&GlobalTransform, Without<MainCamera>>,
    mut ui_state_writer: EventWriter<UiStateChangeset>,
    mut update_writer: EventWriter<UpdateSurfaceInspectorEvent>,
) {
    let control_points: ControlPoints2D = control_points.to_control_points();
    if let Ok((camera_transform, camera)) = camera.single()
        && let Ok(root_transform) = root.single()
        && let Ok(global_transform) = global_transforms.get(trigger.target())
        && let Ok((mut surface_click, click_transform)) = surface_click.get_mut(trigger.target())
    {
        let end = trigger.pointer_location.position;
        let start = end - trigger.delta;

        let dist_to_target =
            (camera_transform.translation() - global_transform.translation()).length();

        if let Ok(end_ray) = camera.viewport_to_world(camera_transform, end)
            && let Ok(start_ray) = camera.viewport_to_world(camera_transform, start)
        {
            let start = start_ray.get_point(dist_to_target);
            let end = end_ray.get_point(dist_to_target);

            let diff = end - start;
            let diff = root_transform
                .compute_affine()
                .inverse()
                .transform_vector3(diff);

            let new_pos = Point::from(click_transform.translation + diff);
            let new_uv = control_points.min_distance_to_point(new_pos);

            surface_click.u = new_uv.params[0];
            surface_click.v = new_uv.params[1];

            ui_state_writer.write(UiStateChangeset {
                u: Some(surface_click.u),
                v: Some(surface_click.v),
                ..default()
            });

            update_writer.write(UpdateSurfaceInspectorEvent);
        }
    }
}

#[allow(clippy::complexity)]
pub fn drag_surface_inspector3d(
    trigger: Trigger<Pointer3d<picking3d::events::Drag>>,
    mut surface_click: Query<
        (&mut SurfaceInspector, &Transform),
        (Without<RenderPoint>, Without<RootTransform>),
    >,
    control_points: Query<(&Transform, &RenderPoint)>,
    root: Query<&Transform, (With<RootTransform>, Without<RenderPoint>)>,
    mut ui_state_writer: EventWriter<UiStateChangeset>,
    mut update_writer: EventWriter<UpdateSurfaceInspectorEvent>,
) {
    let control_points: ControlPoints2D = control_points.to_control_points();
    if let Ok(root_transform) = root.single()
        && let Ok((mut surface_click, click_transform)) = surface_click.get_mut(trigger.target())
    {
        let diff = trigger.event.delta;
        let diff = root_transform
            .compute_affine()
            .inverse()
            .transform_vector3(diff);

        let new_pos = Point::from(click_transform.translation + diff);
        let new_uv = control_points.min_distance_to_point(new_pos);

        surface_click.u = new_uv.params[0];
        surface_click.v = new_uv.params[1];

        ui_state_writer.write(UiStateChangeset {
            u: Some(surface_click.u),
            v: Some(surface_click.v),
            ..default()
        });

        update_writer.write(UpdateSurfaceInspectorEvent);
    }
}
