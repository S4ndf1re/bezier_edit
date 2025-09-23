use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use bevy::{ecs::resource::Resource, math::Vec3};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

use crate::nurbs::{
    bezier_plane::{ControlPoints2D, eval_2d_bezier_curves},
    point::Point,
};

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct TestControlPoint {
    y_idx: usize,
    x_idx: usize,
    point: (f64, f64, f64),
}

impl From<(usize, usize, Vec3)> for TestControlPoint {
    fn from((y_idx, x_idx, point): (usize, usize, Vec3)) -> Self {
        Self {
            y_idx,
            x_idx,
            point: (point.x as f64, point.y as f64, point.z as f64),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct PointEvaluation {
    u: f64,
    v: f64,
    reference: (f64, f64, f64),
    surface_point: (f64, f64, f64),
    dist: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SingleSurfaceEvaluation {
    reference_control_points: Vec<TestControlPoint>,
    test_control_points: Vec<TestControlPoint>,
    resolution: usize,
    evaluations: Vec<PointEvaluation>,
    evaluated: bool,
}

impl SingleSurfaceEvaluation {
    pub fn new_unevaluated(reference: Vec<TestControlPoint>, resolution: usize) -> Self {
        Self {
            reference_control_points: reference,
            resolution,
            test_control_points: Vec::new(),
            evaluations: Vec::new(),
            evaluated: false,
        }
    }

    fn point_list_to_control_2d(control_points: &[TestControlPoint]) -> ControlPoints2D {
        let mut points = HashMap::<usize, Vec<(usize, Point)>>::new();
        for point in control_points.iter() {
            let curve = points.entry(point.y_idx).or_default();
            curve.push((point.x_idx, Point::from(point.point)));
        }

        // First get all points in order for each sub curve
        let mut multi_curves = Vec::<(usize, Vec<Point>)>::new();
        for (i, points) in points.iter_mut() {
            points.sort_by(|a, b| a.0.cmp(&b.0));
            let points = points.iter().map(|p| p.1).collect::<Vec<_>>();
            multi_curves.push((*i, points));
        }

        // Then order the subcurves by index
        multi_curves.sort_by(|a, b| a.0.cmp(&b.0));
        let multi_curves: Vec<Vec<Point>> =
            multi_curves.iter().map(|p| p.1.clone()).collect::<Vec<_>>();

        multi_curves
    }

    /// Evaluate single surface
    pub fn evaluate(&mut self, control_points: Vec<TestControlPoint>) {
        self.test_control_points = control_points;
        let collected_test = Self::point_list_to_control_2d(&self.test_control_points);
        let collected_reference = Self::point_list_to_control_2d(&self.reference_control_points);

        for u in 0..=self.resolution {
            let u = u as f64 / self.resolution as f64;
            for v in 0..=self.resolution {
                let v = v as f64 / self.resolution as f64;

                let test_point = eval_2d_bezier_curves(&collected_test, u, v);
                let reference_point = eval_2d_bezier_curves(&collected_reference, u, v);

                let dist = (reference_point - test_point).magnitude();

                self.evaluations.push(PointEvaluation {
                    u,
                    v,
                    reference: reference_point.into(),
                    surface_point: test_point.into(),
                    dist,
                });
            }
        }

        self.evaluated = true;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationSurface {
    control_points: Vec<TestControlPoint>,
}

impl From<EvaluationSurface> for SingleSurfaceEvaluation {
    fn from(value: EvaluationSurface) -> Self {
        Self::new_unevaluated(value.control_points, 100)
    }
}

#[derive(Serialize, Deserialize, Resource)]
pub struct Evaluation {
    counter: usize,
    reference_surfaces: Vec<EvaluationSurface>,
    evaluations: Vec<SingleSurfaceEvaluation>,
}

impl Evaluation {
    pub fn from_json(file: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(file)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let reference_surfaces: Vec<EvaluationSurface> = serde_json::from_str(&content)?;

        Ok(Self {
            evaluations: Vec::new(),
            reference_surfaces,
            counter: 0,
        })
    }

    pub fn add_reference_surface(&mut self, control_points: Vec<TestControlPoint>) {
        self.reference_surfaces
            .push(EvaluationSurface { control_points });
    }

    /// Start e new evaluation, if possible
    pub fn start_next_evaluation(&mut self) -> Option<Vec<TestControlPoint>> {
        if self.counter < self.reference_surfaces.len() {
            self.evaluations
                .push(self.reference_surfaces[self.counter].clone().into());

            Some(
                self.evaluations
                    .last()
                    .unwrap()
                    .reference_control_points
                    .clone(),
            )
        } else {
            None
        }
    }

    /// End evaluation and make state ready for next evaluation
    pub fn end_evaluation(&mut self, control_points: Vec<TestControlPoint>) {
        if let Some(last) = self.evaluations.last_mut() {
            last.evaluate(control_points);
            self.counter += 1;
        }
    }

    pub fn consolidate_to_json(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(path)?;

        let content = serde_json::to_string(&self)?;

        file.write_all(content.as_bytes())?;

        Ok(())
    }
}

impl Drop for Evaluation {
    fn drop(&mut self) {
        let timestamp = Utc::now();
        let pathbuf = PathBuf::from(format!("{timestamp}.json"));

        let _ = self.consolidate_to_json(pathbuf.as_path());
    }
}
