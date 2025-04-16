use crate::nurbs::point::Point;
use crate::nurbs::util::n_choose_k;
use num::pow;
use std::borrow::Borrow;

pub fn de_casteljau<T: AsRef<[Point]>>(points: T, t: f64) -> Vec<Vec<Point>> {
    let points = points.as_ref();
    let mut stages = Vec::new();

    let mut stage = Vec::new();
    for p in points {
        stage.push(p.clone());
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

pub fn derive_after_de_casteljau(points: &Vec<Vec<Point>>) -> Point {
    let n = points.len() - 1;

    &points[n - 1][1] - &points[n - 1][0]
}

pub fn atiken(points: &[Point], ts: &[f64], t: f64) -> Vec<Vec<Point>> {
    assert_eq!(points.len(), ts.len());

    let mut stages = Vec::new();
    stages.reserve(points.len());

    let n = points.len() - 1;

    let mut stage = Vec::new();
    for p in points {
        stage.push(p.clone());
    }
    stages.push(stage);

    for r in 1..=n {
        let mut new_stage = Vec::new();
        for i in 0..=n - r {
            let pri = &((ts[i + r] - t) / (ts[i + r] - ts[i]) * &stages[r - 1][i])
                + &(&(t - ts[i]) / (ts[i + r] - ts[i]) * &stages[r - 1][i + 1]);
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
    k = k + 1;

    for i in 1..points.len() {
        let p = &points[i];
        let n_c_k = n_choose_k(n, k) as f64;
        k = k + 1;

        factor = &((1.0 - t) * &factor) + &(pow(t, i) * n_c_k * p);
    }

    factor
}
