use bevy::prelude::*;
use bevy_mod_openxr::openxr_session_running;
use bevy_mod_openxr::session::OxrSession;

use super::ControllerActions;

#[derive(Resource, Default)]
pub struct AccumulatedThumbstickInfoLeft {
    x: f32,
    y: f32,
}

impl AccumulatedThumbstickInfoLeft {
    #[allow(unused)]
    pub fn x(&self) -> f32 {
        self.x
    }

    #[allow(unused)]
    pub fn y(&self) -> f32 {
        self.y
    }
}

#[derive(Resource, Default)]
pub struct AccumulatedThumbstickInfoRight {
    x: f32,
    y: f32,
}

impl AccumulatedThumbstickInfoRight {
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
    mut thumbstick_state_left: ResMut<AccumulatedThumbstickInfoLeft>,
    mut thumbstick_state_right: ResMut<AccumulatedThumbstickInfoRight>,
    session: Res<OxrSession>,
) {
    thumbstick_state_left.x = 0.0;
    thumbstick_state_left.y = 0.0;

    thumbstick_state_right.x = 0.0;
    thumbstick_state_right.y = 0.0;

    let left_state_x = actions
        .left
        .thumbstick_x
        .state(&session, openxr::Path::NULL);

    let left_state_y = actions
        .left
        .thumbstick_y
        .state(&session, openxr::Path::NULL);

    let right_state_x = actions
        .right
        .thumbstick_x
        .state(&session, openxr::Path::NULL);

    let right_state_y = actions
        .right
        .thumbstick_y
        .state(&session, openxr::Path::NULL);

    let (x, y) = (left_state_x, left_state_y);
    if let Ok(x) = x
        && x.is_active
    {
        thumbstick_state_left.x += x.current_state;
    }

    if let Ok(y) = y
        && y.is_active
    {
        thumbstick_state_left.y += y.current_state;
    }

    let (x, y) = (right_state_x, right_state_y);
    if let Ok(x) = x
        && x.is_active
    {
        thumbstick_state_right.x += x.current_state;
    }

    if let Ok(y) = y
        && y.is_active
    {
        thumbstick_state_right.y += y.current_state;
    }
}

pub struct ThumbstickPlugin;

impl Plugin for ThumbstickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AccumulatedThumbstickInfoLeft::default());
        app.insert_resource(AccumulatedThumbstickInfoRight::default());
        app.add_systems(
            PostUpdate,
            update_thumbstick_events.run_if(openxr_session_running),
        );
    }
}
