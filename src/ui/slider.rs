use std::sync::Arc;

use bevy::{
    color::palettes::{css::WHITE, tailwind::GRAY_800},
    ecs::relationship::RelatedSpawnerCommands,
    prelude::*,
    sprite::Anchor,
};
use bevy_lunex::{Rl, UiColor, UiDepth, UiLayout, UiMeshPlane3d, prelude::*};

use crate::picking3d::picking_3d::Picking3dInteractable;

#[derive(Event)]
pub struct ChangeSliderValueEvent {
    new_value: f32,
}
impl ChangeSliderValueEvent {
    pub fn new(new_value: f32) -> Self {
        Self { new_value }
    }
}

#[derive(Event)]
pub struct SliderValueChangedEvent {
    pub value: f32,
}

#[derive(Component)]
struct SliderBackground;

fn slider_value_change(
    trigger: Trigger<ChangeSliderValueEvent>,
    mut slider: Query<(&mut UiSlider, &Children)>,
    slider_background: Query<Entity, With<SliderBackground>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text3d: Query<&mut Text3d>,
) {
    if let Ok((mut slider, children)) = slider.get_mut(trigger.target()) {
        slider.set(trigger.new_value);

        for child in children {
            if let Ok(entity) = slider_background.get(*child) {
                commands.get_entity(entity).unwrap().despawn();
            }
        }

        commands
            .get_entity(trigger.target())
            .unwrap()
            .with_children(|ui| {
                spawn_background(ui, slider.as_ref(), &mut materials);
            });

        if let Some(entity) = slider.slider_text
            && let Ok(mut text) = text3d.get_mut(entity)
        {
            *text = Text3d::new(format!("{}: {:.2}", slider.text_prefix, slider.get()));
        }
    }
}

#[allow(clippy::complexity)]
fn slider_drag(
    trigger: Trigger<Pointer<Drag>>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut slider: Query<(&mut UiSlider, &Children)>,
    slider_background: Query<Entity, With<SliderBackground>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text3d: Query<&mut Text3d>,
    root: Query<&GlobalTransform, With<UiLayoutRoot>>,
) {
    let (camera, camera_transform) = camera.single().unwrap();

    if let Ok((mut slider, children)) = slider.get_mut(trigger.target()) {
        let diff = {
            let mouse_start = camera
                .viewport_to_world(
                    camera_transform,
                    trigger.pointer_location.position - trigger.delta,
                )
                .unwrap();

            let mouse_end = camera
                .viewport_to_world(camera_transform, trigger.pointer_location.position)
                .unwrap();

            let start = mouse_start.get_point(1.0);
            let end = mouse_end.get_point(1.0);
            end - start
        };

        let axis = root.single().unwrap().right().as_vec3();
        let direction = (diff.dot(axis)) / (diff.length() * axis.length());

        let delta_x = direction * diff.length() * 10.0;
        let old_value = slider.get();
        slider.set(old_value + delta_x);

        for child in children {
            if let Ok(entity) = slider_background.get(*child) {
                commands.get_entity(entity).unwrap().despawn();
            }
        }

        commands
            .get_entity(trigger.target())
            .unwrap()
            .with_children(|ui| {
                spawn_background(ui, slider.as_ref(), &mut materials);
            })
            .trigger(SliderValueChangedEvent {
                value: slider.get(),
            });

        if let Some(entity) = slider.slider_text
            && let Ok(mut text) = text3d.get_mut(entity)
        {
            *text = Text3d::new(format!("{}: {:.2}", slider.text_prefix, slider.get()));
        }
    }
}

fn spawn_background<'s>(
    ui: &mut RelatedSpawnerCommands<'s, ChildOf>,
    slider: &UiSlider,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let percentage = slider.as_percent();
    // Background
    ui.spawn((
        UiLayout::window()
            .size(Rl((percentage * 100.0, 100.0)))
            .pos(Rl((0.0, 0.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
        UiDepth::Add(10.0),
        UiColor::from(Color::from(WHITE).with_alpha(0.5)),
        UiMeshPlane3d,
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..Default::default()
        })),
        SliderBackground,
        Pickable::IGNORE,
    ));

    ui.spawn((
        UiLayout::window()
            .size(Rl(((1.0 - percentage) * 100.0, 100.0)))
            .pos(Rl((percentage * 100.0, 0.0)))
            .anchor(Anchor::TopLeft)
            .pack(),
        UiDepth::Add(10.0),
        UiColor::from(Color::from(GRAY_800).with_alpha(0.5)),
        UiMeshPlane3d,
        MeshMaterial3d(materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..Default::default()
        })),
        SliderBackground,
        Pickable::IGNORE,
    ));
}

fn spawn_children<'s>(
    ui: &mut RelatedSpawnerCommands<'s, ChildOf>,
    slider: &mut UiSlider,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let value = slider.get();

    let default_mat = materials.add(StandardMaterial {
        base_color_texture: Some(TextAtlas::DEFAULT_IMAGE),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..Default::default()
    });

    // Background
    spawn_background(ui, slider, materials);

    // Slider
    ui.spawn((
        UiLayout::window().size(Rl((100.0, 100.0))).pack(),
        UiDepth::Add(20.0),
        UiMeshPlane3d,
        Visibility::default(),
    ))
    .with_children(|ui| {
        let text_entity = ui
            .spawn((
                UiLayout::window()
                    .pos(Rl((50.0, 50.0)))
                    .size(slider.size)
                    .anchor(Anchor::Center)
                    .pack(),
                UiColor::from(Color::WHITE),
                Text3d::new(format!("{}: {value:.2}", slider.text_prefix)),
                Text3dStyling {
                    size: 64.0,
                    color: Srgba::new(1., 1., 1., 1.),
                    align: TextAlign::Center,
                    font: Arc::from("Rajdhani"),
                    weight: Weight::BOLD,
                    ..Default::default()
                },
                MeshMaterial3d(default_mat),
                Mesh3d::default(),
                // OnHoverSetCursor::new(SystemCursorIcon::Pointer),
                Pickable::IGNORE,
            ))
            .id();
        slider.slider_text = Some(text_entity)
    });
}

fn on_add(
    mut commands: Commands,
    mut added: Query<(Entity, &mut UiSlider), Added<UiSlider>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for slider in added.iter_mut() {
        let entity = slider.0;
        let mut slider = slider.1;

        if let Ok(mut entity) = commands.get_entity(entity) {
            entity
                .insert((
                    UiLayout::solid().size(Rl(100.0)).pack(),
                    UiMeshPlane3d,
                    Picking3dInteractable,
                ))
                .with_children(|ui| {
                    spawn_children(ui, slider.as_mut(), &mut materials);
                })
                .observe(slider_drag)
                .observe(slider_value_change);
        }
    }
}

#[derive(Component)]
#[require(Visibility)]
pub struct UiSlider {
    text_prefix: String,
    min_value: f32,
    max_value: f32,
    value: f32,
    slider_text: Option<Entity>,
    size: UiValue<Vec2>,
}

impl UiSlider {
    pub fn new(
        text_prefix: String,
        min_value: f32,
        max_value: f32,
        size: impl Into<UiValue<Vec2>>,
    ) -> Self {
        Self {
            text_prefix,
            min_value,
            max_value,
            value: 0.0,
            slider_text: None,
            size: size.into(),
        }
    }

    pub fn set(&mut self, new_value: f32) {
        self.value = new_value.clamp(self.min_value, self.max_value);
    }

    pub fn get(&self) -> f32 {
        self.value
    }

    /// get value between 0 and 1 in respect to max and min value
    pub fn as_percent(&self) -> f32 {
        (self.value - self.min_value) / (self.max_value - self.min_value)
    }
}

pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, on_add);
        app.add_event::<ChangeSliderValueEvent>();
        app.add_event::<SliderValueChangedEvent>();
    }
}
