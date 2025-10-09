use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};

use bevy::{color::palettes::css::BLACK, prelude::*};
use bevy::{ecs::resource::Resource, math::Vec3};
use bevy_lunex::prelude::{Text3d, Text3dStyling, TextAlign, TextAtlas, Weight};
#[cfg(feature = "vr_enable")]
use bevy_xr_utils::tracking_utils::XrTrackedView;
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

#[cfg(not(feature = "vr_enable"))]
use crate::MainCamera;
use crate::{
    RootTransform,
    bezier_curve::{
        curvature_display_mode::CurvatureDisplayMode,
        render_info::RenderInformation,
        util::{SurfaceRenderMode, create_mesh_from_control_points},
    },
    nurbs::{
        bezier_plane::{
            ControlPoints2D, ToControlPoints2D, eval_2d_bezier_curves, transpose_control_points,
        },
        point::Point,
    },
    translation_control::proximity_detector::Snappable,
};

use super::{bezier_curve_renderer::ResetDefaultCurveEvent, components::RenderPoint};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, States, Default, Hash, Debug)]
enum EvaluationFlowState {
    #[default]
    Idle,
    Eval,
    Result,
}

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

#[derive(Clone, Copy)]
pub struct BasicSurfaceEvaluationData {
    average_dist: f64,
    max_dist: f64,
    min_dist: f64,
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
    pub fn evaluate(
        &mut self,
        control_points: Vec<TestControlPoint>,
    ) -> BasicSurfaceEvaluationData {
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
        BasicSurfaceEvaluationData {
            average_dist: self.average_dist,
            max_dist: self.max_dist,
            min_dist: self.min_dist,
        }
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
    start_counter: usize,

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
            start_counter: reference_surfaces.len(),
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
    pub fn end_evaluation(
        &mut self,
        control_points: Vec<TestControlPoint>,
    ) -> Option<BasicSurfaceEvaluationData> {
        let last = self.evaluations.last_mut()?;

        if !last.evaluated {
            let basic = last.evaluate(control_points);
            self.counter += 1;
            Some(basic)
        } else {
            None
        }
    }

    /// Consolidate the evaluation and all possible new reference surfaces. The path is assumed to
    /// point to the evals directory (NOT FILE). The file is appended additionally using a
    /// timestamp
    pub fn consolidate_to_json(&self, mut path: PathBuf) -> Result<(), Box<dyn Error>> {
        // consolidate evaluations
        std::fs::create_dir_all(path.as_path())?;

        let timestamp = chrono::Local::now();
        path.push(format!(
            "{}_{}_{}_{}_{}.json",
            timestamp.year(),
            timestamp.month(),
            timestamp.day(),
            timestamp.hour(),
            timestamp.minute()
        ));

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
        self.counter < self.start_counter
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

#[derive(Component)]
pub struct EvaluationPointComponent;

#[derive(Component)]
pub struct OriginalControlPointMarker;

#[derive(Component)]
pub struct EvalTextMarker;

#[derive(Event)]
pub struct NextEvaluationEvent;

fn handle_state_change_event(
    mut reader: EventReader<NextEvaluationEvent>,
    current_state: Res<State<EvaluationFlowState>>,
    mut next_state_res: ResMut<NextState<EvaluationFlowState>>,
) {
    for _ in reader.read() {
        let next_state = match **current_state {
            EvaluationFlowState::Idle => EvaluationFlowState::Eval,
            EvaluationFlowState::Eval => EvaluationFlowState::Result,
            EvaluationFlowState::Result => EvaluationFlowState::Eval,
        };

        next_state_res.set(next_state);
    }
}

#[allow(clippy::complexity)]
fn on_enter_eval_state(
    mut evaluation: ResMut<Evaluation>,
    mut commands: Commands,
    evaluation_surfaces: Query<Entity, With<EvaluationSurfaceComponent>>,
    evaluation_points: Query<Entity, With<EvaluationPointComponent>>,
    original_points: Query<Entity, With<OriginalControlPointMarker>>,
    texts: Query<Entity, With<EvalTextMarker>>,
    root: Query<Entity, With<RootTransform>>,
    images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut reset_default_curve: EventWriter<ResetDefaultCurveEvent>,
    info: Res<RenderInformation>,
) {
    let mut next_surface = None;
    info!("In eval state");

    if let Some(points) = evaluation.start_next_evaluation() {
        next_surface = Some(points);
    }
    // Reset surface
    reset_default_curve.write(ResetDefaultCurveEvent);

    for surface in evaluation_surfaces {
        let _ = commands.get_entity(surface).map(|mut e| e.despawn());
    }

    for point_entity in evaluation_points {
        let _ = commands.get_entity(point_entity).map(|mut e| e.despawn());
    }

    for point_entity in original_points {
        let _ = commands.get_entity(point_entity).map(|mut e| e.despawn());
    }

    for text_entity in texts {
        let _ = commands.get_entity(text_entity).map(|mut e| e.despawn());
    }

    // Display next surface, if present
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

        let points = points.to_control_points();
        let edge_1 = points.first();
        let edge_3 = points.last();
        let transposed_points = transpose_control_points(&points);
        let edge_2 = transposed_points.last();
        let edge_4 = transposed_points.first();

        let small_sphere = meshes.add(Sphere::new(0.05 * info.scale));
        let light_red = materials.add(StandardMaterial::from_color(Srgba::new(
            230.0 / 255.0,
            121.0 / 255.0,
            135.0 / 255.0,
            1.0,
        )));

        for edge in [edge_1, edge_2, edge_3, edge_4] {
            if let Some(edge) = edge {
                for point in edge {
                    commands.spawn((
                        ChildOf(root),
                        Transform::from_translation(Vec3::from(*point)),
                        Mesh3d(small_sphere.clone()),
                        MeshMaterial3d(light_red.clone()),
                        EvaluationPointComponent,
                        Visibility::Inherited,
                        Snappable,
                    ));
                }
            }
        }

        for points in points {
            for p in points {
                commands.spawn((
                    ChildOf(root),
                    Transform::from_translation(Vec3::from(p)),
                    Mesh3d(small_sphere.clone()),
                    MeshMaterial3d(light_red.clone()),
                    OriginalControlPointMarker,
                    Visibility::Hidden,
                ));
            }
        }
    }
}

#[allow(clippy::complexity)]
fn on_enter_result_state(
    mut evaluation: ResMut<Evaluation>,
    mut evaluation_points: Query<
        &mut Visibility,
        (
            With<EvaluationPointComponent>,
            Without<OriginalControlPointMarker>,
        ),
    >,
    mut original_points: Query<
        &mut Visibility,
        (
            With<OriginalControlPointMarker>,
            Without<EvaluationPointComponent>,
        ),
    >,
    control_points: Query<(&Transform, &RenderPoint)>,
    mut next_state_res: ResMut<NextState<EvaluationFlowState>>,
    mut commands: Commands,
    root: Query<Entity, With<RootTransform>>,
    info: Res<RenderInformation>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    info!("In result state");
    let mut points = Vec::new();
    for (vec, RenderPoint(y, x)) in control_points {
        points.push((*y, *x, vec.translation).into());
    }

    if evaluation.can_evaluate_further()
        && let Some(eval_info) = evaluation.end_evaluation(points.clone())
    {
        let root = root.single().unwrap();

        commands.spawn((
            ChildOf(root),
            Transform::from_xyz(0.0, 1.0 * info.scale, 0.0)
                .with_scale(Vec3::ONE * 0.0025 * info.scale),
            EvalTextMarker,
            Name::new("Evaluation Text Marker"),
            Text3d::new(format!(
                "Average Distance: {:.3}\nMin. Distance: {:.3}\nMax. Distance: {:.3}",
                eval_info.average_dist, eval_info.min_dist, eval_info.max_dist,
            )),
            Text3dStyling {
                size: 64.0,
                color: Srgba::new(0., 0., 0., 1.),
                align: TextAlign::Center,
                font: Arc::from("Rajdhani"),
                weight: Weight::BOLD,
                ..Default::default()
            },
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..Default::default()
            })),
            Mesh3d::default(),
            Visibility::Inherited,
        ));
    } else if !evaluation.can_evaluate_further() {
        evaluation.add_reference_surface(points);
        next_state_res.set(EvaluationFlowState::Eval);
    }

    for mut point_entity in &mut evaluation_points {
        *point_entity = Visibility::Hidden;
    }

    for mut point_entity in &mut original_points {
        *point_entity = Visibility::Inherited;
    }
}

#[cfg(not(feature = "vr_enable"))]
#[allow(clippy::complexity)]
fn follow_camera(
    camera: Query<
        &GlobalTransform,
        (
            With<MainCamera>,
            Without<RootTransform>,
            Without<EvalTextMarker>,
        ),
    >,
    root: Query<
        &Transform,
        (
            With<RootTransform>,
            Without<MainCamera>,
            Without<EvalTextMarker>,
        ),
    >,
    mut texts: Query<
        (&mut Transform, &GlobalTransform),
        (
            With<EvalTextMarker>,
            Without<MainCamera>,
            Without<RootTransform>,
        ),
    >,
) {
    if let Ok(camera) = camera.single()
        && let Ok(root) = root.single()
    {
        for (mut text, global_text) in texts.iter_mut() {
            let diff = global_text.translation() - camera.translation();
            let diff = root.compute_affine().inverse().transform_vector3(diff);
            text.look_to(diff, Vec3::Y);
        }
    }
}

#[cfg(feature = "vr_enable")]
fn follow_camera(
    camera: Query<
        &GlobalTransform,
        (
            With<XrTrackedView>,
            Without<EvalTextMarker>,
            Without<RootTransform>,
        ),
    >,
    root: Query<
        &Transform,
        (
            With<RootTransform>,
            Without<XrTrackedView>,
            Without<EvalTextMarker>,
        ),
    >,
    mut texts: Query<
        (&mut Transform, &GlobalTransform),
        (
            With<EvalTextMarker>,
            Without<XrTrackedView>,
            Without<RootTransform>,
        ),
    >,
) {
    if let Ok(camera) = camera.single() {
        for (mut text, global_text) in texts.iter_mut() {
            let diff = global_text.translation() - camera.translation();
            let diff = root.compute_affine().inverse().transform_vector3(diff);
            text.look_to(diff, Vec3::Y);
        }
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

        app.add_systems(OnEnter(EvaluationFlowState::Eval), on_enter_eval_state);
        app.add_systems(OnEnter(EvaluationFlowState::Result), on_enter_result_state);
        app.add_systems(Last, handle_state_change_event);
        app.add_systems(Update, follow_camera);

        app.add_event::<NextEvaluationEvent>();

        app.insert_state(EvaluationFlowState::default());
    }
}
