use super::point::Point;

pub struct BoundingBox3DRange(f64, f64);

impl BoundingBox3DRange {
    pub fn add_value(&mut self, value: f64) {
        self.0 = self.0.min(value);
        self.1 = self.1.max(value);
    }

    pub fn value_contained(&self, value: f64) -> bool {
        self.0 <= value && value <= self.1
    }

    pub fn length(&self) -> f64 {
        self.1 - self.0
    }
}

pub struct BoundingBox3D {
    x: BoundingBox3DRange,
    y: BoundingBox3DRange,
    z: BoundingBox3DRange,
}

impl BoundingBox3D {
    pub fn new() -> Self {
        Self {
            x: BoundingBox3DRange(0.0, 0.0),
            y: BoundingBox3DRange(0.0, 0.0),
            z: BoundingBox3DRange(0.0, 0.0),
        }
    }

    pub fn add_point(&mut self, point: &Point) {
        self.x.add_value(point.x);
        self.y.add_value(point.y);
        self.z.add_value(point.z);
    }

    /// Check if point is contained within bounding box
    pub fn contains_point(&self, point: &Point) -> bool {
        self.x.value_contained(point.x)
            && self.y.value_contained(point.y)
            && self.z.value_contained(point.z)
    }

    pub fn volume(&self) -> f64 {
        self.x.length() * self.y.length() * self.z.length()
    }
}
