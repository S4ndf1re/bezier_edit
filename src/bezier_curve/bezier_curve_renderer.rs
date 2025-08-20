use super::components::*;
use super::curvature_display_mode::{
    ChangeCurvatureDisplayModeEvent, CurvatureDisplayMode, handle_change_curvature,
};
use super::helper_curves::{
    CreateCurveState, RedrawCurvesEvent, commit_curve, commit_plane, enter_create_curve_mode,
    render_curves,
};

#[cfg(feature = "vr_enable")]
use super::helper_curves::add_point_3d;

use super::ortho_camera::create_camera_on_click;
use super::render_info::{
    ChangeSurfaceMeshMode, RenderInformation, SurfaceMeshMode, UVEither, UpdateBoxDimEvent,
    handle_box_dim_event, handle_change_surface_mode,
};
use super::surface_click::{
    SurfaceClickChangeset, bezier_surface_picking, handle_state_change_event, update_surface_click,
};
use super::util::{
    collect_control_points, compute_point_by_params, create_mesh_from_control_points,
    curvature_to_color,
};
use crate::RootTransform;
use crate::bezier_curve::EntityDeletedEvent;
use crate::history::plugin::HistoryUndoEvent;
use crate::nurbs::bezier_plane::{derive_2d, eval_2d_bezier_curves};
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::DisplayIn;
use crate::translation_control::{enable_gizmo, enable_gizmo3d};
use crate::util::update_material_on;
use bevy::app::App;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::view::RenderLayers;
use num::ToPrimitive;

pub type Resolution = (u32, u32);

#[derive(Event)]
pub enum RedrawEvent {
    HighQuality,
    Fast,
}

#[derive(Event)]
pub struct RedrawBoxesEvent;

#[derive(Event)]
pub struct CreateOrthoCameraEvent;

#[derive(Event)]
pub struct CreateCurveEvent;

#[derive(Event)]
pub struct CreatePlaneEvent;

#[derive(Event)]
pub struct DeleteModeEvent;

#[derive(Event)]
pub struct EndModeEvent;

fn handle_create_ortho_camera_event(
    mut reader: EventReader<CreateOrthoCameraEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::CreateOrthoCamera);
}

fn handle_create_curve_event(
    mut reader: EventReader<CreateCurveEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::CreateCurve);
}

fn handle_create_plane_event(
    mut reader: EventReader<CreatePlaneEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::CreatePlane);
}

fn handle_delete_mode_event(
    mut reader: EventReader<DeleteModeEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::Delete);
}

fn handle_end_mode(
    mut reader: EventReader<EndModeEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();
    next_state.set(ControlState::Main)
}

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

// fn drag_point(
//     trigger: Trigger<Pointer<Drag>>,
//     mut query: Query<&mut Transform, (With<RenderPoint>, Without<Camera3d>)>,
//     camera: Single<&Transform, With<Camera3d>>,
// ) {
//     let mut point = query.get_mut(trigger.target()).unwrap();
//
//     point.translation = point.translation
//         + camera.right() * trigger.delta.x * 0.012
//         + camera.up() * trigger.delta.y * -0.012;
// }

#[allow(clippy::complexity)]
fn generate_pointcloud(
    root: Query<Entity, With<RootTransform>>,
    mut events: EventReader<RedrawEvent>,
    mut commands: Commands,
    entities: Query<Entity, With<ResultSurface>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
    scale_info: Res<RenderInformation>,
    mut redraw_boxes: EventWriter<RedrawBoxesEvent>,
) {
    let mut resolution: Resolution = scale_info.fast_resolution;
    if !events.is_empty() {
        // Consume and run redraw. No matter how many events where triggered
        #[allow(clippy::never_loop)]
        for evt in events.read() {
            match evt {
                RedrawEvent::HighQuality => resolution = scale_info.resolution,
                RedrawEvent::Fast => resolution = scale_info.fast_resolution,
            }

            break;
        }
        events.clear()
    } else {
        return;
    }

    // Despawn old, respawn new
    for p in entities.iter() {
        commands.entity(p).despawn();
    }

    let multi_curves = collect_control_points(control_points);
    if scale_info.surface_mesh_mode == SurfaceMeshMode::Mesh {
        let mut color = Color::from(GRAY_500);
        color.set_alpha(0.3);

        let (mesh, image_handle) = create_mesh_from_control_points(
            &multi_curves,
            resolution,
            &scale_info.curvature_mode,
            images,
            scale_info.scale as f64,
        );

        let mat = StandardMaterial {
            base_color_texture: Some(image_handle),
            double_sided: true,
            cull_mode: None,
            ..Default::default()
        };

        {
            let mut root = commands.get_entity(root.single().unwrap()).unwrap();
            root.with_children(|ui| {
                ui.spawn((
                    Transform::default(),
                    ResultSurface,
                    Name::new("Result Surface"),
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(mat)),
                    RenderLayers::from(DisplayIn::Normal),
                ))
                .observe(bezier_surface_picking);
            });
        }
    } else {
        let w = resolution.0;
        let h = resolution.1;

        let mut meshes_lines = Vec::new();

        for line in scale_info.to_line_uv() {
            let verticies = match line {
                UVEither::U(u) => (0..h)
                    .map(|v| compute_point_by_params(&multi_curves, u, v as f64 / ((h - 1) as f64)))
                    .map(|p| p.into())
                    .collect::<Vec<Vec3>>(),
                UVEither::V(v) => (0..w)
                    .map(|u| compute_point_by_params(&multi_curves, u as f64 / ((w - 1) as f64), v))
                    .map(|p| p.into())
                    .collect::<Vec<Vec3>>(),
            };

            let mut mesh = Mesh::new(
                PrimitiveTopology::LineStrip,
                RenderAssetUsages::RENDER_WORLD,
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies);
            meshes_lines.push(mesh);
        }

        {
            let mut root = commands.get_entity(root.single().unwrap()).unwrap();
            root.with_children(|ui| {
                ui.spawn((Transform::default(), ResultSurface, Visibility::default()))
                    .with_children(|ui| {
                        for mesh in meshes_lines {
                            ui.spawn((
                                Mesh3d(meshes.add(mesh)),
                                MeshMaterial3d(materials.add(Color::BLACK)),
                                Name::new("Iso Line"),
                                Pickable::IGNORE,
                                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                            ));
                        }
                    });
            });
        }
    }

    redraw_boxes.write(RedrawBoxesEvent);
}

#[allow(clippy::complexity)]
pub fn redraw_boxes(
    mut events: EventReader<RedrawBoxesEvent>,
    mut commands: Commands,
    root: Query<Entity, With<RootTransform>>,
    boxes: Query<Entity, With<CurveBox>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    scale_info: Res<RenderInformation>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if events.is_empty() {
        return;
    };
    events.clear();

    // Despawn old, respawn new
    for p in boxes.iter() {
        commands.entity(p).despawn();
    }

    // Early return, because line mode hijacks the box count parameters (otherwise, no lines would
    // be visible)
    if scale_info.surface_mesh_mode == SurfaceMeshMode::Lines {
        return;
    }

    let multi_curves = collect_control_points(control_points);
    let mut root = commands.get_entity(root.single().unwrap()).unwrap();
    for (u, v) in scale_info.to_uv_sample() {
        let point = eval_2d_bezier_curves(&multi_curves, u, v);
        let (u_diff, v_diff) = derive_2d(&multi_curves, u, v, 1);
        let normal = &u_diff.cross(&v_diff) * -1.0;
        let (u_diff_2, v_diff_2) = derive_2d(&multi_curves, u, v, 2);

        let color = if scale_info.curvature_mode == CurvatureDisplayMode::None {
            (1.0, 0.0, 0.0)
        } else {
            curvature_to_color(
                &scale_info.curvature_mode,
                &normal,
                &u_diff,
                &v_diff,
                &u_diff_2,
                &v_diff_2,
                scale_info.scale as f64,
            )
        };

        let mesh = Cuboid::new(
            scale_info.box_dim.0 * scale_info.scale,
            scale_info.box_dim.1 * scale_info.scale,
            scale_info.box_dim.2 * scale_info.scale,
        );
        root.with_children(|ui| {
            ui.spawn((
                Transform::from_translation(Vec3::from(point)).aligned_by(
                    Vec3::Z,
                    Vec3::from(u_diff),
                    Vec3::X,
                    Vec3::from(v_diff),
                ),
                Name::new("Box"),
                CurveBox,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(Color::from(Srgba::new(
                    color.0 as f32,
                    color.1 as f32,
                    color.2 as f32,
                    1.0,
                )))),
                RenderLayers::from(DisplayIn::Normal),
            ));
        });
    }
}

pub fn generate_default_curve(
    mut commands: Commands,
    root: Query<Entity, With<RootTransform>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: EventWriter<RedrawEvent>,
    scale_res: Res<RenderInformation>,
) {
    let mut root = commands.get_entity(root.single().unwrap()).unwrap();
    let scale = scale_res.scale;
    let height = scale_res.height;
    let material = materials.add(Color::from(GRAY_400));
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
        root.with_children(|ui| {
            let id = ui
                .spawn((
                    BezierRender,
                    RenderPoint(p.0, p.1),
                    Name::new(format!("Render Point {} {}", p.0, p.1)),
                    Transform::from_xyz(p.2 * scale, p.3 * scale + height, p.4 * scale),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material.clone()),
                    Picking3dInteractable,
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .with_children(|cmd| {
                    // TODO: remove this once rotation is fixed
                    cmd.spawn((
                        Transform::default(),
                        Mesh3d(meshes.add(Cuboid::new(0.07, 0.07, 0.4))),
                        MeshMaterial3d(material.clone()),
                    ));
                })
                .observe(update_material_on::<Pointer<Over>>(material_hover.clone()))
                .observe(update_material_on::<Pointer<Out>>(material.clone()))
                //.observe(drag_point)
                .observe(enable_gizmo::<true>)
                .observe(enable_gizmo3d::<false>)
                .id();
            ids.push(id);
        });
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

            root.with_child((
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLine(p0_id, id),
                Name::new(format!("Render line {p0_id} {id}")),
                MeshMaterial3d(materials.add(Color::BLACK)),
                Mesh3d(meshes.add(mesh)),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
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

            root.with_child((
                Transform::from_xyz(0.0, 0.0, 0.0),
                RenderLine(p0_id, id),
                Name::new(format!("Render line {p0_id} {id}")),
                MeshMaterial3d(materials.add(Color::BLACK)),
                Mesh3d(meshes.add(mesh)),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
            ));
        }
    }

    event_writer.write(RedrawEvent::HighQuality);
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    // mut toggle_writer: EventWriter<ToggleC1Enable>,
    mut history: EventWriter<HistoryUndoEvent>,
    mut change_curvature: EventWriter<ChangeCurvatureDisplayModeEvent>,
    renderinfo: Res<RenderInformation>,
) {
    if keyboard.just_released(KeyCode::KeyU) {
        history.write(HistoryUndoEvent);
    }

    if keyboard.just_released(KeyCode::KeyC) {
        change_curvature.write(ChangeCurvatureDisplayModeEvent(
            renderinfo.curvature_mode.next(),
        ));
    }
}

pub struct BezierRenderPlugin;

impl Plugin for BezierRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_default_curve);
        // app.add_systems(PreUpdate, (handle_keyboard, solve_constraints));
        app.add_systems(PreUpdate, handle_keyboard);
        app.add_systems(
            Update,
            (
                generate_pointcloud,
                update_lines,
                update_surface_click,
                redraw_boxes,
                render_curves,
            ),
        ); // , listen_to_mouse_left_button));
        app.add_systems(
            PostUpdate,
            (
                // handle_c1_points_events,
                handle_state_change_event,
                handle_change_curvature,
                handle_box_dim_event,
            ),
        );

        // Systems for handling state
        app.add_systems(
            PostUpdate,
            (
                handle_change_surface_mode.run_if(in_state(ControlState::Main)),
                handle_create_curve_event.run_if(in_state(ControlState::Main)),
                handle_create_ortho_camera_event.run_if(in_state(ControlState::Main)),
                handle_create_plane_event.run_if(in_state(ControlState::Main)),
                handle_delete_mode_event.run_if(in_state(ControlState::Main)),
                create_camera_on_click.run_if(in_state(ControlState::CreateOrthoCamera)),
                handle_end_mode.run_if(
                    in_state(ControlState::CreateCurve)
                        .or(in_state(ControlState::CreatePlane))
                        .or(in_state(ControlState::Delete))
                        .or(in_state(ControlState::CreateOrthoCamera)),
                ),
            ),
        );

        // Systems for snapping curves creation
        app.add_systems(OnEnter(ControlState::CreateCurve), enter_create_curve_mode);
        app.add_systems(OnExit(ControlState::CreateCurve), commit_curve);
        app.add_systems(OnExit(ControlState::CreatePlane), commit_plane);

        #[cfg(feature = "vr_enable")]
        app.add_systems(
            Update,
            add_point_3d.run_if(
                in_state(ControlState::CreateCurve).or(in_state(ControlState::CreatePlane)),
            ),
        );

        // app.init_resource::<ConstraintState>();
        app.init_resource::<RenderInformation>();
        app.init_resource::<CreateCurveState>();

        app.add_event::<RedrawEvent>();
        // app.add_event::<ToggleC1Enable>();
        app.add_event::<SurfaceClickChangeset>();
        app.add_event::<ChangeCurvatureDisplayModeEvent>();
        app.add_event::<UpdateBoxDimEvent>();
        app.add_event::<RedrawBoxesEvent>();
        app.add_event::<ChangeSurfaceMeshMode>();
        app.add_event::<RedrawCurvesEvent>();
        app.add_event::<CreateCurveEvent>();
        app.add_event::<CreatePlaneEvent>();
        app.add_event::<CreateOrthoCameraEvent>();
        app.add_event::<DeleteModeEvent>();
        app.add_event::<EndModeEvent>();
        app.add_event::<EntityDeletedEvent>();
        app.init_state::<ControlState>();
    }
}
