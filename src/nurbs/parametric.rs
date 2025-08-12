use std::f64;

use rand::Rng;

use super::{
    bezier::{de_casteljau, derive_after_de_casteljau},
    point::Point,
};

pub struct MinDistanceResult<const PARAMS: usize, R> {
    pub params: [f64; PARAMS],
    pub value: R,
    pub distance: f64,
}

pub trait MinDistanceToPoint<const PARAMS: usize, R> {
    fn min_distance_to_point(&self, target: Point) -> MinDistanceResult<PARAMS, R>;
}

/// Parametric functions are functions that map a parameter t to a function value R. Possibly, the
/// function has multiple parameters, that are controllable by the PARAMS const.
pub trait Parametric<const PARAMS: usize, R> {
    /// Evaluate the parametric function
    fn f(&self, ts: &[f64; PARAMS]) -> R;

    /// Get the range that the parameter is contained in
    fn range(&self) -> [[f64; 2]; PARAMS];

    fn derive(&self, ts: &[f64; PARAMS], r: usize) -> R;
}

impl Parametric<1, Point> for &[Point] {
    fn f(&self, ts: &[f64; 1]) -> Point {
        *de_casteljau(self, ts[0]).last().unwrap().last().unwrap()
    }

    fn range(&self) -> [[f64; 2]; 1] {
        [[0.0, 1.0]]
    }

    fn derive(&self, ts: &[f64; 1], r: usize) -> Point {
        derive_after_de_casteljau(&de_casteljau(self, ts[0]), r)
    }
}

impl<T> MinDistanceToPoint<1, Point> for T
where
    T: Parametric<1, Point>,
{
    fn min_distance_to_point(&self, target: Point) -> MinDistanceResult<1, Point> {
        let mut min = f64::MAX;
        let mut min_index = f64::MAX;
        let first_scans_counter = 100; // Modify using a resolution flag
        let eps = 1e-10;

        for scan in 0..=first_scans_counter {
            let t = (scan as f64) / (first_scans_counter as f64);
            let p = self.f(&[t]);
            let diff = &target - &p;
            let distance = diff.magnitude();
            if distance < min {
                min = distance;
                min_index = scan as f64;
            }
        }

        // Now that the probable min index is found, use binary search to actually find the min index.
        // (clamped)

        let t0 = ((min_index - 1.0) / first_scans_counter as f64).max(0.0);
        let t1 = ((min_index + 1.0) / first_scans_counter as f64).min(1.0);
        let f = |t| (&target - &self.f(&[t])).magnitude();

        let mut n = t0;
        let mut m = t1;
        let mut k = 0.0;

        while (m - n) > eps {
            k = (n + m) / 2.0;

            if f(k - eps) < f(k + eps) {
                m = k;
            } else {
                n = k;
            }
        }

        MinDistanceResult {
            params: [k],
            value: self.f(&[k]),
            distance: f(k),
        }
    }
}

pub struct Circle3D {
    pub origin: Point,
    pub radius: f64,
    pub normal: Point,
    pub ortho1_unit: Point,
    pub ortho2_unit: Point,
}

impl Circle3D {
    pub fn new(origin: Point, radius: f64, mut normal: Point) -> Self {
        let mut rng = rand::rng();
        let mut ortho1: Point = rng.random::<(f64, f64, f64)>().into();

        normal = normal.normalize();

        ortho1 = &ortho1 - &((&ortho1 * &normal) * &normal / normal.magnitude().powi(2));
        ortho1 = ortho1.normalize();

        let mut ortho2 = normal.cross(&ortho1);
        ortho2 = ortho2.normalize();

        assert!(
            (&ortho1 * &ortho2).abs() <= f64::EPSILON,
            "Both orthogonals must actually be orthogonal, meaning dot(o1, o2) == 0.0"
        );
        assert!(
            (&ortho1 * &normal).abs() <= f64::EPSILON,
            "Both orthogonal 1 must actually be orthogonal to the normal, meaning dot(o1, n) == 0.0"
        );
        assert!(
            (&ortho2 * &normal).abs() <= f64::EPSILON,
            "Both orthogonal 2 must actually be orthogonal to the normal, meaning dot(o2, n) == 0.0"
        );

        Self {
            origin,
            radius,
            normal,
            ortho1_unit: ortho1,
            ortho2_unit: ortho2,
        }
    }
}

impl Parametric<1, Point> for Circle3D {
    fn f(&self, ts: &[f64; 1]) -> Point {
        &self.origin
            + &(&(&self.ortho1_unit * (self.radius * ts[0].cos()))
                + &(&self.ortho2_unit * (self.radius * ts[0].sin())))
    }

    fn range(&self) -> [[f64; 2]; 1] {
        [[0.0, 2.0 * f64::consts::PI]]
    }

    fn derive(&self, ts: &[f64; 1], _: usize) -> Point {
        &self.origin
            - &(&(self.radius * ts[0].sin() * &self.ortho1_unit)
                + &(self.radius * ts[0].cos() * &self.ortho1_unit))
    }
}
