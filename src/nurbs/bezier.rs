use num::pow;

use crate::nurbs::point::Point;
use crate::nurbs::util::n_choose_k;

use super::util::factorial;

pub fn de_casteljau<T: AsRef<[Point]>>(points: T, t: f64) -> Vec<Vec<Point>> {
    let points = points.as_ref();
    let mut stages = Vec::new();

    let mut stage = Vec::new();
    for p in points {
        stage.push(*p);
    }
    stages.push(stage);

    while stages.last().unwrap().len() >= 2 {
        let mut new_stage = Vec::new();
        for i in 0..stages.last().unwrap().len() - 1 {
            let w_i = (1.0 - t) * stages.last().unwrap()[i].w + t * stages.last().unwrap()[i + 1].w;

            let p_i = &stages.last().unwrap()[i];
            let p_i1 = &stages.last().unwrap()[i + 1];
            new_stage.push(&((1.0 - t) * (p_i.w / w_i) * p_i) + &(t * (p_i1.w / w_i) * p_i1));
        }
        stages.push(new_stage);
    }

    stages
}

pub fn derive_after_de_casteljau(points: &[Vec<Point>], r: usize) -> Point {
    let n = points.len() - 1;

    let mut sum = Point::default();
    for j in 0..=r {
        sum = &sum + &(&points[n - r][j] * ((n_choose_k(r, j) as f64) * pow(-1.0, r - j)));
    }

    ((factorial(n) / factorial(n - r)) as f64) * &sum
}

#[allow(unused)]
pub fn atiken(points: &[Point], ts: &[f64], t: f64) -> Vec<Vec<Point>> {
    assert_eq!(points.len(), ts.len());

    let mut stages = Vec::with_capacity(points.len());

    let n = points.len() - 1;

    let mut stage = Vec::new();
    for p in points {
        stage.push(*p);
    }
    stages.push(stage);

    for r in 1..=n {
        let mut new_stage = Vec::new();
        for i in 0..=n - r {
            let pri = &((ts[i + r] - t) / (ts[i + r] - ts[i]) * &stages[r - 1][i])
                + &((t - ts[i]) / (ts[i + r] - ts[i]) * &stages[r - 1][i + 1]);
            new_stage.push(pri);
        }
        stages.push(new_stage);
    }

    stages
}

#[allow(unused)]
pub fn horner_scheme<T: AsRef<[Point]>>(points: T, t: f64) -> Point {
    let points = points.as_ref();
    let n = points.len() - 1;
    let mut k = 0;

    let mut factor = n_choose_k(n, k) as f64 * &points[0];
    k += 1;

    for (i, p) in points.iter().enumerate() {
        let n_c_k = n_choose_k(n, k) as f64;
        k += 1;

        factor = &((1.0 - t) * &factor) + &(pow(t, i) * n_c_k * p);
    }

    factor
}

/// Split a bezier curve at parameter t, resulting in two sub bezier lines with n control points.
/// Lower is the splitted line defined in the interval [0, t] whereas upper is defined in the
/// interval [c, 1]. Each resunting curve will be defined in [0,1], each.
pub fn split_at<T: AsRef<[Point]>>(points: T, t: f64) -> (Vec<Point>, Vec<Point>) {
    let decas = de_casteljau(points.as_ref(), t);

    let mut left_half = Vec::with_capacity(points.as_ref().len());
    let mut right_half = Vec::with_capacity(points.as_ref().len());

    let n = points.as_ref().len();
    for i in 0..n {
        left_half.push(decas[i][0]);
        right_half.push(decas[n - i - 1][i]);
    }

    (left_half, right_half)
}

/// Compute the parameter t in the interval of [0, 1] (clamped) that has the shortest distance with point p
/// Code is taken from https://stackoverflow.com/questions/2742610/closest-point-on-a-cubic-bezier-curve and translated into this rust implementation
pub fn shortest_distance_to_point<T: AsRef<[Point]>>(points: T, point: Point) -> (f64, Point, f64) {
    // TODO replace this algorithm with a numerically stable solution, and not just the
    // brute-force solution

    let mut min = f64::MAX;
    let mut min_index = f64::MAX;
    let first_scans_counter = 50; // Modify using a resolution flag
    let eps = 1e-10;

    for scan in 0..=first_scans_counter {
        let t = (scan as f64) / (first_scans_counter as f64);
        let p = horner_scheme(&points, t);
        let diff = &point - &p;
        let distance = diff.magnitude();
        if distance < min {
            min = distance;
            min_index = scan as f64;
        }
    }

    // Now that the probable min index is found, use binary search to actually find the min index.
    // (clapmed)

    let t0 = ((min_index - 1.0) / first_scans_counter as f64).max(0.0);
    let t1 = ((min_index + 1.0) / first_scans_counter as f64).min(1.0);
    let f = |t| (&point - &horner_scheme(&points, t)).magnitude();

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

    (k, horner_scheme(&points, k), f(k))
}
