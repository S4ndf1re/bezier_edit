use std::fmt::Display;

use bevy::prelude::*;

use super::{
    bezier_curve_renderer::{RedrawBoxesEvent, RedrawEvent, RedrawLinesEvent, Resolution},
    curvature_display_mode::CurvatureDisplayMode,
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum SurfaceMeshMode {
    #[default]
    Mesh,
    Lines,
}

impl SurfaceMeshMode {
    pub fn next(&self) -> Self {
        match self {
            SurfaceMeshMode::Mesh => SurfaceMeshMode::Lines,
            SurfaceMeshMode::Lines => SurfaceMeshMode::Mesh,
        }
    }
}

impl Display for SurfaceMeshMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SurfaceMeshMode::Mesh => write!(f, "Mesh"),
            SurfaceMeshMode::Lines => write!(f, "Lines"),
        }
    }
}

#[derive(Event)]
pub struct ChangeSurfaceMeshMode(pub SurfaceMeshMode);

#[derive(Event, Default)]
pub struct UpdateBoxDimEvent {
    pub u_box_count: Option<u32>,
    pub v_box_count: Option<u32>,
    pub box_dim: Option<(f32, f32, f32)>,
}

#[derive(Event, Default)]
pub struct UpdateIsoDimEvent {
    pub u_iso_count: Option<u32>,
    pub v_iso_count: Option<u32>,
}

pub enum UVEither {
    U(f64),
    V(f64),
}

#[derive(Resource)]
pub struct RenderInformation {
    pub scale: f32,
    pub height: f32,
    pub resolution: Resolution,
    pub fast_resolution: Resolution,
    pub curvature_mode: CurvatureDisplayMode,
    pub u_iso_count: u32,
    pub v_iso_count: u32,
    pub u_box_count: u32,
    pub v_box_count: u32,
    pub box_dim: (f32, f32, f32),
    pub surface_mesh_mode: SurfaceMeshMode,
}

impl RenderInformation {
    pub fn to_uv_sample(&self) -> Vec<(f64, f64)> {
        if self.u_box_count == 0 || self.v_box_count == 0 {
            return vec![];
        }

        let u_step = 1.0 / self.u_box_count as f64;
        let v_step = 1.0 / self.v_box_count as f64;

        (0..=self.u_box_count)
            .flat_map(|u| {
                (0..=self.v_box_count).map(move |v| ((u as f64) * u_step, (v as f64) * v_step))
            })
            .collect::<Vec<_>>()
    }

    pub fn to_line_uv(&self) -> Vec<UVEither> {
        if self.u_iso_count == 0 || self.v_iso_count == 0 {
            return vec![];
        }

        let u_step = 1.0 / self.u_iso_count as f64;
        let v_step = 1.0 / self.v_iso_count as f64;

        let mut result = Vec::new();
        for i in 0..=self.u_iso_count {
            result.push(UVEither::U((i as f64) * u_step));
        }

        for i in 0..=self.v_iso_count {
            result.push(UVEither::V((i as f64) * v_step));
        }

        result
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
            u_iso_count: 0,
            v_iso_count: 0,
            u_box_count: 0,
            v_box_count: 0,
            box_dim: (0.25, 0.10, 0.25),
            surface_mesh_mode: SurfaceMeshMode::default(),
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
        }

        if let Some(v_count) = evt.v_box_count {
            info.v_box_count = v_count;
        }

        if let Some(dim) = evt.box_dim {
            info.box_dim = dim;
        }
        redraw = true;
    }

    if redraw {
        redraw_writer.write(RedrawBoxesEvent);
    }
}

pub fn handle_iso_dim_event(
    mut reader: EventReader<UpdateIsoDimEvent>,
    mut info: ResMut<RenderInformation>,
    mut redraw_all: EventWriter<RedrawLinesEvent>,
) {
    let mut redraw = false;
    for evt in reader.read() {
        if let Some(u_count) = evt.u_iso_count {
            info.u_iso_count = u_count;
        }

        if let Some(v_count) = evt.v_iso_count {
            info.v_iso_count = v_count;
        }
        redraw = true;
    }

    if redraw {
        redraw_all.write(RedrawLinesEvent::HighQuality);
    }
}

pub fn handle_change_surface_mode(
    mut reader: EventReader<ChangeSurfaceMeshMode>,
    mut info: ResMut<RenderInformation>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    let mut redraw = false;

    for evt in reader.read() {
        info.surface_mesh_mode = evt.0;
        redraw = true;
    }

    if redraw {
        redraw_writer.write(RedrawEvent::HighQuality);
    }
}
