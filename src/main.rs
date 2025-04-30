mod advanced_orbit_controls;
mod bezier_curve_renderer;
mod nurbs;
mod translation_controller;
pub mod util;

use crate::advanced_orbit_controls::AdvancedOrbitControls;
use crate::translation_controller::TranslationController;
use bevy::prelude::*;
use bezier_curve_renderer::*;

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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(BezierRender)
        .add_plugins(AdvancedOrbitControls)
        .add_plugins(TranslationController)
        .add_systems(Startup, setup)
        .run();
}
