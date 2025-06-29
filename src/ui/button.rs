use std::sync::Arc;

use bevy::{
    color::palettes::tailwind::RED_600, ecs::relationship::RelatedSpawnerCommands, prelude::*,
};
use bevy_lunex::{UiStateTrait, prelude::*};

use crate::picking3d::picking_3d::Picking3dInteractable;

fn spawn_children<'s>(
    ui: &mut RelatedSpawnerCommands<'s, ChildOf>,
    button: &mut UiButton,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    ui.spawn((
        UiLayout::window().full().pack(),
        UiHover::new().forward_speed(20.0).backward_speed(4.0),
        UiMeshPlane3d,
    ))
    .with_children(|ui| {
        ui.spawn((
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
        ));
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
                    Picking3dInteractable,
                ))
                .with_children(|ui| {
                    spawn_children(ui, slider.as_mut(), &mut materials);
                })
                .observe(hover_set::<Pointer<Over>, true>)
                .observe(hover_set::<Pointer<Out>, false>);
        }
    }
}

#[derive(Component)]
#[require(Visibility)]
pub struct UiButton {
    text: String,
}

impl UiButton {
    pub fn new(text: String) -> Self {
        Self { text }
    }
}

pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, on_add);
    }
}
