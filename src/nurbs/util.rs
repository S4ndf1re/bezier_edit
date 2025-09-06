use crate::nurbs::point::Point;
use num::pow;

#[allow(unused)]
pub fn sum(n: i32) -> i32 {
    (n * n + n) / 2
}

pub fn factorial(n: usize) -> usize {
    (1..=n).product::<usize>()
}

pub fn n_choose_k(n: usize, k: usize) -> usize {
    let mut mul = 1.0;
    for i in 1..=k {
        mul *= ((n as f64) - (i as f64) + 1.0) / (i as f64);
    }

    // NOTE(Jan): This is from a competetive programming course. Floating point
    // errors should be accounted for by this line
    (mul + 0.01) as usize
}

#[allow(unused)]
pub fn bernstein(i: usize, n: usize, t: f64) -> f64 {
    n_choose_k(n, i) as f64 * pow(t, i) * pow(1.0 - t, n - i)
}

#[allow(unused)]
pub fn bernstein_to_point(i: usize, r: usize, t: f64, bezier_points: &[Point]) -> Point {
    let mut sum = Point::new(0.0, 0.0, 0.0, None);

    if r + i >= bezier_points.len() {
        panic!(
            "r+i = {} must be less than bezier_points.len() = {}",
            r + i,
            bezier_points.len() - 1
        );
    }

    for j in 0..=r {
        sum = sum + (bezier_points[j + r] * bernstein(j, r, t));
    }

    sum
}

#[allow(unused)]
pub fn cubic_hermite_polynome(i: usize, t: f64) -> f64 {
    match i {
        0 => bernstein(0, 3, t) + bernstein(1, 3, t),
        1 => 1.0 / 3.0 * bernstein(1, 3, t),
        2 => -1.0 / 3.0 * bernstein(2, 3, t),
        3 => bernstein(2, 3, t) + bernstein(3, 3, t),
        _ => panic!("i must be contained within the interval [0, 3]. i = {i}"),
    }
}

#[allow(unused)]
pub fn cubic_hermite_polynome_hat(i: usize, t: f64, interval: (f64, f64)) -> f64 {
    let (a, b) = interval;
    match i {
        0 => cubic_hermite_polynome(0, t),
        1 => (b - a) * cubic_hermite_polynome(1, t),
        2 => (b - a) * cubic_hermite_polynome(2, t),
        3 => cubic_hermite_polynome(3, t),
        _ => panic!("i must be contained within the interval [0, 3]. i = {i}"),
    }
}
