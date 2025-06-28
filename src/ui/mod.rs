use std::sync::Arc;

use bevy::{
    color::palettes::tailwind::{GRAY_900, RED_600},
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    sprite::Anchor,
};
use bevy_lunex::{UiStateTrait, prelude::*};
use struct_patch::Patch;

use crate::bezier_curve::render_info::RenderInformation;

#[derive(Component)]
struct UText;

#[derive(Component)]
struct VText;

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
        Transform::from_xyz(0.0, 0.0, 0.0),
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
    materials: &mut Assets<StandardMaterial>,
) {
    ui.spawn((
        Name::new("U-Value"),
        UiLayout::window()
            .pos(Rl((25.0, 50.0)))
            .size((Rw(40.0), Rh(40.0)))
            .anchor(Anchor::BottomCenter)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn((UiLayout::solid().size(Rl(100.0)).pack(), UiMeshPlane3d))
            .with_children(|ui| {
                ui.spawn((
                    Name::new("U-Value text"),
                    UiLayout::new(vec![(UiBase::id(), UiLayout::window().full())]),
                    UiColor::new(vec![(UiBase::id(), Color::WHITE)]),
                    UText,
                    Text3d::new("U: None"),
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
                ))
                .with_children(|ui| {
                    // TODO: Add background based on u, v selection
                });
            });
    });

    ui.spawn((
        Name::new("V-Value"),
        UiLayout::window()
            .pos(Rl((75.0, 50.0)))
            .size((Rw(40.0), Rh(40.0)))
            .anchor(Anchor::BottomCenter)
            .pack(),
    ))
    .with_children(|ui| {
        ui.spawn((UiLayout::window().full().pack(), UiMeshPlane3d))
            .with_children(|ui| {
                ui.spawn((
                    Name::new("V-Value Text"),
                    UiLayout::new(vec![(UiBase::id(), UiLayout::window().full())]),
                    UiColor::new(vec![(UiBase::id(), Color::WHITE)]),
                    VText,
                    Text3d::new("V: None"),
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
    .with_children(|ui| spawn_uv_control(ui, ui_state, materials));
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
    mut u_text: Query<&mut Text3d, (With<UText>, Without<VText>)>,
    mut v_text: Query<&mut Text3d, (With<VText>, Without<UText>)>,
) {
    for evt in event_reader.read() {
        state.apply(evt.clone());
        let mut u_text = u_text.single_mut().unwrap();
        let mut v_text = v_text.single_mut().unwrap();

        *u_text = Text3d::new(format!("U: {:.2}", state.u));
        *v_text = Text3d::new(format!("V: {:.2}", state.v));
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(UiLunexPlugins); // , UiLunexDebugPlugin::<0, 0>));
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
