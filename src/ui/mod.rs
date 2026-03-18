pub mod button;
pub mod slider;

use std::sync::Arc;

#[cfg(not(feature = "vr_enable"))]
use crate::MainCamera;

use crate::bezier_curve::{
    render_info::{ChangeCoordinateMode, CoordinateMode, UpdateIsoDimEvent},
    test_mode::{EnterEvalEvent, EvaluationFlowState},
};
use bevy::{
    color::palettes::tailwind::GRAY_900, ecs::relationship::RelatedSpawnerCommands, prelude::*,
    render::view::RenderLayers, sprite::Anchor,
};
use bevy_lunex::{UiStateTrait, prelude::*};

#[cfg(feature = "vr_enable")]
use bevy_xr_utils::tracking_utils::XrTrackedView;
use button::{ButtonClickedEvent, ButtonPlugin, ChangeTextEvent, UiButton};
use slider::{ChangeSliderValueEvent, SliderPlugin, SliderValueChangedEvent, UiSlider};
use struct_patch::Patch;

use crate::{
    bezier_curve::{
        inspector::SurfaceInspectorChangeset,
        render_info::{RenderInformation, UpdateBoxDimEvent},
    },
    projection::DisplayIn,
};

#[derive(Component)]
struct USlider;

#[derive(Component)]
struct VSlider;

/// Mark the eval state text field. it is required to have a text
#[derive(Component)]
struct EvalStateMarker;

#[derive(Component)]
struct EvalProgressMarker;

#[derive(Resource, Patch)]
#[patch(name = "UiStateChangeset", attribute(derive(Event, Clone, Default)))]
pub struct UiState {
    u: f64,
    v: f64,
    u_box_count: u32,
    v_box_count: u32,
    iso_count: u32,
    box_width: f32,
    box_height: f32,
    box_depth: f32,
    coordinate_mode: CoordinateMode,
    number_of_points_and_curves: usize,
    eval_state: EvaluationFlowState,
    eval_current: usize,
    eval_count: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            u: 0.5,
            v: 0.5,
            u_box_count: 0,
            v_box_count: 0,
            iso_count: 0,
            box_width: 0.25,
            box_height: 0.1,
            box_depth: 0.25,
            coordinate_mode: default(),
            number_of_points_and_curves: 1,
            eval_state: default(),
            eval_count: 0,
            eval_current: 0,
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
             mut writer: EventWriter<SurfaceInspectorChangeset>| {
                state.u = trigger.value as f64;
                writer.write(SurfaceInspectorChangeset {
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
             mut writer: EventWriter<SurfaceInspectorChangeset>| {
                state.v = trigger.value as f64;
                writer.write(SurfaceInspectorChangeset {
                    v: Some(state.v),
                    ..Default::default()
                });
            },
        );
    });
}

fn spawn_layouted(
    ui: &mut RelatedSpawnerCommands<'_, ChildOf>,
    ui_state: Res<UiState>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    ui.spawn((
        Name::new("Layout Second"),
        UiLayout::window()
            .pos(Rl((0.0, 0.0)))
            .size((Rw(100.0), Rh(20.0 * 2.0 / 3.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| spawn_uv_control(ui, &ui_state));

    ui.spawn((
        Name::new("Layout Third"),
        UiLayout::window()
            .pos(Rl((0.0, 20.0 * 2.0 / 3.0)))
            .size((Rw(100.0), Rh(10.0 * 2.0 / 3.0)))
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
            .pos(Rl((0.0, 30.0 * 2.0 / 3.0)))
            .size((Rw(100.0), Rh(20.0 * 2.0 / 3.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(
            UiLayout::window()
                .size(Rl((90.0, 40.0)))
                .pos(Rl((5.0, 30.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("#Iso:".to_owned(), 0.0, 9.0, Rl((100.0, 100.0)));
            slider.set(ui_state.iso_count as f32);
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>,
                 mut state: ResMut<UiState>,
                 mut update_info: EventWriter<UpdateIsoDimEvent>| {
                    state.iso_count = trigger.value as u32;
                    update_info.write(UpdateIsoDimEvent {
                        iso_count: Some(state.iso_count),
                    });
                },
            );
        });
    });

    ui.spawn((
        Name::new("Layout Boxes"),
        UiLayout::window()
            .pos(Rl((0.0, 50.0 * 2.0 / 3.0)))
            .size((Rw(100.0), Rh(50.0 * 2.0 / 3.0)))
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
            let mut slider = UiSlider::new("U #Box:".to_owned(), 0.0, 9.0, Rl((100.0, 100.0)));
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
            let mut slider = UiSlider::new("V #Box:".to_owned(), 0.0, 9.0, Rl((100.0, 100.0)));
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
            let mut slider = UiSlider::new("Width Box:".to_owned(), 0.0, 0.5, Rl((100.0, 100.0)));
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
            let mut slider = UiSlider::new("Height Box:".to_owned(), 0.0, 0.5, Rl((100.0, 100.0)));
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
            let mut slider = UiSlider::new("Depth Box:".to_owned(), 0.0, 0.5, Rl((100.0, 100.0)));
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

    // NOTE: The above layout takes up exactly 66% of the ui. The last 33% can be used for
    // evaluation

    ui.spawn((
        UiLayout::window()
            .size(Rl((100.0, 1.0 / 3.0 * 100.0)))
            .pos(Rl((0.0, 2.0 / 3.0 * 100.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
        Name::new("Evaluation UI Node"),
    ))
    .with_children(|ui| {
        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl(0.0))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Button Next"),
        ))
        .with_children(|ui| {
            ui.spawn(UiButton::new("Next Evaluation".to_string(), 15, Rl(100.0)))
                .observe(
                    |_trigger: Trigger<ButtonClickedEvent>,
                     mut enter_eval_event_writer: EventWriter<EnterEvalEvent>| {
                        enter_eval_event_writer.write(EnterEvalEvent::Next);
                    },
                );
        });

        #[cfg(not(feature = "evaluation"))]
        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl((0.0, 1.0 / 4.0 * 100.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Buttons Surface and Curves"),
        ))
        .with_children(|ui| {
            ui.spawn((
                UiLayout::window()
                    .pos(Rl((2.5, 0.0)))
                    .size(Rl((45.0, 100.0)))
                    .anchor(Anchor::TopLeft)
                    .pack(),
                Name::new("New Surface"),
            ))
            .with_children(|ui| {
                ui.spawn(UiButton::new("Surface".to_string(), 7, Rl(100.0)))
                    .observe(
                        |_trigger: Trigger<ButtonClickedEvent>,
                         mut enter_eval_event_writer: EventWriter<EnterEvalEvent>| {
                            enter_eval_event_writer.write(EnterEvalEvent::Surface);
                        },
                    );
            });

            ui.spawn((
                UiLayout::window()
                    .pos(Rl((52.5, 0.0)))
                    .size(Rl((45.0, 100.0)))
                    .anchor(Anchor::TopLeft)
                    .pack(),
                Name::new("New Curves"),
            ))
            .with_children(|ui| {
                ui.spawn(UiButton::new("Curves".to_string(), 6, Rl(100.0)))
                    .observe(
                        |_trigger: Trigger<ButtonClickedEvent>,
                        state: Res<UiState>,
                         mut enter_eval_event_writer: EventWriter<EnterEvalEvent>| {
                            enter_eval_event_writer.write(EnterEvalEvent::Curves(state.number_of_points_and_curves));
                        },
                    );
            });
        });

        #[cfg(not(feature = "evaluation"))]
        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl((0.0, 2.0 / 4.0 * 100.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Buttons Linear and Precision"),
        ))
        .with_children(|ui| {
            ui.spawn((
                UiLayout::window()
                    .pos(Rl((2.5, 0.0)))
                    .size(Rl((45.0, 100.0)))
                    .anchor(Anchor::TopLeft)
                    .pack(),
                Name::new("New Surface"),
            ))
            .with_children(|ui| {
                ui.spawn(UiButton::new("Linear".to_string(), 6, Rl(100.0)))
                    .observe(
                        |_trigger: Trigger<ButtonClickedEvent>,
                        state: Res<UiState>,
                         mut enter_eval_event_writer: EventWriter<EnterEvalEvent>| {
                            enter_eval_event_writer.write(EnterEvalEvent::Linear(state.number_of_points_and_curves));
                        },
                    );
            });

            ui.spawn((
                UiLayout::window()
                    .pos(Rl((52.5, 0.0)))
                    .size(Rl((45.0, 100.0)))
                    .anchor(Anchor::TopLeft)
                    .pack(),
                Name::new("New Curves"),
            ))
            .with_children(|ui| {
                ui.spawn(UiButton::new("Precision".to_string(), 9, Rl(100.0)))
                    .observe(
                        |_trigger: Trigger<ButtonClickedEvent>,
                        state: Res<UiState>,
                         mut enter_eval_event_writer: EventWriter<EnterEvalEvent>| {
                            enter_eval_event_writer.write(EnterEvalEvent::Precision(state.number_of_points_and_curves));
                        },
                    );
            });
        });

        #[cfg(not(feature = "evaluation"))]
        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl((0.0, 3.0 / 4.0 * 100.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Slider"),
        ))
        .with_children(|ui| {
            let mut slider = UiSlider::new(
                "# of Entities: ".to_owned(),
                1.0,
                10.0,
                Rl((100.0, 100.0)),
            );
            slider.set_to_string_fn(|value| format!("{}", value as i32));
            slider.set(ui_state.box_depth);
            ui.spawn(slider).observe(
                |trigger: Trigger<SliderValueChangedEvent>, mut state: ResMut<UiState>| {
                    state.number_of_points_and_curves = trigger.value.round() as usize;
                },
            );
        });

        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl((0.0, 100.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Eval Text"),
        ))
        .with_children(|ui| {
                    ui.spawn((
                        EvalStateMarker,
                        UiColor::new(vec![
                            (UiBase::id(), Color::WHITE),
                        ]),
                        UiLayout::solid().size(Rl(100.0)).pack(),
                        Text3d::new(ui_state.eval_state.to_string()),
                        Text3dStyling {
                            size: 64.0,
                            color: Srgba::new(1., 1., 1., 1.),
                            align: TextAlign::Center,
                            font: Arc::from("Rajdhani"),
                            weight: Weight::BOLD,
                            ..Default::default()
                        },
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..Default::default()
                        })),
                        Mesh3d::default(),
                        // OnHoverSetCursor::new(SystemCursorIcon::Pointer),
                        Pickable::IGNORE,
                    ));
        });

        ui.spawn((
            UiLayout::window()
                .size(Rl((100.0, 1.0 / 4.0 * 100.0)))
                .pos(Rl((0.0, 125.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
            Name::new("Eval Progress"),
        ))
        .with_children(|ui| {
                    ui.spawn((
                        EvalProgressMarker,
                        UiColor::new(vec![
                            (UiBase::id(), Color::WHITE),
                        ]),
                        UiLayout::solid().size(Rl(100.0)).pack(),
                        Text3d::new(format!("Eval: {}/{}", ui_state.eval_current, ui_state.eval_count)),
                        Text3dStyling {
                            size: 64.0,
                            color: Srgba::new(1., 1., 1., 1.),
                            align: TextAlign::Center,
                            font: Arc::from("Rajdhani"),
                            weight: Weight::BOLD,
                            ..Default::default()
                        },
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..Default::default()
                        })),
                        Mesh3d::default(),
                        // OnHoverSetCursor::new(SystemCursorIcon::Pointer),
                        Pickable::IGNORE,
                    ));
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
            Dimension::from((1.0 * scale_info.scale, 3.0 * scale_info.scale)),
            // The location of the UI panel
            Transform::from_xyz(0.0, 0.0, -5.0 * scale_info.scale),
            RenderLayers::from(DisplayIn::Normal),
        ))
        .with_children(|ui| {
            spawn_background(ui, materials.as_mut()); // spawn_text(ui, materials.as_mut());
            spawn_layouted(ui, ui_state, &mut materials);
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
    mut eval_texts: Query<&mut Text3d, (With<EvalStateMarker>, Without<EvalProgressMarker>)>,
    mut progress_texts: Query<&mut Text3d, (With<EvalProgressMarker>, Without<EvalStateMarker>)>,
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

        for mut t in eval_texts.iter_mut() {
            *t = Text3d::new(state.eval_state.to_string());
        }

        for mut t in progress_texts.iter_mut() {
            *t = Text3d::new(format!("Eval: {}/{}", state.eval_current, state.eval_count));
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
