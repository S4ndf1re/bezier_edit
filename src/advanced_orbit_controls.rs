use crate::RootTransform;
use bevy::app::App;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

#[cfg(feature = "vr_enable")]
use crate::vr_control::thumbstick3d::AccumulatedThumbstickInfo;

#[cfg(not(feature = "vr_enable"))]
use bevy::input::mouse::AccumulatedMouseMotion;

#[derive(Debug, Resource)]
struct CameraSettings {
    pub orbit_distance: f32,
    pub pitch_speed: f32,
    // Clamp pitch to this range
    pub pitch_range: Range<f32>,
    pub yaw_speed: f32,
}

#[cfg(not(feature = "vr_enable"))]
impl Default for CameraSettings {
    fn default() -> Self {
        // Limiting pitch stops some unexpected rotation past 90° up or down.
        let pitch_limit = FRAC_PI_2 - 0.01;
        Self {
            // These values are completely arbitrary, chosen because they seem to produce
            // "sensible" results for this example. Adjust as required.
            orbit_distance: 10.0,
            pitch_speed: 0.003,
            pitch_range: -pitch_limit..pitch_limit,
            yaw_speed: 0.004,
        }
    }
}

#[cfg(feature = "vr_enable")]
impl Default for CameraSettings {
    fn default() -> Self {
        // Limiting pitch stops some unexpected rotation past 90° up or down.
        let pitch_limit = FRAC_PI_2 - 0.01;
        Self {
            // These values are completely arbitrary, chosen because they seem to produce
            // "sensible" results for this example. Adjust as required.
            orbit_distance: 10.0,
            pitch_speed: 0.03,
            pitch_range: -pitch_limit..pitch_limit,
            yaw_speed: 0.04,
        }
    }
}

pub struct AdvancedOrbitControls;

#[cfg(not(feature = "vr_enable"))]
fn orbit(
    mut root: Single<&mut Transform, With<RootTransform>>,
    camera_settings: Res<CameraSettings>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
) {
    if !mouse_buttons.pressed(MouseButton::Right) {
        return;
    }

    let delta = mouse_motion.delta;

    let delta_pitch = delta.y * camera_settings.pitch_speed;
    let delta_yaw = delta.x * camera_settings.yaw_speed;

    let (yaw, pitch, roll) = root.rotation.to_euler(EulerRot::YXZ);

    let pitch = (pitch + delta_pitch).clamp(
        camera_settings.pitch_range.start,
        camera_settings.pitch_range.end,
    );

    let yaw = yaw + delta_yaw;
    root.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    // Adjust target distance
    // let target = Vec3::ZERO;
    // camera.translation = target - camera.forward() * camera_settings.orbit_distance;
}

#[cfg(feature = "vr_enable")]
fn orbit(
    mut root: Single<&mut Transform, With<RootTransform>>,
    camera_settings: Res<CameraSettings>,
    accumulated_thumbstick_info: Res<AccumulatedThumbstickInfo>,
) {
    let delta = Vec2::new(
        accumulated_thumbstick_info.x(),
        accumulated_thumbstick_info.y(),
    );

    let delta_pitch = delta.y * camera_settings.pitch_speed;
    let delta_yaw = delta.x * camera_settings.yaw_speed;

    let (yaw, pitch, roll) = root.rotation.to_euler(EulerRot::YXZ);

    let pitch = (pitch + delta_pitch).clamp(
        camera_settings.pitch_range.start,
        camera_settings.pitch_range.end,
    );

    let yaw = yaw + delta_yaw;
    root.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);

    // Adjust target distance
    // let target = Vec3::ZERO;
    // camera.translation = target - camera.forward() * camera_settings.orbit_distance;
}

impl Plugin for AdvancedOrbitControls {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.add_systems(Update, orbit);
    }
}
