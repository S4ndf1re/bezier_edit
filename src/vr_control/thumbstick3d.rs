use bevy::prelude::*;
use bevy_mod_openxr::openxr_session_running;
use bevy_mod_openxr::session::OxrSession;

use super::ControllerActions;

#[derive(Resource, Default)]
pub struct AccumulatedThumbstickInfo {
    x: f32,
    y: f32,
}
impl AccumulatedThumbstickInfo {
    #[allow(unused)]
    pub fn x(&self) -> f32 {
        self.x
    }

    #[allow(unused)]
    pub fn y(&self) -> f32 {
        self.y
    }
}

fn update_thumbstick_events(
    actions: Res<ControllerActions>,
    mut thumbstick_state: ResMut<AccumulatedThumbstickInfo>,
    session: Res<OxrSession>,
) {
    thumbstick_state.x = 0.0;
    thumbstick_state.y = 0.0;

    let left_state_x = actions
        .left
        .thumbstick_x
        .state(&session, openxr::Path::NULL);

    let left_state_y = actions
        .left
        .thumbstick_x
        .state(&session, openxr::Path::NULL);

    let right_state_x = actions
        .right
        .thumbstick_x
        .state(&session, openxr::Path::NULL);

    let right_state_y = actions
        .right
        .thumbstick_y
        .state(&session, openxr::Path::NULL);

    for (x, y) in [(left_state_x, left_state_y), (right_state_x, right_state_y)].iter() {
        if let Ok(x) = x
            && x.is_active
        {
            thumbstick_state.x += x.current_state;
        }

        if let Ok(y) = y
            && y.is_active
        {
            thumbstick_state.y += y.current_state;
        }
    }
}

pub struct ThumbstickPlugin;

impl Plugin for ThumbstickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AccumulatedThumbstickInfo::default());
        app.add_systems(
            PostUpdate,
            update_thumbstick_events.run_if(openxr_session_running),
        );
    }
}
