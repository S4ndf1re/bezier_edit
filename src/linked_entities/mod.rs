use bevy::{
    ecs::{
        relationship::RelatedSpawnerCommands,
        system::{SystemParam, lifetimeless::Read},
    },
    prelude::*,
};
use std::slice::Iter;

use crate::{
    nurbs::{parametric::Parametric, plane::Plane3d},
    translation_control::translation_controller::{
        CantSnapToCurve, EnableTranslationControl, MoveEntityByDeltaEvent, MovedEntityEvent,
    },
};

pub enum ReversePositioningType {
    OneToOne(Vec3),
    /// The entity in this case will point to a Plane. The plane itself is represented by a
    /// transform
    OrthoProjectedOntoPlane(Entity),
}

impl Default for ReversePositioningType {
    fn default() -> Self {
        Self::OneToOne(Vec3::ZERO)
    }
}

#[derive(Component)]
#[relationship(relationship_target = LinkedEntities)]
pub struct LinkTarget {
    #[relationship]
    pub parent: Entity,
    pub position_type: ReversePositioningType,
}

#[derive(Component)]
#[relationship_target(relationship = LinkTarget, linked_spawn)]
pub struct LinkedEntities(Vec<Entity>);

impl<'a> IntoIterator for &'a LinkedEntities {
    type Item = &'a Entity;
    type IntoIter = Iter<'a, Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Update the parents position to
fn update_parent_position(
    mut event_reader: EventReader<MovedEntityEvent>,
    link_targets: Query<&LinkTarget>,
    mut move_writer: EventWriter<MoveEntityByDeltaEvent>,
) {
    for evt in event_reader.read() {
        if let Ok(target) = link_targets.get(evt.entity) {
            info!("Moving by delta: {}", evt.delta);
            move_writer.write(MoveEntityByDeltaEvent {
                entity: target.parent,
                delta: evt.delta,
            });
        }
    }
}

fn update_linked_position_based_on_mapping(
    original_points: Query<(&LinkedEntities, &Transform), Without<LinkTarget>>,
    mut link_targets: Query<(&mut Transform, &LinkTarget), Without<LinkedEntities>>,
    plane_transforms: Query<&Transform, (Without<LinkTarget>, Without<LinkedEntities>)>,
) {
    for (linked_entities, parent_transform) in original_points {
        for linked_entity in linked_entities {
            if let Ok((mut transform, target)) = link_targets.get_mut(*linked_entity)
                && let ReversePositioningType::OrthoProjectedOntoPlane(plane) = target.position_type
                && let Ok(plane_transform) = plane_transforms.get(plane)
            {
                let plane = Plane3d::from(*plane_transform);
                if let Some((u, v)) =
                    plane.point_projected_on_plane_orthogonal(parent_transform.translation.into())
                {
                    let point = plane.f(&[u, v]);
                    transform.translation = point.into();
                }
            }
        }
    }
}

fn sync_enable_translation_controls(
    mut commands: Commands,
    added_translation_controls: Query<
        (Entity, &EnableTranslationControl),
        Added<EnableTranslationControl>,
    >,
    mut removed_translation_controls: RemovedComponents<EnableTranslationControl>,
    linked_entities: Query<&LinkedEntities>,
    link_targets: Query<&LinkTarget>,
) {
    // Add EnableTranslationControl when one of the linked entities does so
    for (added, enable_event) in added_translation_controls {
        if let Ok(links) = linked_entities.get(added) {
            for link in links {
                if let Ok(target) = link_targets.get(*link) {
                    match target.position_type {
                        ReversePositioningType::OrthoProjectedOntoPlane(plane) => {
                            commands
                                .entity(*link)
                                .insert(EnableTranslationControl::OnlyOnPlane(plane))
                                .insert(CantSnapToCurve::All);
                        }
                        ReversePositioningType::OneToOne(_) => {
                            commands
                                .entity(*link)
                                .insert(*enable_event)
                                .insert(CantSnapToCurve::All);
                        }
                    }
                }
            }
        }

        if let Ok(target) = link_targets.get(added) {
            commands
                .entity(target.parent)
                .insert(EnableTranslationControl::OnlyTranslation);
        }
    }

    // Remove EnableTranslationControl when one of the linked entities does so
    for removed in removed_translation_controls.read() {
        if let Ok(links) = linked_entities.get(removed) {
            for link in links {
                let _ = commands.get_entity(*link).map(|mut l| {
                    l.remove::<EnableTranslationControl>()
                        .remove::<CantSnapToCurve>();
                });
            }
        }

        if let Ok(target) = link_targets.get(removed) {
            let _ = commands.get_entity(target.parent).map(|mut l| {
                l.remove::<EnableTranslationControl>();
            });
        }
    }
}

#[allow(clippy::complexity)]
#[derive(SystemParam)]
pub struct SpawnLinkedEntities<'w, 's> {
    set: ParamSet<
        'w,
        's,
        (
            Query<
                'w,
                's,
                (
                    Read<Transform>,
                    Read<Mesh3d>,
                    Read<MeshMaterial3d<StandardMaterial>>,
                ),
            >,
            Query<'w, 's, Read<Transform>>,
        ),
    >,
}

impl<'w, 's> SpawnLinkedEntities<'w, 's> {
    pub fn spawn_linked_to_entity(
        &mut self,
        root_spawner: &mut RelatedSpawnerCommands<'_, ChildOf>,
        entity: Entity,
        positioning_type: ReversePositioningType,
    ) {
        // let plane_transform = match positioning_type {
        //     ReversePositioningType::OrthoProjectedOntoPlane(plane) => {
        //         // This will get updated later on in the positional update system
        //         if let Ok(transform) = self.set.p1().get(plane) {
        //             Some(*transform)
        //         } else {
        //             None
        //         }
        //     }
        //     ReversePositioningType::OneToOne(_) => None,
        // };

        if let Ok((transform, mesh, material)) = self.set.p0().get(entity) {
            let transform = match positioning_type {
                ReversePositioningType::OrthoProjectedOntoPlane(_) => {
                    // This will get updated later on in the positional update system
                    // let plane_transform = plane_transform
                    //     .expect("This must be present, otherwise the plane does not exist");
                    //
                    // let plane = Plane3d::from(plane_transform);
                    // let projected = plane
                    //     .point_projected_on_plane_orthogonal(transform.translation.into())
                    //     .map(|(u, v)| plane.f(&[u, v]));
                    //
                    // if let Some(projected) = projected {
                    //     let mut transform = *transform;
                    //     transform.translation = projected.into();
                    //     Some(transform)
                    // } else {
                    //     None
                    // }

                    Some(*transform)
                }
                ReversePositioningType::OneToOne(offset) => {
                    let mut transform = *transform;
                    transform.translation += offset;
                    Some(transform)
                }
            };

            if let Some(transform) = transform {
                root_spawner.spawn((
                    transform,
                    mesh.clone(),
                    material.clone(),
                    CantSnapToCurve::All,
                    LinkTarget {
                        parent: entity,
                        position_type: positioning_type,
                    },
                ));
            }
        }
    }
}

#[derive(SystemParam)]
pub struct RemoveLinkedEntities<'w, 's> {
    commands: Commands<'w, 's>,
    linked: Query<'w, 's, Read<LinkedEntities>>,
    targets: Query<'w, 's, (Entity, Read<LinkTarget>)>,
}

impl<'w, 's> RemoveLinkedEntities<'w, 's> {
    pub fn remove_link(&mut self, entity: Entity) {
        if let Ok(links) = self.linked.get(entity) {
            for entity in links {
                let _ = self.commands.get_entity(*entity).map(|mut e| {
                    e.remove::<LinkTarget>();
                });
            }
        }

        if let Ok((_, _)) = self.targets.get(entity) {
            let _ = self.commands.get_entity(entity).map(|mut e| {
                e.remove::<LinkTarget>();
            });
        }
    }

    pub fn remove_link_for_all_on_same_plane(&mut self, plane: Entity) {
        for (entity, target) in self.targets {
            if let ReversePositioningType::OrthoProjectedOntoPlane(target_plane) =
                target.position_type
                && target_plane == plane
            {
                let _ = self.commands.get_entity(entity).map(|mut e| e.despawn());
            }
        }
    }
}

pub struct LinkedEntitiesPlugin;

impl Plugin for LinkedEntitiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_parent_position,
                update_linked_position_based_on_mapping.after(update_parent_position),
            ),
        );
        app.add_systems(PostUpdate, (sync_enable_translation_controls,));
    }
}
