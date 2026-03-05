use bevy::prelude::*;
use bevy_mod_openxr::{openxr_session_running, session::OxrSession};
use openxr::HapticVibration;

use super::ControllerActions;

pub struct Vibration {
    pub amplitude: f32,
    pub frequency: f32,
    pub duration_nano: openxr::Duration,
}

impl Default for Vibration {
    fn default() -> Self {
        Self {
            amplitude: 1.0,
            frequency: openxr::FREQUENCY_UNSPECIFIED,
            duration_nano: openxr::Duration::MIN_HAPTIC,
        }
    }
}

impl Vibration {
    pub fn duration_millis(mut self, millis: i64) -> Self {
        self.duration_nano = openxr::Duration::from_nanos(millis * 1000 * 1000);
        self
    }

    pub fn duration_micros(mut self, micros: i64) -> Self {
        self.duration_nano = openxr::Duration::from_nanos(micros * 1000);
        self
    }
}

#[derive(Event)]
pub struct VibrateLeftEvent(Vibration);

impl VibrateLeftEvent {
    pub fn new(vibration: Vibration) -> Self {
        Self(vibration)
    }
}

#[derive(Event)]
pub struct VibrateRightEvent(Vibration);

impl VibrateRightEvent {
    pub fn new(vibration: Vibration) -> Self {
        Self(vibration)
    }
}

fn listen_left_events(
    mut reader: EventReader<VibrateLeftEvent>,
    actions: Res<ControllerActions>,
    session: Res<OxrSession>,
) {
    for event in reader.read() {
        let vibration = HapticVibration::new()
            .duration(event.0.duration_nano)
            .frequency(event.0.frequency)
            .amplitude(event.0.amplitude);

        let res = actions
            .left
            .output
            .apply_feedback(&session, openxr::Path::NULL, &vibration);

        if res.is_err() {
            error!("{}", res.err().unwrap())
        }
    }
}

fn listen_right_events(
    mut reader: EventReader<VibrateRightEvent>,
    actions: Res<ControllerActions>,
    session: Res<OxrSession>,
) {
    for event in reader.read() {
        let vibration = HapticVibration::new()
            .duration(event.0.duration_nano)
            .frequency(event.0.frequency)
            .amplitude(event.0.amplitude);

        let res = actions
            .right
            .output
            .apply_feedback(&session, openxr::Path::NULL, &vibration);

        if res.is_err() {
            error!("{}", res.err().unwrap())
        }
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
