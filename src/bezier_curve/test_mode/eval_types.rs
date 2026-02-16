use bevy::{color::palettes::css::BLACK, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    bezier_curve::{
        bezier_curve_renderer::ResetDefaultCurveEvent,
        curvature_display_mode::CurvatureDisplayMode,
        render_info::RenderInformation,
        test_mode::{
            BasicSurfaceEvaluationData, EvaluationPointComponent, EvaluationSurfaceComponent,
            OriginalControlPointMarker, PointEvaluation, SingleEvaluation, TestControlPoint,
        },
        util::{SurfaceRenderMode, create_mesh_from_control_points},
    },
    nurbs::bezier_plane::{ToControlPoints2D, eval_2d_bezier_curves, transpose_control_points},
    translation_control::proximity_detector::Snappable,
};

pub trait ToRenderCommandsForEvaluation {
    fn to_render_commands(
        &self,
        commands: &mut Commands,
        root: Entity,
        images: ResMut<Assets<Image>>,
        materials: ResMut<Assets<StandardMaterial>>,
        meshes: ResMut<Assets<Mesh>>,
        info: Res<RenderInformation>,
    );

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData;
}

#[derive(Serialize, Deserialize, Clone)]
pub enum EvaluationType {
    Surface(EvaluationSurface),
    Curve(EvaluationCurves),
    LinearPlacement(EvaluationLinearPlacement),
    PrecisionMovement(EvaluationPrecisionMovement),
}

impl EvaluationType {
    pub fn to_redraw_event(&self) -> ResetDefaultCurveEvent {
        match self {
            Self::Surface(_) => ResetDefaultCurveEvent::Surface,
            // NOTE(Kleinmann): This seems misleading, but the mode should match curves using a surface as best as possible, hence a surface should be spawned
            // NOTE(Kleinmann): This differs for the situation, where a new evaluation entity must be created
            Self::Curve(_) => ResetDefaultCurveEvent::Surface,
            Self::PrecisionMovement(target) => {
                let (_, _, end) = target.end.into();
                let starts = target
                    .start
                    .iter()
                    .map(|s| {
                        let (_, _, s) = (*s).into();
                        s
                    })
                    .collect::<Vec<_>>();
                ResetDefaultCurveEvent::Point {
                    target: end,
                    starts,
                }
            }
            Self::LinearPlacement(line) => {
                let (_, _, start) = line.start.into();
                let (_, _, end) = line.end.into();

                ResetDefaultCurveEvent::Line { start, end }
            }
        }
    }
}

impl ToRenderCommandsForEvaluation for EvaluationType {
    fn to_render_commands(
        &self,
        commands: &mut Commands,
        root: Entity,
        images: ResMut<Assets<Image>>,
        materials: ResMut<Assets<StandardMaterial>>,
        meshes: ResMut<Assets<Mesh>>,
        info: Res<RenderInformation>,
    ) {
        match self {
            Self::Surface(surface) => {
                surface.to_render_commands(commands, root, images, materials, meshes, info)
            }
            Self::LinearPlacement(lin_placement) => {
                lin_placement.to_render_commands(commands, root, images, materials, meshes, info)
            }
            Self::PrecisionMovement(precision_movement) => precision_movement
                .to_render_commands(commands, root, images, materials, meshes, info),
            Self::Curve(curves) => {
                curves.to_render_commands(commands, root, images, materials, meshes, info)
            }
        }
    }

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData {
        match self {
            Self::Surface(surface) => surface.evaluate(result),
            Self::LinearPlacement(lin_placement) => lin_placement.evaluate(result),
            Self::Curve(curves) => curves.evaluate(result),
            Self::PrecisionMovement(precision_movement) => precision_movement.evaluate(result),
        }
    }
}

impl From<EvaluationType> for SingleEvaluation {
    fn from(value: EvaluationType) -> Self {
        Self::new_unevaluated(value, 100)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationSurface {
    /// List of control Points in a 2d fashion
    pub control_points: Vec<TestControlPoint>,
}

impl ToRenderCommandsForEvaluation for EvaluationSurface {
    fn to_render_commands(
        &self,
        commands: &mut Commands,
        root: Entity,
        images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        info: Res<RenderInformation>,
    ) {
        let points = &self.control_points;
        let (mesh, texture) = create_mesh_from_control_points(
            points,
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
            Visibility::Inherited,
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

        for edge in [edge_1, edge_2, edge_3, edge_4].iter().flatten() {
            for point in *edge {
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

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData {
        let collected_test = result.test_control_points.to_control_points();
        let collected_reference = self.control_points.to_control_points();
        let mut avg_dist = 0.0;
        let n = (result.resolution + 1).pow(2) as f64;

        for u in 0..=result.resolution {
            let u = u as f64 / result.resolution as f64;
            for v in 0..=result.resolution {
                let v = v as f64 / result.resolution as f64;

                let test_point = eval_2d_bezier_curves(&collected_test, u, v);
                let reference_point = eval_2d_bezier_curves(&collected_reference, u, v);

                let dist = (reference_point - test_point).magnitude();

                result.evaluations.push(PointEvaluation {
                    u,
                    v,
                    reference: reference_point.into(),
                    surface_point: test_point.into(),
                    dist,
                });

                avg_dist += dist / n;
                result.max_dist = result.max_dist.max(dist);
                result.min_dist = result.min_dist.min(dist);
            }
        }

        result.average_dist = avg_dist;
        result.evaluated = true;
        BasicSurfaceEvaluationData {
            average_dist: result.average_dist,
            max_dist: result.max_dist,
            min_dist: result.min_dist,
            time: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationCurves {
    /// List of curves. Each curve is a list of TestControlPoints
    pub curves: Vec<Vec<TestControlPoint>>,
}

impl ToRenderCommandsForEvaluation for EvaluationCurves {
    fn to_render_commands(
        &self,
        _commands: &mut Commands,
        _root: Entity,
        _images: ResMut<Assets<Image>>,
        _materials: ResMut<Assets<StandardMaterial>>,
        _meshes: ResMut<Assets<Mesh>>,
        _info: Res<RenderInformation>,
    ) {
        todo!()
    }

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData {
        todo!()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationLinearPlacement {
    pub start: TestControlPoint,
    pub end: TestControlPoint,
    pub start_points: Vec<TestControlPoint>,
}

impl ToRenderCommandsForEvaluation for EvaluationLinearPlacement {
    fn to_render_commands(
        &self,
        commands: &mut Commands,
        root: Entity,
        _images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        info: Res<RenderInformation>,
    ) {
        let (_, _, start) = self.start.into();
        let (_, _, end) = self.end.into();
        let diff = end - start;

        let cylinder_mesh =
            meshes.add(Cylinder::new(0.01 * info.scale, diff.length() * info.scale));
        let black = materials.add(Color::from(BLACK));

        commands.spawn((
            ChildOf(root),
            Transform::from_translation(start.midpoint(end)).looking_to(diff, Vec3::Y),
            children![(
                Transform::default(),
                MeshMaterial3d(black),
                Mesh3d(cylinder_mesh),
            )],
        ));
    }

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData {
        let (_, _, end) = self.end.into();
        let (_, _, start) = self.start.into();

        let segment = end - start;
        let length_sq = segment.length_squared();

        let mut min_dist: f64 = f64::MAX;
        let mut max_dist: f64 = f64::MIN;
        let mut avg_dist: f64 = 0.0;

        for point in &result.test_control_points {
            let (_, _, point) = (*point).into();

            let to_point = point - start;

            let dist = if length_sq <= f32::EPSILON {
                to_point.length()
            } else {
                // Project to_point onto segment
                let t = (to_point.dot(segment) / length_sq).clamp(0.0, 1.0);
                let target_on_line = start.lerp(end, t);

                (point - target_on_line).length()
            };

            min_dist = min_dist.min(dist as f64);
            max_dist = max_dist.max(dist as f64);
            avg_dist += dist as f64 / result.test_control_points.len() as f64;
        }

        result.evaluated = true;
        result.min_dist = min_dist;
        result.max_dist = max_dist;
        result.average_dist = avg_dist;

        BasicSurfaceEvaluationData {
            average_dist: avg_dist,
            max_dist,
            min_dist,
            time: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct EvaluationPrecisionMovement {
    pub start: Vec<TestControlPoint>,
    pub end: TestControlPoint,
}

impl ToRenderCommandsForEvaluation for EvaluationPrecisionMovement {
    fn to_render_commands(
        &self,
        commands: &mut Commands,
        root: Entity,
        _images: ResMut<Assets<Image>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        info: Res<RenderInformation>,
    ) {
        let (_, _, end) = self.end.into();

        let sphere_mesh = meshes.add(Sphere::new(0.1 * info.scale));
        let black = materials.add(Color::from(BLACK));

        commands.spawn((
            ChildOf(root),
            Transform::from_translation(end),
            children![(
                Transform::default(),
                MeshMaterial3d(black),
                Mesh3d(sphere_mesh),
            )],
        ));
    }

    fn evaluate(&self, result: &mut SingleEvaluation) -> BasicSurfaceEvaluationData {
        let test_points = &result.test_control_points;

        assert!(test_points.len() == self.start.len());

        let mut min_dist: f64 = f64::MAX;
        let mut max_dist: f64 = f64::MIN;
        let mut avg_dist: f64 = 0.0;

        let (_, _, target) = self.end.into();

        for point in test_points {
            let (_, _, test) = (*point).into();

            let diff = target - test;

            min_dist = min_dist.min(diff.length() as f64);
            max_dist = max_dist.max(diff.length() as f64);
            avg_dist += diff.length() as f64 / test_points.len() as f64;
        }

        result.evaluated = true;
        result.min_dist = min_dist;
        result.max_dist = max_dist;
        result.average_dist = avg_dist;

        BasicSurfaceEvaluationData {
            average_dist: avg_dist,
            max_dist,
            min_dist,
            time: 0,
        }
    }
}
