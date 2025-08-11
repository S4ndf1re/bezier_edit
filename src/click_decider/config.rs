use std::{
    fs::{File, read_to_string},
    io::Write,
};

use bevy::ecs::resource::Resource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Resource, Clone)]
pub struct Config {
    pub left_log_path: String,
    pub right_log_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            left_log_path: "./click_log/left/".to_owned(),
            right_log_path: "./click_log/right/".to_owned(),
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
