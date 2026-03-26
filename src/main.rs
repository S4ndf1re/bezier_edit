mod advanced_orbit_controls;
mod bezier_curve;
pub mod custom_shapes;
mod history;
pub mod linked_entities;
mod nurbs;
pub mod picking3d;
pub mod projection;
pub mod solver;
mod thirdparty_copy;
mod translation_control;
mod ui;
pub mod util;
pub mod vr_control;
pub mod vr_menu;

use crate::advanced_orbit_controls::AdvancedOrbitControls;

#[cfg(feature = "vr_enable")]
use crate::thirdparty_copy::transform_util_copy::{SnapToPosition, SnapToRotation};

use crate::translation_control::control_storage::ControlStorage;

#[cfg(feature = "vr_enable")]
use bevy::app::PluginGroupBuilder;

use bevy::prelude::*;
use bevy_skein::SkeinPlugin;
use bezier_curve::bezier_curve_renderer::*;
use history::plugin::HistoryPlugin;

#[cfg(feature = "vr_enable")]
use crate::bezier_curve::render_info::RenderInformation;

use bevy::render::view::RenderLayers;
use bevy_lunex::prelude::LoadFonts;
use projection::DisplayIn;
use translation_control::translation_controller::TranslationController;
use ui::UiPlugin;

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct RootTransform;

#[derive(Component)]
#[require(Camera)]
pub struct MainCamera;

#[cfg(not(feature = "vr_enable"))]
pub fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-10.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        RenderLayers::from(DisplayIn::Normal),
        MainCamera,
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        RenderLayers::from(DisplayIn::BothNormalAndOrtho),
    ));

    commands.spawn((RootTransform, Name::new("Root Transform")));
}

#[cfg(feature = "vr_enable")]
pub fn setup(
    mut commands: Commands,
    mut rotation_writer: EventWriter<SnapToRotation>,
    mut position_writer: EventWriter<SnapToPosition>,
    mut scale: ResMut<RenderInformation>,
) {
    scale.scale = 0.3;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        RenderLayers::from(DisplayIn::Normal),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    let position =
        Transform::from_xyz(-0.7, -1.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
    position_writer.write(SnapToPosition(position.translation));
    rotation_writer.write(SnapToRotation(position.rotation));

    commands.spawn(RootTransform);
}

#[cfg(not(feature = "vr_enable"))]
fn create_app() -> App {
    use bevy::color::palettes::tailwind::GRAY_700;
    use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
    use linked_entities::LinkedEntitiesPlugin;
    use projection::ProjectionPlugin;
    use vr_control::VrControlPlugin;
    use vr_menu::VrMenuPlugin;

    info!("Creating Non-VR App");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(HistoryPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(BezierRenderPlugin)
        .add_plugins(AdvancedOrbitControls)
        .add_plugins(TranslationController)
        .add_plugins(UiPlugin)
        .add_plugins(SkeinPlugin::default())
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(ProjectionPlugin)
        .add_plugins(LinkedEntitiesPlugin)
        .add_plugins(VrMenuPlugin)
        .add_plugins(VrControlPlugin)
        .init_resource::<ControlStorage>()
        .insert_resource(ClearColor(GRAY_700.into()))
        .insert_resource(LoadFonts {
            font_directories: vec!["assets/fonts".to_owned()],
            ..default()
        })
        .add_systems(Startup, setup.before(generate_default_curve));

    app
}

#[cfg(feature = "vr_enable")]
fn custom_add_xr_plugins<G: PluginGroup>(plugins: G) -> PluginGroupBuilder {
    use bevy::render::RenderPlugin;
    use bevy::window::PresentMode;
    use bevy_mod_openxr::features::handtracking::HandTrackingPlugin;
    use bevy_mod_openxr::features::passthrough::OxrPassthroughPlugin;
    use bevy_mod_openxr::init::OxrInitPlugin;
    use bevy_mod_openxr::poll_events::OxrEventsPlugin;
    use bevy_mod_openxr::reference_space::OxrReferenceSpacePlugin;
    use bevy_mod_openxr::render::OxrRenderPlugin;
    use bevy_mod_openxr::{
        action_binding, action_set_attaching, action_set_syncing, features, spaces,
    };
    use bevy_mod_xr::camera::XrCameraPlugin;
    use bevy_mod_xr::session::XrSessionPlugin;

    #[cfg(feature = "varjo_ready")]
    let mut oxr_plugin = OxrInitPlugin::default();

    #[cfg(not(feature = "varjo_ready"))]
    let oxr_plugin = OxrInitPlugin::default();

    #[cfg(feature = "varjo_ready")]
    {
        oxr_plugin.exts.varjo_xr4_controller_interaction = true;
        oxr_plugin.exts.varjo_quad_views = true;
    }

    plugins
        .build()
        .disable::<RenderPlugin>()
        // .disable::<PipelinedRenderingPlugin>()
        .add_before::<RenderPlugin>(XrSessionPlugin { auto_handle: true })
        .add_before::<RenderPlugin>(oxr_plugin)
        .add(OxrEventsPlugin)
        .add(OxrReferenceSpacePlugin::default())
        .add(OxrRenderPlugin::default())
        .add(OxrPassthroughPlugin)
        .add(HandTrackingPlugin::default())
        .add(XrCameraPlugin)
        .add(action_set_attaching::OxrActionAttachingPlugin)
        .add(action_binding::OxrActionBindingPlugin)
        .add(action_set_syncing::OxrActionSyncingPlugin)
        .add(features::overlay::OxrOverlayPlugin)
        .add(spaces::OxrSpatialPlugin)
        .add(spaces::OxrSpacePatchingPlugin)
        // .add(XrActionPlugin)
        // we should probably handle the exiting ourselfs so that we can correctly end the
        // session and instance
        .set(WindowPlugin {
            primary_window: Some(Window {
                #[cfg(feature = "varjo_ready")]
                transparent: true,
                #[cfg(not(feature = "varjo_ready"))]
                transparent: false,
                present_mode: PresentMode::AutoNoVsync,
                // title: self.app_info.name.clone(),
                ..default()
            }),
            // #[cfg(target_os = "android")]
            // exit_condition: bevy::window::ExitCondition::DontExit,
            #[cfg(target_os = "android")]
            close_when_requested: true,
            ..default()
        })
}

#[cfg(feature = "vr_enable")]
fn create_app() -> App {
    use crate::picking3d::picking_3d::ObjectPicking3d;
    use crate::projection::ProjectionPlugin;
    use crate::vr_control::VrControlPlugin;
    use crate::vr_menu::VrMenuPlugin;
    use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
    use bevy_mod_openxr::resources::OxrSessionConfig;
    use bevy_mod_openxr::types::EnvironmentBlendMode;
    use linked_entities::LinkedEntitiesPlugin;

    info!("Creating VR App");
    let mut app = App::new();
    app.add_plugins(custom_add_xr_plugins(
        DefaultPlugins.build().disable::<PipelinedRenderingPlugin>(),
    ))
    .insert_resource(OxrSessionConfig {
        blend_modes: Some(vec![
            EnvironmentBlendMode::ALPHA_BLEND,
            EnvironmentBlendMode::OPAQUE,
        ]),
        ..default()
    })
    .add_plugins(bevy_mod_xr::hand_debug_gizmos::HandGizmosPlugin)
    .add_plugins(thirdparty_copy::transform_util_copy::TransformUtilitiesPlugin)
    .add_plugins(HistoryPlugin)
    .add_plugins(BezierRenderPlugin)
    .add_plugins(AdvancedOrbitControls)
    .add_plugins(ObjectPicking3d)
    .add_plugins(TranslationController)
    .add_plugins(SkeinPlugin { handle_brp: false })
    .add_plugins(VrControlPlugin)
    .add_plugins(UiPlugin)
    .add_plugins(ProjectionPlugin)
    .add_plugins(LinkedEntitiesPlugin)
    .add_plugins(VrMenuPlugin)
    .add_systems(Startup, (setup.before(generate_default_curve),))
    .insert_resource(LoadFonts {
        font_directories: vec!["assets/fonts".to_owned()],
        ..default()
    })
    .init_resource::<ControlStorage>();

    #[cfg(feature = "varjo_ready")]
    app.insert_resource(ClearColor(Color::NONE));

    #[cfg(not(feature = "varjo_ready"))]
    {
        use bevy::color::palettes::tailwind::GRAY_700;
        app.insert_resource(ClearColor(GRAY_700.into()));
    }

    app
}

fn main() {
    create_app().run();
}
