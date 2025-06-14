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
