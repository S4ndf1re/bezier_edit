use crate::bezier_curve::bezier_curve_renderer::{MinusModeEvent, PlusModeEvent, hover_3d};
use crate::bezier_curve::test_mode::{NextEvaluationEvent, handle_next_eval_event};
use crate::picking3d::events::HoveredBy;
use crate::picking3d::picking_3d;
use crate::translation_control::translation_controller::{
    SetPrismMode, handle_set_prism_mode, handle_toggle_snapping,
};
use crate::vr_control::vibrate::{VibrateLeftEvent, VibrateRightEvent, Vibration};
use crate::vr_control::{AimLeft, AimRight};
#[cfg(feature = "vr_enable")]
use crate::vr_control::{GripLeft, GripRight, trigger::ControllerSqueeze};
use crate::{
    MainCamera,
    bezier_curve::{
        bezier_curve_renderer::{
            CreateCurveEvent, CreateOrthoCameraEvent, DeleteModeEvent, EndModeEvent,
        },
        components::ControlState,
        curvature_display_mode::ChangeCurvatureDisplayModeEvent,
        render_info::{ChangeSurfaceMeshMode, RenderInformation},
    },
    picking3d::{self, events::Pointer3d, picking_3d::Picking3dInteractable},
    translation_control::translation_controller::{
        self, ToggleSnappingBehaviour, TranslationControllerState,
    },
};
use bevy::{
    color::palettes::tailwind::RED_500,
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
    scene::SceneInstanceReady,
};
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Debug, Default, States)]
pub enum AssetLoadingState {
    #[default]
    Loading,
    Loaded,
}

#[derive(AssetCollection, Resource)]
struct GltfAssets {
    #[asset(path = "menu/scene.gltf")]
    menu: Handle<Gltf>,
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct TrashcanMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MagnetMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct CurvatureMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct CameraMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct BlocksMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PencilMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct CheckmarkMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MinusMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlusMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct StepMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct PrismMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct EvaluationMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct EndAnyMode;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MM0;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MM1;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MM5;

#[derive(Component, Reflect)]
#[reflect(Component)]
struct MM10;

#[derive(Component)]
pub struct VrMenuRoot;

#[derive(Resource, Default)]
struct VrMenuState {
    currently_selected: Option<Entity>,
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

fn set_initial_color(
    trigger: Trigger<SceneInstanceReady>,
    mut color_changer: ColorChangerChildren,
) {
    color_changer.set_initial_color(trigger.target());
}

fn trigger_scene_spawn(trigger: Trigger<SceneInstanceReady>, world: &mut World) {
    let root = {
        let mut root = world.query::<(&Transform, &VrMenuRoot)>();
        let root = root.query(world);

        let Ok((root, _)) = root.single() else {
            return;
        };

        *root
    };

    // if components.contains(&world.component_id::<MagnetMode>().unwrap())
    //     && translation_state.curve_snapping == SnappingBehaviour::NoSnap
    // {
    //     color_changer.change_color(trigger.target(), WHITE.into());
    // }
    //
    // if components.contains(&world.component_id::<PrismMode>().unwrap())
    //     && translation_state.prism_mode == translation_controller::PrismMode::Prism
    // {
    //     color_changer.change_color(trigger.target(), LIGHT_BLUE.into());
    // }

    let children = {
        let mut children = world.query::<&Children>();
        children
            .query(world)
            .iter_descendants(trigger.target())
            .collect::<Vec<_>>()
    };

    let mut entities_to_hide = Vec::new();

    for child in children {
        world
            .commands()
            .entity(child)
            .insert(Picking3dInteractable::default());

        let Ok(components) = world.inspect_entity(child) else {
            continue;
        };
        let components = components.collect::<Vec<_>>();

        let components = components.iter().map(|info| info.id()).collect::<Vec<_>>();

        // TODO: add observers and pickable to correct components
        if components.contains(&world.component_id::<MagnetMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut toggle_snap_mode: EventWriter<ToggleSnappingBehaviour>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        toggle_snap_mode.write(ToggleSnappingBehaviour);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut toggle_snap_mode: EventWriter<ToggleSnappingBehaviour>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        toggle_snap_mode.write(ToggleSnappingBehaviour);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<TrashcanMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_delete_mode: EventWriter<DeleteModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_delete_mode.write(DeleteModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_delete_mode: EventWriter<DeleteModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_delete_mode.write(DeleteModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<CurvatureMode>().unwrap()) {
            world
                .commands()
                .entity(child)
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
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        change_curvature_display_mode
                            .write(ChangeCurvatureDisplayModeEvent(info.curvature_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut change_curvature_display_mode: EventWriter<
                        ChangeCurvatureDisplayModeEvent,
                    >,
                          info: Res<RenderInformation>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        change_curvature_display_mode
                            .write(ChangeCurvatureDisplayModeEvent(info.curvature_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<CameraMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_create_camera_mode: EventWriter<CreateOrthoCameraEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_create_camera_mode.write(CreateOrthoCameraEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_create_camera_mode: EventWriter<CreateOrthoCameraEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_create_camera_mode.write(CreateOrthoCameraEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<CheckmarkMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_end_mode: EventWriter<EndModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_end_mode.write(EndModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_end_mode: EventWriter<EndModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_end_mode.write(EndModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<PencilMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut set_create_curve_mode: EventWriter<CreateCurveEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_create_curve_mode.write(CreateCurveEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut set_create_curve_mode: EventWriter<CreateCurveEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        set_create_curve_mode.write(CreateCurveEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<BlocksMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut mesh_mode_writer: EventWriter<ChangeSurfaceMeshMode>,
                          info: Res<RenderInformation>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        mesh_mode_writer
                            .write(ChangeSurfaceMeshMode(info.surface_mesh_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut mesh_mode_writer: EventWriter<ChangeSurfaceMeshMode>,
                          info: Res<RenderInformation>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        mesh_mode_writer
                            .write(ChangeSurfaceMeshMode(info.surface_mesh_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<MinusMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut minus_mode: EventWriter<MinusModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        minus_mode.write(MinusModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut minus_mode: EventWriter<MinusModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        minus_mode.write(MinusModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<PlusMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut plus_mode: EventWriter<PlusModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        plus_mode.write(PlusModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut plus_mode: EventWriter<PlusModeEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        plus_mode.write(PlusModeEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<PrismMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut writer: EventWriter<SetPrismMode>,
                          translation_state: Res<TranslationControllerState>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        writer.write(SetPrismMode(translation_state.prism_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut writer: EventWriter<SetPrismMode>,
                          translation_state: Res<TranslationControllerState>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        writer.write(SetPrismMode(translation_state.prism_mode.next()));
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<EvaluationMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut eval_writer: EventWriter<NextEvaluationEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        eval_writer.write(NextEvaluationEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut eval_writer: EventWriter<NextEvaluationEvent>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        eval_writer.write(NextEvaluationEvent);
                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );
        }

        if components.contains(&world.component_id::<StepMode>().unwrap()) {
            world
                .commands()
                .entity(child)
                .observe(handle_hover_out)
                .observe(handle_hover_out3d)
                .observe(handle_hover_over)
                .observe(handle_hover_over3d)
                .observe(hover_3d)
                .observe(
                    move |_: Trigger<Pointer3d<picking3d::events::Click>>,
                          mut translation_state: ResMut<TranslationControllerState>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        translation_state.step_mode = translation_state.step_mode.next();

                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                )
                .observe(
                    move |_: Trigger<Pointer<Click>>,
                          mut translation_state: ResMut<TranslationControllerState>,
                          mut respawn_menu: EventWriter<RedrawMenuEvent>| {
                        translation_state.step_mode = translation_state.step_mode.next();

                        respawn_menu.write(RedrawMenuEvent(root));
                    },
                );

            if world.resource::<TranslationControllerState>().step_mode
                == translation_controller::StepMode::None
                && !components.contains(&world.component_id::<MM0>().unwrap())
                || world.resource::<TranslationControllerState>().step_mode
                    == translation_controller::StepMode::MM1
                    && !components.contains(&world.component_id::<MM1>().unwrap())
                || world.resource::<TranslationControllerState>().step_mode
                    == translation_controller::StepMode::MM5
                    && !components.contains(&world.component_id::<MM5>().unwrap())
                || world.resource::<TranslationControllerState>().step_mode
                    == translation_controller::StepMode::MM10
                    && !components.contains(&world.component_id::<MM10>().unwrap())
            {
                entities_to_hide.push(child);
            }
        }
    }

    for entity in entities_to_hide {
        world
            .commands()
            .entity(entity)
            .insert((Visibility::Hidden, Picking3dInteractable::Ignore));
    }
}

#[derive(Event)]
struct RedrawMenuEvent(Transform);

fn handle_redraw_menu_event(mut menu: MenuHandler, mut reader: EventReader<RedrawMenuEvent>) {
    for event in reader.read() {
        menu.spawn_at_position_and_orientation(event.0);
    }
}

#[derive(SystemParam)]
pub struct MenuHandler<'w, 's> {
    commands: Commands<'w, 's>,
    menu_roots: Query<'w, 's, Entity, With<VrMenuRoot>>,
    state: ResMut<'w, VrMenuState>,
    info: Res<'w, RenderInformation>,
    models: Res<'w, GltfAssets>,
    gltf: Res<'w, Assets<Gltf>>,
}

impl<'w, 's> MenuHandler<'w, 's> {
    pub fn despawn_menu(&mut self) {
        for root in self.menu_roots.iter() {
            let _ = self.commands.get_entity(root).map(|mut e| e.despawn());
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
        let scale = 1.0;
        #[cfg(not(feature = "vr_enable"))]
        let scale = 5.0;

        self.commands.spawn((
            Transform::default().with_scale(Vec3::ONE * self.info.scale / scale),
            SceneRoot(self.gltf.get(self.models.menu.clone().id()).unwrap().scenes[0].clone()),
            Visibility::Inherited,
            ChildOf(root),
        ));
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
        manager.despawn_menu();
    }
}

pub struct VrMenuPlugin;

impl Plugin for VrMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            spawn_despawn_model_into_scene.run_if(in_state(AssetLoadingState::Loaded)),
        );

        app.add_systems(
            Last,
            handle_redraw_menu_event.run_if(in_state(AssetLoadingState::Loaded)),
        );

        app.init_resource::<VrMenuState>();
        app.add_observer(trigger_scene_spawn);
        app.add_observer(set_initial_color);

        app.add_event::<RedrawMenuEvent>();

        app.init_state::<AssetLoadingState>();
        app.add_loading_state(
            LoadingState::new(AssetLoadingState::Loading)
                .continue_to_state(AssetLoadingState::Loaded)
                .load_collection::<GltfAssets>(),
        );

        app.register_type::<TrashcanMode>();
        app.register_type::<MagnetMode>();
        app.register_type::<CurvatureMode>();
        app.register_type::<CameraMode>();
        app.register_type::<BlocksMode>();
        app.register_type::<PencilMode>();
        app.register_type::<CheckmarkMode>();
        app.register_type::<MinusMode>();
        app.register_type::<PlusMode>();
        app.register_type::<StepMode>();
        app.register_type::<PrismMode>();
        app.register_type::<EvaluationMode>();
        app.register_type::<EndAnyMode>();
        app.register_type::<MM0>();
        app.register_type::<MM1>();
        app.register_type::<MM5>();
        app.register_type::<MM10>();
    }
}
