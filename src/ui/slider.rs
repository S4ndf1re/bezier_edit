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
pub struct SliderValueChangeEvent {
    new_value: f32,
}
impl SliderValueChangeEvent {
    pub fn new(new_value: f32) -> Self {
        Self { new_value }
    }
}

#[derive(Component)]
struct SliderBackground;

fn slider_value_change(
    trigger: Trigger<SliderValueChangeEvent>,
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

fn slider_drag(
    trigger: Trigger<Pointer<Drag>>,
    mut slider: Query<(&mut UiSlider, &Children)>,
    slider_background: Query<Entity, With<SliderBackground>>,
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut text3d: Query<&mut Text3d>,
) {
    if let Ok((mut slider, children)) = slider.get_mut(trigger.target()) {
        let delta_x = trigger.event().delta.x * 0.01;
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
                    .size(Rl((20.0, 100.0)))
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
}

impl UiSlider {
    pub fn new(text_prefix: String, min_value: f32, max_value: f32) -> Self {
        Self {
            text_prefix,
            min_value,
            max_value,
            value: 0.0,
            slider_text: None,
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
    }
}
