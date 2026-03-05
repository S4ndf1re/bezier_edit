use super::bridges::{BridgeSpawner, update_lines};
use super::curvature_display_mode::{
    ChangeCurvatureDisplayModeEvent, CurvatureDisplayMode, handle_change_curvature,
};
use super::degree_manipulation::{
    DecreaseDegreeEvent, IncreaseDegreeEvent, handle_degree_increase_event,
    handle_degree_reduction_event,
};
use super::helper_curves::{
    CreateCurveState, RedrawCurvesEvent, add_point, commit_curve, enter_create_curve_mode,
    render_curves,
};

#[cfg(feature = "vr_enable")]
use super::helper_curves::enter_create_curve_mode_vr;

use super::test_mode::EvaluationPlugin;
use super::{components::*, handle_generic_deleted_event};

#[cfg(feature = "vr_enable")]
use super::helper_curves::add_point_3d;

use super::ortho_camera::create_camera_on_click;

use super::inspector::{
    SurfaceInspectorChangeset, handle_inspector_change_event, update_surface_inspector,
};
#[cfg(feature = "vr_enable")]
use super::ortho_camera::create_camera_on_click3d;
use super::render_info::{
    ChangeCoordinateMode, ChangeSurfaceMeshMode, CoordinateMode, RenderInformation,
    SurfaceMeshMode, UVEither, UpdateBoxDimEvent, UpdateIsoDimEvent, handle_box_dim_event,
    handle_change_coordinate_mode, handle_change_surface_mode, handle_iso_dim_event,
};
use super::util::{
    SurfaceRenderMode, compute_point_by_params, create_mesh_from_control_points, curvature_to_color,
};
use crate::bezier_curve::EntityDeletedEvent;
use crate::bezier_curve::align_mode::{
    HomeRootTransformEvent, RecreateAlignmentChildren, create_alignment_sphere,
    delete_alignment_sphere, handle_home_root_transform, update_cross_bridge,
};
use crate::bezier_curve::bridges::{Bridge, draw_bridge_cylinder};
use crate::bezier_curve::helper_curves::{
    AddPointToCurveEvent, RemovePointFromCurveEvent, add_point_to_curve_handler,
    remove_point_from_curve_handler, spawn_new_curve, update_sphere_positions,
};
use crate::bezier_curve::inspector::{
    UpdateSurfaceInspectorEvent, inspectors_exist, setup_surface_inspector,
};
use crate::custom_shapes::parallelogram::Parallelogram2d;
use crate::history::plugin::HistoryUndoEvent;
use crate::nurbs::bezier_plane::ToControlPoints2D;
use crate::nurbs::parametric::Parametric;
use crate::picking3d::events::{HoveredBy, Pointer3d};
use crate::picking3d::picking_3d::Picking3dInteractable;
use crate::projection::{
    AddBoundingEntityEvent, DisplayIn, UpdateOrthoViews, handle_add_bounding_entity_event,
};
use crate::translation_control::proximity_detector::Snappable;
use crate::translation_control::translation_controller::{
    EnableTranslationControl, EnableTranslationControlType,
};
use crate::translation_control::{enable_gizmo, enable_gizmo3d};
use crate::util::update_material_on;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use crate::{RootTransform, picking3d};
use bevy::app::App;
use bevy::asset::RenderAssetUsages;
use bevy::color::palettes::tailwind::*;
use bevy::ecs::component::HookContext;
use bevy::ecs::system::SystemParam;
use bevy::ecs::system::lifetimeless::Read;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::render::mesh::{PrimitiveTopology, VertexAttributeValues};
use bevy::render::view::RenderLayers;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

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

#[derive(Event)]
pub struct MinusModeEvent;

#[derive(Event)]
pub struct PlusModeEvent;

#[derive(Event)]
pub struct AlignModeEvent;

#[derive(Component)]
pub struct NonSurfaceMarker;

#[derive(Component)]
pub struct RemoveOnResetMarker;

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

fn handle_minus_mode_event(
    mut reader: EventReader<MinusModeEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::Minus);
}
fn handle_plus_mode_event(
    mut reader: EventReader<PlusModeEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::Plus);
}

fn handle_align_mode_event(
    mut reader: EventReader<AlignModeEvent>,
    mut next_state: ResMut<NextState<ControlState>>,
) {
    if reader.is_empty() {
        return;
    }
    reader.clear();

    next_state.set(ControlState::Align);
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

pub fn distribute_redraw_event(
    mut redraw_event_reader: EventReader<RedrawEvent>,
    mut redraw_boxes: EventWriter<RedrawBoxesEvent>,
    mut redraw_iso_lines: EventWriter<RedrawLinesEvent>,
    mut update_ortho_views: EventWriter<UpdateOrthoViews>,
    mut update_inspector: EventWriter<UpdateSurfaceInspectorEvent>,
) {
    for event in redraw_event_reader.read() {
        redraw_boxes.write(RedrawBoxesEvent);
        redraw_iso_lines.write(RedrawLinesEvent::from(*event));
        update_ortho_views.write(UpdateOrthoViews);
        update_inspector.write(UpdateSurfaceInspectorEvent);
    }
}

fn click_on_control_point(
    trigger: Trigger<Pointer<Click>>,
    commands: Commands,
    enable_translation_control: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    mut decrease_degree: EventWriter<DecreaseDegreeEvent>,
    mut increase_degree: EventWriter<IncreaseDegreeEvent>,
) {
    if *state == ControlState::Main {
        enable_gizmo(EnableTranslationControl::new_with_root(
            EnableTranslationControlType::OnlyTranslation,
        ))(trigger, commands, enable_translation_control, state);
    } else if *state == ControlState::Minus {
        decrease_degree.write(DecreaseDegreeEvent);
    } else if *state == ControlState::Plus {
        increase_degree.write(IncreaseDegreeEvent);
    }
}

fn click_on_control_point3d(
    trigger: Trigger<Pointer3d<picking3d::events::Click>>,
    commands: Commands,
    enable_translation_control: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
    mut decrease_degree: EventWriter<DecreaseDegreeEvent>,
    mut increase_degree: EventWriter<IncreaseDegreeEvent>,
) {
    if *state == ControlState::Main {
        enable_gizmo3d(EnableTranslationControl::new_with_root(
            EnableTranslationControlType::OnlyTranslation,
        ))(trigger, commands, enable_translation_control, state);
    } else if *state == ControlState::Minus {
        decrease_degree.write(DecreaseDegreeEvent);
    } else if *state == ControlState::Plus {
        increase_degree.write(IncreaseDegreeEvent);
    }
}

#[allow(clippy::complexity)]
fn generate_pointcloud(
    surface: Query<Entity, With<Surface>>,
    mut events: EventReader<RedrawEvent>,
    mut commands: Commands,
    entities: Query<Entity, With<ResultSurface>>,
    control_points: Query<(&Transform, &RenderPoint), Without<NonSurfaceMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: ResMut<Assets<Image>>,
    scale_info: Res<RenderInformation>,
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
        let _ = commands.get_entity(p).map(|mut e| e.despawn());
    }

    // Return if there are no actual control points
    if control_points.iter().len() == 0 {
        return;
    }

    if scale_info.surface_mesh_mode == SurfaceMeshMode::Mesh {
        let mut color = Color::from(GRAY_500);
        color.set_alpha(0.3);

        let (mesh, image_handle) = create_mesh_from_control_points(
            &control_points,
            resolution,
            &scale_info.curvature_mode,
            images,
            scale_info.scale as f64,
            SurfaceRenderMode::Surface,
        );

        let mat = StandardMaterial {
            base_color_texture: Some(image_handle),
            double_sided: true,
            cull_mode: None,
            reflectance: 1.0,
            ..Default::default()
        };

        {
            let surface = surface.single().unwrap();
            commands.spawn((
                Transform::default(),
                ResultSurface,
                Name::new("Result Surface"),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(mat)),
                RenderLayers::from(DisplayIn::Normal),
                Visibility::Inherited,
                ChildOf(surface),
            ));
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
    control_points: Query<(&Transform, &RenderPoint), Without<NonSurfaceMarker>>,
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
        let _ = commands.get_entity(p).map(|mut e| e.despawn());
    }

    if control_points.iter().len() == 0 {
        return;
    }

    let w = resolution.0;
    let h = resolution.1;

    let mut meshes_lines = Vec::new();

    for line in scale_info.to_line_uv() {
        let verticies = match line {
            UVEither::U(u) => (0..h)
                .into_par_iter()
                .map(|v| compute_point_by_params(&control_points, u, v as f64 / ((h - 1) as f64)))
                .map(|p| p.into())
                .collect::<Vec<Vec3>>(),
            UVEither::V(v) => (0..w)
                .into_par_iter()
                .map(|u| compute_point_by_params(&control_points, u as f64 / ((w - 1) as f64), v))
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
    control_points: Query<(&Transform, &RenderPoint), Without<NonSurfaceMarker>>,
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
        let _ = commands.get_entity(p).map(|mut e| e.despawn());
    }

    if control_points.iter().len() == 0 {
        return;
    }

    let multi_curves = control_points.to_control_points();
    for (u, v) in scale_info.to_uv_sample() {
        let point = multi_curves.f(&[u, v]);
        let [u_diff, v_diff] = multi_curves.derive(&[u, v], 1);
        let normal = u_diff.cross(&v_diff).normalize() * -1.0;
        let [u_diff_2, v_diff_2] = multi_curves.derive(&[u, v], 2);

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

        let color = materials.add(Color::from(Srgba::new(
            color.0 as f32,
            color.1 as f32,
            color.2 as f32,
            1.0,
        )));
        let v_mesh = meshes.add(Cuboid::new(
            0.05 * scale_info.scale,
            0.05 * scale_info.scale,
            scale_info.box_dim.2 * scale_info.scale + 0.1 * scale_info.scale,
        ));
        let v_color = materials.add(Color::from(Srgba::new(0.0, 0.0, 1.0, 1.0)));
        let u_mesh = meshes.add(Cuboid::new(
            0.05 * scale_info.scale,
            0.05 * scale_info.scale,
            scale_info.box_dim.0 * scale_info.scale + 0.1 * scale_info.scale,
        ));
        let u_color = materials.add(Color::from(Srgba::new(0.0, 1.0, 0.0, 1.0)));
        let n_mesh = meshes.add(Cuboid::new(
            0.05 * scale_info.scale,
            0.05 * scale_info.scale,
            scale_info.box_dim.1 * scale_info.scale + 0.1 * scale_info.scale,
        ));
        let n_color = materials.add(Color::from(Srgba::new(0.0, 1.0, 1.0, 1.0)));

        let mut transform = Transform::from_translation(Vec3::from(point));
        let mesh = match scale_info.coordinate_mode {
            CoordinateMode::XYZ => {
                transform.look_to(Vec3::NEG_Z, Vec3::Y);
                meshes.add(Cuboid::new(
                    scale_info.box_dim.0 * scale_info.scale,
                    scale_info.box_dim.1 * scale_info.scale,
                    scale_info.box_dim.2 * scale_info.scale,
                ))
            }
            CoordinateMode::NUV => {
                let mut mesh = Extrusion::new(
                    Parallelogram2d::new(
                        Vec3::from(u_diff).angle_between(Vec3::from(v_diff)),
                        scale_info.box_dim.2 * scale_info.scale,
                        scale_info.box_dim.0 * scale_info.scale,
                    ),
                    scale_info.box_dim.1 * scale_info.scale,
                )
                .mesh()
                .build();

                let quat = Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians());
                {
                    let positions = mesh
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

                let normals = mesh
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

                transform.align(Vec3::NEG_Z, Vec3::from(u_diff), Vec3::Y, Vec3::from(normal));
                meshes.add(mesh)
            }
            CoordinateMode::NU => {
                transform.align(Vec3::NEG_Z, Vec3::from(v_diff), Vec3::Y, Vec3::from(normal));
                meshes.add(Cuboid::new(
                    scale_info.box_dim.0 * scale_info.scale,
                    scale_info.box_dim.1 * scale_info.scale,
                    scale_info.box_dim.2 * scale_info.scale,
                ))
            }
            CoordinateMode::NV => {
                transform.align(Vec3::X, Vec3::from(u_diff), Vec3::Y, Vec3::from(normal));
                meshes.add(Cuboid::new(
                    scale_info.box_dim.0 * scale_info.scale,
                    scale_info.box_dim.1 * scale_info.scale,
                    scale_info.box_dim.2 * scale_info.scale,
                ))
            }
        };
        commands.spawn((
            ChildOf(surface.single().unwrap()),
            transform,
            Name::new("Box"),
            CurveBox,
            Mesh3d(mesh.clone()),
            MeshMaterial3d(color),
            RenderLayers::from(DisplayIn::Normal),
        ));

        commands.spawn((
            ChildOf(surface.single().unwrap()),
            Transform::from_translation(Vec3::from(point))
                .looking_to(Vec3::from(v_diff).normalize_or_zero(), Vec3::from(normal)),
            Name::new("Box"),
            CurveBox,
            Mesh3d(v_mesh.clone()),
            MeshMaterial3d(v_color.clone()),
            RenderLayers::from(DisplayIn::Normal),
        ));
        commands.spawn((
            ChildOf(surface.single().unwrap()),
            Transform::from_translation(Vec3::from(point))
                .looking_to(Vec3::from(u_diff).normalize_or_zero(), Vec3::from(normal)),
            Name::new("Box"),
            CurveBox,
            Mesh3d(u_mesh.clone()),
            MeshMaterial3d(u_color.clone()),
            RenderLayers::from(DisplayIn::Normal),
        ));
        commands.spawn((
            ChildOf(surface.single().unwrap()),
            Transform::from_translation(Vec3::from(point))
                .looking_to(Vec3::from(normal).normalize_or_zero(), Vec3::from(v_diff)),
            Name::new("Box"),
            CurveBox,
            Mesh3d(n_mesh.clone()),
            MeshMaterial3d(n_color.clone()),
            RenderLayers::from(DisplayIn::Normal),
        ));
    }
}

#[allow(clippy::complexity)]
#[derive(SystemParam)]
pub struct SurfaceCreator<'w, 's> {
    pub root: Query<'w, 's, Entity, With<RootTransform>>,
    pub surface: Query<'w, 's, (Entity, Read<ChildOf>), With<Surface>>,
    pub event_writer: EventWriter<'w, RedrawEvent>,
    pub duplicate_set: ParamSet<
        'w,
        's,
        (
            (
                Commands<'w, 's>,
                ResMut<'w, Assets<StandardMaterial>>,
                ResMut<'w, Assets<Mesh>>,
                EventWriter<'w, AddBoundingEntityEvent>,
                Res<'w, RenderInformation>,
            ),
            BridgeSpawner<'w, 's>,
        ),
    >,
}

impl<'w, 's> SurfaceCreator<'w, 's> {
    /// Create a new surface, replacing the old. Make sure to either use the root transform as a parent, or any provided parent.
    /// If parent is none, use the old parent, if a surface existed before
    pub fn create_surface_from_points(
        &mut self,
        points: Vec<(usize, usize, Vec3)>,
        w: usize,
        h: usize,
        parent: Option<Entity>,
    ) {
        let (surface, ids) = {
            let (mut commands, mut materials, mut meshes, mut add_bounding_entities, info) =
                self.duplicate_set.p0();

            let mut root = if let Some(parent) = parent {
                parent
            } else {
                self.root.single().unwrap()
            };

            if let Ok((surface, old_parent)) = self.surface.single() {
                let _ = commands.get_entity(surface).map(|mut e| e.despawn());
                if parent.is_none() {
                    root = old_parent.parent();
                }
            }

            let surface = commands
                .spawn((Surface, ChildOf(root), Visibility::Inherited))
                .id();

            let scale = info.scale;
            let material = materials.add(Color::from(GRAY_400));
            let material_hover = materials.add(Color::from(GRAY_600));
            let sphere = meshes.add(Sphere::new(0.1 * scale).mesh().ico(5).unwrap());

            let mut ids = vec![];

            for p in &points {
                let id = commands
                    .spawn((
                        RenderPoint(p.0, p.1),
                        Name::new(format!("Render Point {} {}", p.0, p.1)),
                        Transform::from_translation(p.2),
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(material.clone()),
                        Picking3dInteractable::NoDrag,
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        Visibility::Inherited,
                        Snappable,
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
                    .observe(click_on_control_point3d)
                    .observe(click_on_control_point)
                    .id();
                add_bounding_entities.write(AddBoundingEntityEvent(id));
                ids.push(id);
            }
            (surface, ids)
        };

        self.duplicate_set
            .p1()
            .spawn_bridges_2d(surface, w, h, &points, &ids);

        self.event_writer.write(RedrawEvent::HighQuality);
    }

    pub fn get_scale(&mut self) -> f32 {
        self.duplicate_set.p0().4.scale
    }
    pub fn get_height(&mut self) -> f32 {
        self.duplicate_set.p0().4.height
    }
}

#[derive(Event, Clone)]
pub enum ResetDefaultCurveEvent {
    Surface,
    Curves(usize),
    Line {
        start: Vec3,
        end: Vec3,
        points: Vec<Vec3>,
    },
    Point {
        target: Vec3,
        starts: Vec<Vec3>,
    },
}

pub fn generate_default_curve(
    mut reader: EventReader<ResetDefaultCurveEvent>,
    mut surface_creator: SurfaceCreator,
    to_remove: Query<Entity, With<RemoveOnResetMarker>>,
) {
    if reader.is_empty() {
        return;
    }
    let event = reader.read().collect::<Vec<_>>()[0].clone();
    reader.clear();

    for entity in to_remove {
        let _ = surface_creator
            .duplicate_set
            .p0()
            .0
            .get_entity(entity)
            .map(|mut e| e.despawn());
    }

    let root = surface_creator.root.single().expect("must be present");
    let parent = surface_creator
        .duplicate_set
        .p0()
        .0
        .spawn((
            RemoveOnResetMarker,
            ChildOf(root),
            Transform::default(),
            Visibility::Inherited,
        ))
        .id();

    match event {
        ResetDefaultCurveEvent::Surface => {
            let (w, h): (usize, usize) = (2, 2);
            let surface_width = 5.0;
            let surface_height = 5.0;
            let step_x = surface_width / (w as f32 - 1.0);
            let step_y = surface_height / (h as f32 - 1.0);

            let min_x = -surface_width / 2.0;
            let min_y = -surface_height / 2.0;

            let mut points = vec![];
            let mut curr_x = min_x;
            let mut curr_y = min_y;
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    points.push((
                        y as usize,
                        x as usize,
                        Vec3::new(curr_x, 0.0, curr_y) * surface_creator.get_scale()
                            + surface_creator.get_height(),
                    ));
                    curr_x += step_x;
                }
                curr_x = min_x;
                curr_y += step_y;
            }

            surface_creator.create_surface_from_points(points, w, h, Some(parent));
        }
        ResetDefaultCurveEvent::Point { starts, target } => {
            let (mut commands, mut materials, mut meshes, _add_bounding_entity, info) =
                surface_creator.duplicate_set.p0();

            let sphere = meshes.add(Sphere::new(0.1 * info.scale));
            let red = materials.add(Color::from(RED_600));
            let blue = materials.add(Color::from(BLUE_600));

            commands
                .spawn((
                    ChildOf(parent),
                    RenderPoint(0, 0),
                    NonSurfaceMarker,
                    Transform::from_translation(target),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(red),
                ))
                .observe(hover_3d)
                //.observe(drag_point)
                .observe(click_on_control_point3d)
                .observe(click_on_control_point);

            for (idx, point) in starts.iter().enumerate() {
                commands
                    .spawn((
                        ChildOf(parent),
                        RenderPoint(1, idx),
                        Transform::from_translation(*point),
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(blue.clone()),
                    ))
                    .observe(hover_3d)
                    //.observe(drag_point)
                    .observe(click_on_control_point3d)
                    .observe(click_on_control_point);
            }
        }
        ResetDefaultCurveEvent::Line { start, end, points } => {
            let (mut commands, mut materials, mut meshes, _add_bounding_entity, info) =
                surface_creator.duplicate_set.p0();

            let sphere = meshes.add(Sphere::new(0.1 * info.scale));
            let red = materials.add(Color::from(RED_600));
            let blue = materials.add(Color::from(BLUE_600));

            let a = commands
                .spawn((
                    ChildOf(parent),
                    RenderPoint(0, 0),
                    NonSurfaceMarker,
                    Transform::from_translation(start),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(red.clone()),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .observe(hover_3d)
                //.observe(drag_point)
                .observe(click_on_control_point3d)
                .observe(click_on_control_point)
                .id();

            let b = commands
                .spawn((
                    RenderPoint(0, 0),
                    ChildOf(parent),
                    NonSurfaceMarker,
                    Transform::from_translation(end),
                    Mesh3d(sphere.clone()),
                    MeshMaterial3d(red.clone()),
                    RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                ))
                .observe(hover_3d)
                //.observe(drag_point)
                .observe(click_on_control_point3d)
                .observe(click_on_control_point)
                .id();

            let diff = end - start;

            commands.spawn((
                Transform::from_translation(start).looking_at(end, Vec3::Y),
                Bridge { a, b },
                Name::new(format!("Render line {a} {b}")),
                RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                Visibility::Inherited,
                ChildOf(parent),
                related!(
                    Children[draw_bridge_cylinder(
                        &mut meshes,
                        &mut materials,
                        diff.length(),
                        info.scale,
                    )]
                ),
            ));

            for (idx, point) in points.iter().enumerate() {
                commands
                    .spawn((
                        RenderPoint(1, idx),
                        NonSurfaceMarker,
                        Transform::from_translation(*point),
                        Mesh3d(sphere.clone()),
                        MeshMaterial3d(blue.clone()),
                        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
                        ChildOf(parent),
                    ))
                    .observe(hover_3d)
                    //.observe(drag_point)
                    .observe(click_on_control_point3d)
                    .observe(click_on_control_point);
            }
        }
        ResetDefaultCurveEvent::Curves(n) => {
            for i in 0..n {
                let (inner_parent, mut spawned_points) = {
                    let (mut commands, mut materials, mut meshes, _add_bounding_entity, info) =
                        surface_creator.duplicate_set.p0();

                    spawn_new_curve(
                        &mut commands,
                        parent,
                        &mut meshes,
                        &mut materials,
                        &[Vec3::new(i as f32, 0.0, 0.0), Vec3::new(i as f32, 0.0, 1.0)],
                        &info,
                        false,
                    )
                };

                if spawned_points.len() >= 2 {
                    spawned_points.sort_by_key(|v| v.0);

                    let points: Vec<(usize, Vec3)> =
                        spawned_points.iter().map(|v| (v.0, v.2)).collect();
                    let ids: Vec<Entity> = spawned_points.iter().map(|v| v.1).collect();
                    surface_creator.duplicate_set.p1().spawn_bridges_1d(
                        inner_parent,
                        points.len(),
                        &points,
                        &ids,
                    );

                    // Spawn helper spheres
                    let mut points = points;
                    points.sort_by_key(|p| p.0);
                }
            }
        }
    }
}

fn startup(mut writer: EventWriter<ResetDefaultCurveEvent>) {
    writer.write(ResetDefaultCurveEvent::Surface);
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
        app.add_systems(Startup, startup);
        // Requires root transform
        app.add_systems(
            PostStartup,
            setup_surface_inspector.run_if(not(inspectors_exist)),
        );

        // app.add_systems(PreUpdate, (handle_keyboard, solve_constraints));
        app.add_systems(
            PreUpdate,
            (
                handle_keyboard,
                generate_default_curve.run_if(on_event::<ResetDefaultCurveEvent>),
            ),
        );
        app.add_systems(
            Update,
            (
                (generate_pointcloud, distribute_redraw_event).run_if(on_event::<RedrawEvent>),
                update_lines,
                update_surface_inspector
                    .run_if(inspectors_exist.and(on_event::<UpdateSurfaceInspectorEvent>)),
                redraw_boxes.after(generate_pointcloud),
                redraw_iso_lines.after(generate_pointcloud),
                render_curves,
                update_sphere_positions,
            ),
        ); // , listen_to_mouse_left_button));
        app.add_systems(
            PostUpdate,
            (
                // handle_c1_points_events,
                handle_inspector_change_event
                    .run_if(on_event::<SurfaceInspectorChangeset>.and(inspectors_exist)),
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
                handle_minus_mode_event.run_if(in_state(ControlState::Main)),
                handle_plus_mode_event.run_if(in_state(ControlState::Main)),
                handle_align_mode_event.run_if(in_state(ControlState::Main)),
                create_camera_on_click.run_if(in_state(ControlState::CreateOrthoCamera)),
                #[cfg(feature = "vr_enable")]
                create_camera_on_click3d.run_if(in_state(ControlState::CreateOrthoCamera)),
                handle_end_mode.run_if(
                    in_state(ControlState::CreateCurve)
                        .or(in_state(ControlState::CreatePlane))
                        .or(in_state(ControlState::Delete))
                        .or(in_state(ControlState::Minus))
                        .or(in_state(ControlState::Plus))
                        .or(in_state(ControlState::Align))
                        .or(in_state(ControlState::CreateOrthoCamera)),
                ),
            ),
        );

        // Systems for snapping curves creation
        app.add_systems(OnEnter(ControlState::CreateCurve), enter_create_curve_mode);
        app.add_systems(OnExit(ControlState::CreateCurve), commit_curve);

        #[cfg(feature = "vr_enable")]
        {
            use crate::bezier_curve::helper_curves::exit_create_curve_mode_vr;

            app.add_systems(
                OnEnter(ControlState::CreateCurve),
                enter_create_curve_mode_vr,
            );
            app.add_systems(OnExit(ControlState::CreateCurve), exit_create_curve_mode_vr);
        }

        // Align Mode Enter and Exit, and Updates
        app.add_systems(OnEnter(ControlState::Align), create_alignment_sphere);
        app.add_systems(OnExit(ControlState::Align), delete_alignment_sphere);
        app.add_systems(
            Update,
            update_cross_bridge.run_if(in_state(ControlState::Align)),
        );

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
                remove_point_from_curve_handler.run_if(on_event::<RemovePointFromCurveEvent>),
                add_point_to_curve_handler.run_if(on_event::<AddPointToCurveEvent>),
            ),
        );

        app.add_systems(
            PostUpdate,
            (
                handle_degree_increase_event
                    .run_if(on_event::<IncreaseDegreeEvent>)
                    // This must run after the add bounding entity, otherwise the transforms are
                    // not set correctly
                    .after(handle_add_bounding_entity_event),
                handle_degree_reduction_event
                    .run_if(on_event::<DecreaseDegreeEvent>)
                    // This must run after the add bounding entity, otherwise the transforms are
                    // not set correctly
                    .after(handle_add_bounding_entity_event),
            ),
        );

        app.add_systems(
            PostUpdate,
            handle_home_root_transform.run_if(on_event::<HomeRootTransformEvent>),
        );

        app.add_systems(PostUpdate, handle_generic_deleted_event);

        // app.init_resource::<ConstraintState>();
        app.init_resource::<RenderInformation>();
        app.init_resource::<CreateCurveState>();

        app.add_event::<RedrawEvent>();
        // app.add_event::<ToggleC1Enable>();
        app.add_event::<SurfaceInspectorChangeset>();
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
        app.add_event::<MinusModeEvent>();
        app.add_event::<PlusModeEvent>();
        app.add_event::<EntityDeletedEvent>();
        app.add_event::<IncreaseDegreeEvent>();
        app.add_event::<DecreaseDegreeEvent>();
        app.add_event::<ResetDefaultCurveEvent>();
        app.add_event::<RemovePointFromCurveEvent>();
        app.add_event::<AddPointToCurveEvent>();
        app.add_event::<UpdateSurfaceInspectorEvent>();
        app.add_event::<AlignModeEvent>();
        app.add_event::<RecreateAlignmentChildren>();
        app.add_event::<HomeRootTransformEvent>();
        app.init_state::<ControlState>();

        app.add_plugins(EvaluationPlugin);
    }
}
