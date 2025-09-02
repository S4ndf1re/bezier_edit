use bevy::color::palettes::tailwind::*;
use bevy::prelude::*;
use std::slice::Iter;

pub struct ControlDirection {
    pub normalized: Vec3,
    pub color: Color,
    pub hover_color: Color,
    pub shadow_color: Color,
    pub with_rotation: bool,
}

impl ControlDirection {
    pub fn new(
        vec: Vec3,
        color: Color,
        hover_color: Color,
        shadow_color: Color,
        with_rotation: bool,
    ) -> Self {
        Self {
            normalized: vec * 1.0 / vec.length(),
            color,
            hover_color,
            shadow_color,
            with_rotation,
        }
    }
}

#[derive(Resource)]
pub struct ControlStorage {
    pub arrows: Vec<ControlDirection>,
}

impl ControlStorage {
    #[allow(unused)]
    pub fn new() -> Self {
        Self { arrows: Vec::new() }
    }

    #[allow(unused)]
    pub fn add_direction(&mut self, direction: ControlDirection) {
        self.arrows.push(direction);
    }

    pub fn iter(&self) -> Iter<'_, ControlDirection> {
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
                    Color::from(GRAY_500),
                    true,
                ),
                ControlDirection::new(
                    Vec3::new(0.0, 1.0, 0.0),
                    Color::from(GREEN_600),
                    Color::from(GREEN_800),
                    Color::from(GRAY_500),
                    true,
                ),
                ControlDirection::new(
                    Vec3::new(0.0, 0.0, 1.0),
                    Color::from(BLUE_600),
                    Color::from(BLUE_800),
                    Color::from(GRAY_500),
                    true,
                ),
            ],
        }
    }
}
