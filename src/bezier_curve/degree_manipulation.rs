use bevy::prelude::*;

#[derive(Event)]
pub struct ReduceDegreeEvent;

#[derive(Event)]
pub struct IncreaseDegreeEvent;

pub fn handle_degree_increase_event() {}

pub fn handle_degree_reduction_event() {}
