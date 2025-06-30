use crate::RootTransform;
use bevy::app::App;
use bevy::prelude::*;

#[cfg(feature = "vr_enable")]
use crate::vr_control::thumbstick3d::AccumulatedThumbstickInfo;

#[cfg(not(feature = "vr_enable"))]
use bevy::input::mouse::AccumulatedMouseMotion;

#[derive(Debug, Resource)]
struct CameraSettings {
    pub pitch_speed: f32,
    // Clamp pitch to this range
    pub yaw_speed: f32,
}

#[cfg(not(feature = "vr_enable"))]
impl Default for CameraSettings {
    fn default() -> Self {
        // Limiting pitch stops some unexpected rotation past 90° up or down.
        Self {
            // These values are completely arbitrary, chosen because they seem to produce
            // "sensible" results for this example. Adjust as required.
            pitch_speed: 0.003,
            yaw_speed: 0.004,
        }
    }
}

#[cfg(feature = "vr_enable")]
impl Default for CameraSettings {
    fn default() -> Self {
        // Limiting pitch stops some unexpected rotation past 90° up or down.
        Self {
            // These values are completely arbitrary, chosen because they seem to produce
            // "sensible" results for this example. Adjust as required.
            pitch_speed: 0.03,
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

    root.rotation = Quat::from_axis_angle(Vec3::Z, delta_pitch)
        * Quat::from_axis_angle(Vec3::Y, delta_yaw)
        * root.rotation;
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

    root.rotation = Quat::from_axis_angle(Vec3::Z, delta_pitch)
        * Quat::from_axis_angle(Vec3::Y, delta_yaw)
        * root.rotation;
}

impl Plugin for AdvancedOrbitControls {
    fn build(&self, app: &mut App) {
        app.init_resource::<CameraSettings>();
        app.add_systems(Update, orbit);
    }
}
