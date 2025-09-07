use std::collections::HashMap;

use bevy::prelude::*;

pub struct AccumulatedPosition {
    start: Vec3,
    current_diff_accumulated: Vec3,
}

impl AccumulatedPosition {
    pub fn new(start: Vec3) -> Self {
        Self {
            start,
            current_diff_accumulated: Vec3::ZERO,
        }
    }

    pub fn add_diff(&mut self, diff: Vec3) {
        self.current_diff_accumulated += diff;
    }

    pub fn current_diff(&self) -> Vec3 {
        self.current_diff_accumulated
    }

    pub fn current_point(&self) -> Vec3 {
        self.current_diff_accumulated + self.start
    }

    pub fn reset(&mut self, new_start: Vec3) {
        self.current_diff_accumulated = Vec3::ZERO;
        self.start = new_start;
    }
}

#[derive(Resource, Default)]
pub struct AccumulatedMovementStore {
    movements: HashMap<Entity, AccumulatedPosition>,
}

impl AccumulatedMovementStore {
    pub fn start_movement_entity(&mut self, entity: Entity, start: Vec3) {
        let entry = self
            .movements
            .entry(entity)
            .or_insert(AccumulatedPosition::new(start));
        entry.reset(start);
    }

    pub fn add_diff(&mut self, entity: &Entity, diff: Vec3) {
        if let Some(value) = self.movements.get_mut(entity) {
            value.add_diff(diff);
        }
    }

    pub fn current_diff(&self, entity: &Entity) -> Option<Vec3> {
        self.movements.get(entity).map(|v| v.current_diff())
    }

    pub fn current_point(&self, entity: &Entity) -> Option<Vec3> {
        self.movements.get(entity).map(|v| v.current_point())
    }

    pub fn reset(&mut self, entity: Entity, new_start: Vec3) {
        self.start_movement_entity(entity, new_start);
    }

    pub fn end_movement(&mut self, entity: &Entity) {
        self.movements.remove(entity);
    }
}
