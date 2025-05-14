use crate::nurbs::bezier_plane::{derive_2d, eval_2d_bezier_curves};
use crate::nurbs::point::Point;
use crate::picking3d::picking_3d;
use crate::picking3d::picking_3d::{Picking3dInteractable, Pointer3d};
use crate::translation_control::translation_controller::EnableTranslationControl;
use crate::util::update_material_on;
use bevy::app::App;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use std::collections::HashMap;

pub type Resolution = (u32, u32);

#[derive(Resource)]
pub struct ScaleInformation {
    pub scale: f32,
    pub height: f32,
}

impl Default for ScaleInformation {
    fn default() -> Self {
        Self {
            scale: 1.0,
            height: 0.0,
        }
    }
}

#[derive(Event)]
pub struct RedrawEvent(pub Resolution);

#[derive(Component)]
pub struct RenderPoint(usize, usize);

#[derive(Component)]
struct ResultPoint;

#[derive(Component)]
pub struct BezierRender;

fn enable_gizmo(
    trigger: Trigger<Pointer<Click>>,
    query: Query<&RenderPoint>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
) {
    if query.get(trigger.target()).is_err() {
        return;
    }

    let mut entity = commands.get_entity(trigger.target()).unwrap();

    if enabled.get(trigger.target()).is_ok() {
        entity.remove::<EnableTranslationControl>();
    } else {
        entity.insert(EnableTranslationControl);
    }
}

fn enable_gizmo3d(
    trigger: Trigger<Pointer3d<picking_3d::Click>>,
    query: Query<&RenderPoint>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
) {
    if query.get(trigger.target()).is_err() {
        return;
    }

    let mut entity = commands.get_entity(trigger.target()).unwrap();

    if enabled.get(trigger.target()).is_ok() {
        entity.remove::<EnableTranslationControl>();
    } else {
        entity.insert(EnableTranslationControl);
    }
}

fn drag_point(
    trigger: Trigger<Pointer<Drag>>,
    mut query: Query<&mut Transform, (With<RenderPoint>, Without<Camera3d>)>,
    camera: Single<&Transform, With<Camera3d>>,
) {
    let mut point = query.get_mut(trigger.target()).unwrap();

    point.translation = point.translation
        + camera.right() * trigger.delta.x * 0.012
        + camera.up() * trigger.delta.y * -0.012;
}

fn generate_pointcloud(
    mut events: EventReader<RedrawEvent>,
    mut commands: Commands,
    entities: Query<Entity, With<ResultPoint>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut resolution: Resolution = (200, 200);
    if !events.is_empty() {
        // Consume and run redraw. No matter how many events where triggered
        for evt in events.read() {
            resolution = evt.0;
            break;
        }
        events.clear()
    } else {
        return;
    }

    for p in entities.iter() {
        commands.entity(p).despawn();
    }

    let mut points = HashMap::<usize, Vec<(usize, Point)>>::new();
    for (transform, render_point) in control_points.iter() {
        let curve = points.entry(render_point.0).or_default();
        curve.push((
            render_point.1,
            Point::new(
                transform.translation.x as f64,
                transform.translation.y as f64,
                transform.translation.z as f64,
                Some(1.0),
            ),
        ));
    }

    // First get all points in order for each sub curve
    let mut multi_curves = Vec::<(usize, Vec<Point>)>::new();
    for (i, points) in points.iter_mut() {
        points.sort_by(|a, b| a.0.cmp(&b.0));
        let points = points.iter().map(|p| p.1).collect::<Vec<_>>();
        multi_curves.push((*i, points));
    }

    // Then order the subcurves by index
    multi_curves.sort_by(|a, b| a.0.cmp(&b.0));
    let multi_curves: Vec<Vec<Point>> =
        multi_curves.iter().map(|p| p.1.clone()).collect::<Vec<_>>();

    let mut mat = StandardMaterial::default();
    mat.base_color = Color::from(GRAY_500);
    mat.cull_mode = None;

    let mut computed_points: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    // let mut uvs: Vec<[f32; 2]> = vec![];

    let mut indizes: Vec<u32> = vec![];

    let w = resolution.0;
    let h = resolution.1;

    for u in 0..w {
        for v in 0..h {
            let resulting_point = eval_2d_bezier_curves(
                &multi_curves,
                (u as f64) / ((w as f64) - 1.0),
                (v as f64) / ((h as f64) - 1.0),
            );

            computed_points.push([
                resulting_point.x as f32,
                resulting_point.y as f32,
                resulting_point.z as f32,
            ]);

            let (u_diff, v_diff) = derive_2d(
                &multi_curves,
                (u as f64) / ((w as f64) - 1.0),
                (v as f64) / ((h as f64) - 1.0),
            );
            let normal = &u_diff.cross(&v_diff) * (1.0 / u_diff.cross(&v_diff).magnitude());
            normals.push([-normal.x as f32, -normal.y as f32, -normal.z as f32]);
            // uvs.push([0.0, 0.0]);

            // commands.spawn((
            //     ResultPoint,
            //     Transform::from_xyz(
            //         resulting_point.x as f32,
            //         resulting_point.y as f32,
            //         resulting_point.z as f32,
            //     ),
            //     Mesh3d(sphere.clone()),
            //     MeshMaterial3d(result_mat.clone()),
            // ));
        }
    }

    for u in 0..(w - 1) {
        for v in 0..(h - 1) {
            // Compute indizes using simple 2d => 1d conversion
            indizes.push(u * w + v);
            indizes.push(u * w + v + 1);
            indizes.push((u + 1) * w + v + 1);

            indizes.push(u * w + v);
            indizes.push((u + 1) * w + v + 1);
            indizes.push((u + 1) * w + v);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, computed_points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    // mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indizes));

    commands.spawn((
        Transform::default(),
        ResultPoint,
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(mat)),
    ));
}

pub fn generate_default_curve(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: EventWriter<RedrawEvent>,
    mut scale_res: Res<ScaleInformation>,
) {
    let scale = scale_res.scale;
    let height = scale_res.height;
    let material = materials.add(Color::from(GRAY_400));
    let material_hover = materials.add(Color::from(GRAY_600));
    let sphere = meshes.add(Sphere::new(0.1 * scale).mesh().ico(5).unwrap());

    let points = vec![
        (0, 0, -1.5, 0.0, -1.5),
        (0, 1, -0.5, 0.0, -1.5),
        (0, 2, 0.5, 0.0, -1.5),
        (0, 3, 1.5, 0.0, -1.5),
        //
        (1, 0, -1.5, 0.0, -0.5),
        (1, 1, -0.5, 0.0, -0.5),
        (1, 2, 0.5, 0.0, -0.5),
        (1, 3, 1.5, 0.0, -0.5),
        //
        (2, 0, -1.5, 0.0, 0.5),
        (2, 1, -0.5, 0.0, 0.5),
        (2, 2, 0.5, 0.0, 0.5),
        (2, 3, 1.5, 0.0, 0.5),
        //
        (3, 0, -1.5, 0.0, 1.5),
        (3, 1, -0.5, 0.0, 1.5),
        (3, 2, 0.5, 0.0, 1.5),
        (3, 3, 1.5, 0.0, 1.5),
    ];

    for p in points {
        commands
            .spawn((
                BezierRender,
                RenderPoint(p.0, p.1),
                Transform::from_xyz(p.2 * scale, p.3 * scale + height, p.4 * scale),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material.clone()),
                Picking3dInteractable,
            ))
            .observe(update_material_on::<Pointer<Over>>(material_hover.clone()))
            .observe(update_material_on::<Pointer<Out>>(material.clone()))
            //.observe(drag_point)
            .observe(enable_gizmo)
            .observe(enable_gizmo3d);
    }

    event_writer.write(RedrawEvent((200, 200)));
}

impl Plugin for BezierRender {
    fn build(&self, app: &mut App) {
        app.add_event::<RedrawEvent>();
        app.init_resource::<ScaleInformation>();
        app.add_systems(Startup, generate_default_curve);
        app.add_systems(Update, generate_pointcloud); // , listen_to_mouse_left_button));
    }
}
