use crate::bezier_curve::bezier_curve_renderer::hover_3d;
use crate::bezier_curve::test_mode::NextEvaluationEvent;
use crate::picking3d::events::HoveredBy;
use crate::picking3d::picking_3d;
use crate::translation_control::translation_controller::SetPrismMode;
use crate::vr_control::trigger::ControllerTrigger;
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use crate::vr_control::{AimLeft, AimRight};
#[cfg(feature = "vr_enable")]
use crate::vr_control::{
    GripLeft, GripRight,
    trigger::{ControllerSqueeze, ControllerTrigger},
};
use crate::{
    MainCamera,
    bezier_curve::{
        bezier_curve_renderer::{
            CreateCurveEvent, CreateOrthoCameraEvent, DeleteModeEvent, EndModeEvent,
        },
        components::ControlState,
        curvature_display_mode::ChangeCurvatureDisplayModeEvent,
        degree_manipulation::{DecreaseDegreeEvent, IncreaseDegreeEvent},
        render_info::{ChangeSurfaceMeshMode, RenderInformation},
    },
    picking3d::{self, events::Pointer3d, picking_3d::Picking3dInteractable},
    translation_control::translation_controller::{
        self, SnappingBehaviour, ToggleSnappingBehaviour, TranslationControllerState,
    },
};
use bevy::color::palettes::css::{BLACK, BLUE, LIGHT_BLUE};
use bevy::{
    color::palettes::{css::WHITE, tailwind::RED_500},
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
    scene::SceneInstanceReady,
};

// TODO: add prisma as optionality
#[derive(Resource, Default)]
struct GltfAssets {
    magnet: Option<Handle<Gltf>>,
    trashcan: Option<Handle<Gltf>>,
    curve: Option<Handle<Gltf>>,
    camera: Option<Handle<Gltf>>,
    blocks: Option<Handle<Gltf>>,
    pencil: Option<Handle<Gltf>>,
    checkmark: Option<Handle<Gltf>>,
    minus: Option<Handle<Gltf>>,
    plus: Option<Handle<Gltf>>,
    mm_0: Option<Handle<Gltf>>,
    mm_1: Option<Handle<Gltf>>,
    mm_5: Option<Handle<Gltf>>,
    mm_10: Option<Handle<Gltf>>,
    prism: Option<Handle<Gltf>>,
}

#[derive(Component)]
struct TrashcanMode;

#[derive(Component)]
struct MagnetMode;

#[derive(Component)]
struct CurvatureMode;

#[derive(Component)]
struct CameraMode;

#[derive(Component)]
struct BlocksMode;

#[derive(Component)]
struct PencilMode;

#[derive(Component)]
struct CheckmarkMode;

#[derive(Component)]
struct MinusMode;

#[derive(Component)]
struct PlusMode;

#[derive(Component, Clone, Copy)]
struct StepMode;

#[derive(Component)]
struct PrismMode;

#[derive(Component)]
struct EvaluationMode;

#[derive(Component)]
pub struct VrMenuRoot;

#[derive(Resource, Default)]
struct VrMenuState {
    currently_selected: Option<Entity>,
}

fn spawn_magnet(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.05 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        MagnetMode,
    )
}

fn spawn_trash(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.3 * scale)
            .with_rotation(Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        TrashcanMode,
    )
}

fn spawn_curveature(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.05 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        CurvatureMode,
    )
}

fn spawn_camera(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.2 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        CameraMode,
    )
}

fn spawn_blocks(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Y, -45.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.03 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        BlocksMode,
    )
}

fn spawn_pencil(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.02 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        PencilMode,
    )
}

fn spawn_checkmark(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.01 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        CheckmarkMode,
    )
}

fn spawn_minus(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.01 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        MinusMode,
    )
}

fn spawn_plus(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Z, 90.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.01 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        StepMode,
    )
}

fn spawn_text(
    scale: f32,
    step_mode: &translation_controller::StepMode,
    gltf: &Res<Assets<Gltf>>,
    models: &Res<GltfAssets>,
) -> impl Bundle {
    let model = match *step_mode {
        translation_controller::StepMode::None => gltf
            .get(
                models
                    .mm_0
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap(),
        translation_controller::StepMode::MM1 => gltf
            .get(
                models
                    .mm_1
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap(),
        translation_controller::StepMode::MM5 => gltf
            .get(
                models
                    .mm_5
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap(),
        translation_controller::StepMode::MM10 => gltf
            .get(
                models
                    .mm_10
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap(),
    };
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::X, 180.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Y, 180.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Z, 10.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.1 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(model.scenes[0].clone()),
        Picking3dInteractable::default(),
        PlusMode,
    )
}

fn spawn_prism(scene: Handle<Scene>, scale: f32) -> impl Bundle {
    let rotation = Quat::from_axis_angle(Vec3::X, -90.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::X, 180.0_f32.to_radians())
        * Quat::from_axis_angle(Vec3::Y, 180.0_f32.to_radians());

    (
        Transform::from_xyz(0.0, 0.0, -0.2 * scale)
            .with_scale(Vec3::ONE * 0.0001 * scale)
            .with_rotation(rotation),
        Visibility::Inherited,
        SceneRoot(scene),
        Picking3dInteractable::default(),
        PrismMode,
    )
}

#[derive(Component)]
pub struct OriginalMaterial(StandardMaterial);

#[derive(SystemParam)]
struct ColorChangerChildren<'w, 's> {
    commands: Commands<'w, 's>,
    children: Query<'w, 's, Read<Children>>,
    materials: Query<'w, 's, Read<MeshMaterial3d<StandardMaterial>>>,
    materials_assets: ResMut<'w, Assets<StandardMaterial>>,
    originals: Query<'w, 's, Read<OriginalMaterial>>,
}

impl<'w, 's> ColorChangerChildren<'w, 's> {
    fn set_initial_color(&mut self, parent: Entity) {
        for child in self.children.iter_descendants(parent) {
            if let Ok(mat_handle) = self.materials.get(child) {
                let original =
                    if let Some(material) = self.materials_assets.get_mut(mat_handle.id()) {
                        material.clone()
                    } else {
                        StandardMaterial::default()
                    };

                self.commands
                    .entity(child)
                    .insert((OriginalMaterial(original),));
            }
        }
    }
    fn change_color(&mut self, parent: Entity, color: Color) {
        for child in self.children.iter_descendants(parent) {
            if let Ok(mat_handle) = self.materials.get(child) {
                let original =
                    if let Some(material) = self.materials_assets.get_mut(mat_handle.id()) {
                        material.clone()
                    } else {
                        StandardMaterial::default()
                    };

                let mut duplicate = original.clone();
                duplicate.base_color = color;

                self.commands
                    .entity(child)
                    .insert((MeshMaterial3d(self.materials_assets.add(duplicate)),));
            }
        }
    }

    fn change_color_back(&mut self, parent: Entity) {
        for child in self.children.iter_descendants(parent) {
            if let Ok(original) = self.originals.get(child) {
                self.commands.entity(child).insert((MeshMaterial3d(
                    self.materials_assets.add(original.0.clone()),
                ),));
            }
        }
    }
}

fn handle_hover_over(
    trigger: Trigger<Pointer<Over>>,
    mut state: ResMut<VrMenuState>,
    mut color_changer: ColorChangerChildren,
) {
    state.currently_selected = Some(trigger.target());

    color_changer.change_color(trigger.target(), RED_500.into());
}

fn handle_hover_over3d(
    trigger: Trigger<Pointer3d<picking3d::events::MoveIn>>,
    mut state: ResMut<VrMenuState>,
    mut color_changer: ColorChangerChildren,
) {
    state.currently_selected = Some(trigger.target());

    color_changer.change_color(trigger.target(), RED_500.into());
}

fn handle_hover_out(
    trigger: Trigger<Pointer<Out>>,
    mut state: ResMut<VrMenuState>,
    mut color_changer: ColorChangerChildren,
) {
    state.currently_selected = None;

    color_changer.change_color_back(trigger.target());
}

fn handle_hover_out3d(
    trigger: Trigger<Pointer3d<picking3d::events::MoveOut>>,
    mut state: ResMut<VrMenuState>,
    mut color_changer: ColorChangerChildren,
) {
    state.currently_selected = None;

    color_changer.change_color_back(trigger.target());
}

fn trigger_scene_spawn(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    mut color_changer: ColorChangerChildren,
    translation_state: Res<TranslationControllerState>,
    magnets: Query<&MagnetMode>,
    prisms: Query<&PrismMode>,
    children: Query<&Children>,
) {
    color_changer.set_initial_color(trigger.target());

    if magnets.get(trigger.target()).is_ok()
        && translation_state.curve_snapping == SnappingBehaviour::NoSnap
    {
        color_changer.change_color(trigger.target(), WHITE.into());
    }

    if prisms.get(trigger.target()).is_ok()
        && translation_state.prism_mode == translation_controller::PrismMode::Prism
    {
        color_changer.change_color(trigger.target(), LIGHT_BLUE.into());
    }

    for child in children.iter_descendants(trigger.target()) {
        commands
            .entity(child)
            .insert(Picking3dInteractable::default());
    }
}

#[derive(SystemParam)]
pub struct MenuHandler<'w, 's> {
    commands: Commands<'w, 's>,
    menu_roots: Query<'w, 's, Entity, With<VrMenuRoot>>,
    state: ResMut<'w, VrMenuState>,
    info: Res<'w, RenderInformation>,
    control_state: Res<'w, State<ControlState>>,
    translation_state: ResMut<'w, TranslationControllerState>,
    models: Res<'w, GltfAssets>,
    gltf: Res<'w, Assets<Gltf>>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

impl<'w, 's> MenuHandler<'w, 's> {
    pub fn despawn_menu(&mut self) {
        for root in self.menu_roots.iter() {
            self.commands.entity(root).despawn();
        }
        self.state.currently_selected = None;
    }

    /// Spawn the ui using the transforms viewing orientation and position, to make the ui visible
    pub fn spawn_at_position_and_orientation(&mut self, transform: Transform) {
        self.despawn_menu();

        let top_level_transform = transform;
        let root = self
            .commands
            .spawn((VrMenuRoot, top_level_transform, Visibility::Inherited))
            .id();

        #[cfg(feature = "vr_enable")]
        let scale = 3.0;
        #[cfg(not(feature = "vr_enable"))]
        let scale = 1.0;

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 15.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .magnet
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                MagnetMode,
                children![spawn_magnet(
                    model.scenes[0].clone(),
                    self.info.scale * scale
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut toggle_snap_mode: EventWriter<ToggleSnappingBehaviour>,
                      mut menu: MenuHandler| {
                    toggle_snap_mode.write(ToggleSnappingBehaviour);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut toggle_snap_mode: EventWriter<ToggleSnappingBehaviour>,
                      mut menu: MenuHandler| {
                    toggle_snap_mode.write(ToggleSnappingBehaviour);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, -15.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .trashcan
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                TrashcanMode,
                children![spawn_trash(
                    model.scenes[0].clone(),
                    self.info.scale * scale
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut set_delete_mode: EventWriter<DeleteModeEvent>,
                      mut menu: MenuHandler| {
                    set_delete_mode.write(DeleteModeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut set_delete_mode: EventWriter<DeleteModeEvent>,
                      mut menu: MenuHandler| {
                    set_delete_mode.write(DeleteModeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 45.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .curve
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                CurvatureMode,
                children![spawn_curveature(
                    model.scenes[0].clone(),
                    self.info.scale * scale
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut change_curvature_display_mode: EventWriter<
                    ChangeCurvatureDisplayModeEvent,
                >,
                      info: Res<RenderInformation>,
                      mut menu: MenuHandler| {
                    change_curvature_display_mode
                        .write(ChangeCurvatureDisplayModeEvent(info.curvature_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut change_curvature_display_mode: EventWriter<
                    ChangeCurvatureDisplayModeEvent,
                >,
                      info: Res<RenderInformation>,
                      mut menu: MenuHandler| {
                    change_curvature_display_mode
                        .write(ChangeCurvatureDisplayModeEvent(info.curvature_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, -45.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .camera
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                CameraMode,
                children![spawn_camera(
                    model.scenes[0].clone(),
                    self.info.scale * scale
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut set_create_camera_mode: EventWriter<CreateOrthoCameraEvent>,
                      mut menu: MenuHandler| {
                    set_create_camera_mode.write(CreateOrthoCameraEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut set_create_camera_mode: EventWriter<CreateOrthoCameraEvent>,
                      mut menu: MenuHandler| {
                    set_create_camera_mode.write(CreateOrthoCameraEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        if *self.control_state == ControlState::CreateCurve {
            let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
            transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 75.0_f32.to_radians()));
            let model = self
                .gltf
                .get(
                    self.models
                        .checkmark
                        .clone()
                        .expect("must be loaded to run this system")
                        .id(),
                )
                .unwrap();

            self.commands
                .spawn((
                    transform,
                    CheckmarkMode,
                    children![spawn_checkmark(
                        model.scenes[0].clone(),
                        self.info.scale * scale,
                    )],
                    Visibility::Inherited,
                    ChildOf(root),
                ))
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_end_mode: EventWriter<EndModeEvent>,
                          mut menu: MenuHandler| {
                        set_end_mode.write(EndModeEvent);
                        menu.spawn_at_position_and_orientation(top_level_transform);
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_end_mode: EventWriter<EndModeEvent>,
                          mut menu: MenuHandler| {
                        set_end_mode.write(EndModeEvent);
                        menu.spawn_at_position_and_orientation(top_level_transform);
                    },
                );
        } else {
            let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
            transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 75.0_f32.to_radians()));
            let model = self
                .gltf
                .get(
                    self.models
                        .pencil
                        .clone()
                        .expect("must be loaded to run this system")
                        .id(),
                )
                .unwrap();

            self.commands
                .spawn((
                    transform,
                    PencilMode,
                    children![spawn_pencil(
                        model.scenes[0].clone(),
                        self.info.scale * scale,
                    )],
                    Visibility::Inherited,
                    ChildOf(root),
                ))
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_create_curve_mode: EventWriter<CreateCurveEvent>,
                          mut menu: MenuHandler| {
                        set_create_curve_mode.write(CreateCurveEvent);
                        menu.spawn_at_position_and_orientation(top_level_transform);
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_create_curve_mode: EventWriter<CreateCurveEvent>,
                          mut menu: MenuHandler| {
                        set_create_curve_mode.write(CreateCurveEvent);
                        menu.spawn_at_position_and_orientation(top_level_transform);
                    },
                );
        }

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, -75.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .blocks
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                BlocksMode,
                children![spawn_blocks(
                    model.scenes[0].clone(),
                    self.info.scale * scale,
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut mesh_mode_writer: EventWriter<ChangeSurfaceMeshMode>,
                      info: Res<RenderInformation>,
                      mut menu: MenuHandler| {
                    mesh_mode_writer.write(ChangeSurfaceMeshMode(info.surface_mesh_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut mesh_mode_writer: EventWriter<ChangeSurfaceMeshMode>,
                      info: Res<RenderInformation>,
                      mut menu: MenuHandler| {
                    mesh_mode_writer.write(ChangeSurfaceMeshMode(info.surface_mesh_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 105.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .minus
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                MinusMode,
                children![spawn_minus(
                    model.scenes[0].clone(),
                    self.info.scale * scale,
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut decrease_degree: EventWriter<DecreaseDegreeEvent>,
                      mut menu: MenuHandler| {
                    decrease_degree.write(DecreaseDegreeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut decrease_degree: EventWriter<DecreaseDegreeEvent>,
                      mut menu: MenuHandler| {
                    decrease_degree.write(DecreaseDegreeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, -105.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .plus
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                PlusMode,
                children![spawn_plus(model.scenes[0].clone(), self.info.scale * scale,)],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut increase_degree: EventWriter<IncreaseDegreeEvent>,
                      mut menu: MenuHandler| {
                    increase_degree.write(IncreaseDegreeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut increase_degree: EventWriter<IncreaseDegreeEvent>,
                      mut menu: MenuHandler| {
                    increase_degree.write(IncreaseDegreeEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, 155.0_f32.to_radians()));

        self.commands
            .spawn((
                transform,
                StepMode,
                children![spawn_text(
                    self.info.scale * scale,
                    &self.translation_state.step_mode,
                    &self.gltf,
                    &self.models,
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>, mut menu: MenuHandler| {
                    menu.translation_state.step_mode = menu.translation_state.step_mode.next();

                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(move |_: Trigger<Pointer<Click>>, mut menu: MenuHandler| {
                menu.translation_state.step_mode = menu.translation_state.step_mode.next();

                menu.spawn_at_position_and_orientation(top_level_transform);
            });

        let mut transform = Transform::default().looking_to(Vec3::Y, Vec3::NEG_Z);
        transform.rotate(Quat::from_axis_angle(Vec3::NEG_Z, -135.0_f32.to_radians()));
        let model = self
            .gltf
            .get(
                self.models
                    .prism
                    .clone()
                    .expect("must be loaded to run this system")
                    .id(),
            )
            .unwrap();

        self.commands
            .spawn((
                transform,
                PrismMode,
                children![spawn_prism(
                    model.scenes[0].clone(),
                    self.info.scale * scale,
                )],
                Visibility::Inherited,
                ChildOf(root),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut writer: EventWriter<SetPrismMode>,
                      mut menu: MenuHandler| {
                    writer.write(SetPrismMode(menu.translation_state.prism_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut writer: EventWriter<SetPrismMode>,
                      mut menu: MenuHandler| {
                    writer.write(SetPrismMode(menu.translation_state.prism_mode.next()));
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );

        // Spawn center to start evaluation
        self.commands
            .spawn((
                Transform::from_xyz(0.0, -0.08 * scale, 0.0),
                EvaluationMode,
                Visibility::Inherited,
                ChildOf(root),
                Mesh3d(self.meshes.add(Sphere::new(0.05 * self.info.scale))),
                MeshMaterial3d(
                    self.materials
                        .add(StandardMaterial::from_color(Color::from(BLACK))),
                ),
                Picking3dInteractable::default(),
            ))
            .observe(handle_hover_out)
            .observe(handle_hover_out3d)
            .observe(handle_hover_over)
            .observe(handle_hover_over3d)
            .observe(hover_3d)
            .observe(
                move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                      mut eval_writer: EventWriter<NextEvaluationEvent>,
                      mut menu: MenuHandler| {
                    eval_writer.write(NextEvaluationEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            )
            .observe(
                move |_: Trigger<Pointer<Click>>,
                      mut eval_writer: EventWriter<NextEvaluationEvent>,
                      mut menu: MenuHandler| {
                    eval_writer.write(NextEvaluationEvent);
                    menu.spawn_at_position_and_orientation(top_level_transform);
                },
            );
    }
}

#[cfg(not(feature = "vr_enable"))]
fn spawn_despawn_model_into_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut manager: MenuHandler,
    camera: Query<(&GlobalTransform, &Camera), With<MainCamera>>,
    window: Query<&Window>,
) {
    if let Ok(window) = window.single()
        && let Ok((camera_transform, camera)) = camera.single()
        && let Some(cursor_pos) = window.cursor_position()
        && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)
    {
        let translation = ray.get_point(1.0);
        let transform = Transform::from_translation(translation)
            .looking_to(-ray.direction, camera_transform.up());
        // .with_rotation(Quat::from_axis_angle(Vec3::X, 90.0_f32.to_radians()));

        if keyboard.just_pressed(KeyCode::KeyM) {
            manager.spawn_at_position_and_orientation(transform);
        } else if keyboard.just_released(KeyCode::KeyM) {
            manager.despawn_menu();
        }
    }
}

#[cfg(feature = "vr_enable")]
fn spawn_despawn_model_into_scene(
    left_tracked: Query<&GlobalTransform, With<AimLeft>>,
    right_tracked: Query<&GlobalTransform, With<AimRight>>,
    squeeze: Res<ControllerSqueeze>,
    is_menu_existent: Query<Entity, With<VrMenuRoot>>,
    mut manager: MenuHandler,
) {
    let mut spawn_menu = false;
    let mut transform = Transform::default();

    if squeeze.left > 0.2
        && let Ok(transform_left) = left_tracked.single()
    {
        spawn_menu = true;
        transform = transform_left.compute_transform();
    }

    if squeeze.right > 0.2
        && let Ok(transform_right) = right_tracked.single()
    {
        spawn_menu = true;
        transform = transform_right.compute_transform();
    }

    let vr_menu_exists = is_menu_existent.single().is_ok();

    if spawn_menu && !vr_menu_exists {
        let translation = transform.translation;
        let transform = Transform::from_translation(translation)
            .looking_to(-transform.forward(), transform.up());
        manager.spawn_at_position_and_orientation(transform);
    } else if !spawn_menu && vr_menu_exists {
        manager.apply_menu_state();
        manager.despawn_menu();
    }
}

fn setup_models(server: ResMut<AssetServer>, mut models: ResMut<GltfAssets>) {
    let magnet: Handle<Gltf> = server.load("magnet/scene.gltf");
    let trashcan: Handle<Gltf> = server.load("garbage_can__trashcan__bin/scene.gltf");
    let curve: Handle<Gltf> = server.load("saddle_wires/scene.gltf");
    let camera: Handle<Gltf> = server.load("polaroid_camera/scene.gltf");
    let blocks: Handle<Gltf> = server.load("box_mode/scene.gltf");
    let pencil: Handle<Gltf> = server.load("pencil/scene.gltf");
    let checkmark: Handle<Gltf> = server.load("checkmark/scene.gltf");
    let minus: Handle<Gltf> = server.load("minus/scene.gltf");
    let plus: Handle<Gltf> = server.load("plus/scene.gltf");
    let mm_0: Handle<Gltf> = server.load("0mm/scene.gltf");
    let mm_1: Handle<Gltf> = server.load("1mm/scene.gltf");
    let mm_5: Handle<Gltf> = server.load("5mm/scene.gltf");
    let mm_10: Handle<Gltf> = server.load("10mm/scene.gltf");
    let prism: Handle<Gltf> = server.load("diamond/scene.gltf");

    models.magnet = Some(magnet);
    models.trashcan = Some(trashcan);
    models.curve = Some(curve);
    models.camera = Some(camera);
    models.blocks = Some(blocks);
    models.pencil = Some(pencil);
    models.checkmark = Some(checkmark);
    models.minus = Some(minus);
    models.plus = Some(plus);

    models.mm_0 = Some(mm_0);
    models.mm_1 = Some(mm_1);
    models.mm_5 = Some(mm_5);
    models.mm_10 = Some(mm_10);

    models.prism = Some(prism);
}

fn are_models_loaded(
    models: Res<GltfAssets>,
    assets: Res<Assets<Gltf>>,
    mut loaded: Local<bool>,
) -> bool {
    if *loaded {
        return *loaded;
    }

    if let Some(magnet) = &models.magnet
        && assets.get(magnet.id()).is_none()
    {
        return false;
    }

    if let Some(trashcan) = &models.trashcan
        && assets.get(trashcan.id()).is_none()
    {
        return false;
    }

    if let Some(curve) = &models.curve
        && assets.get(curve.id()).is_none()
    {
        return false;
    }

    if let Some(camera) = &models.camera
        && assets.get(camera.id()).is_none()
    {
        return false;
    }

    if let Some(blocks) = &models.blocks
        && assets.get(blocks.id()).is_none()
    {
        return false;
    }

    if let Some(pencil) = &models.pencil
        && assets.get(pencil.id()).is_none()
    {
        return false;
    }

    if let Some(checkmark) = &models.checkmark
        && assets.get(checkmark.id()).is_none()
    {
        return false;
    }

    if let Some(minus) = &models.minus
        && assets.get(minus.id()).is_none()
    {
        return false;
    }

    if let Some(plus) = &models.plus
        && assets.get(plus.id()).is_none()
    {
        return false;
    }

    if let Some(mm_0) = &models.mm_0
        && assets.get(mm_0.id()).is_none()
    {
        return false;
    }

    if let Some(mm_5) = &models.mm_5
        && assets.get(mm_5.id()).is_none()
    {
        return false;
    }

    if let Some(mm_10) = &models.mm_10
        && assets.get(mm_10.id()).is_none()
    {
        return false;
    }

    if let Some(prism) = &models.prism
        && assets.get(prism.id()).is_none()
    {
        return false;
    }

    *loaded = true;
    *loaded
}

pub struct VrMenuPlugin;

impl Plugin for VrMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_models);
        app.add_systems(
            PostUpdate,
            spawn_despawn_model_into_scene.run_if(are_models_loaded),
        );
        app.init_resource::<VrMenuState>();
        app.init_resource::<GltfAssets>();
        app.add_observer(trigger_scene_spawn);
    }
}
