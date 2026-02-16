use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::bezier_curve::test_mode::{
    EvaluationType, PointEvaluation, TestControlPoint, ToRenderCommandsForEvaluation,
};

#[derive(Clone, Copy)]
pub struct BasicSurfaceEvaluationData {
    pub average_dist: f64,
    pub max_dist: f64,
    pub min_dist: f64,
    pub time: u128,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SingleEvaluation {
    pub average_dist: f64,
    pub max_dist: f64,
    pub min_dist: f64,
    pub reference_control_points: EvaluationType,
    pub test_control_points: Vec<TestControlPoint>,
    pub resolution: usize,
    pub evaluations: Vec<PointEvaluation>,
    pub evaluated: bool,
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    pub instant: Instant,
    pub time: u128,
}

impl SingleEvaluation {
    pub fn new_unevaluated(reference: EvaluationType, resolution: usize) -> Self {
        Self {
            reference_control_points: reference,
            resolution,
            test_control_points: Vec::new(),
            evaluations: Vec::new(),
            average_dist: 0.0,
            max_dist: f64::MIN,
            min_dist: f64::MAX,
            evaluated: false,
            instant: Instant::now(),
            time: 0,
        }
    }

    pub fn start(&mut self) {
        self.instant = Instant::now();
    }

    /// Evaluate single surface
    pub fn evaluate(
        &mut self,
        control_points: Vec<TestControlPoint>,
    ) -> BasicSurfaceEvaluationData {
        let duration = self.instant.elapsed();
        self.test_control_points = control_points;
        self.time = duration.as_millis();
        let reference = self.reference_control_points.clone();
        let mut result = reference.evaluate(self);

        // Fallback, in case the evalute implementation does set these values differently or not at all
        self.average_dist = result.average_dist;
        self.min_dist = result.min_dist;
        self.max_dist = result.max_dist;

        // Mark as evaluated, in case evalute implementation does not actually set this flag
        self.evaluated = true;

        // Set time
        result.time = self.time;
        result
    }
}
