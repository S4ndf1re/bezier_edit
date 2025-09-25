use bevy::{
    ecs::system::{SystemParam, lifetimeless::Read},
    prelude::*,
};

use super::translation_controller::CantSnapToEntities;

/// Snappable, so that a entity may be snapped upon
#[derive(Component)]
pub struct Snappable;

#[derive(SystemParam)]
/// Detect the direction(with magnitude) to the closest projected point for all cameras
pub struct ProximitySnappingDetector<'w, 's> {
    transforms: Query<'w, 's, Read<Transform>>,
    snappable: Query<'w, 's, Entity, With<Snappable>>,
}

impl<'w, 's> ProximitySnappingDetector<'w, 's> {
    /// Detect the closest point on any projection. Return the Directional vector that the
    /// snappable_entity must move in order to snap exactly over the projected entity
    pub fn detect_closest_snappable_entity(
        &self,
        snappable_entity: Entity,
        next_move_delta: Vec3,
        cant_snap_to_entities: &Option<CantSnapToEntities>,
    ) -> Option<Vec3> {
        let mut min_dist = f32::MAX;
        let mut min_delta = None;

        for snappable in self.snappable {
            if let Some(cant_snap) = cant_snap_to_entities.clone() {
                match cant_snap {
                    CantSnapToEntities::None => (),
                    CantSnapToEntities::All => continue,
                    CantSnapToEntities::Single(entity) => {
                        if snappable == entity {
                            continue;
                        }
                    }
                    CantSnapToEntities::Multiple(entities) => {
                        if entities.contains(&snappable) {
                            continue;
                        }
                    }
                }
            }
            if snappable == snappable_entity {
                continue;
            }

            if let Ok(mut a_translation) =
                self.transforms.get(snappable_entity).map(|t| t.translation)
                && let Ok(b_translation) = self.transforms.get(snappable).map(|t| t.translation)
            {
                a_translation += next_move_delta;
                let delta = b_translation - a_translation;
                let dist = (delta).length();

                if dist < min_dist {
                    min_dist = dist;
                    min_delta = Some(delta);
                }
            }
        }

        min_delta
    }
}
