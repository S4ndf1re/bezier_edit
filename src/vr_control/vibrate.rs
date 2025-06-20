use bevy::prelude::*;
use bevy_mod_openxr::{openxr_session_running, session::OxrSession};
use openxr::{Duration, HapticVibration};

use super::ControllerActions;

pub struct Vibration {
    pub amplitude: f32,
    pub frequency: f32,
    pub duration_ms: i64,
}

#[derive(Event)]
pub struct VibrateLeftEvent(Vibration);

#[derive(Event)]
pub struct VibrateRightEvent(Vibration);

fn listen_left_events(
    mut reader: EventReader<VibrateLeftEvent>,
    actions: Res<ControllerActions>,
    session: Res<OxrSession>,
) {
    for event in reader.read() {
        let vibration = HapticVibration::new()
            .duration(Duration::from_nanos(event.0.duration_ms * 1000 * 1000))
            .frequency(event.0.frequency)
            .amplitude(event.0.amplitude);

        let _ = actions
            .left
            .output
            .apply_feedback(&session, openxr::Path::NULL, &vibration);
    }
}

fn listen_right_events(
    mut reader: EventReader<VibrateRightEvent>,
    actions: Res<ControllerActions>,
    session: Res<OxrSession>,
) {
    for event in reader.read() {
        let vibration = HapticVibration::new()
            .duration(Duration::from_nanos(event.0.duration_ms * 1000 * 1000))
            .frequency(event.0.frequency)
            .amplitude(event.0.amplitude);

        let _ = actions
            .right
            .output
            .apply_feedback(&session, openxr::Path::NULL, &vibration);
    }
}

pub struct VibrationPlugin;

impl Plugin for VibrationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<VibrateLeftEvent>();
        app.add_event::<VibrateRightEvent>();
        app.add_systems(
            PostUpdate,
            (listen_left_events, listen_right_events).run_if(openxr_session_running),
        );
    }
}
