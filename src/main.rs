#![feature(let_chains)]

mod advanced_orbit_controls;
mod bezier_curve;
pub mod click_decider;
mod history;
mod nurbs;
pub mod picking3d;
pub mod projection;
pub mod solver;
mod thirdparty_copy;
mod translation_control;
mod ui;
pub mod util;
pub mod vr_control;

use crate::advanced_orbit_controls::AdvancedOrbitControls;
use crate::thirdparty_copy::transform_util_copy::{SnapToPosition, SnapToRotation};
use crate::translation_control::control_storage::ControlStorage;
use bevy::prelude::*;
use bevy_xr_utils::xr_utils_actions::{XRUtilsActionSystemSet, XRUtilsActionsPlugin};
use bezier_curve::bezier_curve_renderer::*;
use history::plugin::HistoryPlugin;

use crate::bezier_curve::render_info::RenderInformation;
use bevy::render::view::RenderLayers;
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
fn setup(mut commands: Commands) {
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
    ));

    commands.spawn(RootTransform);
}

#[cfg(feature = "vr_enable")]
fn setup(
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

    commands.spawn((RootTransform));
}

#[cfg(not(feature = "vr_enable"))]
fn create_app() -> App {
    use projection::ProjectionPlugin;

    info!("Creating Non-VR App");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(HistoryPlugin)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(BezierRenderPlugin)
        .add_plugins(AdvancedOrbitControls)
        .add_plugins(TranslationController)
        .add_plugins(UiPlugin)
        .add_plugins(ProjectionPlugin)
        .init_resource::<ControlStorage>()
        .add_systems(Startup, setup.before(generate_default_curve));

    app
}

#[cfg(feature = "vr_enable")]
fn create_app() -> App {
    use crate::vr_control::VrControlPlugin;
    use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
    use bevy_mod_openxr::add_xr_plugins;
    use bevy_mod_openxr::resources::OxrSessionConfig;
    use bevy_mod_openxr::types::EnvironmentBlendMode;
    use click_decider::TracingPlugin;
    use picking3d::picking_3d::ObjectPicking3d;

    info!("Creating VR App");
    let mut app = App::new();
    app.add_plugins(add_xr_plugins(
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
    .init_resource::<ControlStorage>()
    .add_plugins(ObjectPicking3d)
    .add_plugins(TranslationController)
    .add_plugins(VrControlPlugin)
    .add_plugins(UiPlugin)
    .add_plugins(TracingPlugin)
    .add_systems(Startup, (setup.before(generate_default_curve),))
    .insert_resource(ClearColor(Color::NONE));

    app
}

fn main() {
    create_app().run();
}
