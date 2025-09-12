use std::collections::HashMap;

use super::curvature_display_mode::{
    ChangeCurvatureDisplayModeEvent, CurvatureDisplayMode, handle_change_curvature,
};
use super::degree_manipulation::{
    DecreaseDegreeEvent, IncreaseDegreeEvent, handle_degree_increase_event,
    handle_degree_reduction_event,
};
use super::helper_curves::{
    CreateCurveState, RedrawCurvesEvent, add_point, commit_curve, commit_plane,
    enter_create_curve_mode, render_curves,
};
use super::{components::*, handle_generic_deleted_event};

#[cfg(feature = "vr_enable")]
use super::helper_curves::add_point_3d;

use super::ortho_camera::create_camera_on_click;

#[cfg(feature = "vr_enable")]
use super::ortho_camera::create_camera_on_click3d;
use super::render_info::{
    ChangeCoordinateMode, ChangeSurfaceMeshMode, CoordinateMode, RenderInformation,
    SurfaceMeshMode, UVEither, UpdateBoxDimEvent, UpdateIsoDimEvent, handle_box_dim_event,
    handle_change_coordinate_mode, handle_change_surface_mode, handle_iso_dim_event,
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
use crate::nurbs::bezier_plane::{ControlPoints2D, derive_2d, eval_2d_bezier_curves};
use crate::picking3d::events::{HoveredBy, Pointer3d};
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::{
    AddBoundingEntityEvent, BoundingEntitiesManager, DisplayIn, UpdateOrthoViews,
};
use crate::translation_control::translation_controller::{
    CantSnapToEntities, EnableTranslationControl, MovedEntityEvent,
};
use crate::translation_control::{enable_gizmo, enable_gizmo3d};
use crate::util::update_material_on;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use bevy::app::App;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::css::BLACK;
use bevy::color::palettes::tailwind::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::system::SystemParam;
use bevy::ecs::world::{DeferredWorld, OnDespawn};
use bevy::prelude::*;
use bevy::render::mesh::PrimitiveTopology;
use bevy::render::view::RenderLayers;
use num::ToPrimitive;

pub type Resolution = (u32, u32);

#[derive(Event, Clone, Copy)]
pub enum RedrawEvent {
    HighQuality,
    Fast,
}

#[derive(Event)]
pub enum RedrawLinesEvent {
    HighQuality,
    Fast,
}

impl From<RedrawEvent> for RedrawLinesEvent {
    fn from(value: RedrawEvent) -> Self {
        match value {
            RedrawEvent::HighQuality => Self::HighQuality,
            RedrawEvent::Fast => Self::Fast,
        }
    }
}

#[derive(Component)]
#[require(Transform)]
#[component(on_despawn = generic_on_despawn_trigger)]
pub struct Surface;

#[derive(Event)]
pub struct RedrawBoxesEvent;

#[derive(Component)]
struct Bridge {
    pub a: Entity,
    pub b: Entity,
}

impl Default for Bridge {
    fn default() -> Self {
        Self {
            a: Entity::PLACEHOLDER,
            b: Entity::PLACEHOLDER,
        }
    }
}

#[derive(Component)]
#[relationship(relationship_target = CompleteBridgeCenters)]
#[component(on_despawn = generic_on_despawn_trigger)]
struct CompleteBridgeCenter {
    #[relationship]
    pub complete_bridge: Entity,
    pub single_segment: Option<Entity>,
}

#[derive(Component)]
#[relationship_target(relationship = CompleteBridgeCenter, linked_spawn)]
struct CompleteBridgeCenters(Vec<Entity>);

#[derive(Component)]
#[relationship(relationship_target = CompleteBridge)]
#[require(Bridge)]
struct BridgeConnector(pub Entity);

#[derive(Component)]
#[relationship_target(relationship=BridgeConnector, linked_spawn)]
struct CompleteBridge(Vec<Entity>);

#[derive(Component)]
struct BridgeMarker;

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

pub fn generic_on_despawn_trigger(mut world: DeferredWorld, context: HookContext) {
    let mut writer = world.resource_mut::<Events<EntityDeletedEvent>>();
    writer.send(EntityDeletedEvent(context.entity));
}

pub fn hover_3d(
    trigger: Trigger<Pointer3d<crate::picking3d::events::MoveIn>>,
    mut left_vibrate: EventWriter<VibrateLeftEvent>,
    mut right_vibrate: EventWriter<VibrateRightEvent>,
) {
    match trigger.controler {
        HoveredBy::Left => {
            left_vibrate.write(VibrateLeftEvent::new(Vibration::default()));
        }
        HoveredBy::Right => {
            right_vibrate.write(VibrateRightEvent::new(Vibration::default()));
        }
    }
}

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

fn draw_bridge_cylinder(
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    length: f32,
    scale: f32,
) -> impl Bundle {
    let cylinder = meshes.add(Cylinder::new(0.02 * scale, length));
    let material = materials.add(StandardMaterial::from_color(BLACK));

    (
        Mesh3d(cylinder),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, 0.0, -length / 2.0)
            .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())),
        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
        Visibility::Inherited,
        BridgeMarker,
    )
}

#[allow(clippy::complexity)]
fn update_lines(
    mut commands: Commands,
    mut lines: Query<
        (&Bridge, Entity, &Children, &mut Transform),
        (Without<BridgeMarker>, Without<CompleteBridgeCenter>),
    >,
    mut changeable: Query<
        (&mut Transform, &mut Mesh3d),
        (
            Without<Bridge>,
            With<BridgeMarker>,
            Without<CompleteBridgeCenter>,
        ),
    >,
    mut bridge_centers: Query<
        (&mut Transform, &CompleteBridgeCenter),
        (Without<BridgeMarker>, Without<Bridge>),
    >,
    transforms: Query<
        &Transform,
        (
            Without<BridgeMarker>,
            Without<Bridge>,
            Without<CompleteBridgeCenter>,
        ),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    info: Res<RenderInformation>,
) {
    for (line, line_entity, children, mut transform) in lines.iter_mut() {
        let (entity1, entity2) = {
            let entity1 = transforms.get(line.a);
            let entity2 = transforms.get(line.b);
            (entity1, entity2)
        };

        if entity1.is_err() || entity2.is_err() {
            commands.get_entity(line_entity).unwrap().despawn();
            continue;
        }

        let origin = entity1.unwrap().translation;
        let target = entity2.unwrap().translation;
        let diff = target - origin;
        let length = diff.length();

        for child in children {
            if let Ok((mut transform, mut mesh3d)) = changeable.get_mut(*child) {
                let cylinder = meshes.add(Cylinder::new(0.02 * info.scale, length));
                *transform = Transform::from_xyz(0.0, 0.0, -length / 2.0)
                    .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians()));
                mesh3d.0 = cylinder;
            }
        }

        *transform = Transform::from_translation(origin).looking_at(target, Vec3::Y);
    }

    for (mut transform, center) in bridge_centers.iter_mut() {
        if let Some(segment) = center.single_segment
            && let Ok(line) = lines.get(segment)
        {
            let a = line.0.a;
            let b = line.0.b;

            let transform_a = transforms.get(a).unwrap().translation;
            let transform_b = transforms.get(b).unwrap().translation;

            let midpoint = transform_a * 0.5 + transform_b * 0.5;

            transform.translation = midpoint;
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

pub fn distribute_redraw_event(
    mut redraw_event_reader: EventReader<RedrawEvent>,
    mut redraw_boxes: EventWriter<RedrawBoxesEvent>,
    mut redraw_iso_lines: EventWriter<RedrawLinesEvent>,
    mut update_ortho_views: EventWriter<UpdateOrthoViews>,
) {
    for event in redraw_event_reader.read() {
        redraw_boxes.write(RedrawBoxesEvent);
        redraw_iso_lines.write(RedrawLinesEvent::from(*event));
        update_ortho_views.write(UpdateOrthoViews);
    }
}

#[allow(clippy::complexity)]
fn generate_pointcloud(
    surface: Query<Entity, With<Surface>>,
    mut events: EventReader<RedrawEvent>,
    mut commands: Commands,
    entities: Query<Entity, With<ResultSurface>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
    scale_info: Res<RenderInformation>,
) {
    let mut resolution: Resolution = scale_info.fast_resolution;
    let mut event = RedrawEvent::Fast;
    if !events.is_empty() {
        // Consume and run redraw. No matter how many events where triggered
        #[allow(clippy::never_loop)]
        for evt in events.read() {
            match evt {
                RedrawEvent::HighQuality => resolution = scale_info.resolution,
                RedrawEvent::Fast => resolution = scale_info.fast_resolution,
            }

            event = *evt;

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
            let surface = surface.single().unwrap();
            commands
                .spawn((
                    Transform::default(),
                    ResultSurface,
                    Name::new("Result Surface"),
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(mat)),
                    RenderLayers::from(DisplayIn::Normal),
                    Visibility::Inherited,
                    ChildOf(surface),
                ))
                .observe(bezier_surface_picking);
        }
    }
}

#[allow(clippy::complexity)]
pub fn redraw_iso_lines(
    surface: Query<Entity, With<Surface>>,
    mut commands: Commands,
    mut events: EventReader<RedrawLinesEvent>,
    entities: Query<Entity, With<ResultLines>>,
    scale_info: Res<RenderInformation>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut resolution: Resolution = scale_info.fast_resolution;
    if !events.is_empty() {
        // Consume and run redraw. No matter how many events where triggered
        #[allow(clippy::never_loop)]
        for evt in events.read() {
            match evt {
                RedrawLinesEvent::HighQuality => resolution = scale_info.resolution,
                RedrawLinesEvent::Fast => resolution = scale_info.fast_resolution,
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
        let surface = surface.single().unwrap();
        commands
            .spawn((
                Transform::default(),
                ResultLines,
                Visibility::default(),
                ChildOf(surface),
            ))
            .with_children(|ui| {
                for mesh in meshes_lines {
                    ui.spawn((
                        Mesh3d(meshes.add(mesh)),
                        MeshMaterial3d(materials.add(Color::BLACK)),
                        Name::new("Iso Line"),
                        Pickable::IGNORE,
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                    ));
                }
            });
    }
}

#[allow(clippy::complexity)]
pub fn redraw_boxes(
    mut events: EventReader<RedrawBoxesEvent>,
    mut commands: Commands,
    surface: Query<Entity, With<Surface>>,
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

    let multi_curves = collect_control_points(control_points);
    for (u, v) in scale_info.to_uv_sample() {
        let point = eval_2d_bezier_curves(&multi_curves, u, v);
        let (u_diff, v_diff) = derive_2d(&multi_curves, u, v, 1);
        let normal = u_diff.cross(&v_diff) * -1.0;
        let (u_diff_2, v_diff_2) = derive_2d(&multi_curves, u, v, 2);

        let color = if scale_info.curvature_mode == CurvatureDisplayMode::None {
            (1.0, 0.0, 0.0)
        } else {
            curvature_to_color(
                &scale_info.curvature_mode,
                normal,
                u_diff,
                v_diff,
                u_diff_2,
                v_diff_2,
                scale_info.scale as f64,
            )
        };

        let mesh = Cuboid::new(
            scale_info.box_dim.0 * scale_info.scale,
            scale_info.box_dim.1 * scale_info.scale,
            scale_info.box_dim.2 * scale_info.scale,
        );

        let mut transform = Transform::from_translation(Vec3::from(point));
        if scale_info.coordinate_mode == CoordinateMode::NUV {
            transform.align(Vec3::NEG_Z, Vec3::from(v_diff), Vec3::X, Vec3::from(u_diff));
            transform.look_to(Vec3::from(v_diff), Vec3::from(normal));
        } else {
            transform.look_to(Vec3::NEG_Z, Vec3::Y);
        }
        commands.spawn((
            ChildOf(surface.single().unwrap()),
            transform,
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
    }
}

#[derive(SystemParam)]
pub struct SurfaceCreator<'w, 's> {
    commands: Commands<'w, 's>,
    pub info: Res<'w, RenderInformation>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    root: Query<'w, 's, Entity, With<RootTransform>>,
    surface: Query<'w, 's, Entity, With<Surface>>,
    event_writer: EventWriter<'w, RedrawEvent>,
    add_bounding_entities: EventWriter<'w, AddBoundingEntityEvent>,
}

impl<'w, 's> SurfaceCreator<'w, 's> {
    pub fn create_surface_from_points(
        &mut self,
        points: Vec<(usize, usize, Vec3)>,
        w: usize,
        h: usize,
    ) {
        let root = self.root.single().unwrap();

        if let Ok(surface) = self.surface.single() {
            info!("Deleting old surface");
            self.commands.entity(surface).despawn();
        }

        info!("Creating new surface");
        let surface = self
            .commands
            .spawn((Surface, ChildOf(root), Visibility::Inherited))
            .id();

        let scale = self.info.scale;
        let material = self.materials.add(Color::from(GRAY_400));
        let material_hover = self.materials.add(Color::from(GRAY_600));
        let sphere = self
            .meshes
            .add(Sphere::new(0.1 * scale).mesh().ico(5).unwrap());

        let mut ids = vec![];

        for p in &points {
            let id = self
                .commands
                .spawn((
                    RenderPoint(p.0, p.1),
                    Name::new(format!("Render Point {} {}", p.0, p.1)),
                    Transform::from_translation(p.2),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(material.clone()),
                    Picking3dInteractable::default(),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                    Visibility::Inherited,
                    ChildOf(surface),
                ))
                .observe(update_material_on::<Pointer<Over>>(material_hover.clone()))
                .observe(update_material_on::<Pointer<Out>>(material.clone()))
                .observe(update_material_on::<
                    Pointer3d<crate::picking3d::events::MoveIn>,
                >(material_hover.clone()))
                .observe(update_material_on::<
                    Pointer3d<crate::picking3d::events::MoveIn>,
                >(material.clone()))
                .observe(hover_3d)
                //.observe(drag_point)
                .observe(enable_gizmo(EnableTranslationControl::OnlyTranslation))
                .observe(enable_gizmo3d(EnableTranslationControl::OnlyTranslation))
                .id();
            self.add_bounding_entities.write(AddBoundingEntityEvent(id));
            ids.push(id);
        }

        let mut x_bridges = Vec::new();
        let mut y_bridges = Vec::new();

        for x in 0..w {
            x_bridges.push(
                self.commands
                    .spawn((
                        CompleteBridge(Vec::new()),
                        Name::new(format!("x_bridge {x}")),
                        Transform::default(),
                        Visibility::Inherited,
                        ChildOf(surface),
                    ))
                    .id(),
            );
        }

        for y in 0..h {
            y_bridges.push(
                self.commands
                    .spawn((
                        CompleteBridge(Vec::new()),
                        Name::new(format!("y_bridge {y}")),
                        Transform::default(),
                        Visibility::Inherited,
                        ChildOf(surface),
                    ))
                    .id(),
            );
        }

        let to_xy = |i| (i % w, i / w);
        let from_xy = |x, y| x + y * w;

        let x_mid = w / 2;
        let y_mid = h / 2;
        // Draw lines between neighbouring controls points to generate a visible grid
        for i in 0..points.len() {
            let p0 = points[i];
            let p0_id = ids[i];

            let (x, y) = to_xy(i);
            let (_, y_next) = to_xy(i + 1);

            // Check if we are not at the edge of the array (next index is one row up)
            if y == y_next
                && let Some(px) = points.get(i + 1)
            {
                let id = *ids.get(i + 1).expect("must be present");

                let origin = p0.2;
                let target = px.2;
                let diff = target - origin;

                let bridge_id = self
                    .commands
                    .spawn((
                        Transform::from_translation(origin).looking_at(target, Vec3::Y),
                        Bridge { a: p0_id, b: id },
                        BridgeConnector(y_bridges[y]),
                        Name::new(format!("Render line {p0_id} {id}")),
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                        ChildOf(surface),
                        related!(
                            Children[draw_bridge_cylinder(
                                &mut self.meshes,
                                &mut self.materials,
                                diff.length(),
                                scale
                            )]
                        ),
                    ))
                    .id();

                let is_x_mid = x < x_mid && x + 1 >= x_mid;
                if is_x_mid {
                    let complete_bridge = y_bridges[y];

                    let center = origin + diff * 0.5;
                    let material = self.materials.add(Color::from(GREEN_800));
                    let material_hover = self.materials.add(Color::from(GREEN_600));

                    let id = self
                        .commands
                        .spawn((
                            Transform::from_translation(center),
                            CompleteBridgeCenter {
                                complete_bridge,
                                single_segment: Some(bridge_id),
                            },
                            ChildOf(surface),
                            Visibility::Inherited,
                            Mesh3d(self.meshes.add(Sphere::new(0.1 * scale))),
                            MeshMaterial3d(material.clone()),
                            Picking3dInteractable::default(),
                            CantSnapToEntities::All,
                        ))
                        .observe(enable_gizmo(EnableTranslationControl::OnlyTranslation))
                        .observe(enable_gizmo3d(EnableTranslationControl::OnlyTranslation))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material_hover.clone()))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material.clone()))
                        .observe(hover_3d)
                        .observe(moved_complete_bridge)
                        .id();
                    self.add_bounding_entities.write(AddBoundingEntityEvent(id));
                }
            }

            let next_row_idx = from_xy(x, y + 1);
            // Check if next row is still in range
            if y + 1 < h
                && let Some(py) = points.get(next_row_idx)
            {
                let id = *ids.get(i + w).expect("must be present");

                let origin = p0.2;
                let target = py.2;
                let diff = target - origin;

                let bridge_id = self
                    .commands
                    .spawn((
                        Transform::from_translation(origin).looking_at(target, Vec3::Y),
                        Bridge { a: p0_id, b: id },
                        BridgeConnector(x_bridges[x]),
                        Name::new(format!("Render line {p0_id} {id}")),
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                        ChildOf(surface),
                        related!(
                            Children[draw_bridge_cylinder(
                                &mut self.meshes,
                                &mut self.materials,
                                diff.length(),
                                scale
                            )]
                        ),
                    ))
                    .id();

                let is_y_mid = y < y_mid && y + 1 >= y_mid;
                if is_y_mid {
                    let center = origin + diff * 0.5;
                    let complete_bridge = x_bridges[x];

                    let material = self.materials.add(Color::from(GREEN_800));
                    let material_hover = self.materials.add(Color::from(GREEN_600));

                    let id = self
                        .commands
                        .spawn((
                            Transform::from_translation(center),
                            CompleteBridgeCenter {
                                complete_bridge,
                                single_segment: Some(bridge_id),
                            },
                            ChildOf(surface),
                            Visibility::Inherited,
                            Mesh3d(self.meshes.add(Sphere::new(0.1 * scale))),
                            MeshMaterial3d(material.clone()),
                            Picking3dInteractable::default(),
                            CantSnapToEntities::All,
                        ))
                        .observe(enable_gizmo(EnableTranslationControl::OnlyTranslation))
                        .observe(enable_gizmo3d(EnableTranslationControl::OnlyTranslation))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material_hover.clone()))
                        .observe(update_material_on::<
                            Pointer3d<crate::picking3d::events::MoveIn>,
                        >(material.clone()))
                        .observe(hover_3d)
                        .observe(moved_complete_bridge)
                        .id();
                    self.add_bounding_entities.write(AddBoundingEntityEvent(id));
                }
            }

            // let is_y_mid = y > y_mid && y < y_mid + 1;
        }

        self.event_writer.write(RedrawEvent::HighQuality);
    }
}

pub fn generate_default_curve(mut surface_creator: SurfaceCreator) {
    let (w, h): (usize, usize) = (6, 6);
    let min_x = -w.to_f32().unwrap() / 2.0 + if w % 2 == 0 { 0.5 } else { 0.0 };
    let min_y = -h.to_f32().unwrap() / 2.0 + if h % 2 == 0 { 0.5 } else { 0.0 };

    let mut points = vec![];
    let mut curr_x = min_x;
    let mut curr_y = min_y;
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            points.push((
                y as usize,
                x as usize,
                Vec3::new(curr_x, 0.0, curr_y) * surface_creator.info.scale
                    + surface_creator.info.height,
            ));
            curr_x += 1.0;
        }
        curr_x = min_x;
        curr_y += 1.0;
    }

    surface_creator.create_surface_from_points(points, w, h);
}

fn moved_complete_bridge(
    trigger: Trigger<MovedEntityEvent>,
    centers: Query<&CompleteBridgeCenter>,
    complete_brigdes: Query<&CompleteBridge>,
    bridge_segments: Query<&Bridge>,
    mut transforms: Query<&mut Transform>,
) {
    let Ok(center) = centers.get(trigger.target()) else {
        return;
    };

    let Ok(bridges) = complete_brigdes.get(center.complete_bridge) else {
        return;
    };

    let mut entity_transform_mapping = HashMap::new();
    for bridge in &bridges.0 {
        if let Ok(bridge) = bridge_segments.get(*bridge) {
            if let Ok(transform) = transforms.get(bridge.a) {
                entity_transform_mapping.insert(bridge.a, *transform);
            }
            if let Ok(transform) = transforms.get(bridge.b) {
                entity_transform_mapping.insert(bridge.b, *transform);
            }
        }
    }

    for v in entity_transform_mapping.values_mut() {
        v.translation += trigger.event().delta;
    }

    // apply edited translations to world
    for (k, v) in &entity_transform_mapping {
        if let Ok(mut transform) = transforms.get_mut(*k) {
            transform.translation = v.translation;
        }
    }

    // compute the new position of the sphere that defines the bridges movement
    if let Ok(mut transform) = transforms.get_mut(trigger.target())
        && let Ok(segment) = bridge_segments.get(center.single_segment.expect("Must be present"))
    {
        let transform_a = entity_transform_mapping[&segment.a].translation;
        let transform_b = entity_transform_mapping[&segment.b].translation;

        let diff = transform_b - transform_a;

        transform.translation = transform_a + 0.5 * diff;
    }
}

fn handle_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    // mut toggle_writer: EventWriter<ToggleC1Enable>,
    mut history: EventWriter<HistoryUndoEvent>,
    mut change_curvature: EventWriter<ChangeCurvatureDisplayModeEvent>,
    mut increase_degree: EventWriter<IncreaseDegreeEvent>,
    mut decrease_degree: EventWriter<DecreaseDegreeEvent>,
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

    if keyboard.just_released(KeyCode::KeyI) {
        increase_degree.write(IncreaseDegreeEvent);
    }

    if keyboard.just_released(KeyCode::KeyD) {
        decrease_degree.write(DecreaseDegreeEvent);
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
                (generate_pointcloud, distribute_redraw_event).run_if(on_event::<RedrawEvent>),
                update_lines,
                update_surface_click,
                redraw_boxes.after(generate_pointcloud),
                redraw_iso_lines.after(generate_pointcloud),
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
                handle_iso_dim_event,
            ),
        );

        // Systems for handling state
        app.add_systems(
            PostUpdate,
            (
                handle_change_coordinate_mode.run_if(in_state(ControlState::Main)),
                handle_change_surface_mode.run_if(in_state(ControlState::Main)),
                handle_create_curve_event.run_if(in_state(ControlState::Main)),
                handle_create_ortho_camera_event.run_if(in_state(ControlState::Main)),
                handle_create_plane_event.run_if(in_state(ControlState::Main)),
                handle_delete_mode_event.run_if(in_state(ControlState::Main)),
                create_camera_on_click.run_if(in_state(ControlState::CreateOrthoCamera)),
                #[cfg(feature = "vr_enable")]
                create_camera_on_click3d.run_if(in_state(ControlState::CreateOrthoCamera)),
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
            add_point_3d
                .run_if(in_state(ControlState::CreateCurve).or(in_state(ControlState::CreatePlane)))
                .after(render_curves),
        );

        app.add_systems(
            Update,
            add_point
                .run_if(in_state(ControlState::CreateCurve).or(in_state(ControlState::CreatePlane)))
                .after(render_curves),
        );

        app.add_systems(
            PostUpdate,
            (
                handle_degree_increase_event.run_if(on_event::<IncreaseDegreeEvent>),
                handle_degree_reduction_event.run_if(on_event::<DecreaseDegreeEvent>),
            ),
        );

        app.add_systems(PostUpdate, handle_generic_deleted_event);

        // app.init_resource::<ConstraintState>();
        app.init_resource::<RenderInformation>();
        app.init_resource::<CreateCurveState>();

        app.add_event::<RedrawEvent>();
        // app.add_event::<ToggleC1Enable>();
        app.add_event::<SurfaceClickChangeset>();
        app.add_event::<ChangeCurvatureDisplayModeEvent>();
        app.add_event::<UpdateBoxDimEvent>();
        app.add_event::<UpdateIsoDimEvent>();
        app.add_event::<RedrawBoxesEvent>();
        app.add_event::<RedrawLinesEvent>();
        app.add_event::<ChangeSurfaceMeshMode>();
        app.add_event::<ChangeCoordinateMode>();
        app.add_event::<RedrawCurvesEvent>();
        app.add_event::<CreateCurveEvent>();
        app.add_event::<CreatePlaneEvent>();
        app.add_event::<CreateOrthoCameraEvent>();
        app.add_event::<DeleteModeEvent>();
        app.add_event::<EndModeEvent>();
        app.add_event::<EntityDeletedEvent>();
        app.add_event::<IncreaseDegreeEvent>();
        app.add_event::<DecreaseDegreeEvent>();
        app.init_state::<ControlState>();
    }
}
