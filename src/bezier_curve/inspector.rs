use std::os::unix::process::parent_id;

use bevy::{color::palettes::tailwind::RED_400, prelude::*, render::mesh::VertexAttributeValues};
use struct_patch::Patch;

use crate::{
    MainCamera, RootTransform,
    custom_shapes::parallelogram::Parallelogram2d,
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
pub struct InspectorUDiff;

#[derive(Component)]
pub struct InspectorVDiff;

#[derive(Component)]
pub struct InspectorNormal;

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
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        (
            With<SurfaceInspectorMesh>,
            Without<SurfaceInspector>,
            Without<RenderPoint>,
        ),
    >,
    normal_query: Query<&InspectorNormal>,
    u_query: Query<&InspectorUDiff>,
    v_query: Query<&InspectorVDiff>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scale_res: Res<RenderInformation>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    let scale = scale_res.scale;
    let points = set.p1().to_control_points();

    for (surface, mut parent_transform, children) in set.p0() {
        let point = points.f(&[surface.u, surface.v]);
        let [u_diff, v_diff] = points.derive(&[surface.u, surface.v], 1);

        let normal = u_diff.cross(&v_diff).normalize() * -1.0;

        parent_transform.translation = Vec3::new(point.x as f32, point.y as f32, point.z as f32);
        parent_transform.look_to(Vec3::from(normal), Vec3::from(u_diff));

        for child in children {
            if let Ok((mut mesh, mut color, mut transform)) = meshes_query.get_mut(*child) {
                if normal_query.get(*child).is_ok() {
                    let mut parallelogram_mesh = Extrusion::new(
                        Parallelogram2d::new(
                            Vec3::from(u_diff).angle_between(Vec3::from(v_diff)),
                            0.07 * scale,
                            0.07 * scale,
                        ),
                        0.5 * scale,
                    )
                    .mesh()
                    .build();

                    let quat = Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians());
                    {
                        let positions = parallelogram_mesh
                            .attribute_mut(Mesh::ATTRIBUTE_POSITION)
                            .expect("Otherwise, mesh is broken");
                        if let VertexAttributeValues::Float32x3(inner) = positions {
                            for pos in inner {
                                let vec = Vec3::new(pos[0], pos[1], pos[2]);
                                let vec = quat.mul_vec3(vec);
                                pos[0] = vec.x;
                                pos[1] = vec.y;
                                pos[2] = vec.z;
                            }
                        }
                    }

                    let normals = parallelogram_mesh
                        .attribute_mut(Mesh::ATTRIBUTE_NORMAL)
                        .expect("Otherwise, mesh is broken");
                    if let VertexAttributeValues::Float32x3(inner) = normals {
                        for norm in inner {
                            let vec = Vec3::new(norm[0], norm[1], norm[2]);
                            let vec = quat.mul_vec3(vec);
                            norm[0] = vec.x;
                            norm[1] = vec.y;
                            norm[2] = vec.z;
                        }
                    }

                    let u_diff_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(u_diff));
                    let n_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(normal));

                    transform.align(Vec3::NEG_Z, u_diff_vec, Vec3::Y, n_vec);

                    *mesh = Mesh3d(meshes.add(parallelogram_mesh));
                } else if u_query.get(*child).is_ok() {
                    let u_mesh = meshes.add(Cuboid::new(0.05 * scale, 0.05 * scale, 0.3 * scale));
                    let u_color = materials.add(Color::from(Srgba::new(0.0, 1.0, 0.0, 1.0)));
                    *mesh = Mesh3d(u_mesh);
                    *color = MeshMaterial3d(u_color);
                    let u_diff_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(u_diff));

                    let n_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(normal));
                    transform.look_to(u_diff_vec, Vec3::from(n_vec));
                } else if v_query.get(*child).is_ok() {
                    let v_mesh = meshes.add(Cuboid::new(0.05 * scale, 0.05 * scale, 0.3 * scale));
                    let v_color = materials.add(Color::from(Srgba::new(0.0, 0.0, 1.0, 1.0)));
                    *mesh = Mesh3d(v_mesh);
                    *color = MeshMaterial3d(v_color);
                    let v_diff_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(v_diff));

                    let n_vec = parent_transform
                        .compute_affine()
                        .inverse()
                        .transform_vector3(Vec3::from(normal));
                    transform.look_to(v_diff_vec, Vec3::from(n_vec));
                }
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
    mut update_writer: EventWriter<UpdateSurfaceInspectorEvent>,
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
            Picking3dInteractable::default(),
            children![
                (
                    Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                    MeshMaterial3d(material.clone()),
                    Mesh3d(normal_pointer.clone()),
                    Visibility::Inherited,
                    Picking3dInteractable::default(),
                    SurfaceInspectorMesh,
                    InspectorNormal,
                ),
                (
                    Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                    MeshMaterial3d(material.clone()),
                    Mesh3d(normal_pointer.clone()),
                    Visibility::Inherited,
                    SurfaceInspectorMesh,
                    InspectorUDiff,
                ),
                (
                    Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                    MeshMaterial3d(material.clone()),
                    Mesh3d(normal_pointer.clone()),
                    Visibility::Inherited,
                    SurfaceInspectorMesh,
                    InspectorVDiff,
                ),
            ],
        ))
        .observe(drag_surface_inspector)
        .observe(drag_surface_inspector3d);
    });

    update_writer.write(UpdateSurfaceInspectorEvent);
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
