use std::collections::{HashMap, VecDeque};

use bevy::{ecs::world::OnDespawn, prelude::*};

use crate::bezier_curve::bezier_curve_renderer::RedrawEvent;

#[derive(Event)]
pub enum HistoryLogEvent {
    Begin(Entity, Option<Transform>),
    End(Entity, Option<Transform>),
}

#[derive(Event)]
pub struct HistoryPopEvent(Entity);

#[derive(Event)]
pub struct HistoryUndoEvent;

#[derive(Debug)]
struct HistoryEntry {
    start: Transform,
    end: Option<Transform>,
}

#[derive(Resource)]
struct HistoryResource {
    history: HashMap<Entity, VecDeque<HistoryEntry>>,
    global_history: VecDeque<Entity>,
    max_list_size: usize,
}

impl HistoryResource {
    pub fn new(max_list_size: usize) -> Self {
        Self {
            history: HashMap::new(),
            global_history: VecDeque::new(),
            max_list_size,
        }
    }

    pub fn entity_first_seen(&self, entity: &Entity) -> bool {
        !self.history.contains_key(entity)
    }

    pub fn remove_entity(&mut self, entity: &Entity) {
        self.history.remove(entity);
    }

    pub fn add_starting(&mut self, entity: Entity, transform: Transform) {
        let entry = self.history.entry(entity).or_default();
        if entry.is_empty() || entry.back().unwrap().end.is_some() {
            entry.push_back(HistoryEntry {
                start: transform,
                end: None,
            });
        } else {
            entry.back_mut().unwrap().end = Some(transform);
            entry.push_back(HistoryEntry {
                start: transform,
                end: None,
            });
        }

        while entry.len() > self.max_list_size {
            entry.pop_front();
        }
    }

    pub fn add_ending(&mut self, entity: Entity, transform: Transform) {
        let entry = self.history.entry(entity).or_default();
        if !entry.is_empty() {
            if entry.back().unwrap().end.is_none() {
                entry.back_mut().unwrap().end = Some(transform);
            } else {
                entry.push_back(HistoryEntry {
                    start: entry.back().unwrap().end.unwrap(),
                    end: Some(transform),
                });
            }

            self.global_history.push_back(entity);
            while self.global_history.len() > self.max_list_size {
                self.global_history.pop_front();
            }
        }
    }

    pub fn pop(&mut self, entity: &Entity) -> Option<HistoryEntry> {
        self.history.get_mut(entity)?.pop_back()
    }

    pub fn pop_last_entity(&mut self) -> Option<Entity> {
        self.global_history.pop_back()
    }
}

fn despawn_entity(trigger: Trigger<OnDespawn>, mut history: ResMut<HistoryResource>) {
    history.remove_entity(&trigger.target());
}

fn remove_entity(trigger: Trigger<OnRemove, Transform>, mut history: ResMut<HistoryResource>) {
    history.remove_entity(&trigger.target());
}

fn listen_to_history_log_events(
    mut commands: Commands,
    mut reader: EventReader<HistoryLogEvent>,
    mut history: ResMut<HistoryResource>,
    query: Query<&Transform>,
) {
    for evt in reader.read() {
        match evt {
            HistoryLogEvent::Begin(entity, trans) => {
                let transform = query.get(*entity);
                if transform.is_err() {
                    return;
                }

                if history.entity_first_seen(entity) {
                    commands
                        .get_entity(*entity)
                        .unwrap()
                        .observe(despawn_entity)
                        .observe(remove_entity);
                }

                if trans.is_some() {
                    history.add_starting(*entity, trans.unwrap());
                } else {
                    history.add_starting(*entity, *transform.unwrap());
                }
            }
            HistoryLogEvent::End(entity, trans) => {
                let transform = query.get(*entity);
                if transform.is_err() {
                    return;
                }
                if trans.is_some() {
                    history.add_ending(*entity, trans.unwrap());
                } else {
                    history.add_ending(*entity, *transform.unwrap());
                }
            }
        }
    }
}

fn listen_to_history_pop_events(
    mut reader: EventReader<HistoryPopEvent>,
    mut history: ResMut<HistoryResource>,
    mut query: Query<&mut Transform>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    let mut redraw = false;
    for evt in reader.read() {
        if let Some(last_location) = history.pop(&evt.0)
            && let Ok(mut transform) = query.get_mut(evt.0)
        {
            *transform = last_location.start;
            redraw = true;
        }
    }
    if redraw {
        redraw_writer.write(RedrawEvent::HighQuality);
    }
}

fn listen_to_history_undo_events(
    mut reader: EventReader<HistoryUndoEvent>,
    mut history: ResMut<HistoryResource>,
    mut query: Query<&mut Transform>,
    mut redraw_writer: EventWriter<RedrawEvent>,
) {
    let mut redraw = false;
    for _evt in reader.read() {
        loop {
            let entity = history.pop_last_entity();
            if let Some(entity) = entity {
                if let Ok(mut transform) = query.get_mut(entity) {
                    if let Some(last_location) = history.pop(&entity) {
                        *transform = last_location.start;
                        redraw = true;
                    }
                    break;
                }
            } else {
                break;
            }
        }
    }

    if redraw {
        redraw_writer.write(RedrawEvent::HighQuality);
    }
}

pub struct HistoryPlugin;

impl Plugin for HistoryPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.insert_resource(HistoryResource::new(40));
        app.add_event::<HistoryLogEvent>();
        app.add_event::<HistoryPopEvent>();
        app.add_event::<HistoryUndoEvent>();
        app.add_systems(PreUpdate, listen_to_history_undo_events);
        app.add_systems(
            PostUpdate,
            listen_to_history_log_events.before(listen_to_history_pop_events),
        );
    }
}
