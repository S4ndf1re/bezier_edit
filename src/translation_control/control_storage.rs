use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use std::slice::Iter;

pub struct ControlDirection {
    pub vec: Vec3,
    pub normalized: Vec3,
    pub color: Color,
    pub hover_color: Color,
}

impl ControlDirection {
    pub fn new(vec: Vec3, color: Color, hover_color: Color) -> Self {
        Self {
            vec,
            normalized: vec * 1.0 / vec.length(),
            color,
            hover_color,
        }
    }
}

#[derive(Resource)]
pub struct ControlStorage {
    pub arrows: Vec<ControlDirection>,
}

impl ControlStorage {
    pub fn new() -> Self {
        Self { arrows: Vec::new() }
    }

    pub fn add_direction(&mut self, direction: ControlDirection) {
        self.arrows.push(direction);
    }

    pub fn iter(&self) -> Iter<ControlDirection> {
        self.arrows.iter()
    }
}

impl Default for ControlStorage {
    fn default() -> Self {
        Self {
            arrows: vec![
                ControlDirection::new(
                    Vec3::new(1.0, 0.0, 0.0),
                    Color::from(RED_600),
                    Color::from(RED_800),
                ),
                ControlDirection::new(
                    Vec3::new(0.0, 1.0, 0.0),
                    Color::from(GREEN_600),
                    Color::from(GREEN_800),
                ),
                ControlDirection::new(
                    Vec3::new(0.0, 0.0, 1.0),
                    Color::from(BLUE_600),
                    Color::from(BLUE_800),
                ),
                ControlDirection::new(
                    Vec3::new(0.0, 1.0, 1.0),
                    Color::from(PURPLE_600),
                    Color::from(PURPLE_800),
                ),
            ],
        }
    }
}
