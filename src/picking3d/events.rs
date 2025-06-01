use bevy::ecs::query::QueryData;
use bevy::ecs::traversal::Traversal;
use bevy::math::Vec3;
use bevy::prelude::{ChildOf, Component, Entity, Event, Reflect};
use std::fmt::Debug;

#[derive(Clone, Copy, Reflect, Debug)]
pub struct Click;

#[derive(Clone, Copy, Reflect, Debug)]
pub struct MoveIn;

#[derive(Clone, Copy, Reflect, Debug)]
pub struct MoveOut;

#[derive(Clone, Copy, Reflect, Debug)]
pub struct DragStart;

#[derive(Clone, Copy, Reflect, Debug)]
pub struct Drag {
    pub start_entity_position: Vec3,
    pub current_entity_position: Vec3,
    pub delta: Vec3,
}

#[derive(Clone, Copy, Reflect, Debug)]
pub struct DragEnd;

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq, Debug)]
pub enum HoveredBy {
    Left,
    Right,
}

/// The Pointer3d Structure represents any picking event
#[derive(Component, Clone, Copy)]
pub struct Pointer3d<E>
where
    E: Clone + Copy + Reflect,
{
    /// The 3d Position of the controller that triggered the event
    pub position: Vec3,
    /// The entity, that triggered the event, i.e. the controller
    pub hit_entity: Entity,
    /// The event type itself. This may contain additional information
    pub event: E,
    pub controler: HoveredBy,
}

/// A traversal query (i.e. it implements [`Traversal`]) intended for use with [`Pointer`] events.
///
/// This will always traverse to the parent, if the entity being visited has one. Otherwise, it
/// propagates to the pointer's window and stops there.
#[derive(QueryData)]
pub struct Pointer3dTraversal {
    child_of: Option<&'static ChildOf>,
}

impl<E> Traversal<Pointer3d<E>> for Pointer3dTraversal
where
    E: Debug + Clone + Copy + Reflect,
{
    fn traverse(item: Self::Item<'_>, _: &Pointer3d<E>) -> Option<Entity> {
        let Pointer3dTraversalItem { child_of } = item;

        // Send event to parent, if it has one.
        if let Some(child_of) = child_of {
            return Some(child_of.parent());
        };

        None
    }
}

impl<E> Event for Pointer3d<E>
where
    E: Debug + Clone + Copy + Reflect,
{
    type Traversal = Pointer3dTraversal;
    const AUTO_PROPAGATE: bool = true;
}
