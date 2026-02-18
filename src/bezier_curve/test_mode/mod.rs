use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};

use bevy::prelude::*;
use bevy::{ecs::resource::Resource, math::Vec3};
use bevy_lunex::prelude::{Text3d, Text3dStyling, TextAlign, TextAtlas, Weight};
#[cfg(feature = "vr_enable")]
use bevy_xr_utils::tracking_utils::XrTrackedView;
use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

pub mod eval_types;
pub use eval_types::*;

pub mod single_eval;
pub use single_eval::*;

#[cfg(not(feature = "vr_enable"))]
use crate::MainCamera;

use super::{bezier_curve_renderer::ResetDefaultCurveEvent, components::RenderPoint};
use crate::{
    RootTransform,
    bezier_curve::{
        bezier_curve_renderer::RedrawEvent,
        helper_curves::{ControlCurve, ControlCurvePoint},
        render_info::RenderInformation,
    },
    nurbs::{
        bezier_plane::{ControlPoints2D, ToControlPoints2D},
        point::Point,
    },
};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, States, Default, Hash, Debug)]
enum EvaluationFlowState {
    #[default]
    Idle,
    Eval,
    Result,
    Creation,
    Created,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct TestControlPoint {
    pub y_idx: usize,
    pub x_idx: usize,
    pub point: (f64, f64, f64),
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
            points.sort_by_key(|a| a.0);
            let points = points.iter().map(|p| p.1).collect::<Vec<_>>();
            multi_curves.push((*i, points));
        }

        // Then order the subcurves by index
        multi_curves.sort_by_key(|a| a.0);
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

#[derive(Serialize, Resource)]
pub struct Evaluation {
    #[serde(skip)]
    counter: usize,
    #[serde(skip)]
    start_counter: usize,

    #[serde(skip)]
    reference_surfaces: Vec<EvaluationType>,
    #[serde(skip)]
    pathbuf_references: PathBuf,

    evaluations: Vec<SingleEvaluation>,

    #[serde(skip)]
    current_creation_type: Option<EnterEvalEvent>,
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

        let reference_surfaces: Vec<EvaluationType> = serde_json::from_str(&content)?;

        Ok(Self {
            evaluations: Vec::new(),
            start_counter: reference_surfaces.len(),
            reference_surfaces,
            counter: 0,
            pathbuf_references: PathBuf::from(path),
            current_creation_type: default(),
        })
    }

    pub fn add_reference_surface(&mut self, type_of_eval: EvaluationType) {
        self.reference_surfaces.push(type_of_eval);
    }

    /// Start e new evaluation, if possible
    pub fn start_next_evaluation(&mut self) -> Option<EvaluationType> {
        if self.can_evaluate_further() {
            let mut eval: SingleEvaluation = self.reference_surfaces[self.counter].clone().into();
            eval.start();

            self.evaluations.push(eval);

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
pub struct EvaluationMarker;

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
            EvaluationFlowState::Idle => EvaluationFlowState::Idle,
            EvaluationFlowState::Eval => EvaluationFlowState::Result,
            EvaluationFlowState::Result => EvaluationFlowState::Idle,
            EvaluationFlowState::Creation => EvaluationFlowState::Created,
            EvaluationFlowState::Created => EvaluationFlowState::Idle,
        };

        next_state_res.set(next_state);
    }
}

#[derive(Default, Clone, Debug, Event)]
pub enum EnterEvalEvent {
    #[default]
    Next,
    Surface,
    Curves(usize),
    Linear(usize),
    Precision(usize),
}

impl From<EnterEvalEvent> for ResetDefaultCurveEvent {
    fn from(value: EnterEvalEvent) -> Self {
        match value {
            EnterEvalEvent::Next => ResetDefaultCurveEvent::Surface,
            EnterEvalEvent::Surface => ResetDefaultCurveEvent::Surface,
            EnterEvalEvent::Curves(n) => ResetDefaultCurveEvent::Curves(n),
            EnterEvalEvent::Linear(n) => {
                let start = Vec3::ZERO;
                let end = Vec3::ONE;

                let mut points = Vec::new();

                for i in 0..n {
                    let p = Vec3::new(i as f32, 1.0, 0.0);
                    points.push(p);
                }

                ResetDefaultCurveEvent::Line { start, end, points }
            }
            EnterEvalEvent::Precision(n) => {
                let start = Vec3::ZERO;

                let mut points = Vec::new();

                for i in 0..n {
                    let p = Vec3::new(i as f32, 1.0, 0.0);
                    points.push(p);
                }

                ResetDefaultCurveEvent::Point {
                    target: start,
                    starts: points,
                }
            }
        }
    }
}

impl TryFrom<(EnterEvalEvent, Vec<TestControlPoint>)> for EvaluationType {
    type Error = String;
    fn try_from(
        value: (EnterEvalEvent, Vec<TestControlPoint>),
    ) -> std::result::Result<Self, Self::Error> {
        match value.0 {
            EnterEvalEvent::Next => Err("next cannot be converted".to_owned()),
            EnterEvalEvent::Curves(_) => {
                let points = value.1.to_control_points();

                let curves: Vec<Vec<TestControlPoint>> = points
                    .into_iter()
                    .enumerate()
                    .map(|inner| {
                        inner
                            .1
                            .into_iter()
                            .enumerate()
                            .map(|v| TestControlPoint {
                                y_idx: inner.0,
                                x_idx: v.0,
                                point: v.1.into(),
                            })
                            .collect()
                    })
                    .collect();
                Ok(EvaluationType::Curve(EvaluationCurves { curves }))
            }
            EnterEvalEvent::Surface => Ok(EvaluationType::Surface(EvaluationSurface {
                control_points: value.1,
            })),
            EnterEvalEvent::Linear(n) => {
                let points = value.1.to_control_points();

                let curves: Vec<Vec<TestControlPoint>> = points
                    .into_iter()
                    .enumerate()
                    .map(|inner| {
                        inner
                            .1
                            .into_iter()
                            .enumerate()
                            .map(|v| TestControlPoint {
                                y_idx: inner.0,
                                x_idx: v.0,
                                point: v.1.into(),
                            })
                            .collect()
                    })
                    .collect();

                // Assertions for form of control points
                assert!(curves.len() == 2);
                assert!(curves[0].len() == 2);
                assert!(curves[1].len() == n);

                let start = curves[0][0];
                let end = curves[0][1];
                let start_points = curves[1].clone();

                Ok(EvaluationType::LinearPlacement(EvaluationLinearPlacement {
                    start,
                    end,
                    start_points,
                }))
            }
            EnterEvalEvent::Precision(n) => {
                let points = value.1.to_control_points();

                let curves: Vec<Vec<TestControlPoint>> = points
                    .into_iter()
                    .enumerate()
                    .map(|inner| {
                        inner
                            .1
                            .into_iter()
                            .enumerate()
                            .map(|v| TestControlPoint {
                                y_idx: inner.0,
                                x_idx: v.0,
                                point: v.1.into(),
                            })
                            .collect()
                    })
                    .collect();

                // Assertions for form of control points
                assert!(curves.len() == 2);
                assert!(curves[0].len() == 1);
                assert!(curves[1].len() == n);

                let end = curves[0][0];
                let start_points = curves[1].clone();

                Ok(EvaluationType::PrecisionMovement(
                    EvaluationPrecisionMovement {
                        start: start_points,
                        end,
                    },
                ))
            }
        }
    }
}

#[allow(clippy::complexity)]
fn on_enter_idle_state(
    mut commands: Commands,
    evaluation_parents: Query<Entity, With<EvaluationMarker>>,
    texts: Query<Entity, With<EvalTextMarker>>,
    mut new_surface_writer: EventWriter<ResetDefaultCurveEvent>,
) {
    // NOTE(Kleinmann): Only cleanup
    // NOTE(Kleinmann): Assume that each evaluation has a parent
    for parent in evaluation_parents {
        let _ = commands.get_entity(parent).map(|mut e| e.despawn());
    }

    for text_entity in texts {
        let _ = commands.get_entity(text_entity).map(|mut e| e.despawn());
    }

    // NOTE: When returning to idle mode, let the user play around with a surface. This inherently leads to a better experience
    new_surface_writer.write(ResetDefaultCurveEvent::Surface);
}

#[allow(clippy::complexity)]
fn on_enter_eval_state(
    mut eval_state_reader: EventReader<EnterEvalEvent>,
    mut evaluation: ResMut<Evaluation>,
    mut commands: Commands,
    evaluation_parents: Query<Entity, With<EvaluationMarker>>,
    texts: Query<Entity, With<EvalTextMarker>>,
    root: Query<Entity, With<RootTransform>>,
    images: ResMut<Assets<Image>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    mut reset_default_curve: EventWriter<ResetDefaultCurveEvent>,
    info: Res<RenderInformation>,
    mut next_state: ResMut<NextState<EvaluationFlowState>>,
    current_state: Res<State<EvaluationFlowState>>,
    redraw_event: EventWriter<RedrawEvent>,
) {
    match current_state.get() {
        EvaluationFlowState::Creation => {
            next_state.set(EvaluationFlowState::Created);
            return;
        }
        EvaluationFlowState::Eval => {
            next_state.set(EvaluationFlowState::Result);
            return;
        }
        _ => (),
    }

    if eval_state_reader.is_empty() {
        return;
    }

    // Set the current evaluation state, in case this is not an evaluation
    evaluation.current_creation_type =
        Some(eval_state_reader.read().collect::<Vec<_>>()[0].clone());
    eval_state_reader.clear();

    let mut next_surface = None;
    info!("In eval state");

    if matches!(
        evaluation.current_creation_type.as_ref().expect("just set"),
        EnterEvalEvent::Next
    ) && let Some(points) = evaluation.start_next_evaluation()
    {
        next_surface = Some(points);
        next_state.set(EvaluationFlowState::Eval);
    } else {
        next_state.set(EvaluationFlowState::Creation);
    }

    // NOTE(Kleinmann): Assume that each evaluation has a parent
    for parent in evaluation_parents {
        let _ = commands.get_entity(parent).map(|mut e| e.despawn());
    }

    for text_entity in texts {
        let _ = commands.get_entity(text_entity).map(|mut e| e.despawn());
    }

    // Display next surface, if present
    if let Some(points) = next_surface {
        let root = root
            .single()
            .expect("Initialization error. Root Transform must be present at all times");

        // Spawn parent, that is a child of root. This parent will get deleted, once a new evaluation starts
        let parent = commands
            .spawn((
                EvaluationMarker,
                ChildOf(root),
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();

        // Spawn shape
        points.to_render_commands(
            &mut commands,
            parent,
            images,
            materials,
            meshes,
            redraw_event,
            info,
        );

        reset_default_curve.write(points.to_redraw_event());
    } else {
        // Reset surface
        reset_default_curve.write(
            evaluation
                .current_creation_type
                .clone()
                .expect("just set")
                .into(),
        );
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
                "Average Distance: {:.3}\nMin. Distance: {:.3}\nMax. Distance: {:.3}\nTime (s): {:.3}",
                eval_info.average_dist, eval_info.min_dist, eval_info.max_dist, eval_info.time as f64 / 1000.0
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
    }

    for mut point_entity in &mut evaluation_points {
        *point_entity = Visibility::Hidden;
    }

    for mut point_entity in &mut original_points {
        *point_entity = Visibility::Inherited;
    }

    // Reset current creation type here
    evaluation.current_creation_type = None;

    // Reset flow state to idle, use ui to change back to evaluation or creation
    next_state_res.set(EvaluationFlowState::Idle);
}

#[allow(clippy::complexity)]
fn on_enter_create_state(
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
    curves: Query<&Children, With<ControlCurve>>,
    curve_points: Query<(&Transform, &ControlCurvePoint), Without<RenderPoint>>,
    mut next_state_res: ResMut<NextState<EvaluationFlowState>>,
) {
    let mut points = Vec::new();
    for (vec, RenderPoint(y, x)) in control_points {
        points.push((*y, *x, vec.translation).into());
    }

    dbg!(evaluation.current_creation_type.clone());
    if let Some(eval_type) = evaluation.current_creation_type.clone()
        && !matches!(eval_type, EnterEvalEvent::Next)
    {
        let points = if matches!(eval_type, EnterEvalEvent::Curves(_)) {
            // TODO: Collect curves
            let mut test_points: Vec<TestControlPoint> = Vec::new();
            for (idx, curve_children) in curves.iter().enumerate() {
                for child in curve_children {
                    if let Ok((pos, curve_point)) = curve_points.get(*child) {
                        let pos = pos.translation;
                        test_points.push(TestControlPoint {
                            y_idx: idx,
                            x_idx: curve_point.0,
                            point: (pos.x as f64, pos.y as f64, pos.z as f64),
                        });
                    }
                }
            }
            test_points
        } else {
            points
        };
        evaluation.add_reference_surface(
            (eval_type, points)
                .try_into()
                .expect("Critical error, this should never trigger out of the 'next' eval type"),
        );
        next_state_res.set(EvaluationFlowState::Idle);
    }

    for mut point_entity in &mut evaluation_points {
        *point_entity = Visibility::Hidden;
    }

    for mut point_entity in &mut original_points {
        *point_entity = Visibility::Inherited;
    }

    // Reset current creation type here
    evaluation.current_creation_type = None;
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

pub struct EvaluationPlugin;

impl Plugin for EvaluationPlugin {
    fn build(&self, app: &mut App) {
        let buf = PathBuf::from("evaluation/reference_surfaces.json");
        app.insert_resource(
            Evaluation::from_json(buf.as_path())
                .expect("This must be here, in order to start evaluation"),
        );

        app.add_systems(
            PostUpdate,
            on_enter_eval_state.run_if(on_event::<EnterEvalEvent>),
        );
        app.add_systems(OnEnter(EvaluationFlowState::Result), on_enter_result_state);
        app.add_systems(OnEnter(EvaluationFlowState::Idle), on_enter_idle_state);
        app.add_systems(OnEnter(EvaluationFlowState::Created), on_enter_create_state);
        app.add_systems(Last, handle_state_change_event);
        app.add_systems(Update, follow_camera);

        app.add_event::<NextEvaluationEvent>();

        app.insert_state(EvaluationFlowState::default());

        app.add_event::<EnterEvalEvent>();
    }
}
