use crate::nurbs::bezier::{de_casteljau, derive_after_de_casteljau};
use crate::nurbs::point::Point;

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

pub fn derive_2d(control_points: &ControlPoints2D, u: f64, v: f64) -> (Point, Point) {
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
        new_control[i] = derive_after_de_casteljau(&de_casteljau(&control_points[i], u))
    }

    let u_diff = *de_casteljau(&new_control, v)
        .last()
        .unwrap()
        .last()
        .unwrap();

    // Do the same for the v vector. Notice however, that we have to invert column and row storage, and also swap parameters
    let mut new_control = vec![Point::new(0.0, 0.0, 0.0, None); m + 1];

    for i in 0..=n {
        let mut inner_control_points = vec![];
        inner_control_points.reserve(m + 1);
        for j in 0..=m {
            inner_control_points.push(control_points[j][i]);
        }

        new_control[i] = derive_after_de_casteljau(&de_casteljau(&inner_control_points, v))
    }

    let v_diff = *de_casteljau(&new_control, u)
        .last()
        .unwrap()
        .last()
        .unwrap();

    (u_diff, v_diff)
}
