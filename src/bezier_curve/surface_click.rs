use bevy::{color::palettes::tailwind::RED_400, prelude::*};
use rand::seq::IndexedRandom;
use struct_patch::Patch;

use crate::{
    RootTransform,
    nurbs::{
        bezier_plane::{ToControlPoints2D, derive_2d, determine_u_v, eval_2d_bezier_curves},
        point::Point,
    },
    ui::UiStateChangeset,
};

use super::{components::RenderPoint, render_info::RenderInformation};

#[derive(Component, Patch)]
#[patch(
    name = "SurfaceClickChangeset",
    attribute(derive(Event, Clone, Default))
)]
pub struct SurfaceClick {
    pub u: f64,
    pub v: f64,
}

#[derive(Component)]
pub struct SurfaceClickMesh;

#[allow(clippy::complexity)]
pub fn bezier_surface_picking(
    trigger: Trigger<Pointer<Click>>,
    root: Query<Entity, With<RootTransform>>,
    root_transform: Query<&GlobalTransform, With<RootTransform>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    old_click: Query<Entity, With<SurfaceClick>>,
    scale_res: Res<RenderInformation>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut ui_state_writer: EventWriter<UiStateChangeset>,
) {
    let scale = scale_res.scale;

    for old in old_click.iter() {
        let _ = commands.get_entity(old).map(|mut e| {
            e.despawn();
        });
    }

    let control_points = control_points.to_control_points();

    let material = materials.add(Color::from(RED_400));
    let sphere = meshes.add(Sphere::new(0.07 * scale).mesh().ico(5).unwrap());

    let mut root = commands.get_entity(root.single().unwrap()).unwrap();
    if let Some(click_coords) = trigger.hit.position {
        let root_transform = root_transform.single().unwrap();
        let click_coords = root_transform
            .affine()
            .inverse()
            .transform_point3(click_coords);

        // Currently the volume vaule is completely arbitrary
        let possible_hits = determine_u_v(
            &control_points,
            &Point::new(
                click_coords.x as f64,
                click_coords.y as f64,
                click_coords.z as f64,
                None,
            ),
            0.0000001,
        );

        let mut u = 0.0;
        let mut v = 0.0;
        if let Some(hits) = possible_hits {
            for hit in hits {
                u = hit.0;
                v = hit.1;
                let evaluated = eval_2d_bezier_curves(&control_points, hit.0, hit.1);
                let (u_diff, v_diff) = derive_2d(&control_points, hit.0, hit.1, 1);
                let normal = u_diff.cross(&v_diff);
                let normal_pointer = meshes.add(Cuboid::new(
                    0.07 * scale,
                    0.07 * scale,
                    0.5 * scale,
                    // (normal.magnitude() as f32) * scale,
                ));

                root.with_children(|ui| {
                    ui.spawn((
                        SurfaceClick { u, v },
                        Name::new(format!("SurfaceClick({u}, {v})")),
                        MeshMaterial3d(material.clone()),
                        Mesh3d(sphere.clone()),
                        Transform::from_xyz(
                            evaluated.x as f32,
                            evaluated.y as f32,
                            evaluated.z as f32,
                        )
                        .looking_to(Into::<Vec3>::into(-1.0 * normal), Vec3::Y),
                        Visibility::Inherited,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Transform::from_xyz(
                                0.0,
                                0.0,
                                -0.25 * scale,
                                // -(normal.magnitude() as f32) / 2.0 * scale,
                            ),
                            MeshMaterial3d(material.clone()),
                            Mesh3d(normal_pointer.clone()),
                            SurfaceClickMesh,
                            Visibility::Inherited,
                        ));
                    });
                });
            }
            ui_state_writer.write(UiStateChangeset {
                u: Some(u),
                v: Some(v),
                ..Default::default()
            });
        }
    }
}

#[allow(clippy::complexity)]
pub fn update_surface_click(
    mut set: ParamSet<(
        Query<(&SurfaceClick, &mut Transform, &Children), Without<SurfaceClickMesh>>,
        Query<(&Transform, &RenderPoint)>,
    )>,
    mut meshes_query: Query<
        (&mut Mesh3d, &mut Transform),
        (
            With<SurfaceClickMesh>,
            Without<SurfaceClick>,
            Without<RenderPoint>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    scale_res: Res<RenderInformation>,
) {
    let scale = scale_res.scale;
    let points = set.p1().to_control_points();

    for (surface, mut transform, children) in set.p0() {
        let point = eval_2d_bezier_curves(&points, surface.u, surface.v);
        let (u_diff, v_diff) = derive_2d(&points, surface.u, surface.v, 1);
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

#[allow(clippy::complexity)]
pub fn handle_state_change_event(
    mut reader: EventReader<SurfaceClickChangeset>,
    mut clicks: Query<&mut SurfaceClick>,
    root: Query<Entity, With<RootTransform>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scale_res: Res<RenderInformation>,
) {
    let scale = scale_res.scale;
    if clicks.is_empty() {
        let mut surface_click = SurfaceClick { u: 0.5, v: 0.5 };
        for evt in reader.read() {
            surface_click.apply(evt.clone());
        }

        let material = materials.add(Color::from(RED_400));
        let sphere = meshes.add(Sphere::new(0.07 * scale).mesh().ico(5).unwrap());
        let normal_pointer = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.5 * scale));

        let mut root = commands.get_entity(root.single().unwrap()).unwrap();

        let control_points = control_points.to_control_points();
        if control_points.is_empty() {
            return;
        }
        let evaluated = eval_2d_bezier_curves(&control_points, surface_click.u, surface_click.v);
        let (u_diff, v_diff) = derive_2d(&control_points, surface_click.u, surface_click.v, 1);
        let normal = u_diff.cross(&v_diff);

        root.with_children(|ui| {
            ui.spawn((
                surface_click,
                MeshMaterial3d(material.clone()),
                Mesh3d(sphere.clone()),
                Transform::from_xyz(evaluated.x as f32, evaluated.y as f32, evaluated.z as f32)
                    .looking_to(Into::<Vec3>::into(-1.0 * normal), Vec3::Y),
                Visibility::Inherited,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                    MeshMaterial3d(material.clone()),
                    Mesh3d(normal_pointer.clone()),
                    Visibility::Inherited,
                ));
            });
        });
    } else {
        for evt in reader.read() {
            for mut click in clicks.iter_mut() {
                click.apply(evt.clone());
            }
        }
    }
}
