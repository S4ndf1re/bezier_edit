use bevy::prelude::*;

#[derive(Component)]
pub struct SurfaceClick(pub f64, pub f64);

#[derive(Component)]
pub struct RenderPoint(pub usize, pub usize);

#[derive(Component)]
#[require(Transform, Visibility)]
pub struct C1ControlPoint(pub i32, pub i32, pub usize, pub usize);

#[derive(Component)]
pub struct ResultSurface;

#[derive(Component)]
pub struct BezierRender;

#[derive(Component)]
#[require(Mesh3d)]
pub struct RenderLine(pub Entity, pub Entity);
