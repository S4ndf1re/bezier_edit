use crate::picking3d::events::HoveredBy;
use bevy::prelude::Resource;

#[derive(Resource)]
pub struct Pointer3dState {
    grab_left_prev_state: bool,
    grab_right_prev_state: bool,
    toggled_left_since: u32,
    toggled_right_since: u32,
    max_ticks: u32,
}

impl Pointer3dState {
    pub fn new(max_ticks: u32) -> Self {
        Self {
            grab_left_prev_state: false,
            grab_right_prev_state: false,
            toggled_left_since: 0,
            toggled_right_since: 0,
            max_ticks,
        }
    }

    pub fn set_state(&mut self, state: bool, controller: &HoveredBy) {
        match controller {
            HoveredBy::Left => {
                if self.grab_left_prev_state != state {
                    self.toggled_left_since = 0;
                }
                self.grab_left_prev_state = state
            }
            HoveredBy::Right => {
                if self.grab_right_prev_state != state {
                    self.toggled_right_since = 0;
                }
                self.grab_right_prev_state = state
            }
        }
    }

    pub fn add_tick(&mut self) {
        if self.grab_left_prev_state {
            self.toggled_left_since += 1;
        }

        if self.grab_right_prev_state {
            self.toggled_right_since += 1;
        }
    }

    pub fn is_grabbing(&self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.grab_left_prev_state,
            HoveredBy::Right => self.grab_right_prev_state,
        }
    }

    /// Test if the controller is toggled in shorter than n ticks (<)
    pub fn is_just_toggled(&self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.toggled_left_since < self.max_ticks,
            HoveredBy::Right => self.toggled_right_since < self.max_ticks,
        }
    }
}
