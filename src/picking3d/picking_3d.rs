use crate::bezier_curve::render_info::RenderInformation;
use crate::click_decider::{AddLeftTrace, AddRightTrace};
use crate::picking3d::events::{
    Click, Drag, DragEnd, DragStart, HoveredBy, MoveIn, MoveOut, Pointer3d,
};
use crate::picking3d::picking_state::PickingState;
use crate::picking3d::pointer_state::Pointer3dState;
use crate::vr_control::trigger::{ControllerSqueeze, ControllerTrigger};
use crate::vr_control::{AimLeft, AimRight, GripLeft, GripRight};
use bevy::color::palettes::css::POWDER_BLUE;
use bevy::math::bounding::{Aabb3d, BoundingSphere, IntersectsVolume};
use bevy::math::Vec3;
use bevy::prelude::*;

use super::picking_state::VectorState;

#[derive(Component)]
pub struct AimLineMarker(HoveredBy);

#[derive(Component)]
pub struct AimLineRayMarker;

/// Component to shift picking center according the the later found global transpose
#[derive(Component, Clone, Copy)]
pub struct Picking3dTranslation(pub Vec3);

#[derive(Component, Clone, Copy, Default, Eq, Ord, PartialOrd, PartialEq)]
pub enum Picking3dInteractable {
    #[default]
    Default,
    Ignore,
}

#[derive(Component)]
struct MoveMarker {
    entity: Entity,
    global_start: Vec3,
    current_position: Vec3,
}

#[derive(Component)]
pub enum CustomPicking3dHitbox {
    Sphere(f32),
    /// Aabb from half size
    AaBb(Vec3),
}

#[allow(clippy::complexity)]
fn check_intersections(
    mut commands: Commands,
    // mut event_writer: EventWriter<Intersection>,
    pickable: Query<
        (
            &GlobalTransform,
            Entity,
            Option<&CustomPicking3dHitbox>,
            &Picking3dInteractable,
        ),
        (Without<GripLeft>, Without<GripRight>),
    >,
    left_tracked: Single<(&GlobalTransform, Entity), With<GripLeft>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<GripRight>>,
    aim_lines: Query<(&GlobalTransform, &AimLineMarker)>,
    res_scale: Res<RenderInformation>,
    mut state: ResMut<PickingState>,
    mut mesh_ray_casting: MeshRayCast,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.1 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.1 * scale);
    let mut left_aim = None;
    let mut right_aim = None;

    for aim in aim_lines {
        match aim.1.0 {
            HoveredBy::Left => left_aim = Some(aim),
            HoveredBy::Right => right_aim = Some(aim),
        }
    }

    for p in pickable {
        if *p.3 == Picking3dInteractable::Ignore {
            continue;
        }
        let test: Box<dyn IntersectsVolume<BoundingSphere>> = if p.2.is_some() {
            match p.2.unwrap() {
                CustomPicking3dHitbox::Sphere(s) => {
                    Box::new(BoundingSphere::new(p.0.translation(), *s))
                }
                CustomPicking3dHitbox::AaBb(aabb) => {
                    Box::new(Aabb3d::new(p.0.translation(), *aabb))
                }
            }
        } else {
            Box::new(BoundingSphere::new(p.0.translation(), 0.1 * scale))
        };

        if test.intersects(&bb_sphere_left) {
            // event_writer.write(Intersection::Left(p.1));
            if !state.contains_entity_and_controller(&p.1, &HoveredBy::Left) {
                commands.trigger_targets(
                    Pointer3d {
                        position: left_tracked.0.translation(),
                        hit_entity: left_tracked.1,
                        event: MoveIn,
                        controler: HoveredBy::Left,
                    },
                    p.1,
                );
            }
            state.ensure_inserted(p.1, HoveredBy::Left, p.0.translation());
        }

        if test.intersects(&bb_sphere_right) {
            // event_writer.write(Intersection::Right(p.1));
            if !state.contains_entity_and_controller(&p.1, &HoveredBy::Right) {
                commands.trigger_targets(
                    Pointer3d {
                        position: right_tracked.0.translation(),
                        hit_entity: right_tracked.1,
                        event: MoveIn,
                        controler: HoveredBy::Right,
                    },
                    p.1,
                );
            }
            state.ensure_inserted(p.1, HoveredBy::Right, p.0.translation());
        }
    }

    for (aim, tracked, hovered_by) in [
        (left_aim, left_tracked.1, HoveredBy::Left),
        (right_aim, right_tracked.1, HoveredBy::Right),
    ] {
        if let Some(aim) = aim {
            let ray = Ray3d::new(aim.0.translation(), aim.0.forward());
            let collisions = mesh_ray_casting.cast_ray(
                ray,
                &MeshRayCastSettings {
                    filter: &|entity| pickable.get(entity).is_ok(),
                    ..Default::default()
                }
                .with_visibility(RayCastVisibility::Any)
                .always_early_exit(),
            );

            for collision in collisions.iter() {
                let (transform, _, _, _) = pickable
                    .get(collision.0)
                    .expect("This is already checked in the mesh_ray_casting filter option");
                if !state.contains_entity_and_controller(&collision.0, &hovered_by) {
                    commands.trigger_targets(
                        Pointer3d {
                            position: collision.1.point,
                            hit_entity: tracked,
                            event: MoveIn,
                            controler: hovered_by,
                        },
                        collision.0,
                    );
                }
                state.ensure_inserted(collision.0, hovered_by, transform.translation());
            }
        }
    }
}

/// Test if all previously hovered elements are no longer hovered.
/// This case can only occur, if picking is disabled for a controller (i.e. the controller is not picking an object).
#[allow(clippy::complexity)]
fn test_all_hovered(
    mut commands: Commands,
    pickable: Query<
        (
            &GlobalTransform,
            Entity,
            Option<&CustomPicking3dHitbox>,
            &Picking3dInteractable,
        ),
        (Without<GripLeft>, Without<GripRight>),
    >,
    left_tracked: Single<(&GlobalTransform, Entity), With<GripLeft>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<GripRight>>,
    aim_lines: Query<(&GlobalTransform, &AimLineMarker)>,
    res_scale: Res<RenderInformation>,
    mut res_picked: ResMut<PickingState>,
    mut mesh_ray_casting: MeshRayCast,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.1 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.1 * scale);
    let mut left_aim = None;
    let mut right_aim = None;

    for aim in aim_lines {
        match aim.1.0 {
            HoveredBy::Left => left_aim = Some(aim),
            HoveredBy::Right => right_aim = Some(aim),
        }
    }

    let mut to_remove = Vec::new();

    for entity in &mut res_picked.iter_all() {
        let p = pickable.get(*entity);
        if p.is_err() {
            to_remove.push((*entity, HoveredBy::Left));
            to_remove.push((*entity, HoveredBy::Right));
            continue;
        }
        let p = p.unwrap();

        if *p.3 == Picking3dInteractable::Ignore {
            to_remove.push((*entity, HoveredBy::Left));
            to_remove.push((*entity, HoveredBy::Right));
            continue;
        }

        let test: Box<dyn IntersectsVolume<BoundingSphere>> = if p.2.is_some() {
            match p.2.unwrap() {
                CustomPicking3dHitbox::Sphere(s) => {
                    Box::new(BoundingSphere::new(p.0.translation(), *s))
                }
                CustomPicking3dHitbox::AaBb(aabb) => {
                    Box::new(Aabb3d::new(p.0.translation(), *aabb))
                }
            }
        } else {
            Box::new(BoundingSphere::new(p.0.translation(), 0.1 * scale))
        };

        let mut is_first_ray_hit = false;
        if let Some(aim) = left_aim {
            let ray = Ray3d::new(aim.0.translation(), aim.0.forward());

            let collisions = mesh_ray_casting.cast_ray(
                ray,
                &MeshRayCastSettings {
                    filter: &|entity| pickable.get(entity).is_ok(),
                    ..Default::default()
                }
                .with_visibility(RayCastVisibility::Any)
                .always_early_exit(),
            );

            if collisions
                .iter()
                .position(|entity| entity.0 == p.1)
                .is_some()
            {
                is_first_ray_hit = true;
            }
        }
        if !test.intersects(&bb_sphere_left) && !is_first_ray_hit {
            to_remove.push((p.1, HoveredBy::Left));
        }

        let mut is_first_ray_hit = false;
        if let Some(aim) = right_aim {
            let ray = Ray3d::new(aim.0.translation(), aim.0.forward());

            let collisions = mesh_ray_casting.cast_ray(
                ray,
                &MeshRayCastSettings {
                    filter: &|entity| pickable.get(entity).is_ok(),
                    ..Default::default()
                }
                .with_visibility(RayCastVisibility::Any)
                .always_early_exit(),
            );

            if collisions
                .iter()
                .position(|entity| entity.0 == p.1)
                .is_some()
            {
                is_first_ray_hit = true;
            }
        }
        if !test.intersects(&bb_sphere_right) && !is_first_ray_hit {
            to_remove.push((p.1, HoveredBy::Right));
        }
    }

    for removable in to_remove {
        if res_picked.remove_from_entity(&removable.0, &removable.1) {
            commands.trigger_targets(
                Pointer3d {
                    position: left_tracked.0.translation(),
                    hit_entity: match removable.1 {
                        HoveredBy::Left => left_tracked.1,
                        HoveredBy::Right => right_tracked.1,
                    },
                    event: MoveOut,
                    controler: removable.1,
                },
                removable.0,
            );
        }
    }
}

fn tick(mut pointer_state: ResMut<Pointer3dState>) {
    pointer_state.add_tick();
}

#[allow(clippy::too_many_arguments)]
fn handle_input_grab(
    mut commands: Commands,
    trigger: Res<ControllerTrigger>,
    mut pointer_state: ResMut<Pointer3dState>,
    transform_query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity), With<GripLeft>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<GripRight>>,
    mut picking_state: ResMut<PickingState>,
    mut moved_marked_query: Query<(&GlobalTransform, &mut MoveMarker, Entity)>,
    mut left_writer: EventWriter<AddLeftTrace>,
    mut right_writer: EventWriter<AddRightTrace>,
    mut click_writer: EventWriter<Pointer3d<Click>>,
    info: Res<RenderInformation>,
) {
    for (state, hover_by) in [
        (trigger.left, HoveredBy::Left),
        (trigger.right, HoveredBy::Right),
    ] {
        let tracked = match hover_by {
            HoveredBy::Left => *left_tracked,
            HoveredBy::Right => *right_tracked,
        };

        match hover_by {
            HoveredBy::Left => {
                left_writer.write(AddLeftTrace {
                    transform: tracked.0.compute_transform(),
                    click_value: state as f64,
                });
            }
            HoveredBy::Right => {
                right_writer.write(AddRightTrace {
                    transform: tracked.0.compute_transform(),
                    click_value: state as f64,
                });
            }
        }

        let current_state = state > 0.2;

        // The first time the current state is true (after beeing false), set up state to check if
        // dragging has started.
        // NOTE: This assumes, that the targets contained in the entity list are stationary
        if current_state && !pointer_state.is_grabbing(&hover_by) {
            // This is the start of the click. Compute all needed vectors to determine if
            // dragging should start later
            let forward = tracked.0.forward().as_vec3();
            let pos = tracked.0.translation();
            let entities_to_loop_over = picking_state.iter(&hover_by).copied().collect::<Vec<_>>();

            for entity in entities_to_loop_over {
                if let Ok(entity_transform) = transform_query.get(entity) {
                    let pos_to_entity = entity_transform.translation() - pos;
                    picking_state.insert_vector_store_for_entity(
                        &hover_by,
                        entity,
                        VectorState::new(pos_to_entity, forward, pos),
                    );
                }
            }
        }

        // Check if drag ends
        if !current_state
            && pointer_state.is_grabbing(&hover_by)
            && picking_state.check_is_dragging(&hover_by)
        {
            // End Drag here, new state is false, old one was true for >n ticks
            for entity in picking_state.iter(&hover_by) {
                let entity_global_position = transform_query.get(*entity);
                if entity_global_position.is_err() {
                    continue;
                }
                let entity_global_position = entity_global_position.unwrap();
                commands.trigger_targets(
                    Pointer3d {
                        controler: hover_by,
                        hit_entity: tracked.1,
                        event: DragEnd,
                        position: entity_global_position.translation(),
                    },
                    *entity,
                );
            }
            picking_state.set_dragging(false, &hover_by);

            for (_, _, entity) in moved_marked_query.iter() {
                commands.get_entity(entity).unwrap().despawn();
            }
        } else if current_state && pointer_state.is_grabbing(&hover_by) {
            // Check if dragging starts and send continous drag events
            if !picking_state.check_is_dragging(&hover_by) {
                let mut should_start_dragging = false;
                let forward = tracked.0.forward().as_vec3();

                for entity in picking_state.iter(&hover_by) {
                    if let Ok(transform) = transform_query.get(*entity)
                        && let Some(store) =
                            picking_state.get_vector_store_for_entity(&hover_by, entity)
                    {
                        let to_target = transform.translation() - tracked.0.translation();

                        let alpha_diff = store.get_delta_alpha(to_target);
                        let beta_diff = store.get_delta_beta(to_target, forward);
                        let origin_diff = store.get_delta_origin(tracked.0.translation());

                        if alpha_diff > 0.05 || beta_diff > 0.05 || origin_diff > 0.05 * info.scale
                        {
                            should_start_dragging = true;
                        }
                    }
                }

                if should_start_dragging {
                    for entity in picking_state.iter(&hover_by) {
                        let transform = transform_query.get(*entity).unwrap();
                        let translation = match picking_state.get_start_transform(entity) {
                            Some(vec) => vec,
                            None => transform.translation(),
                        };
                        let dist = translation - tracked.0.translation();

                        // The controller may be rotated. In order to properly spawn the child, use the inverse rotation.
                        let dist = tracked.0.rotation().inverse().mul_vec3(dist);
                        let dist = dist / tracked.0.scale();

                        commands.spawn((
                            ChildOf(tracked.1),
                            Transform::from_translation(dist),
                            Visibility::default(),
                            MoveMarker {
                                entity: *entity,
                                global_start: transform.translation(),
                                current_position: tracked.0.transform_point(dist),
                            },
                        ));

                        commands.trigger_targets(
                            Pointer3d {
                                controler: hover_by,
                                hit_entity: tracked.1,
                                event: DragStart,
                                position: transform.translation(),
                            },
                            *entity,
                        );
                    }

                    picking_state.set_dragging(true, &hover_by);
                }
            } else if picking_state.check_is_dragging(&hover_by) {
                for (transform, mut marker, _) in moved_marked_query.iter_mut() {
                    if picking_state.contains_entity(&marker.entity, &hover_by) {
                        let entity_global_position = transform_query.get(marker.entity).unwrap();

                        // Dispatch Drag event on entity. Use old state for positional calculation
                        commands.trigger_targets(
                            Pointer3d {
                                controler: hover_by,
                                hit_entity: tracked.1,
                                event: Drag {
                                    start_entity_position: marker.global_start,
                                    current_entity_position: transform.translation(),
                                    delta: transform.translation() - marker.current_position,
                                },
                                position: entity_global_position.translation(),
                            },
                            marker.entity,
                        );

                        // Update marker to new state
                        marker.current_position = transform.translation();
                    }
                }
            }
        } else if !current_state
            && pointer_state.is_grabbing(&hover_by)
            && !picking_state.check_is_dragging(&hover_by)
        {
            // Handle click if we are still not dragging
            let mut sended_event = false;
            // Click event here, since the new state is false, the old state was true and the state change lasted only <n ticks
            for entity in picking_state.iter(&hover_by) {
                let entity_global_position = transform_query.get(*entity);
                if entity_global_position.is_err() {
                    continue;
                }
                let entity_global_position = entity_global_position.unwrap();
                commands.trigger_targets(
                    Pointer3d {
                        hit_entity: tracked.1,
                        controler: hover_by,
                        position: entity_global_position.translation(),
                        event: Click,
                    },
                    *entity,
                );
                sended_event = true;
            }

            // Only send when not clicking on anything else. This may inhibit some functionality,
            // but is needed to handle click events and still use the ui
            if !sended_event {
                // NOTE: This is another position, since we are not clicking on anything.
                click_writer.write(Pointer3d {
                    hit_entity: tracked.1,
                    controler: hover_by,
                    position: tracked.0.translation(),
                    event: Click,
                });
            }
        }

        pointer_state.set_state(current_state, &hover_by);
        picking_state.set_pressed(&hover_by, current_state);
    }
}

#[allow(clippy::complexity)]
pub fn update_aim_line(
    mut aim_ray: Query<(Entity, &ChildOf), (With<AimLineRayMarker>, Without<AimLineMarker>)>,
    aim_query_left: Query<&GlobalTransform, With<AimLeft>>,
    aim_query_right: Query<&GlobalTransform, With<AimRight>>,
    mut aim_line: Query<
        (&mut Transform, &AimLineMarker),
        (Without<AimLineRayMarker>, Without<Mesh3d>),
    >,
    mut set: ParamSet<(
        ResMut<Assets<Mesh>>,
        MeshRayCast,
        Query<(&mut Transform, &mut Mesh3d)>,
    )>,
    scale: Res<RenderInformation>,
) {
    for (ray_entity, ray) in aim_ray.iter_mut() {
        if let Ok(mut line) = aim_line.get_mut(ray.parent()) {
            if line.1.0 == HoveredBy::Left
                && let Ok(left) = aim_query_left.single()
            {
                *line.0 = left.compute_transform();
                let ray = Ray3d::new(left.translation(), left.forward());
                let length = {
                    let mut mesh_ray_cast = set.p1();
                    let hits = mesh_ray_cast.cast_ray(ray, &Default::default());
                    if !hits.is_empty() {
                        hits[0].1.distance
                    } else {
                        1000.0 // simulate infinity
                    }
                };

                let mesh3d = Mesh3d::from(set.p0().add(Cuboid::new(
                    0.01 * scale.scale,
                    0.01 * scale.scale,
                    length,
                )));
                if let Ok((mut transform, mut mesh)) = set.p2().get_mut(ray_entity) {
                    *mesh = mesh3d;
                    *transform = Transform::from_xyz(
                        -0.005 * scale.scale,
                        -0.005 * scale.scale,
                        -length / 2.0,
                    );
                }
            }

            if line.1.0 == HoveredBy::Right
                && let Ok(right) = aim_query_right.single()
            {
                *line.0 = right.compute_transform();
                let ray = Ray3d::new(right.translation(), right.forward());
                let length = {
                    let mut mesh_ray_cast = set.p1();
                    let hits = mesh_ray_cast.cast_ray(ray, &Default::default());
                    if !hits.is_empty() {
                        hits[0].1.distance
                    } else {
                        1000.0 // simulate infinity
                    }
                };

                let mesh3d = Mesh3d::from(set.p0().add(Cuboid::new(
                    0.01 * scale.scale,
                    0.01 * scale.scale,
                    length,
                )));
                if let Ok((mut transform, mut mesh)) = set.p2().get_mut(ray_entity) {
                    *mesh = mesh3d;
                    *transform = Transform::from_xyz(
                        -0.005 * scale.scale,
                        -0.005 * scale.scale,
                        -length / 2.0,
                    );
                }
            }
        }
    }
}

#[allow(clippy::complexity)]
pub fn show_aim(
    mut commands: Commands,
    squeeze: Res<ControllerSqueeze>,
    aim_query_left: Query<&GlobalTransform, With<AimLeft>>,
    aim_query_right: Query<&GlobalTransform, With<AimRight>>,
    aim_line: Query<(Entity, &AimLineMarker)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut set: ParamSet<(ResMut<Assets<Mesh>>, MeshRayCast)>,
    scale: Res<RenderInformation>,
) {
    // for line in aim_line {
    //     if line.1.0 == HoveredBy::Left && squeeze.left <= 0.2
    //         || line.1.0 == HoveredBy::Right && squeeze.right <= 0.2
    //     {
    //         commands.get_entity(line.0).unwrap().despawn();
    //     }
    // }

    for (squeeze, aim_query, hovered_by) in [
        (squeeze.left, aim_query_left.single(), HoveredBy::Left),
        (squeeze.right, aim_query_right.single(), HoveredBy::Right),
    ] {
        if let Ok(aim) = aim_query
            && aim_line
                .iter()
                .position(|line| line.1.0 == hovered_by)
                .is_none()
        {
            let ray = Ray3d::new(aim.translation(), aim.forward());
            let length = {
                let mut mesh_ray_cast = set.p1();
                let hits = mesh_ray_cast.cast_ray(ray, &Default::default());
                if !hits.is_empty() {
                    hits[0].1.distance
                } else {
                    1000.0 // simulate infinity
                }
            };

            let material = materials.add(Color::from(POWDER_BLUE));
            let mesh = set
                .p0()
                .add(Cuboid::new(0.01 * scale.scale, 0.01 * scale.scale, length));

            commands
                .spawn((
                    AimLineMarker(hovered_by),
                    aim.compute_transform(),
                    Visibility::default(),
                ))
                .with_children(|ui| {
                    ui.spawn((
                        Transform::from_xyz(
                            -0.005 * scale.scale,
                            -0.005 * scale.scale,
                            -length / 2.0,
                        ),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        AimLineRayMarker,
                    ));
                });
        }
    }
}

pub struct ObjectPicking3d;

impl Plugin for ObjectPicking3d {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (handle_input_grab, update_aim_line))
            .add_systems(PostUpdate, check_intersections)
            .add_systems(PostUpdate, test_all_hovered)
            .add_systems(PostUpdate, tick)
            .add_systems(PostUpdate, show_aim)
            .add_event::<Pointer3d<Click>>()
            .add_event::<Pointer3d<MoveIn>>()
            .add_event::<Pointer3d<MoveOut>>()
            .add_event::<Pointer3d<DragStart>>()
            .add_event::<Pointer3d<Drag>>()
            .add_event::<Pointer3d<DragEnd>>()
            .insert_resource(PickingState::new())
            .insert_resource(Pointer3dState::new(30)); // On 90 FPS (VR Standard), 30 Ticks correspond to 2 * 0,1666666667 s = 0,3333333333 s
    }
}
