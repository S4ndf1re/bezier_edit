use crate::nurbs::point::Point;

/// Constraint Curve into a c1 continuitiy to another curve.
/// NOTE: since we are handeling bezier surfaces, it may be helpful to remember that u0=0, u1=1
/// and u2=2, hence all inner constraint distance relations are 1
pub struct C1Constraint {
    /// The point that holds the value of the non included point, e.g. of a possible other surface
    point: Point,
    /// The theoretical index if the point would be within the point list
    theoretical_relative_coordinates: (i32, i32),
    /// The index of the point that is included within the coliniar relation. Meaning the third
    /// point may be computed doing the following: referenced_point - theoretical_relative_coordinates = direction from referenced_point to respect the c1 constraint
    referenced_point: (usize, usize),
}

impl C1Constraint {
    pub fn new(
        point: Point,
        theoretical_relative_coordinates: (i32, i32),
        referenced_point: (usize, usize),
    ) -> Self {
        Self {
            point,
            theoretical_relative_coordinates,
            referenced_point,
        }
    }

    pub fn compute(&self, points: &[Vec<Point>]) -> Option<Point> {
        let ref_point = points
            .get(self.referenced_point.0)?
            .get(self.referenced_point.1)?;

        let diff = ref_point - &self.point;

        Some(ref_point + &diff)
    }

    /// Invert the compute operation, meaning that, considering the ref_point and the target point,
    /// comput the point that is not part of points, that is needed to fullfill C1 continuity
    pub fn inverse(&self, points: &[Vec<Point>]) -> Option<Point> {
        let idx = self.target_idx();

        let point = points.get(idx.0)?.get(idx.1)?;

        let ref_point = points
            .get(self.referenced_point.0)?
            .get(self.referenced_point.1)?;

        let diff = ref_point - point;

        Some(ref_point + &diff)
    }

    pub fn target_idx(&self) -> (usize, usize) {
        let direction = (
            self.referenced_point.0 as i32 - self.theoretical_relative_coordinates.0,
            self.referenced_point.1 as i32 - self.theoretical_relative_coordinates.1,
        );

        // This can sefely be cast to usize, since the index will always be > 0
        (
            (self.referenced_point.0 as i32 + direction.0) as usize,
            (self.referenced_point.1 as i32 + direction.1) as usize,
        )
    }
}

pub struct Constraints {
    pub c1_constraints: Vec<C1Constraint>,
}

pub struct Solver;

impl Solver {
    pub fn solve_constraints(points: &[Vec<Point>], constraints: Constraints) -> Vec<Vec<Point>> {
        let mut points = points.to_vec();

        for c1 in constraints.c1_constraints.iter() {
            let target_idx = c1.target_idx();
            if let Some(new_point) = c1.compute(&points)
                && let Some(Some(point)) = points
                    .get_mut(target_idx.0)
                    .map(|l| l.get_mut(target_idx.1))
            {
                *point = new_point;
            }
        }

        points
    }
}
