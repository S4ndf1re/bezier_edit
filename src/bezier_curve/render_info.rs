use std::fmt::Display;

use bevy::prelude::*;

use super::{
    bezier_curve_renderer::{RedrawBoxesEvent, RedrawEvent, RedrawLinesEvent, Resolution},
    curvature_display_mode::CurvatureDisplayMode,
};

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum CoordinateMode {
    #[default]
    XYZ,
    NUV,
    NU,
    NV,
}

impl CoordinateMode {
    pub fn next(self) -> Self {
        match self {
            Self::XYZ => Self::NU,
            Self::NU => Self::NV,
            Self::NV => Self::NUV,
            Self::NUV => Self::XYZ,
        }
    }
}

impl Display for CoordinateMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoordinateMode::XYZ => write!(f, "XYZ"),
            CoordinateMode::NUV => write!(f, "NUV"),
            CoordinateMode::NU => write!(f, "NU"),
            CoordinateMode::NV => write!(f, "NV"),
        }
    }
}

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

#[derive(Event)]
pub struct ChangeCoordinateMode(pub CoordinateMode);

#[derive(Event, Default)]
pub struct UpdateBoxDimEvent {
    pub u_box_count: Option<u32>,
    pub v_box_count: Option<u32>,
    pub box_dim: Option<(f32, f32, f32)>,
}

#[derive(Event, Default)]
pub struct UpdateIsoDimEvent {
    pub iso_count: Option<u32>,
}

pub enum UVEither {
    U(f64),
    V(f64),
}

#[derive(Resource, Clone)]
pub struct RenderInformation {
    pub scale: f32,
    pub height: f32,
    pub resolution: Resolution,
    pub fast_resolution: Resolution,
    pub curvature_mode: CurvatureDisplayMode,
    pub iso_count: u32,
    pub u_box_count: u32,
    pub v_box_count: u32,
    pub box_dim: (f32, f32, f32),
    pub surface_mesh_mode: SurfaceMeshMode,
    pub coordinate_mode: CoordinateMode,
}

impl RenderInformation {
    pub fn to_uv_sample(&self) -> Vec<(f64, f64)> {
        if self.u_box_count == 0 || self.v_box_count == 0 {
            return vec![];
        }

        let u_step = 1.0 / (self.u_box_count as f64 + 1.0);
        let v_step = 1.0 / (self.v_box_count as f64 + 1.0);

        (0..=self.u_box_count + 1)
            .flat_map(|u| {
                (0..=self.v_box_count + 1).map(move |v| ((u as f64) * u_step, (v as f64) * v_step))
            })
            .collect::<Vec<_>>()
    }

    pub fn to_line_uv(&self) -> Vec<UVEither> {
        if self.iso_count == 0 {
            return vec![];
        }

        let step = 1.0 / (self.iso_count as f64 + 1.0);

        let mut result = Vec::new();

        // Start and end are fixed and always present
        result.push(UVEither::U(0.0));
        result.push(UVEither::V(0.0));

        // NOTE: add 1 to actually reach the end, since one is added to the step as wel
        for i in 0..=self.iso_count + 1 {
            let uv_coord = (i as f64) * step;
            result.push(UVEither::U(uv_coord));
            result.push(UVEither::V(uv_coord));
        }

        result
    }
}

impl Default for RenderInformation {
    fn default() -> Self {
        Self {
            scale: 1.0,
            height: 0.0,
            resolution: (100, 100),
            fast_resolution: (25, 25),
            curvature_mode: CurvatureDisplayMode::None,
            iso_count: 0,
            u_box_count: 0,
            v_box_count: 0,
            box_dim: (0.25, 0.10, 0.25),
            surface_mesh_mode: SurfaceMeshMode::default(),
            coordinate_mode: CoordinateMode::default(),
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
        if let Some(count) = evt.iso_count {
            info.iso_count = count;
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

pub fn handle_change_coordinate_mode(
    mut reader: EventReader<ChangeCoordinateMode>,
    mut info: ResMut<RenderInformation>,
    mut redraw_writer: EventWriter<RedrawBoxesEvent>,
) {
    let mut redraw = false;

    for evt in reader.read() {
        info.coordinate_mode = evt.0;
        redraw = true;
    }

    if redraw {
        redraw_writer.write(RedrawBoxesEvent);
    }
}
