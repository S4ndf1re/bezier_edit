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
            let diff = target - p;
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
        let f = |t| (target - self.f(&[t])).magnitude();

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

impl<T> MinDistanceToPoint<2, Point> for T
where
    T: Parametric<2, Point>,
{
    fn min_distance_to_point(&self, target: Point) -> MinDistanceResult<2, Point> {
        let mut min = f64::MAX;
        let mut min_index = [f64::MAX; 2];
        let first_scans_counter = 100; // Modify using a resolution flag
        let eps = 1e-10;

        for i in 0..=first_scans_counter {
            for j in 0..=first_scans_counter {
                let u = (i as f64) / (first_scans_counter as f64);
                let v = (j as f64) / (first_scans_counter as f64);
                let p = self.f(&[u, v]);
                let diff = target - p;
                let distance = diff.magnitude();
                if distance < min {
                    min = distance;
                    min_index = [i as f64, j as f64];
                }
            }
        }

        // Now that the probable min index is found, use binary search to actually find the min index.
        // (clamped)

        let (t0_u, t0_v) = (
            ((min_index[0] - 1.0) / first_scans_counter as f64).max(0.0),
            ((min_index[1] - 1.0) / first_scans_counter as f64).max(0.0),
        );
        let (t1_u, t1_v) = (
            ((min_index[0] + 1.0) / first_scans_counter as f64).min(1.0),
            ((min_index[1] + 1.0) / first_scans_counter as f64).min(1.0),
        );
        let f = |u, v| (target - self.f(&[u, v])).magnitude();

        let (mut n_u, mut n_v) = (t0_u, t0_v);
        let (mut m_u, mut m_v) = (t1_u, t1_v);
        let mut k_u = 0.0;
        let mut k_v = 0.0;

        while (m_u - n_u) > eps && (m_v - n_v) > eps {
            k_u = (n_u + m_u) / 2.0;
            k_v = (n_v + m_v) / 2.0;

            if f(k_u - eps, k_v) < f(k_u + eps, k_v) {
                m_u = k_u;
            } else {
                n_u = k_u;
            }

            if f(k_u, k_v - eps) < f(k_u, k_v + eps) {
                m_v = k_v;
            } else {
                n_v = k_v;
            }
        }

        MinDistanceResult {
            params: [k_u, k_v],
            value: self.f(&[k_u, k_v]),
            distance: f(k_u, k_v),
        }
    }
}
