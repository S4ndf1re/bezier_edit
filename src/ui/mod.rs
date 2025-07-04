pub mod button;
pub mod slider;

use bevy::{
    color::palettes::tailwind::GRAY_900, ecs::relationship::RelatedSpawnerCommands, prelude::*,
    sprite::Anchor,
};
use bevy_lunex::{UiStateTrait, prelude::*};
use button::{ButtonClickedEvent, ButtonPlugin, ChangeTextEvent, UiButton};
use slider::{ChangeSliderValueEvent, SliderPlugin, SliderValueChangedEvent, UiSlider};
use struct_patch::Patch;

use crate::{
    bezier_curve::{
        curvature_display_mode::{ChangeCurvatureDisplayModeEvent, CurvatureDisplayMode},
        render_info::{
            ChangeSurfaceMeshMode, RenderInformation, SurfaceMeshMode, UpdateBoxDimEvent,
        },
        surface_click::SurfaceClickChangeset,
    },
    history::plugin::HistoryUndoEvent,
};
use bevy_xr_utils::tracking_utils::XrTrackedView;

#[derive(Component)]
struct USlider;

#[derive(Component)]
struct VSlider;

#[derive(Resource, Default, Patch)]
#[patch(name = "UiStateChangeset", attribute(derive(Event, Clone, Default)))]
pub struct UiState {
    u: f64,
    v: f64,
    curvature_mode: CurvatureDisplayMode,
    u_box_count: u32,
    v_box_count: u32,
    surface_mesh_mode: SurfaceMeshMode,
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

fn spawn_lock_buttons(ui: &mut RelatedSpawnerCommands<'_, ChildOf>) {
    ui.spawn((
        Name::new("Button Background Lock"),
        UiLayout::window()
            .pos(Rl((25.0, 50.0)))
            .size((Rw(40.0), Rh(90.0)))
            .anchor(Anchor::Center)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(UiButton::new("Lock".to_owned(), 6, Rl(100.0)));
    });

    ui.spawn((
        Name::new("Button Background Unlock"),
        UiLayout::window()
            .pos(Rl((75.0, 50.0)))
            .size((Rw(40.0), Rh(90.0)))
            .anchor(Anchor::Center)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(UiButton::new("Unlock".to_owned(), 6, Rl(100.0)));
    });
}

fn spawn_uv_control(ui: &mut RelatedSpawnerCommands<'_, ChildOf>) {
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
        slider.set(0.5);
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
        slider.set(0.5);
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

fn spawn_layouted(ui: &mut RelatedSpawnerCommands<'_, ChildOf>, ui_state: ResMut<UiState>) {
    ui.spawn((
        Name::new("Layout First"),
        UiLayout::window()
            .pos(Rl(0.0))
            .size((Rw(100.0), Rh(10.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ));
    // .with_children(spawn_lock_buttons);

    ui.spawn((
        Name::new("Layout Second"),
        UiLayout::window()
            .pos(Rl((0.0, 10.0)))
            .size((Rw(100.0), Rh(30.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(spawn_uv_control);

    ui.spawn((
        Name::new("Layout Third"),
        UiLayout::window()
            .pos(Rl((0.0, 40.0)))
            .size((Rw(100.0), Rh(15.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(
            UiLayout::window()
                .size(Rl((40.0, 100.0)))
                .pos(Rl((5.0, 0.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            ui.spawn(UiButton::new(
                format!("{}", &ui_state.curvature_mode),
                4,
                Rl(100.0),
            ))
            .observe(
                |trigger: Trigger<ButtonClickedEvent>,
                 mut commands: Commands,
                 mut state: ResMut<UiState>,
                 mut writer: EventWriter<ChangeCurvatureDisplayModeEvent>| {
                    state.curvature_mode = state.curvature_mode.next();
                    if let Ok(mut entity) = commands.get_entity(trigger.target()) {
                        entity.trigger(ChangeTextEvent::new(format!("{}", state.curvature_mode)));
                    }
                    writer.write(ChangeCurvatureDisplayModeEvent(state.curvature_mode));
                },
            );
        });

        ui.spawn(
            UiLayout::window()
                .size(Rl((40.0, 100.0)))
                .pos(Rl((55.0, 0.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            ui.spawn(UiButton::new("Undo".to_owned(), 4, Rl(100.0)))
                .observe(
                    |_: Trigger<ButtonClickedEvent>, mut writer: EventWriter<HistoryUndoEvent>| {
                        writer.write(HistoryUndoEvent);
                    },
                );
        });
    });

    ui.spawn((
        Name::new("Layout Fourth"),
        UiLayout::window()
            .pos(Rl((0.0, 55.0)))
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
            let mut slider = UiSlider::new("U #Box:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set(0.0);
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
                .size(Rl((90.0, 40.0)))
                .pos(Rl((5.0, 55.0)))
                .anchor(Anchor::TopLeft)
                .pack(),
        )
        .with_children(|ui| {
            let mut slider = UiSlider::new("V #Box:".to_owned(), 0.0, 100.0, Rl((100.0, 100.0)));
            slider.set_to_string_fn(|value| format!("{}", value as u32));
            slider.set(0.0);
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
    });

    ui.spawn((
        Name::new("Layout Fifth"),
        UiLayout::window()
            .pos(Rl((5.0, 75.0)))
            .size((Rw(90.0), Rh(10.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn(UiButton::new(
            format!("MeshMode: {}", ui_state.surface_mesh_mode),
            14,
            Rl(100.0),
        ))
        .observe(
            |trigger: Trigger<ButtonClickedEvent>,
             mut commands: Commands,
             mut state: ResMut<UiState>,
             mut writer: EventWriter<ChangeSurfaceMeshMode>| {
                state.surface_mesh_mode = state.surface_mesh_mode.next();
                writer.write(ChangeSurfaceMeshMode(state.surface_mesh_mode));
                if let Ok(mut entity) = commands.get_entity(trigger.target()) {
                    entity.trigger(ChangeTextEvent::new(format!(
                        "MeshMode: {}",
                        state.surface_mesh_mode
                    )));
                }
            },
        );
    });
}

fn build_ui(
    mut commands: Commands,
    ui_state: ResMut<UiState>,
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
        ))
        .with_children(|ui| {
            spawn_background(ui, materials.as_mut()); // spawn_text(ui, materials.as_mut());
            spawn_layouted(ui, ui_state);
        });
}

#[cfg(not(feature = "vr_enable"))]
fn follow_camera(
    camera: Query<&Transform, (With<Camera>, Without<UiLayoutRoot>)>,
    mut ui: Query<&mut Transform, (With<UiLayoutRoot>, Without<Camera>)>,
) {
    let camera = camera.single().unwrap();
    for mut ui in ui.iter_mut() {
        let diff = ui.translation - camera.translation;
        ui.look_to(diff, Vec3::Y);
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
