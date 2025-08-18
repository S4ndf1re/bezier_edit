pub mod control_storage;
pub mod translation_controller;

use bevy::prelude::*;
use translation_controller::EnableTranslationControl;

use crate::{
    bezier_curve::components::ControlState,
    click_decider::LogTrace,
    picking3d::events::{self, Pointer3d},
};

pub fn enable_gizmo<const WITH_ROTATION: bool>(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    state: Res<State<ControlState>>,
) {
    #[allow(clippy::collapsible_if)]
    if *state == ControlState::Main {
        if let Ok(mut entity) = commands.get_entity(trigger.target()) {
            if enabled.get(trigger.target()).is_ok() {
                entity.remove::<EnableTranslationControl>();
            } else {
                entity.insert(EnableTranslationControl::new(WITH_ROTATION));
            }
        }
    }
}

pub fn enable_gizmo3d<const WITH_ROTATION: bool>(
    trigger: Trigger<Pointer3d<events::Click>>,
    mut commands: Commands,
    enabled: Query<&EnableTranslationControl>,
    mut trace_log_writer: EventWriter<LogTrace>,
    state: Res<State<ControlState>>,
) {
    #[allow(clippy::collapsible_if)]
    if *state == ControlState::Main {
        if let Ok(mut entity) = commands.get_entity(trigger.target()) {
            if enabled.get(trigger.target()).is_ok() {
                entity.remove::<EnableTranslationControl>();
            } else {
                entity.insert(EnableTranslationControl::new(WITH_ROTATION));
            }
            println!("Sending log trace event");
            trace_log_writer.write(LogTrace::default());
        }
    }
}
