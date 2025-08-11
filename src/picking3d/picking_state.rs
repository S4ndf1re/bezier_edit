use crate::picking3d::events::HoveredBy;
use bevy::math::Vec3;
use bevy::prelude::{Entity, Resource};
use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet, hash_map};

#[derive(Resource)]
pub struct PickingState {
    hovered_entities: HashMap<Entity, HashSet<HoveredBy>>,
    hovered_by_left: HashSet<Entity>,
    hovered_by_right: HashSet<Entity>,
    start_position: HashMap<Entity, Vec3>,
    pub is_dragging_left: bool,
    pub is_dragging_right: bool,
}

impl PickingState {
    pub fn new() -> Self {
        Self {
            hovered_entities: HashMap::new(),
            hovered_by_left: HashSet::new(),
            hovered_by_right: HashSet::new(),
            start_position: HashMap::new(),
            is_dragging_left: false,
            is_dragging_right: false,
        }
    }

    fn check_is_dragging(&self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.is_dragging_left,
            HoveredBy::Right => self.is_dragging_right,
        }
    }
    pub fn contains_entity_and_controller(&self, entity: &Entity, controller: &HoveredBy) -> bool {
        self.hovered_entities.contains_key(entity)
            && self
                .hovered_entities
                .get(entity)
                .unwrap()
                .contains(controller)
    }

    pub fn ensure_inserted(&mut self, entity: Entity, controller: HoveredBy, start_pos: Vec3) {
        if self.check_is_dragging(&controller) {
            return;
        }

        self.hovered_entities
            .entry(entity)
            .or_default()
            .insert(controller);

        match controller {
            HoveredBy::Left => {
                self.hovered_by_left.insert(entity);
            }
            HoveredBy::Right => {
                self.hovered_by_right.insert(entity);
            }
        }

        self.start_position.insert(entity, start_pos);
    }

    pub fn remove_from_entity(&mut self, entity: &Entity, controller: &HoveredBy) -> bool {
        if self.check_is_dragging(controller) {
            return false;
        }

        let removed = if self.hovered_entities.contains_key(entity) {
            let removed = self
                .hovered_entities
                .get_mut(entity)
                .unwrap()
                .remove(controller);

            if self.hovered_entities.get(entity).unwrap().is_empty() {
                self.hovered_entities.remove(entity).is_some() && removed
            } else {
                false
            }
        } else {
            false
        };

        self.hovered_entities
            .get_mut(entity)
            .map(|set| set.remove(controller));

        let removed_inverse = match controller {
            HoveredBy::Left => self.hovered_by_left.remove(entity),
            HoveredBy::Right => self.hovered_by_right.remove(entity),
        };

        // assert_eq!(
        //     removed, removed_inverse,
        //     "This case should never ever happen. If this case happens, a programming error can be assumed"
        // );
        removed_inverse && removed
    }

    pub fn iter(&self, controller: &HoveredBy) -> Iter<Entity> {
        match controller {
            HoveredBy::Left => self.hovered_by_left.iter(),
            HoveredBy::Right => self.hovered_by_right.iter(),
        }
    }

    pub fn iter_all(&self) -> hash_map::Keys<'_, Entity, HashSet<HoveredBy>> {
        self.hovered_entities.keys()
    }

    pub fn contains_entity(&self, entity: &Entity, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.hovered_by_left.contains(entity),
            HoveredBy::Right => self.hovered_by_right.contains(entity),
        }
    }

    pub fn set_dragging(&mut self, is_dragging: bool, controller: &HoveredBy) {
        match controller {
            HoveredBy::Left => self.is_dragging_left = is_dragging,
            HoveredBy::Right => self.is_dragging_right = is_dragging,
        }
    }

    pub fn get_start_transform(&self, entity: &Entity) -> Option<Vec3> {
        self.start_position.get(entity).map(|v| *v)
    }
}
