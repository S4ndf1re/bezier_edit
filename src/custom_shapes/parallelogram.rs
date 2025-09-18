use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::mesh::{Extrudable, Indices, PerimeterSegment, PrimitiveTopology};
use bevy::{
    math::{
        Isometry2d, Vec2,
        bounding::{Aabb2d, Bounded2d, BoundingCircle},
    },
    render::mesh::{MeshBuilder, Meshable},
};

pub struct Parallelogram2dBuilder {
    parallelogram: Parallelogram2d,
}

#[derive(Clone, Copy)]
pub struct Parallelogram2d {
    x_length: f32,
    z_length: f32,
    angle: f32,
}

impl Parallelogram2d {
    pub fn new(angle: f32, x_length: f32, z_length: f32) -> Self {
        Self {
            x_length,
            z_length,
            angle,
        }
    }

    #[inline]
    fn get_directional_vectors(&self) -> (Vec2, Vec2) {
        let x = Vec2::new(0.0, self.x_length);
        let z = Vec2::new(0.0, self.z_length);

        let quat = Quat::from_axis_angle(Vec3::Z, self.angle);
        let x_3d = quat.mul_vec3(Vec3::new(x.x, x.y, 0.0));
        let x = Vec2::new(x_3d.x, x_3d.y);

        (x, z)
    }

    /// To points in counter clockwise orentation
    fn as_points(&self) -> Vec<Vec2> {
        let mut points = [Vec2::default(); 4];

        let (x, z) = self.get_directional_vectors();

        points[0] = Vec2::ZERO;
        points[1] = z;
        points[2] = z + x;
        points[3] = x;

        let midpoint: Vec2 = points.iter().map(|p| p / 4.0).sum();
        points.iter().map(|p| p - midpoint).collect()
    }
}

impl Primitive2d for Parallelogram2d {}

impl Bounded2d for Parallelogram2d {
    fn aabb_2d(&self, isometry: impl Into<Isometry2d>) -> Aabb2d {
        Aabb2d::from_point_cloud(isometry, &self.as_points())
    }

    fn bounding_circle(&self, isometry: impl Into<bevy::math::Isometry2d>) -> BoundingCircle {
        BoundingCircle::from_point_cloud(isometry, &self.as_points())
    }
}

impl MeshBuilder for Parallelogram2dBuilder {
    fn build(&self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );

        let points = self.parallelogram.as_points();

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            points.iter().map(|p| [p.x, p.y, 0.0]).collect::<Vec<_>>(),
        );
        mesh.insert_indices(Indices::U32(vec![0, 1, 3, 1, 2, 3]));
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4]);

        mesh
    }
}

impl Meshable for Parallelogram2d {
    type Output = Parallelogram2dBuilder;
    fn mesh(&self) -> Self::Output {
        Parallelogram2dBuilder {
            parallelogram: *self,
        }
    }
}

impl Extrudable for Parallelogram2dBuilder {
    fn perimeter(&self) -> Vec<bevy::render::mesh::PerimeterSegment> {
        vec![PerimeterSegment::Flat {
            indices: vec![0, 1, 2, 3, 0],
        }]
    }
}
