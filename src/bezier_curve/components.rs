use bevy::prelude::*;

use super::bezier_curve_renderer::generic_on_despawn_trigger;

#[derive(States, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Debug)]
pub enum ControlState {
    #[default]
    Main,
    CreateCurve,
    CreatePlane,
    Delete,
    CreateOrthoCamera,
    Minus,
    Plus,
    Align,
}

#[derive(Component)]
#[component(on_despawn = generic_on_despawn_trigger)]
pub struct RenderPoint(pub usize, pub usize);

// #[derive(Component)]
// #[require(Transform, Visibility)]
// pub struct C1ControlPoint(pub i32, pub i32, pub usize, pub usize);

#[derive(Component)]
pub struct ResultSurface;

#[derive(Component)]
pub struct ResultLines;

#[derive(Component)]
pub struct CurveBox;
