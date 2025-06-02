use super::point::Point;

#[derive(Debug)]
pub struct BoundingBox3DRange(f64, f64);

impl BoundingBox3DRange {
    pub fn new() -> Self {
        Self(f64::MAX, f64::MIN)
    }

    pub fn add_value(&mut self, value: f64) {
        self.0 = self.0.min(value);
        self.1 = self.1.max(value);
    }

    pub fn value_contained(&self, value: f64) -> bool {
        if self.1 < self.0 {
            return false;
        }
        self.0 - f32::EPSILON as f64 <= value && value <= self.1 + f32::EPSILON as f64
    }

    pub fn length(&self) -> f64 {
        if self.1 < self.0 {
            return 0.0;
        }
        self.1 - self.0
    }
}

#[derive(Debug)]
pub struct BoundingBox3D {
    x: BoundingBox3DRange,
    y: BoundingBox3DRange,
    z: BoundingBox3DRange,
}

impl BoundingBox3D {
    pub fn new() -> Self {
        Self {
            x: BoundingBox3DRange::new(),
            y: BoundingBox3DRange::new(),
            z: BoundingBox3DRange::new(),
        }
    }

    pub fn add_point(&mut self, point: &Point) {
        self.x.add_value(point.x);
        self.y.add_value(point.y);
        self.z.add_value(point.z);
    }

    /// Check if point is contained within bounding box
    pub fn contains_point(&self, point: &Point) -> bool {
        (self.x.value_contained(point.x) || self.x.length() < f64::EPSILON)
            && (self.y.value_contained(point.y) || self.y.length() < f64::EPSILON)
            && (self.z.value_contained(point.z) || self.z.length() < f64::EPSILON)
    }

    pub fn volume(&self) -> f64 {
        let mut volume = 1.0;
        if self.x.length() > f64::EPSILON {
            volume *= self.x.length();
        }
        if self.y.length() > f64::EPSILON {
            volume *= self.y.length();
        }
        if self.z.length() > f64::EPSILON {
            volume *= self.z.length();
        }

        volume
    }
}
