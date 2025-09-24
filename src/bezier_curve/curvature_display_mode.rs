use std::fmt::Display;

use bevy::prelude::*;

use super::{bezier_curve_renderer::RedrawEvent, render_info::RenderInformation};

#[derive(Event)]
pub struct ChangeCurvatureDisplayModeEvent(pub CurvatureDisplayMode);

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CurvatureDisplayMode {
    #[default]
    None,
    U,
    V,
    Both,
    CustomColor(Color),
}

impl CurvatureDisplayMode {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::U,
            Self::U => Self::V,
            Self::V => Self::Both,
            Self::Both => Self::None,
            Self::CustomColor(c) => Self::CustomColor(c),
        }
    }
}

impl Display for CurvatureDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CurvatureDisplayMode::None => write!(f, "None"),
            CurvatureDisplayMode::U => write!(f, "U"),
            CurvatureDisplayMode::V => write!(f, "V"),
            CurvatureDisplayMode::Both => write!(f, "Both"),
            CurvatureDisplayMode::CustomColor(c) => write!(f, "Custom({:?})", c),
        }
    }
}

pub fn handle_change_curvature(
    mut reader: EventReader<ChangeCurvatureDisplayModeEvent>,
    mut state: ResMut<RenderInformation>,
    mut redraw: EventWriter<RedrawEvent>,
) {
    let mut event_received = false;

    for evt in reader.read() {
        event_received = true;

        state.curvature_mode = evt.0;
    }

    if event_received {
        redraw.write(RedrawEvent::HighQuality);
    }
}
