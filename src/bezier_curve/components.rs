use bevy::prelude::*;

#[derive(States, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Debug)]
pub enum ControlState {
    #[default]
    Main,
    CreateCurve,
    CreatePlane,
    Delete,
    CreateOrthoCamera,
}

#[derive(Component)]
pub struct RenderPoint(pub usize, pub usize);

// #[derive(Component)]
// #[require(Transform, Visibility)]
// pub struct C1ControlPoint(pub i32, pub i32, pub usize, pub usize);

#[derive(Component)]
pub struct ResultSurface;

#[derive(Component)]
pub struct CurveBox;
