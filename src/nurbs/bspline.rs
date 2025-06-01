use crate::nurbs::bezier::horner_scheme;
use crate::nurbs::point::Point;
use nalgebra::DMatrix;
use nalgebra::DVector;
use num::pow::Pow;
use std::ops::Mul;

fn delta_u_i(us: &[f64], i: usize) -> f64 {
    us[i + 1] - us[i]
}
fn de_boor_idx(i: i32) -> usize {
    if i < -1 {
        panic!("i must not be less than -1. i={i}");
    }
    (i + 1) as usize
}

pub fn generate_intervals_chordale_distance<T: AsRef<[Point]>>(ds: T, l: usize) -> Vec<f64> {
    let ds = ds.as_ref();
    if ds.len() != l + 3 {
        panic!(
            "ds length mismatch. Must match l+3={}, but is {}",
            l + 3,
            ds.len()
        );
    }

    if l == 1 {
        return vec![0.0, 1.0];
    }

    let mut result = vec![0.0; l + 1];
    result[0] = 0.0;
    result[1] = (&ds[de_boor_idx(1)] - &ds[de_boor_idx(-1)]).magnitude();

    for i in 2..=l - 1 {
        result[i] = result[i - 1]
            + (&ds[de_boor_idx(i as i32)] - &ds[de_boor_idx((i - 1) as i32)]).magnitude();
    }

    result[l] = result[l - 1]
        + (&ds[de_boor_idx((l + 1) as i32)] - &ds[de_boor_idx((l - 1) as i32)]).magnitude();

    result
}

fn generate_control_points<T: AsRef<[Point]>>(ds: T, us: &[f64], l: usize) -> Vec<Point> {
    let ds = ds.as_ref();
    let mut bs = vec![Point::new(0.0, 0.0, 0.0, None); 3 * l + 1];

    bs[0] = ds[de_boor_idx(-1)];
    bs[1] = ds[de_boor_idx(0)];
    bs[2] = &(delta_u_i(us, 1) / (delta_u_i(us, 0) + delta_u_i(us, 1)) * &ds[de_boor_idx(0)])
        + &(delta_u_i(us, 0) / (delta_u_i(us, 0) + delta_u_i(us, 1)) * &ds[de_boor_idx(1)]);

    for i in 2..=l - 1 {
        let delta = delta_u_i(us, i - 2) + delta_u_i(us, i - 1) + delta_u_i(us, i);
        bs[3 * i - 2] = &((delta_u_i(us, i - 1) + delta_u_i(us, i)) / delta
            * &ds[de_boor_idx((i - 1) as i32)])
            + &(delta_u_i(us, i - 2) / delta * &ds[de_boor_idx(i as i32)]);

        bs[3 * i - 1] = &((delta_u_i(us, i - 2) + delta_u_i(us, i - 1)) / delta
            * &ds[de_boor_idx((i) as i32)])
            + &(delta_u_i(us, i) / delta * &ds[de_boor_idx((i - 1) as i32)]);
    }

    bs[3 * l - 2] = &(delta_u_i(us, l - 1) / (delta_u_i(us, l - 2) + delta_u_i(us, l - 1))
        * &ds[de_boor_idx((l - 1) as i32)])
        + &(delta_u_i(us, l - 2) / (delta_u_i(us, l - 2) + delta_u_i(us, l - 1))
            * &ds[de_boor_idx(l as i32)]);
    bs[3 * l - 1] = ds[de_boor_idx(l as i32)];
    bs[3 * l] = ds[de_boor_idx((l + 1) as i32)];

    for i in 1..=l - 1 {
        bs[3 * i] = &(delta_u_i(us, i) / (delta_u_i(us, i - 1) + delta_u_i(us, i))
            * &bs[3 * i - 1])
            + &(delta_u_i(us, i - 1) / (delta_u_i(us, i - 1) + delta_u_i(us, i)) * &bs[3 * i + 1]);
    }

    bs
}

pub fn build_c2_spline<T: AsRef<[Point]>>(
    points: T,
    l: usize,
) -> (Vec<Point>, Vec<f64>, Vec<Point>) {
    let points = points.as_ref();
    let us = generate_intervals_chordale_distance(points, l);
    let bs = generate_control_points(points, &us[..], l);

    (bs, us, Vec::from(points))
}

fn find_us_idx<S: AsRef<[f64]>>(us: S, u: f64) -> Option<usize> {
    let us = us.as_ref();
    let mut idx = None;

    // U must be between the whole interval
    assert!(us[0] <= u && u <= us[us.len() - 1]);

    // TODO optimize using binary search
    for i in 0..us.len() - 1 {
        if us[i] <= u && u <= us[i + 1] {
            idx = Some(i);
            break;
        }
    }

    idx
}

pub fn eval_bspline<T: AsRef<[Point]>, S: AsRef<[f64]>>(points: T, us: S, u: f64) -> Point {
    let points = points.as_ref();
    let us = us.as_ref();

    let idx = find_us_idx(us, u).expect("Interval must be found");

    let t = (u - us[idx]) / (us[idx + 1] - us[idx]);

    horner_scheme(points, t)
}

fn x_diff(xs: &[f64], i: i32, l: usize) -> f64 {
    if i < 0 {
        0.0
    } else if i == l as i32 {
        0.0
    } else {
        xs[(i + 1) as usize] - xs[i as usize]
    }
}

fn solve<T: AsRef<[f64]>, S: AsRef<[f64]>>(
    mat: &DMatrix<f64>,
    points: T,
    us: S,
    l: usize,
) -> Vec<f64> {
    let us = us.as_ref();
    let points = points.as_ref();
    let mut rhs = DVector::<f64>::from_element(l + 1, 0.0);

    rhs[0] = points[0];
    rhs[l] = points[l];

    for i in 0..=l - 1 {
        let i = i as i32;
        rhs[i as usize] = (x_diff(us, i - 1, l) + x_diff(us, i, l)) * points[i as usize];
    }

    let d_vec = mat.mul(rhs);
    assert!(d_vec.ncols() == 1 && d_vec.nrows() == l + 1);

    let mut result = vec![0.0; l + 3];
    result[0] = points[0];
    result[l + 2] = points[l];

    for i in 1..=l + 1 {
        result[i] = d_vec[i - 1];
    }

    result
}
pub fn cubic_bspline_interpolation<T: AsRef<[Point]>>(
    points: T,
    l: usize,
) -> (Vec<Point>, Vec<f64>, Vec<Point>) {
    let points = points.as_ref();

    // Generate intervals using chordale distance
    let us = {
        let mut us = vec![0.0; l + 1];
        us[0] = 0.0;
        for i in 1..=l {
            us[i] = us[i - 1] + (&points[i] - &points[i - 1]).magnitude();
        }
        let us_max = us.last().expect("Interval must be found");
        us.iter()
            .map(|&u| u / us_max * l as f64)
            .collect::<Vec<f64>>()
    };

    let mut alpha = vec![0.0; l + 1];
    let mut beta = vec![0.0; l + 1];
    let mut gamma = vec![0.0; l + 1];

    for i in 1..l {
        let i = i as i32;
        let delta_1 = x_diff(&us, i - 2, l) + x_diff(&us, i - 1, l) + x_diff(&us, i, l);
        let delta_2 = x_diff(&us, i - 1, l) + x_diff(&us, i + 1, l) + x_diff(&us, i, l);

        alpha[i as usize] = x_diff(&us, i, l).pow(2) / delta_1;
        beta[i as usize] = x_diff(&us, i, l) * (x_diff(&us, i - 2, l) + x_diff(&us, i - 1, l))
            / delta_1
            + x_diff(&us, i - 1, l) * (x_diff(&us, i, l) + x_diff(&us, i + 1, l)) / delta_2;
        gamma[i as usize] = x_diff(&us, i - 1, l).pow(2) / delta_2;
    }

    let points_x: Vec<f64> = points.iter().map(|p| p.x).collect();
    let points_y: Vec<f64> = points.iter().map(|p| p.y).collect();
    let points_z: Vec<f64> = points.iter().map(|p| p.z).collect();

    let mut matrix = DMatrix::<f64>::from_element(l + 1, l + 1, 0.0);
    matrix[(0, 0)] = 1.0;
    matrix[(l, l)] = 1.0;
    for i in 1..=l - 1 {
        matrix[(i, i - 1)] = alpha[i];
        matrix[(i, i)] = beta[i];
        matrix[(i, i + 1)] = gamma[i];
    }

    let m_inv = matrix
        .try_inverse()
        .expect("Inverse matrix must be invertible");

    let dx = solve(&m_inv, &points_x, &us, l);
    let dy = solve(&m_inv, &points_y, &us, l);
    let dz = solve(&m_inv, &points_z, &us, l);

    let mut d = vec![Point::new(0.0, 0.0, 0.0, None); l + 3];
    for i in 0..dx.len() {
        d[i] = Point::new(dx[i], dy[i], dz[i], None);
    }

    let bs = generate_control_points(&d, &us[..], l);

    (bs, us, d)
}

pub fn de_boor<T: AsRef<[Point]>, S: AsRef<[f64]>>(
    control: T,
    us: S,
    u: f64,
    n: usize,
    l: usize,
) -> Point {
    let control = control.as_ref();
    let us = us.as_ref();
    assert!(us[n - 1] <= u && u <= us[l + n - 1], "s(u) is not defined");

    let idx = find_us_idx(&us, u).expect("Interval must be found");

    let rank = us
        .iter()
        .filter(|ui| (**ui - u).abs() <= f64::EPSILON)
        .count();
    if rank >= n {
        return control[idx];
    }

    let mut stages = Vec::new();
    stages.reserve(n + 1);

    let mut stage = vec![Point::new(0.0, 0.0, 0.0, None); n + 1];
    for i in idx + 1 - n..=idx + 1 {
        stage[(idx + 1) - i] = control[i];
    }
    stages.push(stage);

    for k in 1..=n - rank {
        stage = vec![Point::new(0.0, 0.0, 0.0, None); n + 1 - k];
        for i in idx + 1 - n + k..=idx + 1 {
            let d_i = &stages[k - 1][(idx + 1) - i];
            let d_i1 = &stages[k - 1][(idx + 1) - (i - 1)];

            let alpha_k_i = (u - us[i - 1]) / (us[i + n - k] - us[i - 1]);
            let w_k_i = (1.0 - alpha_k_i) * d_i1.w + alpha_k_i * d_i.w;

            stage[(idx + 1) - i] = &(&((1.0 - alpha_k_i) * d_i1.w * d_i1)
                + &(alpha_k_i * d_i.w * d_i))
                * (1.0 / w_k_i);
        }
        stages.push(stage);
    }

    stages[n - rank][0] // This leads to the idx to always be 0, since 0 = idx+1 - (idx+1)
}
