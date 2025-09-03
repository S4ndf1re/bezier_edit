pub mod bezier_curve_renderer;
pub mod components;
pub mod curvature_display_mode;
pub mod helper_curves;
pub mod ortho_camera;
pub mod render_info;
pub mod surface_click;
pub mod util;

use bevy::prelude::*;

#[derive(Event)]
pub struct EntityDeletedEvent(pub Entity);
