use num::pow::Pow;

use crate::nurbs::bezier::{de_casteljau, derive_after_de_casteljau, split_at};
use crate::nurbs::point::Point;

use super::bounding_box_3d::BoundingBox3D;

pub type ControlPoints2D = Vec<Vec<Point>>;

/// Assume a mxn grid of control points, where m and n are degrees of a bezier curve, meaning m+1 and n+1 points are needed.
/// First compute m+1 bezier curves of degree n. Use the resulting points to compute a single bezier curve of m+1 points
pub fn eval_2d_bezier_curves(control_points: &ControlPoints2D, u: f64, v: f64) -> Point {
    let m = control_points.len() - 1;
    assert!(m > 1);

    let n = control_points
        .first()
        .unwrap() // Ok, since assert! prevents empty lists
        .len()
        - 1;
    assert!(n > 1);

    let mut new_control = vec![Point::new(0.0, 0.0, 0.0, None); m + 1];

    for i in 0..=m {
        new_control[i] = *de_casteljau(&control_points[i], u)
            .last()
            .unwrap()
            .last()
            .unwrap();
    }

    *de_casteljau(&new_control, v)
        .last()
        .unwrap()
        .last()
        .unwrap()
}

pub fn derive_2d(control_points: &ControlPoints2D, u: f64, v: f64, r: usize) -> (Point, Point) {
    let m = control_points.len() - 1;
    assert!(m > 1);

    let n = control_points
        .first()
        .unwrap() // Ok, since assert! prevents empty lists
        .len()
        - 1;
    assert!(n > 1);

    // Compute the control points the same as for the eval2d function, but use derive_after_de_casteljau for derivative control points
    let mut new_control = vec![Point::new(0.0, 0.0, 0.0, None); m + 1];

    for i in 0..=m {
        new_control[i] = derive_after_de_casteljau(&de_casteljau(&control_points[i], u), r)
    }

    let u_diff = *de_casteljau(&new_control, v)
        .last()
        .unwrap()
        .last()
        .unwrap();

    // Do the same for the v vector. Notice however, that we have to invert column and row storage, and also swap parameters
    let mut new_control = vec![Point::new(0.0, 0.0, 0.0, None); m + 1];

    #[allow(clippy::needless_range_loop)]
    for i in 0..=n {
        let mut inner_control_points = Vec::with_capacity(m + 1);
        for j in 0..=m {
            inner_control_points.push(control_points[j][i]);
        }

        new_control[i] = derive_after_de_casteljau(&de_casteljau(&inner_control_points, v), r)
    }

    let v_diff = *de_casteljau(&new_control, u)
        .last()
        .unwrap()
        .last()
        .unwrap();

    (u_diff, v_diff)
}

/// Imagine a top down view on a 2d surface. The structure represents the split in the middle on
/// both axis
pub struct SurfaceSplit {
    pub bottom_left: ControlPoints2D,
    pub bottom_right: ControlPoints2D,

    pub top_left: ControlPoints2D,
    pub top_right: ControlPoints2D,
}

/// Split a surface into 4 parts. View `SurfaceSplit` for further information
pub fn split_surface(control_points: &ControlPoints2D, u: f64, v: f64) -> SurfaceSplit {
    let m = control_points.len() - 1;
    assert!(m > 1);

    let n = control_points
        .first()
        .unwrap() // Ok, since assert! prevents empty lists
        .len()
        - 1;
    assert!(n > 1);

    let mut upper = Vec::with_capacity(m + 1);
    let mut lower = Vec::with_capacity(m + 1);

    for inner_points in control_points {
        let (lower_splitted, upper_splitted) = split_at(inner_points, u);
        upper.push(upper_splitted);
        lower.push(lower_splitted);
    }

    let mut upper_left = vec![vec![Point::default(); n + 1]; m + 1];
    let mut upper_right = vec![vec![Point::default(); n + 1]; m + 1];

    for i in 0..=n {
        let mut tmp_points = Vec::with_capacity(m + 1);
        // Iterate over all inner
        for inner_points in upper.iter() {
            tmp_points.push(inner_points[i]);
        }

        let (left_splitted, right_splitted) = split_at(tmp_points, v);

        for j in 0..=m {
            upper_left[j][i] = left_splitted[j];
            upper_right[j][i] = right_splitted[j];
        }
    }

    let mut lower_left = vec![vec![Point::default(); n + 1]; m + 1];
    let mut lower_right = vec![vec![Point::default(); n + 1]; m + 1];

    for i in 0..=n {
        let mut tmp_points = Vec::with_capacity(m + 1);
        // Iterate over all inner
        for inner_points in lower.iter() {
            tmp_points.push(inner_points[i]);
        }

        let (left_splitted, right_splitted) = split_at(tmp_points, v);

        for j in 0..=m {
            lower_left[j][i] = left_splitted[j];
            lower_right[j][i] = right_splitted[j];
        }
    }

    SurfaceSplit {
        bottom_left: lower_left,
        bottom_right: lower_right,
        top_left: upper_left,
        top_right: upper_right,
    }
}

pub fn minmax_box(points: &ControlPoints2D) -> BoundingBox3D {
    let mut bbox = BoundingBox3D::new();

    for list in points.iter() {
        for p in list.iter() {
            bbox.add_point(p);
        }
    }

    bbox
}

fn determine_u_v_rec(
    points: &ControlPoints2D,
    to_test: &Point,
    real_u: f64,
    real_v: f64,
    depth: f64,
    box_volume_threshold: f64,
) -> Option<Vec<(f64, f64)>> {
    let bbox = minmax_box(points);
    if !bbox.contains_point(to_test) {
        return None;
    }

    if bbox.volume() < box_volume_threshold {
        return Some(vec![(real_u, real_v)]);
    }

    let splitted = split_surface(points, 1.0 / 2.0, 1.0 / 2.0);
    let mut result = vec![];

    let next_center = (1.0 / 2.0).pow(depth + 1.0);

    if let Some(solutions) = determine_u_v_rec(
        &splitted.bottom_left,
        to_test,
        real_u - next_center,
        real_v - next_center,
        depth + 1.0,
        box_volume_threshold,
    ) {
        result.extend(solutions);
    }

    if let Some(solutions) = determine_u_v_rec(
        &splitted.bottom_right,
        to_test,
        real_u - next_center,
        real_v + next_center,
        depth + 1.0,
        box_volume_threshold,
    ) {
        result.extend(solutions);
    }

    if let Some(solutions) = determine_u_v_rec(
        &splitted.top_left,
        to_test,
        real_u + next_center,
        real_v - next_center,
        depth + 1.0,
        box_volume_threshold,
    ) {
        result.extend(solutions);
    }

    if let Some(solutions) = determine_u_v_rec(
        &splitted.top_right,
        to_test,
        real_u + next_center,
        real_v + next_center,
        depth + 1.0,
        box_volume_threshold,
    ) {
        result.extend(solutions);
    }

    if result.is_empty() {
        return None;
    }

    Some(result)
}

/// Try to find u and v coordinate using numerical approximation until each sub bounding box has
/// volume `box_volume_threshold`.
pub fn determine_u_v(
    points: &ControlPoints2D,
    to_test: &Point,
    box_volume_threshold: f64,
) -> Option<Vec<(f64, f64)>> {
    if !minmax_box(points).contains_point(to_test) {
        return None;
    }

    determine_u_v_rec(
        points,
        to_test,
        1.0 / 2.0,
        1.0 / 2.0,
        1.0,
        box_volume_threshold,
    )
}
