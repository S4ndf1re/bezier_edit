mod advanced_orbit_controls;
mod bezier_curve_renderer;
mod nurbs;
mod translation_control;
pub mod util;

use crate::advanced_orbit_controls::AdvancedOrbitControls;
use crate::translation_control::control_storage::ControlStorage;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy_mod_openxr::add_xr_plugins;
use bevy_mod_openxr::init::OxrInitPlugin;
use bevy_mod_openxr::resources::OxrInstance;
use bevy_mod_openxr::types::EnvironmentBlendMode;
use bevy_xr_utils::tracking_utils::XrTrackedView;
use bevy_xr_utils::transform_utils;
use bevy_xr_utils::transform_utils::{SnapToPosition, SnapToRotation};
use bevy_xr_utils::xr_utils_actions::{XRUtilsActionSystemSet, XRUtilsActionsPlugin};
use bezier_curve_renderer::*;
use translation_control::translation_controller::TranslationController;

#[cfg(not(feature = "vr_enable"))]
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
}

#[cfg(feature = "vr_enable")]
fn setup(
    mut commands: Commands,
    mut rotation_writer: EventWriter<SnapToRotation>,
    mut position_writer: EventWriter<SnapToPosition>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    let position = Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
}

#[cfg(feature = "vr_enable")]
fn snap_to_position(
    mut rotation_writer: EventWriter<SnapToRotation>,
    mut position_writer: EventWriter<SnapToPosition>,
) {
    let position =
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
    position_writer.send(SnapToPosition(position.translation));
    //    rotation_writer.send(SnapToRotation(position.rotation));
}

#[cfg(not(feature = "vr_enable"))]
fn create_app() -> App {
    info!("Creating Non-VR App");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(BezierRender)
        .add_plugins(AdvancedOrbitControls)
        .add_plugins(TranslationController)
        .init_resource::<ControlStorage>()
        .add_systems(Startup, setup);

    app
}

#[cfg(feature = "vr_enable")]
fn create_actions() {}
#[cfg(feature = "vr_enable")]
fn create_app() -> App {
    info!("Creating VR App");
    let mut app = App::new();
    app.add_plugins(
        add_xr_plugins(DefaultPlugins.build().disable::<PipelinedRenderingPlugin>()).set(
            OxrInitPlugin {
                blend_modes: Some(vec![
                    EnvironmentBlendMode::ALPHA_BLEND,
                    EnvironmentBlendMode::ADDITIVE,
                    EnvironmentBlendMode::OPAQUE,
                ]),
                ..Default::default()
            },
        ),
    )
    .add_plugins(bevy_xr_utils::hand_gizmos::HandGizmosPlugin)
    .add_plugins(transform_utils::TransformUtilitiesPlugin)
    .add_plugins(XRUtilsActionsPlugin)
    .add_plugins(MeshPickingPlugin)
    .add_plugins(BezierRender)
    //.add_plugins(AdvancedOrbitControls)
    .add_plugins(TranslationController)
    .add_systems(
        Startup,
        (
            setup,
            create_actions.before(XRUtilsActionSystemSet::CreateEvents),
        ),
    )
    .add_systems(Update, snap_to_position)
    .insert_resource(ClearColor(Color::NONE));

    app
}

fn main() {
    create_app().run();
}
