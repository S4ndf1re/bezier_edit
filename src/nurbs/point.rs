use bevy::math::Vec3;
use num::traits::clamp_min;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone, Copy, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
}

impl Point {
    pub fn new(x: f64, y: f64, z: f64, w: Option<f64>) -> Point {
        Point {
            x,
            y,
            z,
            w: clamp_min(w.unwrap_or(1.0), 0.0),
        }
    }

    pub fn magnitude(&self) -> f64 {
        (self * self).sqrt()
    }

    pub fn cross(&self, other: &Self) -> Point {
        Point::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
            None,
        )
    }
}

impl Add<&Point> for &Point {
    type Output = Point;
    fn add(self, rhs: &Point) -> Self::Output {
        Point::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z, Some(self.w))
    }
}

impl Mul<&Point> for &Point {
    type Output = f64;
    fn mul(self, rhs: &Point) -> Self::Output {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
}

impl Mul<f64> for &Point {
    type Output = Point;
    fn mul(self, rhs: f64) -> Self::Output {
        Point::new(self.x * rhs, self.y * rhs, self.z * rhs, None)
    }
}

impl Mul<&Point> for f64 {
    type Output = Point;
    fn mul(self, rhs: &Point) -> Self::Output {
        rhs * self
    }
}

impl Sub for &Point {
    type Output = Point;
    fn sub(self, rhs: &Point) -> Self::Output {
        Point::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z, Some(self.w))
    }
}

impl Div<f64> for Point {
    type Output = Point;
    fn div(self, rhs: f64) -> Self::Output {
        Point::new(self.x / rhs, self.y / rhs, self.z / rhs, Some(self.w))
    }
}

impl From<Point> for Vec3 {
    fn from(value: Point) -> Self {
        Vec3::new(value.x as f32, value.y as f32, value.z as f32)
    }
}

impl From<Vec3> for Point {
    fn from(value: Vec3) -> Self {
        Point::new(value.x as f64, value.y as f64, value.z as f64, None)
    }
}

impl From<&Point> for [f32; 3] {
    fn from(value: &Point) -> Self {
        [value.x as f32, value.y as f32, value.z as f32]
    }
}

impl From<Point> for [f32; 3] {
    fn from(value: Point) -> Self {
        [value.x as f32, value.y as f32, value.z as f32]
    }
}
