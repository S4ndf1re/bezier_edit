use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use bevy::{color::palettes::css::BLACK, prelude::*};
use bevy::{ecs::resource::Resource, math::Vec3};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

use crate::{
    RootTransform,
    bezier_curve::{
        curvature_display_mode::CurvatureDisplayMode,
        render_info::RenderInformation,
        util::{SurfaceRenderMode, create_mesh_from_control_points},
    },
    nurbs::{
        bezier_plane::{ControlPoints2D, ToControlPoints2D, eval_2d_bezier_curves},
        point::Point,
    },
};

use super::{bezier_curve_renderer::ResetDefaultCurveEvent, components::RenderPoint};

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

impl From<TestControlPoint> for (usize, usize, Vec3) {
    fn from(value: TestControlPoint) -> Self {
        (
            value.y_idx,
            value.x_idx,
            Vec3::new(
                value.point.0 as f32,
                value.point.1 as f32,
                value.point.2 as f32,
            ),
        )
    }
}

impl ToControlPoints2D for &Vec<TestControlPoint> {
    fn to_control_points(self) -> ControlPoints2D {
        let mut points = HashMap::<usize, Vec<(usize, Point)>>::new();
        for point in self.iter() {
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
    average_dist: f64,
    max_dist: f64,
    min_dist: f64,
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
            average_dist: 0.0,
            max_dist: f64::MIN,
            min_dist: f64::MAX,
            evaluated: false,
        }
    }

    /// Evaluate single surface
    pub fn evaluate(&mut self, control_points: Vec<TestControlPoint>) {
        self.test_control_points = control_points;
        let collected_test = self.test_control_points.to_control_points();
        let collected_reference = self.reference_control_points.to_control_points();
        let mut avg_dist = 0.0;
        let n = (self.resolution + 1).pow(2) as f64;

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

                avg_dist += dist / n;
                self.max_dist = self.max_dist.max(dist);
                self.min_dist = self.min_dist.min(dist);
            }
        }

        self.average_dist = avg_dist;
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

#[derive(Serialize, Resource)]
pub struct Evaluation {
    #[serde(skip)]
    counter: usize,

    #[serde(skip)]
    reference_surfaces: Vec<EvaluationSurface>,
    #[serde(skip)]
    pathbuf_references: PathBuf,

    evaluations: Vec<SingleSurfaceEvaluation>,
}

impl Evaluation {
    pub fn from_json(path: &Path) -> Result<Self, Box<dyn Error>> {
        let mut file = if let Ok(file) = File::open(path) {
            file
        } else {
            let mut file = File::create(path)?;
            write!(file, "[]")?;
            file
        };

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        let reference_surfaces: Vec<EvaluationSurface> = serde_json::from_str(&content)?;

        Ok(Self {
            evaluations: Vec::new(),
            reference_surfaces,
            counter: 0,
            pathbuf_references: PathBuf::from(path),
        })
    }

    pub fn add_reference_surface(&mut self, control_points: Vec<TestControlPoint>) {
        self.reference_surfaces
            .push(EvaluationSurface { control_points });
    }

    /// Start e new evaluation, if possible
    pub fn start_next_evaluation(&mut self) -> Option<Vec<TestControlPoint>> {
        if self.can_evaluate_further() {
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
        if let Some(last) = self.evaluations.last_mut()
            && !last.evaluated
        {
            last.evaluate(control_points);
            self.counter += 1;
        }
    }

    /// Consolidate the evaluation and all possible new reference surfaces. The path is assumed to
    /// point to the evals directory (NOT FILE). The file is appended additionally using a
    /// timestamp
    pub fn consolidate_to_json(&self, mut path: PathBuf) -> Result<(), Box<dyn Error>> {
        // consolidate evaluations
        std::fs::create_dir_all(path.as_path())?;

        let timestamp = Utc::now();
        path.push(format!("{timestamp}.json"));

        let mut file = File::create(path)?;
        let content = serde_json::to_string(&self)?;
        file.write_all(content.as_bytes())?;

        // Consolidate possibly new reference surfaces
        let mut file = File::create(self.pathbuf_references.as_path())?;
        let content = serde_json::to_string(&self.reference_surfaces)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    pub fn can_evaluate_further(&self) -> bool {
        self.counter < self.reference_surfaces.len()
    }
}

impl Drop for Evaluation {
    fn drop(&mut self) {
        let pathbuf = PathBuf::from("evaluation/evals/");

        let res = self.consolidate_to_json(pathbuf);
        if res.is_ok() {
            println!("Consilidated Evaluation");
        } else {
            println!("Error occured during consolidation: {}", res.err().unwrap());
        }
    }
}

#[derive(Component)]
pub struct EvaluationSurfaceComponent;

#[derive(Event)]
pub struct NextEvaluationEvent;

#[allow(clippy::complexity)]
fn handle_next_eval_event(
    mut reader: EventReader<NextEvaluationEvent>,
    mut evaluation: ResMut<Evaluation>,
    mut commands: Commands,
    evaluation_surfaces: Query<Entity, With<EvaluationSurfaceComponent>>,
    root: Query<Entity, With<RootTransform>>,
    images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut reset_default_curve: EventWriter<ResetDefaultCurveEvent>,
    info: Res<RenderInformation>,
) {
    let mut next_surface = None;
    let mut reset_curve = false;

    for _ in reader.read() {
        if evaluation.can_evaluate_further() {
            let mut points = Vec::new();
            for (vec, RenderPoint(y, x)) in control_points {
                points.push((*y, *x, vec.translation).into());
            }

            evaluation.end_evaluation(points);

            if let Some(points) = evaluation.start_next_evaluation() {
                next_surface = Some(points);
            }
        } else {
            let mut points = Vec::new();
            for (vec, RenderPoint(y, x)) in control_points {
                points.push((*y, *x, vec.translation).into());
            }
            info!("Adding reference surface");
            evaluation.add_reference_surface(points);
        }

        reset_curve = true;
    }

    if reset_curve {
        reset_default_curve.write(ResetDefaultCurveEvent);

        for surface in evaluation_surfaces {
            let _ = commands.get_entity(surface).map(|mut e| e.despawn());
        }
    }

    if let Some(points) = next_surface {
        let root = root
            .single()
            .expect("Initialization error. Root Transform must be present at all times");

        let (mesh, texture) = create_mesh_from_control_points(
            &points,
            (30, 30),
            &CurvatureDisplayMode::CustomColor(BLACK.into()),
            images,
            info.scale as f64,
            SurfaceRenderMode::Triangles,
        );

        let mat = StandardMaterial {
            base_color_texture: Some(texture),
            double_sided: true,
            cull_mode: None,
            ..Default::default()
        };

        commands.spawn((
            ChildOf(root),
            Transform::default(),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(materials.add(mat)),
            EvaluationSurfaceComponent,
        ));
    }
}

pub struct EvaluationPlugin;

impl Plugin for EvaluationPlugin {
    fn build(&self, app: &mut App) {
        let buf = PathBuf::from("evaluation/reference_surfaces.json");
        app.insert_resource(
            Evaluation::from_json(buf.as_path())
                .expect("This must be here, in order to start evaluation"),
        );

        app.add_systems(Last, handle_next_eval_event);

        app.add_event::<NextEvaluationEvent>();
    }
}
