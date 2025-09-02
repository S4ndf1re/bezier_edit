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

pub struct ControlPlane {
    pub axis: Vec3,
    pub normal: Vec3,
    pub color: Color,
    pub hover_color: Color,
}

impl ControlPlane {
    pub fn new(axis: Vec3, normal: Vec3, color: Color, hover_color: Color) -> Self {
        Self {
            axis,
            normal,
            color,
            hover_color,
        }
    }
}

#[derive(Resource)]
pub struct ControlStorage {
    pub arrows: Vec<ControlDirection>,
    pub planes: Vec<ControlPlane>,
}

impl ControlStorage {
    #[allow(unused)]
    pub fn new() -> Self {
        Self {
            arrows: Vec::new(),
            planes: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn add_direction(&mut self, direction: ControlDirection) {
        self.arrows.push(direction);
    }

    pub fn iter_arrows(&self) -> Iter<'_, ControlDirection> {
        self.arrows.iter()
    }

    pub fn iter_planes(&self) -> Iter<'_, ControlPlane> {
        self.planes.iter()
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
            planes: vec![
                ControlPlane::new(
                    Vec3::new(1.0, 1.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                    Color::from(YELLOW_600),
                    Color::from(YELLOW_800),
                ),
                ControlPlane::new(
                    Vec3::new(1.0, 0.0, 1.0),
                    Vec3::new(0.0, 1.0, 0.0),
                    Color::from(PURPLE_600),
                    Color::from(PURPLE_800),
                ),
                ControlPlane::new(
                    Vec3::new(0.0, 1.0, 1.0),
                    Vec3::new(1.0, 0.0, 0.0),
                    Color::from(CYAN_600),
                    Color::from(CYAN_800),
                ),
            ],
        }
    }
}
