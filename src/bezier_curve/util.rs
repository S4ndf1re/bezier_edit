use bevy::asset::RenderAssetUsages;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{color::palettes::css::BLACK, prelude::*};
use num::pow;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::nurbs::bezier_plane::{ControlPoints2D, derive_2d, eval_2d_bezier_curves};
use crate::nurbs::point::Point;

use super::bezier_curve_renderer::Resolution;

#[derive(Clone, Copy, Ord, Eq, PartialEq, PartialOrd)]
pub enum CurvatureDisplayMode {
    None,
    U,
    V,
    Both,
}

impl CurvatureDisplayMode {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::U,
            Self::U => Self::V,
            Self::V => Self::Both,
            Self::Both => Self::None,
        }
    }
}

pub struct ComputationResultBezierSurface {
    pub u: u32,
    pub v: u32,
    pub point: Point,
    pub normal: Point,
    pub u_diff_1: Point,
    pub v_diff_1: Point,
    pub uvs: [f32; 2],
    pub u_diff_2: Point,
    pub v_diff_2: Point,
}

pub fn compute_points(
    control_points: &ControlPoints2D,
    u: u32,
    v: u32,
    w: u32,
    h: u32,
) -> ComputationResultBezierSurface {
    let uvs = [
        ((u as f64) / ((w as f64) - 1.0)),
        ((v as f64) / ((h as f64) - 1.0)),
    ];

    let resulting_point = eval_2d_bezier_curves(control_points, uvs[0], uvs[1]);

    let (u_diff_1, v_diff_1) = derive_2d(control_points, uvs[0], uvs[1], 1);
    let normal = &u_diff_1.cross(&v_diff_1) * -1.0;

    let (u_diff_2, v_diff_2) = derive_2d(control_points, uvs[0], uvs[1], 2);

    let uvs = [uvs[0] as f32, uvs[1] as f32];

    ComputationResultBezierSurface {
        u,
        v,
        point: resulting_point,
        normal,
        u_diff_1,
        v_diff_1,
        uvs,
        u_diff_2,
        v_diff_2,
    }
}

pub fn create_mesh_from_control_points(
    control_points: &ControlPoints2D,
    resolution: Resolution,
    curvature_mode: &CurvatureDisplayMode,
    mut images: ResMut<Assets<Image>>,
    scale: f64,
) -> (Mesh, Handle<Image>) {
    let w = resolution.0;
    let h = resolution.1;

    let indices = (0..w)
        .flat_map(|u| (0..h).map(move |v| (u, v)))
        .collect::<Vec<_>>();

    let computed = indices
        .par_iter()
        .map(|(u, v)| compute_points(control_points, *u, *v, w, h))
        .collect::<Vec<_>>();

    let mut computed_points: Vec<[f32; 3]> = Vec::with_capacity(computed.len());
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(computed.len());
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(computed.len());
    let mut image = Image::new_fill(
        // 2D image of size 256x256
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        // Initialize it with a beige color
        &(BLACK.to_u8_array()),
        // Use the same encoding as the color we set
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    for local_point in computed {
        computed_points.push(local_point.point.into());
        uvs.push(local_point.uvs);
        normals.push(local_point.normal.into());

        if let Some(pixel) = image.pixel_bytes_mut(UVec3::new(local_point.u, local_point.v, 0)) {
            let color = curvature_to_color(
                curvature_mode,
                &local_point.normal,
                &local_point.u_diff_1,
                &local_point.v_diff_1,
                &local_point.u_diff_2,
                &local_point.v_diff_2,
                scale,
            );
            pixel[0] = { color.0 * u8::MAX as f64 } as u8;
            pixel[1] = { color.1 * u8::MAX as f64 } as u8;
            pixel[2] = { color.2 * u8::MAX as f64 } as u8;
        }
    }

    let handle = images.add(image);

    let mut indizes: Vec<u32> = vec![];
    for u in 0..(w - 1) {
        for v in 0..(h - 1) {
            // Compute indizes using simple 2d => 1d conversion
            indizes.push(u * w + v);
            indizes.push(u * w + v + 1);
            indizes.push((u + 1) * w + v + 1);

            indizes.push(u * w + v);
            indizes.push((u + 1) * w + v + 1);
            indizes.push((u + 1) * w + v);
        }
    }
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, computed_points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indizes));

    (mesh, handle)
}

fn curvature_to_color(
    curvature_mode: &CurvatureDisplayMode,
    normal: &Point,
    u_diff_1: &Point,
    v_diff_1: &Point,
    u_diff_2: &Point,
    v_diff_2: &Point,
    scale: f64,
) -> (f64, f64, f64) {
    if *curvature_mode == CurvatureDisplayMode::None {
        (0.5, 0.5, 0.5)
    } else {
        let (direction, magnitude) = match *curvature_mode {
            CurvatureDisplayMode::U => {
                let direction = u_diff_2 * normal / (u_diff_2.magnitude() * normal.magnitude());
                let magnitude = u_diff_1.cross(u_diff_2).magnitude() / pow(u_diff_1.magnitude(), 3);
                (direction, magnitude)
            }
            CurvatureDisplayMode::V => {
                let direction = v_diff_2 * normal / (v_diff_2.magnitude() * normal.magnitude());
                let magnitude = v_diff_1.cross(v_diff_2).magnitude() / pow(v_diff_1.magnitude(), 3);
                (direction, magnitude)
            }
            _ => {
                let sum_1 = (u_diff_1 + v_diff_1) / 2.0;
                let sum_2 = (u_diff_2 + v_diff_2) / 2.0;
                let direction = &sum_2 * normal / (sum_2.magnitude() * normal.magnitude());
                let magnitude = sum_1.cross(&sum_2).magnitude() / pow(sum_1.magnitude(), 3);
                (direction, magnitude)
            }
        };
        // Mapping 0.0 -> 10_000.0 (and to infitiy) to 300 -> 120 (allow for wrapping over
        // 0)
        // Mapping -0.0 -> -10_000.0 (and to -infinity) 120 -> 300
        let radius = if magnitude < f64::EPSILON {
            f64::INFINITY
        } else {
            1.0 / magnitude
        };

        let max_radius: f64 = 10.0 * scale;
        #[allow(clippy::collapsible_else_if)]
        let hue = if direction >= 0.0 {
            if radius > max_radius {
                120.0
            } else {
                let ratio = radius / max_radius;
                let hue = ratio * 140.0;
                if ratio <= 20.0 {
                    hue + 340.0
                } else {
                    hue - 20.0
                }
            }
        } else {
            if radius > max_radius {
                120.0
            } else {
                let ratio = radius / max_radius;
                let hue = ratio * 140.0;
                260.0 - hue
            }
        };
        let color = Hsva::new(hue as f32, 1.0, 1.0, 1.0);
        let color = Srgba::from(color);

        (color.red as f64, color.green as f64, color.blue as f64)
    }
}
