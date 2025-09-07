use std::sync::Arc;

use bevy::{
    color::palettes::tailwind::RED_600, ecs::relationship::RelatedSpawnerCommands, prelude::*,
};
use bevy_lunex::{UiStateTrait, prelude::*};

use crate::picking3d::{self, events::Pointer3d, picking_3d::Picking3dInteractable};

#[derive(Event)]
pub struct ButtonClickedEvent;

#[derive(Event)]
pub struct ChangeTextEvent {
    new_text: String,
}

impl ChangeTextEvent {
    pub fn new(new_text: String) -> Self {
        Self { new_text }
    }
}

fn handle_change_text(
    trigger: Trigger<ChangeTextEvent>,
    mut text: Query<&mut Text3d>,
    mut button: Query<&mut UiButton>,
) {
    if let Ok(mut button) = button.get_mut(trigger.target())
        && let Some(entity) = button.text_entity
        && let Ok(mut text) = text.get_mut(entity)
    {
        button.set_text(trigger.new_text.clone());
        *text = Text3d::new(button.text.clone());
    }
}

fn button_clicked(trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(trigger.target()) {
        entity.trigger(ButtonClickedEvent);
    }
}

fn button_clicked3d(trigger: Trigger<Pointer3d<picking3d::events::Click>>, mut commands: Commands) {
    if let Ok(mut entity) = commands.get_entity(trigger.target()) {
        entity.trigger(ButtonClickedEvent);
    }
}

fn spawn_children<'s>(
    ui: &mut RelatedSpawnerCommands<'s, ChildOf>,
    button: &mut UiButton,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    ui.spawn((
        UiLayout::window().full().pack(),
        UiHover::new().forward_speed(20.0).backward_speed(4.0),
        UiMeshPlane3d,
        Picking3dInteractable::default(),
    ))
    .with_children(|ui| {
        ui.spawn((
            UiLayout::window()
                .pos(Rl(50.0))
                .size(button.size)
                .anchor(Anchor::Center)
                .pack(),
            UiHover::new().forward_speed(20.0).backward_speed(4.0),
            UiMeshPlane3d,
        ))
        .with_children(|ui| {
            let entity = ui
                .spawn((
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
                    Text3d::new(button.text.clone()),
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
                .id();
            button.text_entity = Some(entity);
        });
    });
}

fn on_add(
    mut commands: Commands,
    mut added: Query<(Entity, &mut UiButton), Added<UiButton>>,
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
                    OnHoverSetCursor::new(bevy::window::SystemCursorIcon::Pointer),
                    Picking3dInteractable::default(),
                ))
                .with_children(|ui| {
                    spawn_children(ui, slider.as_mut(), &mut materials);
                })
                .observe(hover_set::<Pointer<Over>, true>)
                .observe(hover_set::<Pointer<Out>, false>)
                .observe(hover_set::<Pointer3d<picking3d::events::MoveIn>, true>)
                .observe(hover_set::<Pointer3d<picking3d::events::MoveOut>, false>)
                .observe(button_clicked)
                .observe(button_clicked3d)
                .observe(handle_change_text);
        }
    }
}

#[derive(Component)]
#[require(Visibility)]
pub struct UiButton {
    text: String,
    text_entity: Option<Entity>,
    min_length: usize,
    size: UiValue<Vec2>,
}

impl UiButton {
    pub fn new(text: String, min_length: usize, size: impl Into<UiValue<Vec2>>) -> Self {
        let mut this = Self {
            text,
            text_entity: None,
            min_length,
            size: size.into(),
        };

        this.assert_text_length();

        this
    }

    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.assert_text_length();
    }

    fn assert_text_length(&mut self) {
        while self.text.len() < self.min_length {
            self.text.push('_');
        }
    }
}

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, on_add);
        app.add_event::<ChangeTextEvent>();
    }
}
