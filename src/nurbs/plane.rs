use bevy::transform::components::Transform;

use super::{parametric::Parametric, point::Point};

pub struct Ray3d {
    origin: Point,
    direction: Point,
}

impl Ray3d {
    pub fn new(origin: Point, direction: Point) -> Self {
        Self { origin, direction }
    }

    pub fn plane_intersection(&self, plane: &Plane3d) -> Option<f64> {
        let denom = plane.normal * self.direction;

        // NOTE: Choose any eps that is regarded as sufficient. The closer to 0, the more exact
        // the intersection will be
        if denom.abs() > 0.000001 {
            let t = ((plane.origin - self.origin) * plane.normal) / denom;

            if t >= 0.0 {
                return Some(t);
            }
        }

        None
    }

    pub fn plane_intersection_both_ends(&self, plane: &Plane3d) -> Option<f64> {
        let denom = plane.normal * self.direction;

        // NOTE: Choose any eps that is regarded as sufficient. The closer to 0, the more exact
        // the intersection will be
        if denom.abs() > 0.000001 {
            let t = ((plane.origin - self.origin) * plane.normal) / denom;

            return Some(t);
        }

        None
    }
}

impl Parametric<1, Point> for Ray3d {
    fn f(&self, ts: &[f64; 1]) -> Point {
        self.origin + (ts[0] * self.direction)
    }

    fn derive(&self, _: &[f64; 1], _: usize) -> [Point; 1] {
        [self.direction]
    }

    fn range(&self) -> [[f64; 2]; 1] {
        unimplemented!()
    }
}

pub struct Plane3d {
    origin: Point,
    normal: Point,
    u: Point,
    v: Point,
}

impl Plane3d {
    pub fn new(origin: Point, normal: Point, u: Point, v: Point) -> Self {
        assert!((u * normal).abs() <= 0.00001,);
        assert!((v * normal).abs() <= 0.00001);

        Self {
            origin,
            normal: normal.normalize(),
            u: u.normalize(),
            v: v.normalize(),
        }
    }

    // same as new, but without asserts that (n and u) and (n and v) are orthogonal
    pub fn new_unchecked(origin: Point, normal: Point, u: Point, v: Point) -> Self {
        Self {
            origin,
            normal: normal.normalize(),
            u: u.normalize(),
            v: v.normalize(),
        }
    }

    /// Project a point onto the plane using orthogonal projection. Return the evaluation
    /// parameters u and v
    pub fn point_projected_on_plane_orthogonal(&self, point: Point) -> Option<(f64, f64)> {
        let ray = Ray3d {
            origin: point,
            direction: -1.0 * self.normal,
        };

        if let Some(hit) = ray.plane_intersection(self) {
            let point = ray.f(&[hit]);
            let delta_p = point - self.origin;

            return Some((delta_p * self.u, delta_p * self.v));
        }

        None
    }
}

impl From<Transform> for Plane3d {
    fn from(value: Transform) -> Self {
        Self::new(
            value.translation.into(),
            value.forward().as_vec3().into(),
            value.left().as_vec3().into(),
            value.up().as_vec3().into(),
        )
    }
}

impl Parametric<2, Point> for Plane3d {
    fn f(&self, ts: &[f64; 2]) -> Point {
        ((self.u * ts[0]) + (self.v * ts[1])) + self.origin
    }

    fn derive(&self, _: &[f64; 2], _: usize) -> [Point; 2] {
        unimplemented!()
    }

    fn range(&self) -> [[f64; 2]; 2] {
        unimplemented!()
    }
}
