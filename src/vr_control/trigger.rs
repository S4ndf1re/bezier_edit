use bevy::prelude::*;
use bevy_mod_openxr::openxr_session_running;
use bevy_mod_openxr::session::OxrSession;

use super::ControllerActions;

#[derive(Default, Resource)]
pub struct ControllerTrigger {
    pub left: f32,
    pub right: f32,
}

#[derive(Default, Resource)]
pub struct ControllerSqueeze {
    pub left: f32,
    pub right: f32,
}

fn update_trigger_events(
    actions: Res<ControllerActions>,
    mut trigger: ResMut<ControllerTrigger>,
    session: Res<OxrSession>,
) {
    trigger.left = 0.0;
    trigger.right = 0.0;

    let left_state = actions.left.trigger.state(&session, openxr::Path::NULL);

    let right_state = actions.right.trigger.state(&session, openxr::Path::NULL);

    if let Ok(left) = left_state
        && left.is_active
    {
        trigger.left = left.current_state
    }

    if let Ok(right) = right_state
        && right.is_active
    {
        trigger.right = right.current_state
    }
}

fn update_squeeze_events(
    actions: Res<ControllerActions>,
    mut squeeze: ResMut<ControllerSqueeze>,
    session: Res<OxrSession>,
) {
    squeeze.left = 0.0;
    squeeze.right = 0.0;

    let left_state = actions.left.squeeze.state(&session, openxr::Path::NULL);

    let right_state = actions.right.squeeze.state(&session, openxr::Path::NULL);

    if let Ok(left) = left_state
        && left.is_active
    {
        squeeze.left = left.current_state
    }

    if let Ok(right) = right_state
        && right.is_active
    {
        squeeze.right = right.current_state
    }
}

pub struct TriggerPlugin;

impl Plugin for TriggerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ControllerTrigger>();
        app.init_resource::<ControllerSqueeze>();
        app.add_systems(
            PostUpdate,
            update_trigger_events.run_if(openxr_session_running),
        );

        app.add_systems(
            PostUpdate,
            update_squeeze_events.run_if(openxr_session_running),
        );
    }
}
