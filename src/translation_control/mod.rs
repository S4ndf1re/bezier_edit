pub mod control_storage;
pub mod obligatory_drag_params;
pub mod translation_controller;

use bevy::prelude::*;
use translation_controller::EnableTranslationControl;

use crate::{
    bezier_curve::components::ControlState,
    click_decider::LogTrace,
    picking3d::events::{self, Pointer3d},
};

#[allow(clippy::complexity)]
pub fn enable_gizmo(
    enable_translation_control: EnableTranslationControl,
) -> impl Fn(Trigger<Pointer<Click>>, Commands, Query<&EnableTranslationControl>, Res<State<ControlState>>)
{
    move |trigger, mut commands, enabled, state| {
        #[allow(clippy::collapsible_if)]
        if *state == ControlState::Main {
            if let Ok(mut entity) = commands.get_entity(trigger.target()) {
                if enabled.get(trigger.target()).is_ok() {
                    entity.remove::<EnableTranslationControl>();
                } else {
                    entity.insert(enable_translation_control);
                }
            }
        }
    }
}

#[allow(clippy::complexity)]
pub fn enable_gizmo3d(
    enable_translation_control: EnableTranslationControl,
) -> impl Fn(
    Trigger<Pointer3d<events::Click>>,
    Commands,
    Query<&EnableTranslationControl>,
    EventWriter<LogTrace>,
    Res<State<ControlState>>,
) {
    move |trigger, mut commands, enabled, mut trace_log_writer, state| {
        #[allow(clippy::collapsible_if)]
        if *state == ControlState::Main {
            if let Ok(mut entity) = commands.get_entity(trigger.target()) {
                if enabled.get(trigger.target()).is_ok() {
                    entity.remove::<EnableTranslationControl>();
                } else {
                    entity.insert(enable_translation_control);
                }
                trace_log_writer.write(LogTrace::default());
            }
        }
    }
}
