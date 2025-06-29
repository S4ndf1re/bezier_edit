pub mod slider;

use std::sync::Arc;

use bevy::{
    color::palettes::tailwind::{GRAY_900, RED_600},
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    sprite::Anchor,
};
use bevy_lunex::{UiStateTrait, prelude::*};
use slider::{SliderPlugin, SliderValueChangeEvent, UiSlider};
use struct_patch::Patch;

use crate::bezier_curve::{render_info::RenderInformation, surface_click::SurfaceClickChangeset};

#[derive(Component)]
struct USlider;

#[derive(Component)]
struct VSlider;

#[derive(Resource, Default, Patch)]
#[patch(name = "UiStateChangeset", attribute(derive(Event, Clone)))]
pub struct UiState {
    u: f64,
    v: f64,
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

fn spawn_lock_buttons(
    ui: &mut RelatedSpawnerCommands<'_, ChildOf>,
    materials: &mut Assets<StandardMaterial>,
) {
    ui.spawn((
        Name::new("Button Background Lock"),
        UiLayout::window()
            .pos(Rl((25.0, 50.0)))
            .size((Rw(40.0), Rh(90.0)))
            .anchor(Anchor::Center)
            .pack(),
        OnHoverSetCursor::new(bevy::window::SystemCursorIcon::Pointer),
    ))
    .with_children(|ui| {
        ui.spawn((
            UiLayout::solid().size(Rl(100.0)).pack(),
            UiHover::new().forward_speed(20.0).backward_speed(4.0),
            UiMeshPlane3d,
        ))
        .with_children(|ui| {
            ui.spawn((
                Name::new("Button Lock"),
                UiLayout::new(vec![
                    (UiBase::id(), UiLayout::window().full()),
                    (
                        UiHover::id(),
                        UiLayout::window()
                            .anchor(Anchor::Center)
                            .pos(Rl(50.0))
                            .size(Rl(120.0)),
                    ),
                ]),
                UiHover::new().forward_speed(20.0).backward_speed(4.0),
                UiColor::new(vec![
                    (UiBase::id(), Color::WHITE),
                    (UiHover::id(), RED_600.with_alpha(1.2).into()),
                ]),
                Text3d::new("Lock"),
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
    })
    .observe(hover_set::<Pointer<Over>, true>)
    .observe(hover_set::<Pointer<Out>, false>);

    ui.spawn((
        Name::new("Button Background Unlock"),
        UiLayout::window()
            .pos(Rl((75.0, 50.0)))
            .size((Rw(40.0), Rh(90.0)))
            .anchor(Anchor::Center)
            .pack(),
        OnHoverSetCursor::new(bevy::window::SystemCursorIcon::Pointer),
    ))
    .with_children(|ui| {
        ui.spawn((
            UiLayout::window().full().pack(),
            UiHover::new().forward_speed(20.0).backward_speed(4.0),
            UiMeshPlane3d,
        ))
        .with_children(|ui| {
            ui.spawn((
                Name::new("Button Unlock"),
                UiLayout::new(vec![
                    (UiBase::id(), UiLayout::window().full()),
                    (
                        UiHover::id(),
                        UiLayout::window()
                            .anchor(Anchor::Center)
                            .pos(Rl(50.0))
                            .size(Rl(120.0)),
                    ),
                ]),
                UiHover::new().forward_speed(20.0).backward_speed(4.0),
                UiColor::new(vec![
                    (UiBase::id(), Color::WHITE),
                    (UiHover::id(), RED_600.with_alpha(1.2).into()),
                ]),
                Text3d::new("Unlock"),
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
    })
    .observe(hover_set::<Pointer<Over>, true>)
    .observe(hover_set::<Pointer<Out>, false>);
}

fn spawn_uv_control(
    ui: &mut RelatedSpawnerCommands<'_, ChildOf>,
    #[allow(unused)] ui_state: ResMut<UiState>,
) {
    ui.spawn((
        Name::new("U-Value"),
        UiLayout::window()
            .pos(Rl((50.0, 00.0)))
            .size((Rw(90.0), Rh(40.0)))
            .anchor(Anchor::TopCenter)
            .pack(),
    ))
    .with_children(|ui| {
        let mut slider = UiSlider::new("U".to_owned(), 0.0, 1.0);
        slider.set(0.5);
        ui.spawn((slider, USlider));
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
        let mut slider = UiSlider::new("V".to_owned(), 0.0, 1.0);
        slider.set(0.5);
        ui.spawn((slider, VSlider));
    });
}

fn spawn_layouted(
    ui: &mut RelatedSpawnerCommands<'_, ChildOf>,
    ui_state: ResMut<UiState>,
    materials: &mut Assets<StandardMaterial>,
) {
    ui.spawn((
        Name::new("Layout First"),
        UiLayout::window()
            .pos(Rl(0.0))
            .size((Rw(100.0), Rh(20.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| spawn_lock_buttons(ui, materials));

    ui.spawn((
        Name::new("Layout Second"),
        UiLayout::window()
            .pos(Rl((0.0, 20.0)))
            .size((Rw(100.0), Rh(40.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| spawn_uv_control(ui, ui_state));

    ui.spawn((
        Name::new("Layout Third"),
        UiLayout::window()
            .pos(Rl((0.0, 60.0)))
            .size((Rw(100.0), Rh(20.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
    ))
    .with_children(|ui| {});
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
            Dimension::from((0.818 * scale_info.scale, 0.965 * scale_info.scale)),
            // The location of the UI panel
            Transform::from_xyz(0.0, 1.0, 0.0),
        ))
        .with_children(|ui| {
            spawn_background(ui, materials.as_mut()); // spawn_text(ui, materials.as_mut());
            spawn_layouted(ui, ui_state, materials.as_mut());
        });
}

#[cfg(not(feature = "vr_enable"))]
fn follow_camera(
    camera: Query<&Transform, (With<Camera>, Without<UiLayoutRoot>)>,
    mut ui: Query<&mut Transform, (With<UiLayoutRoot>, Without<Camera>)>,
) {
    let camera = camera.single().unwrap();
    for mut ui in ui.iter_mut() {
        ui.look_at(-camera.translation, Vec3::Y);
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
                slider.trigger(SliderValueChangeEvent::new(state.u as f32));
            }
        }

        for v_slider in v_sliders {
            if let Ok(mut slider) = commands.get_entity(v_slider) {
                slider.trigger(SliderValueChangeEvent::new(state.v as f32));
            }
        }
    }
}

fn handle_ui_updates(
    mut writer: EventWriter<SurfaceClickChangeset>,
    u_slider: Query<&UiSlider, (Changed<UiSlider>, With<USlider>, Without<VSlider>)>,
    v_slider: Query<&UiSlider, (Changed<UiSlider>, With<VSlider>, Without<USlider>)>,
) {
    let mut patch = SurfaceClickChangeset { u: None, v: None };

    for slider in u_slider {
        patch.u = Some(slider.get() as f64);
    }

    for slider in v_slider {
        patch.v = Some(slider.get() as f64);
    }

    writer.write(patch);
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins); // , UiLunexDebugPlugin::<0, 0>));
        app.add_plugins(SliderPlugin);
        app.add_event::<UiStateChangeset>();
        app.init_resource::<UiState>();
        app.insert_resource(LoadFonts {
            font_directories: vec!["assets/fonts".to_owned()],
            ..default()
        });
        app.add_systems(Startup, build_ui);
        app.add_systems(Update, follow_camera.chain());
        app.add_systems(PostUpdate, (handle_ui_state_change, handle_ui_updates));
    }
}
