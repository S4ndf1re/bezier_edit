use crate::picking3d::events::HoveredBy;
use bevy::math::Vec3;
use bevy::prelude::{Entity, Resource};
use std::collections::hash_set::Iter;
use std::collections::{hash_map, HashMap, HashSet};

#[derive(Clone, Copy)]
// Define a specific state of the forward and a to target vector.
// Two angles define a possible change in movement, so that an drag event must start.
// Alpha: The angle between the two to_target (old and new) vectors.
// Beta: The angle between the to_target vector and the forward vector of each state (old, new)
pub struct VectorState {
    to_target: Vec3,
    #[allow(unused)]
    forward: Vec3,
    origin: Vec3,
    beta: f32,
}

impl VectorState {
    pub fn new(to_target: Vec3, forward: Vec3, origin: Vec3) -> Self {
        let to_target = to_target.normalize_or_zero();
        let forward = forward.normalize_or_zero();
        Self {
            to_target,
            forward,
            origin,
            beta: forward.dot(to_target),
        }
    }

    // Return the angle between the two to_target vectors
    // Since 1.0 would be no movement, subtract the alpha angle from 1.0
    pub fn get_delta_alpha(&self, to_target: Vec3) -> f32 {
        1.0 - self.to_target.dot(to_target.normalize_or_zero()).abs()
    }

    // Return the difference between the old beta angle and the new beta angle
    pub fn get_delta_beta(&self, to_target: Vec3, forward: Vec3) -> f32 {
        let beta = forward
            .normalize_or_zero()
            .dot(to_target.normalize_or_zero());
        (beta - self.beta).abs()
    }

    pub fn get_delta_origin(&self, origin: Vec3) -> f32 {
        (origin - self.origin).length()
    }
}

#[derive(Resource)]
pub struct PickingState {
    hovered_entities: HashMap<Entity, HashSet<HoveredBy>>,
    hovered_by_left: HashSet<Entity>,
    hovered_by_right: HashSet<Entity>,
    start_position: HashMap<Entity, Vec3>,
    vector_store_left: HashMap<Entity, VectorState>,
    vector_store_right: HashMap<Entity, VectorState>,
    pub is_dragging_left: bool,
    pub is_dragging_right: bool,
    pub is_pressed_left: bool,
    pub is_pressed_right: bool,
}

impl PickingState {
    pub fn new() -> Self {
        Self {
            hovered_entities: HashMap::new(),
            hovered_by_left: HashSet::new(),
            hovered_by_right: HashSet::new(),
            start_position: HashMap::new(),
            vector_store_left: HashMap::new(),
            vector_store_right: HashMap::new(),
            is_dragging_left: false,
            is_dragging_right: false,
            is_pressed_left: false,
            is_pressed_right: false,
        }
    }

    pub fn check_is_dragging(&self, controller: &HoveredBy) -> bool {
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

    pub fn ensure_inserted(
        &mut self,
        entity: Entity,
        controller: HoveredBy,
        start_pos: Vec3,
    ) -> bool {
        if self.check_is_dragging(&controller) {
            return false;
        }
        if self.check_pressed(&controller) {
            return false;
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
        true
    }

    pub fn remove_from_entity(&mut self, entity: &Entity, controller: &HoveredBy) -> bool {
        if self.check_is_dragging(controller) {
            return false;
        }

        if self.check_pressed(controller) {
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

    pub fn iter(&self, controller: &HoveredBy) -> Iter<'_, Entity> {
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

    pub fn set_pressed(&mut self, controller: &HoveredBy, pressed: bool) {
        match controller {
            HoveredBy::Left => self.is_pressed_left = pressed,
            HoveredBy::Right => self.is_pressed_right = pressed,
        }
    }

    pub fn check_pressed(&mut self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.is_pressed_left,
            HoveredBy::Right => self.is_pressed_right,
        }
    }

    pub fn get_start_transform(&self, entity: &Entity) -> Option<Vec3> {
        self.start_position.get(entity).copied()
    }

    pub fn insert_vector_store_for_entity(
        &mut self,
        controller: &HoveredBy,
        entity: Entity,
        store: VectorState,
    ) {
        match controller {
            HoveredBy::Left => self.vector_store_left.insert(entity, store),
            HoveredBy::Right => self.vector_store_right.insert(entity, store),
        };
    }

    pub fn get_vector_store_for_entity(
        &self,
        controller: &HoveredBy,
        entity: &Entity,
    ) -> Option<VectorState> {
        match controller {
            HoveredBy::Left => self.vector_store_left.get(entity).copied(),
            HoveredBy::Right => self.vector_store_right.get(entity).copied(),
        }
    }
}
