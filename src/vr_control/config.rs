use std::{
    fs::{File, read_to_string},
    io::Write,
};

use bevy::ecs::resource::Resource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ControllerConfig {
    pub aim: String,
    pub grip: String,
    pub trigger: String,
    pub squeeze: String,
    pub thumbstick_x: String,
    pub thumbstick_y: String,
    pub output: String,
}

#[derive(Serialize, Deserialize, Resource)]
pub struct Config {
    pub interaction_profile: String,
    pub left: ControllerConfig,
    pub right: ControllerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interaction_profile: "/interaction_profiles/oculus/touch_controller".to_owned(),
            left: ControllerConfig {
                aim: "/user/hand/left/input/aim/pose".to_owned(),
                grip: "/user/hand/left/input/grip/pose".to_owned(),
                trigger: "/user/hand/left/input/trigger/value".to_owned(),
                squeeze: "/user/hand/left/input/squeeze/value".to_owned(),
                thumbstick_x: "/user/hand/left/input/thumbstick/x".to_owned(),
                thumbstick_y: "/user/hand/left/input/thumbstick/y".to_owned(),
                output: "/user/hand/right/output/haptic".to_owned(),
            },
            right: ControllerConfig {
                aim: "/user/hand/right/input/aim/pose".to_owned(),
                grip: "/user/hand/right/input/grip/pose".to_owned(),
                trigger: "/user/hand/right/input/trigger/value".to_owned(),
                squeeze: "/user/hand/left/input/squeeze/value".to_owned(),
                thumbstick_x: "/user/hand/right/input/thumbstick/x".to_owned(),
                thumbstick_y: "/user/hand/right/input/thumbstick/y".to_owned(),
                output: "/user/hand/right/output/haptic".to_owned(),
            },
        }
    }
}

impl Config {
    pub fn read_or_create_default(path: &str) -> Config {
        if let Ok(content) = read_to_string(path)
            && let Ok(config) = serde_json::from_str::<Config>(&content)
        {
            return config;
        }

        let config = Config::default();

        let content = serde_json::to_string(&config);
        if let Ok(mut file) = File::create(path)
            && let Ok(content) = content
        {
            let _ = file.write_all(content.as_bytes());
        }

        config
    }
}
