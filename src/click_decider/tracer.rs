use bevy::prelude::*;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::Write;
use std::{collections::VecDeque, fs::File, path::PathBuf, time::Instant};

use super::config::Config;

pub type ButtonPressValue = f64;
pub type ButtonPressDerivative = f64;

#[derive(Default)]
pub enum ControllerSide {
    #[default]
    Left,
    Right,
}

#[derive(Serialize, Deserialize)]
pub struct ControllerTrace {
    timed_transforms: VecDeque<(u128, Transform)>,
    controller_derivatives: VecDeque<(u128, ButtonPressValue, ButtonPressDerivative)>,
    max_entries: usize,

    #[serde(skip_serializing, skip_deserializing)]
    start_time: Option<Instant>,

    #[serde(skip_serializing, skip_deserializing)]
    config: Config,
    #[serde(skip_serializing, skip_deserializing)]
    side: ControllerSide,
}

impl ControllerTrace {
    pub fn new(
        config: Config,
        start_value_controller: ButtonPressValue,
        side: ControllerSide,
        max_entries: usize,
    ) -> Self {
        let now = Instant::now();
        let elapsed = now.elapsed().as_micros();
        Self {
            timed_transforms: VecDeque::new(),
            start_time: Some(now),
            max_entries,
            controller_derivatives: VecDeque::from([(elapsed, start_value_controller, 0.0)]),
            config,
            side,
        }
    }
    pub fn update(&mut self, next: Transform, button_press_value: ButtonPressValue) {
        if self.start_time.is_none() {
            self.start_time = Some(Instant::now());
        }

        let time = self
            .start_time
            .expect("Invalid if missing")
            .elapsed()
            .as_micros();
        self.timed_transforms.push_back((time, next));

        if self.controller_derivatives.is_empty() {
            self.controller_derivatives
                .push_back((time, button_press_value, 0.0));
        } else {
            let old = self.controller_derivatives[self.controller_derivatives.len() - 1];
            let diff = -button_press_value - old.1;
            let derivative = diff / ((time - old.0) as f64);

            self.controller_derivatives
                .push_back((time, button_press_value, derivative));
        }

        while self.controller_derivatives.len() > self.max_entries {
            self.controller_derivatives.pop_front();
        }

        while self.timed_transforms.len() > self.max_entries {
            self.timed_transforms.pop_front();
        }
    }

    pub fn log_current_transforms(&self) -> Result<(), Box<dyn Error>> {
        let path = match self.side {
            ControllerSide::Left => &self.config.left_log_path,
            ControllerSide::Right => &self.config.right_log_path,
        };

        let mut pathbuf = PathBuf::new();
        pathbuf.push(path);
        let time = Utc::now().to_rfc3339();
        pathbuf.push(time);

        let mut file = File::create(pathbuf.as_path())?;

        let data = serde_json::to_string(self)?;
        file.write_all(data.as_bytes())?;

        Ok(())
    }
}
