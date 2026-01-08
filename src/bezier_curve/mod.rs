pub mod align_mode;
pub mod bezier_curve_renderer;
pub mod bridges;
pub mod components;
pub mod curvature_display_mode;
pub mod degree_manipulation;
pub mod helper_curves;
pub mod inspector;
pub mod ortho_camera;
pub mod render_info;
pub mod test_mode;
pub mod util;

use bevy::prelude::*;

use crate::projection::BoundingEntitiesManager;

#[derive(Event)]
pub struct EntityDeletedEvent(pub Entity);

pub fn handle_generic_deleted_event(
    mut reader: EventReader<EntityDeletedEvent>,
    mut bounding_entities: BoundingEntitiesManager,
) {
    for evt in reader.read() {
        bounding_entities.remove_entity(&evt.0);
    }
}
