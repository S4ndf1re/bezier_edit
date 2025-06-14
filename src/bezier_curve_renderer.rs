use crate::history::plugin::HistoryUndoEvent;
use crate::nurbs::bezier_plane::{
    ControlPoints2D, derive_2d, determine_u_v, eval_2d_bezier_curves, split_surface,
};
use crate::nurbs::point::Point;
use crate::picking3d::events;
use crate::picking3d::events::Pointer3d;
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::solver::{C1Constraint, Constraints, Solver};
use crate::translation_control::translation_controller::EnableTranslationControl;
use crate::util::update_material_on;
use bevy::app::App;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::BLACK;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use num::ToPrimitive;
use std::collections::HashMap;

pub type Resolution = (u32, u32);

#[derive(Resource, Default)]
pub struct ConstraintState {
    c1_enabled: bool,
}

#[derive(Event)]
pub struct ToggleC1Enable;

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

#[derive(Component)]
pub struct SurfaceClick(f64, f64);

#[derive(Event)]
pub struct RedrawEvent(pub Resolution);

#[derive(Component)]
pub struct RenderPoint(usize, usize);

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct C1ControlPoint(i32, i32, usize, usize);

#[derive(Component)]
struct ResultSurface;

#[derive(Component)]
pub struct BezierRender;

#[derive(Component)]
#[require(Mesh3d)]
pub struct RenderLine(Entity, Entity);

fn update_lines(
    mut commands: Commands,
    mut lines: Query<(&RenderLine, Entity, &Mesh3d)>,
    transforms: Query<&Transform>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (line, line_entity, mesh3d) in lines.iter_mut() {
        let (entity1, entity2) = {
            let entity1 = transforms.get(line.0);
            let entity2 = transforms.get(line.1);
            (entity1, entity2)
        };

        if entity1.is_err() || entity2.is_err() {
            commands.get_entity(line_entity).unwrap().despawn();
        }

        let mesh = meshes.get_mut(mesh3d).expect("Must be here");

        if let Some(attrib) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
            *attrib = vec![entity1.unwrap().translation, entity2.unwrap().translation].into();
        } else {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![entity1.unwrap().translation, entity2.unwrap().translation],
            );
        }
    }
}

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

fn enable_gizmo_shadow_points(
    trigger: Trigger<Pointer<Click>>,
    query: Query<&C1ControlPoint>,
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
    trigger: Trigger<Pointer3d<events::Click>>,
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

fn collect_control_points(control_points: Query<(&Transform, &RenderPoint)>) -> ControlPoints2D {
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

    multi_curves
}

fn create_mesh_from_control_points(
    control_points: &ControlPoints2D,
    resolution: Resolution,
    mut images: ResMut<Assets<Image>>,
) -> (Mesh, Handle<Image>) {
    let mut computed_points: Vec<[f32; 3]> = vec![];
    let mut normals: Vec<[f32; 3]> = vec![];
    let mut uvs: Vec<[f32; 2]> = vec![];

    let mut indizes: Vec<u32> = vec![];

    let w = resolution.0;
    let h = resolution.1;

    let mut image = Image::new_fill(
        // 2D image of size 256x256
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(BLACK.to_u8_array()),
        // Use the same encoding as the color we set
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    for u in 0..w {
        for v in 0..h {
            let resulting_point = eval_2d_bezier_curves(
                control_points,
                (u as f64) / ((w as f64) - 1.0),
                (v as f64) / ((h as f64) - 1.0),
            );

            uvs.push([
                ((u as f64) / ((w as f64) - 1.0)) as f32,
                ((v as f64) / ((h as f64) - 1.0)) as f32,
            ]);

            computed_points.push([
                resulting_point.x as f32,
                resulting_point.y as f32,
                resulting_point.z as f32,
            ]);

            let (u_diff, v_diff) = derive_2d(
                control_points,
                (u as f64) / ((w as f64) - 1.0),
                (v as f64) / ((h as f64) - 1.0),
                1,
            );
            let normal = &u_diff.cross(&v_diff);
            normals.push([-normal.x as f32, -normal.y as f32, -normal.z as f32]);

            let (u_diff, v_diff) = derive_2d(
                control_points,
                (u as f64) / ((w as f64) - 1.0),
                (v as f64) / ((h as f64) - 1.0),
                2,
            );

            const SCALE: f64 = 20.0;
            let sum = &u_diff + &v_diff;
            let direction = &(sum / sum.magnitude()) * &Point::new(0.0, 1.0, 0.0, None);
            let len = sum.magnitude();
            if let Some(pixel) = image.pixel_bytes_mut(UVec3::new(u, v, 0)) {
                if direction >= 0.0 {
                    pixel[0] = { len / SCALE * u8::MAX as f64 } as u8;
                    pixel[1] = { 0.0 * u8::MAX as f64 / 5.0 } as u8;
                    pixel[2] = { 0.0 * u8::MAX as f64 / 5.0 } as u8;
                } else {
                    pixel[0] = { 0.0 * u8::MAX as f64 / 5.0 } as u8;
                    pixel[1] = { 0.0 * u8::MAX as f64 / 5.0 } as u8;
                    pixel[2] = { len / SCALE * u8::MAX as f64 } as u8;
                }
            }
        }
    }

    let handle = images.add(image);

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
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indizes));

    (mesh, handle)
}

fn generate_pointcloud(
    mut events: EventReader<RedrawEvent>,
    mut commands: Commands,
    entities: Query<Entity, With<ResultSurface>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
) {
    let mut resolution: Resolution = (200, 200);
    if !events.is_empty() {
        // Consume and run redraw. No matter how many events where triggered
        #[allow(clippy::never_loop)]
        for evt in events.read() {
            resolution = evt.0;
            break;
        }
        events.clear()
    } else {
        return;
    }

    let multi_curves = collect_control_points(control_points);
    let mut color = Color::from(GRAY_500);
    color.set_alpha(0.3);

    let (mesh, image_handle) = create_mesh_from_control_points(&multi_curves, resolution, images);

    let mat = StandardMaterial {
        base_color_texture: Some(image_handle),
        double_sided: true,
        cull_mode: None,
        ..Default::default()
    };

    // Despawn old, respawn new
    for p in entities.iter() {
        commands.entity(p).despawn();
    }

    commands
        .spawn((
            Transform::default(),
            ResultSurface,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(mat)),
        ))
        .observe(bezier_surface_picking);
}

pub fn bezier_surface_picking(
    trigger: Trigger<Pointer<Click>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    old_click: Query<Entity, With<SurfaceClick>>,
    scale_res: Res<ScaleInformation>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {
    let scale = scale_res.scale;

    for old in old_click.iter() {
        let _ = commands.get_entity(old).map(|mut e| {
            e.despawn();
        });
    }

    let control_points = collect_control_points(control_points);

    let material = materials.add(Color::from(RED_400));
    let sphere = meshes.add(Sphere::new(0.07 * scale).mesh().ico(5).unwrap());
    let normal_pointer = meshes.add(Cuboid::new(0.07 * scale, 0.07 * scale, 0.5 * scale));

    if let Some(click_coords) = trigger.hit.position {
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

        if let Some(hits) = possible_hits {
            for hit in hits {
                let evaluated = eval_2d_bezier_curves(&control_points, hit.0, hit.1);
                let (u_diff, v_diff) = derive_2d(&control_points, hit.0, hit.1, 1);
                let normal = &u_diff.cross(&v_diff);

                commands
                    .spawn((
                        SurfaceClick(hit.0, hit.1),
                        MeshMaterial3d(material.clone()),
                        Mesh3d(sphere.clone()),
                        Transform::from_xyz(
                            evaluated.x as f32,
                            evaluated.y as f32,
                            evaluated.z as f32,
                        )
                        .looking_to(Into::<Vec3>::into(-1.0 * normal), Vec3::Y),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Transform::from_xyz(0.0, 0.0, -0.25 * scale),
                            MeshMaterial3d(material.clone()),
                            Mesh3d(normal_pointer.clone()),
                        ));
                    });
            }
        }
    }
}

#[allow(clippy::complexity)]
pub fn update_surface_click(
    mut set: ParamSet<(
        Query<(&SurfaceClick, &mut Transform)>,
        Query<(&Transform, &RenderPoint)>,
    )>,
) {
    let points = collect_control_points(set.p1());

    for (surface, mut transform) in set.p0() {
        let point = eval_2d_bezier_curves(&points, surface.0, surface.1);
        let (u_diff, v_diff) = derive_2d(&points, surface.0, surface.1, 1);
        let normal = &u_diff.cross(&v_diff);

        transform.translation = Vec3::new(point.x as f32, point.y as f32, point.z as f32);
        transform.look_to(Into::<Vec3>::into(-1.0 * normal), Vec3::Y);
    }
}

pub fn generate_default_curve(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: EventWriter<RedrawEvent>,
    scale_res: Res<ScaleInformation>,
) {
    let scale = scale_res.scale;
    let height = scale_res.height;
    let material = materials.add(Color::from(GRAY_400));
    let material_shadow = materials.add(Color::from(GRAY_700));
    let material_hover = materials.add(Color::from(GRAY_600));
    let sphere = meshes.add(Sphere::new(0.1 * scale).mesh().ico(5).unwrap());

    let (w, h): (usize, usize) = (6, 6);
    let min_x = -w.to_f32().unwrap() / 2.0 + if w % 2 == 0 { 0.5 } else { 0.0 };
    let min_y = -h.to_f32().unwrap() / 2.0 + if h % 2 == 0 { 0.5 } else { 0.0 };

    let mut points = vec![];
    let mut c1_control_points: Vec<(i32, i32, usize, usize)> = vec![];
    let mut curr_x = min_x;
    let mut curr_y = min_y;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if y == 0 {
                c1_control_points.push((-1, x, y as usize, x as usize));
            } else if y == (h as i32) - 1 {
                c1_control_points.push((y + 1, x, y as usize, x as usize));
            }

            if x == 0 {
                c1_control_points.push((y, -1, y as usize, x as usize));
            } else if x == (w as i32) - 1 {
                c1_control_points.push((y, x + 1, y as usize, x as usize));
            }
            points.push((y as usize, x as usize, curr_x, 0.0, curr_y));
            curr_x += 1.0;
        }
        curr_x = min_x;
        curr_y += 1.0;
    }

    let mut ids = vec![];

    for p in &points {
        let id = commands
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
            .observe(enable_gizmo3d)
            .id();
        ids.push(id);
    }

    for p in c1_control_points {
        // TODO, determine the position based on the initial grid using c1 conditions
        commands
            .spawn((
                C1ControlPoint(p.0, p.1, p.2, p.3),
                Mesh3d(sphere.clone()),
                MeshMaterial3d(material_shadow.clone()),
                Visibility::Hidden,
            ))
            .observe(enable_gizmo_shadow_points);
    }

    // Draw lines between neighbouring controls points to generate a visible grid
    for i in 0..points.len() {
        let p0 = points[i];
        let p0_id = ids[i];

        if i + 1 < (i / w + 1) * w
            && let Some(py) = points.get(i + 1)
        {
            let id = *ids.get(i + 1).expect("must be present");
            let mut mesh = Mesh::new(
                PrimitiveTopology::LineList,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    Vec3::new(p0.2 * scale, p0.3 * scale, p0.4 * scale),
                    Vec3::new(py.2 * scale, py.3 * scale, py.4 * scale),
                ],
            );

            commands.spawn((
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLine(p0_id, id),
                MeshMaterial3d(materials.add(Color::BLACK)),
                Mesh3d(meshes.add(mesh)),
            ));
        }

        if i + w < points.len()
            && let Some(px) = points.get(i + 2)
        {
            let id = *ids.get(i + w).expect("must be present");
            let mut mesh = Mesh::new(
                PrimitiveTopology::LineList,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    Vec3::new(p0.2 * scale, p0.3 * scale, p0.4 * scale),
                    Vec3::new(px.2 * scale, px.3 * scale, px.4 * scale),
                ],
            );

            commands.spawn((
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLine(p0_id, id),
                MeshMaterial3d(materials.add(Color::BLACK)),
                Mesh3d(meshes.add(mesh)),
            ));
        }
    }

    event_writer.write(RedrawEvent((200, 200)));
}

#[allow(clippy::complexity)]
fn handle_c1_points_events(
    mut reader: EventReader<ToggleC1Enable>,
    mut constraint_state: ResMut<ConstraintState>,
    mut set: ParamSet<(
        Query<(&mut Transform, &C1ControlPoint, &mut Visibility)>,
        Query<(&Transform, &RenderPoint)>,
    )>,
) {
    if !reader.is_empty() {
        reader.clear();
        constraint_state.c1_enabled = !constraint_state.c1_enabled;
    } else {
        return;
    }

    let points = collect_control_points(set.p1());

    for (mut trans, c1_point, mut visibility) in set.p0().iter_mut() {
        if constraint_state.c1_enabled {
            let constraint = C1Constraint::new(
                Point::default(),
                (c1_point.0, c1_point.1),
                (c1_point.2, c1_point.3),
            );

            if let Some(new_position) = constraint.inverse(&points) {
                trans.translation = new_position.into();
                *visibility = Visibility::Visible;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

#[allow(clippy::complexity)]
fn solve_constraints(
    constraint_state: Res<ConstraintState>,
    mut set: ParamSet<(
        Query<(&Transform, &C1ControlPoint)>,
        Query<(&mut Transform, &RenderPoint)>,
    )>,
) {
    let points = collect_control_points(set.p1().as_readonly());

    let mut constraints = vec![];

    for (trans, c1_point) in set.p0().iter() {
        if constraint_state.c1_enabled {
            let constraint = C1Constraint::new(
                trans.translation.into(),
                (c1_point.0, c1_point.1),
                (c1_point.2, c1_point.3),
            );
            constraints.push(constraint);
        }
    }

    let points = Solver::solve_constraints(
        &points,
        Constraints {
            c1_constraints: constraints,
        },
    );

    for (mut trans, render_point) in set.p1().iter_mut() {
        trans.translation = points[render_point.0][render_point.1].into();
    }
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut toggle_writer: EventWriter<ToggleC1Enable>,
    mut history: EventWriter<HistoryUndoEvent>,
) {
    if keyboard.just_released(KeyCode::Space) {
        toggle_writer.write(ToggleC1Enable);
    }

    if keyboard.just_released(KeyCode::KeyU) {
        history.write(HistoryUndoEvent);
    }
}

impl Plugin for BezierRender {
    fn build(&self, app: &mut App) {
        app.add_event::<RedrawEvent>();
        app.init_resource::<ScaleInformation>();
        app.add_systems(Startup, generate_default_curve);
        app.add_systems(PreUpdate, (handle_keyboard, solve_constraints));
        app.add_systems(
            Update,
            (generate_pointcloud, update_lines, update_surface_click),
        ); // , listen_to_mouse_left_button));
        app.add_systems(PostUpdate, handle_c1_points_events);
        app.init_resource::<ConstraintState>();
        app.add_event::<ToggleC1Enable>();
    }
}
