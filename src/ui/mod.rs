pub mod button;
pub mod slider;

use crate::{
    MainCamera,
    bezier_curve::render_info::{ChangeCoordinateMode, CoordinateMode, UpdateIsoDimEvent},
};
use bevy::{
    color::palettes::tailwind::GRAY_900, ecs::relationship::RelatedSpawnerCommands, prelude::*,
    render::view::RenderLayers, sprite::Anchor,
};
use bevy_lunex::{UiStateTrait, prelude::*};
use bevy_xr_utils::tracking_utils::XrTrackedView;
use button::{ButtonClickedEvent, ButtonPlugin, ChangeTextEvent, UiButton};
use slider::{ChangeSliderValueEvent, SliderPlugin, SliderValueChangedEvent, UiSlider};
use struct_patch::Patch;

use crate::{
    bezier_curve::{
        render_info::{RenderInformation, UpdateBoxDimEvent},
        surface_click::SurfaceClickChangeset,
    },
    history::plugin::HistoryUndoEvent,
    projection::DisplayIn,
};

#[derive(Component)]
struct USlider;

#[derive(Component)]
struct VSlider;

#[derive(Resource, Patch)]
#[patch(name = "UiStateChangeset", attribute(derive(Event, Clone, Default)))]
pub struct UiState {
    u: f64,
    v: f64,
    u_box_count: u32,
    v_box_count: u32,
    u_iso_count: u32,
    v_iso_count: u32,
    box_width: f32,
    box_height: f32,
    box_depth: f32,
    coordinate_mode: CoordinateMode,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            u: 0.5,
            v: 0.5,
            u_box_count: 0,
            v_box_count: 0,
            u_iso_count: 0,
            v_iso_count: 0,
            box_width: 0.25,
            box_height: 0.1,
            box_depth: 0.25,
            coordinate_mode: CoordinateMode::default(),
        }
    }
}

fn spawn_background<'a>(
    ui: &'a mut RelatedSpawnerCommands<'_, ChildOf>,
    materials: &mut Assets<StandardMaterial>,
) -> EntityCommands<'a> {
    ui.spawn((
        Name::new("Background"),
        UiDepth::Set(-100.0),
        UiLayout::solid()
            .size(Rl(100.0))
            .scaling(Scaling::Fill)
            .pack(),
        UiColor::new(vec![(UiBase::id(), GRAY_900.with_alpha(0.6))]),
        // Provide a material to this mesh
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..Default::default()
        })),
        UiMeshPlane3d,
    ))
}

fn spawn_uv_control(ui: &mut RelatedSpawnerCommands<'_, ChildOf>, ui_state: &Res<UiState>) {
    ui.spawn((
        Name::new("U-Value"),
        UiLayout::window()
            .pos(Rl((50.0, 00.0)))
            .size((Rw(90.0), Rh(40.0)))
            .anchor(Anchor::TopCenter)
            .pack(),
    ))
    .with_children(|ui| {
        let mut slider = UiSlider::new("U".to_owned(), 0.0, 1.0, Rl((40.0, 100.0)));
        slider.set(ui_state.u as f32);
        slider.set_to_string_fn(|value| format!("{value:.3}"));
        ui.spawn((slider, USlider)).observe(
            |trigger: Trigger<SliderValueChangedEvent>,
             mut state: ResMut<UiState>,
             mut writer: EventWriter<SurfaceClickChangeset>| {
                state.u = trigger.value as f64;
                writer.write(SurfaceClickChangeset {
                    u: Some(state.u),
                    ..Default::default()
                });
            },
        );
    });

    ui.spawn((
        Name::new("V-Value"),
        UiLayout::window()
            .pos(Rl((50.0, 50.0)))
            .size((Rw(90.0), Rh(40.0)))
            .anchor(Anchor::TopCenter)
            .pack(),
    ))
    .with_children(|ui| {
        let mut slider = UiSlider::new("V".to_owned(), 0.0, 1.0, Rl((40.0, 100.0)));
        slider.set(ui_state.v as f32);
        slider.set_to_string_fn(|value| format!("{value:.3}"));
        ui.spawn((slider, VSlider)).observe(
            |trigger: Trigger<SliderValueChangedEvent>,
             mut state: ResMut<UiState>,
             mut writer: EventWriter<SurfaceClickChangeset>| {
                state.v = trigger.value as f64;
                writer.write(SurfaceClickChangeset {
                    v: Some(state.v),
                    ..Default::default()
                });
            },
        );
    });
}

fn spawn_layouted(ui: &mut RelatedSpawnerCommands<'_, ChildOf>, ui_state: Res<UiState>) {
    ui.spawn((
        Name::new("Layout Second"),
        UiLayout::window()
            .pos(Rl((0.0, 0.0)))
            .size((Rw(100.0), Rh(20.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| spawn_uv_control(ui, &ui_state));

    ui.spawn((
        Name::new("Layout Third"),
        UiLayout::window()
            .pos(Rl((0.0, 20.0)))
            .size((Rw(100.0), Rh(10.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(
            UiLayout::window()
                .size(Rl((40.0, 100.0)))
                .pos(Rl((0.0, 0.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            ui.spawn(UiButton::new(
                ui_state.coordinate_mode.to_string(),
                3,
                Rl(100.0),
            ))
            .observe(
                |trigger: Trigger<ButtonClickedEvent>,
                 mut commands: Commands,
                 mut state: ResMut<UiState>,
                 mut writer: EventWriter<ChangeCoordinateMode>| {
                    state.coordinate_mode = state.coordinate_mode.next();
                    writer.write(ChangeCoordinateMode(state.coordinate_mode));
                    commands
                        .entity(trigger.target())
                        .trigger(ChangeTextEvent::new(state.coordinate_mode.to_string()));
                },
            );
        });
    });

    ui.spawn((
        Name::new("Layout Iso Lines"),
        UiLayout::window()
            .pos(Rl((0.0, 30.0)))
            .size((Rw(100.0), Rh(20.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 40.0)))
                .pos(Rl((5.0, 5.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("U #Iso:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set(ui_state.u_iso_count as f32);
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateIsoDimEvent>| {
                    state.u_iso_count = trigger.value as u32;
                    update_info.write(UpdateIsoDimEvent {
                        u_iso_count: Some(state.u_iso_count),
                        ..Default::default()
                    });
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 40.0)))
                .pos(Rl((5.0, 50.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("V #Iso:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            slider.set(ui_state.v_iso_count as f32);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateIsoDimEvent>| {
                    state.v_iso_count = trigger.value as u32;
                    update_info.write(UpdateIsoDimEvent {
                        v_iso_count: Some(state.v_iso_count),
                        ..Default::default()
                    });
                },
            );
        });
    });

    ui.spawn((
        Name::new("Layout Boxes"),
        UiLayout::window()
            .pos(Rl((0.0, 50.0)))
            .size((Rw(100.0), Rh(50.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 20.0)))
                .pos(Rl((5.0, 0.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("U #Box:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set(ui_state.u_box_count as f32);
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateBoxDimEvent>| {
                    state.u_box_count = trigger.value as u32;
                    update_info.write(UpdateBoxDimEvent {
                        u_box_count: Some(state.u_box_count),
                        ..Default::default()
                    });
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 20.0)))
                .pos(Rl((5.0, 20.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("V #Box:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            slider.set(ui_state.v_box_count as f32);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateBoxDimEvent>| {
                    state.v_box_count = trigger.value as u32;
                    update_info.write(UpdateBoxDimEvent {
                        v_box_count: Some(state.v_box_count),
                        ..Default::default()
                    });
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 20.0)))
                .pos(Rl((5.0, 40.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("Width Box:".to_owned(), 0.0, 2.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{value:.2}"));
            slider.set(ui_state.box_width);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateBoxDimEvent>| {
                    state.box_width = (trigger.value * 100.0).round() / 100.0;
                    update_info.write(UpdateBoxDimEvent {
                        box_dim: Some((state.box_width, state.box_height, state.box_depth)),
                        ..Default::default()
                    });
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 20.0)))
                .pos(Rl((5.0, 60.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("Height Box:".to_owned(), 0.0, 2.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{value:.2}"));
            slider.set(ui_state.box_height);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateBoxDimEvent>| {
                    state.box_height = (trigger.value * 100.0).round() / 100.0;
                    update_info.write(UpdateBoxDimEvent {
                        box_dim: Some((state.box_width, state.box_height, state.box_depth)),
                        ..Default::default()
                    });
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 20.0)))
                .pos(Rl((5.0, 80.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("Depth Box:".to_owned(), 0.0, 2.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{value:.2}"));
            slider.set(ui_state.box_depth);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateBoxDimEvent>| {
                    state.box_depth = (trigger.value * 100.0).round() / 100.0;
                    update_info.write(UpdateBoxDimEvent {
                        box_dim: Some((state.box_width, state.box_height, state.box_depth)),
                        ..Default::default()
                    });
                },
            );
        });
    });
}

fn build_ui(
    mut commands: Commands,
    ui_state: Res<UiState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scale_info: Res<RenderInformation>,
) {
    // Spawn it 3 times
    // Spawn the floating UI panel
    commands
        .spawn((
            // Required to mark this as 3D
            UiRoot3d,
            // Use this constructor to init 3D settings
            UiLayoutRoot::new_3d(),
            // Provide default size instead of camera
            Dimension::from((1.0 * scale_info.scale, 2.0 * scale_info.scale)),
            // The location of the UI panel
            Transform::from_xyz(0.0, 0.0, -5.0 * scale_info.scale),
            RenderLayers::from(DisplayIn::Normal),
        ))
        .with_children(|ui| {
            spawn_background(ui, materials.as_mut()); // spawn_text(ui, materials.as_mut());
            spawn_layouted(ui, ui_state);
        });
}

#[cfg(not(feature = "vr_enable"))]
fn follow_camera(
    camera: Query<&Transform, (With<MainCamera>, Without<UiLayoutRoot>)>,
    mut ui: Query<&mut Transform, (With<UiLayoutRoot>, Without<Camera>)>,
) {
    if let Ok(camera) = camera.single() {
        for mut ui in ui.iter_mut() {
            let diff = ui.translation - camera.translation;
            ui.look_to(diff, Vec3::Y);
        }
    }
}

#[cfg(feature = "vr_enable")]
fn follow_camera(
    camera: Query<&GlobalTransform, (With<XrTrackedView>, Without<UiLayoutRoot>)>,
    mut ui: Query<&mut Transform, (With<UiLayoutRoot>, Without<XrTrackedView>)>,
) {
    if let Ok(camera) = camera.single() {
        for mut ui in ui.iter_mut() {
            let diff = camera.translation() - ui.translation;
            ui.look_to(-diff, Vec3::Y);
        }
    }
}

fn handle_ui_state_change(
    mut event_reader: EventReader<UiStateChangeset>,
    mut state: ResMut<UiState>,
    u_sliders: Query<Entity, (With<USlider>, Without<VSlider>)>,
    v_sliders: Query<Entity, (With<VSlider>, Without<USlider>)>,
    mut commands: Commands,
) {
    for evt in event_reader.read() {
        state.apply(evt.clone());

        for u_slider in u_sliders {
            if let Ok(mut slider) = commands.get_entity(u_slider) {
                slider.trigger(ChangeSliderValueEvent::new(state.u as f32));
            }
        }

        for v_slider in v_sliders {
            if let Ok(mut slider) = commands.get_entity(v_slider) {
                slider.trigger(ChangeSliderValueEvent::new(state.v as f32));
            }
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins); // , UiLunexDebugPlugin::<0, 0>));
        app.add_plugins((SliderPlugin, ButtonPlugin));
        app.add_event::<UiStateChangeset>();
        app.init_resource::<UiState>();
        app.insert_resource(LoadFonts {
            font_directories: vec!["assets/fonts".to_owned()],
            ..default()
        });
        app.add_systems(Startup, build_ui);
        app.add_systems(Update, follow_camera.chain());
        app.add_systems(PostUpdate, handle_ui_state_change);
    }
}
