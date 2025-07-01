use bevy::prelude::*;

use super::{
    bezier_curve_renderer::{RedrawBoxesEvent, Resolution},
    curvature_display_mode::CurvatureDisplayMode,
};

#[derive(Event, Default)]
pub struct UpdateBoxDimEvent {
    pub u_box_count: Option<u32>,
    pub v_box_count: Option<u32>,
    pub box_dim: Option<(f32, f32, f32)>,
}

#[derive(Resource)]
pub struct RenderInformation {
    pub scale: f32,
    pub height: f32,
    pub resolution: Resolution,
    pub fast_resolution: Resolution,
    pub curvature_mode: CurvatureDisplayMode,
    pub u_box_count: u32,
    pub v_box_count: u32,
    pub box_dim: (f32, f32, f32),
}

impl RenderInformation {
    pub fn to_uv_sample(&self) -> Vec<(f64, f64)> {
        let u_step = 1.0 / self.u_box_count as f64;
        let v_step = 1.0 / self.v_box_count as f64;

        (0..=self.u_box_count)
            .flat_map(|u| {
                (0..=self.v_box_count).map(move |v| ((u as f64) * u_step, (v as f64) * v_step))
            })
            .collect::<Vec<_>>()
    }
}

impl Default for RenderInformation {
    fn default() -> Self {
        Self {
            scale: 1.0,
            height: 0.0,
            resolution: (300, 300),
            fast_resolution: (50, 50),
            curvature_mode: CurvatureDisplayMode::None,
            u_box_count: 0,
            v_box_count: 0,
            box_dim: (0.25, 0.10, 0.25),
        }
    }
}

pub fn handle_box_dim_event(
    mut reader: EventReader<UpdateBoxDimEvent>,
    mut info: ResMut<RenderInformation>,
    mut redraw_writer: EventWriter<RedrawBoxesEvent>,
) {
    let mut redraw = false;
    for evt in reader.read() {
        if let Some(u_count) = evt.u_box_count {
            info.u_box_count = u_count;
            redraw = true;
        }

        if let Some(v_count) = evt.v_box_count {
            info.v_box_count = v_count;
            redraw = true;
        }

        if let Some(dim) = evt.box_dim {
            info.box_dim = dim;
            redraw = true;
        }
    }

    if redraw {
        redraw_writer.write(RedrawBoxesEvent);
    }
}
