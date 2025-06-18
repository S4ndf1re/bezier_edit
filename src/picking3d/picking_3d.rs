use crate::bezier_curve::bezier_curve_renderer::RenderInformation;
use crate::picking3d::events::{
    Click, Drag, DragEnd, DragStart, HoveredBy, MoveIn, MoveOut, Pointer3d,
};
use crate::picking3d::picking_state::PickingState;
use crate::picking3d::pointer_state::Pointer3dState;
use bevy::math::bounding::{BoundingSphere, IntersectsVolume};
use bevy::math::Vec3;
use bevy::prelude::{
    App, ChildOf, Commands, Component, Entity, EventWriter, GlobalTransform, IntoScheduleConfigs,
    Plugin, PostUpdate, Query, Res, ResMut, Single, Startup, Transform, Update, Visibility, With,
    Without,
};
use bevy_mod_openxr::action_binding::{OxrSendActionBindings, OxrSuggestActionBinding};
use bevy_mod_xr::actions::ActionType;
use bevy_mod_xr::session::{XrSessionCreated, XrTracker};
use bevy_xr_utils::tracking_utils::{
    ControllerActions, TrackingUtilitiesPlugin, XrTrackedLeftGrip, XrTrackedRightGrip,
    XrTrackedView,
};
use bevy_xr_utils::xr_utils_actions::{
    ActiveSet, XRUtilsAction, XRUtilsActionSet, XRUtilsActionState, XRUtilsActionSystemSet,
    XRUtilsActionsPlugin, XRUtilsBinding,
};

/// Component to shift picking center according the the later found global transpose
#[derive(Component, Clone, Copy)]
pub struct Picking3dTranslation(pub Vec3);

#[derive(Component, Clone, Copy)]
pub struct Picking3dInteractable;

#[derive(Component)]
struct MoveMarker {
    entity: Entity,
    global_start: Vec3,
    current_position: Vec3,
}

#[derive(Component)]
struct GrabActionMarker(HoveredBy);

#[allow(clippy::complexity)]
fn check_intersections(
    mut commands: Commands,
    // mut event_writer: EventWriter<Intersection>,
    controls_points: Query<
        (&GlobalTransform, Entity, Option<&Picking3dTranslation>),
        (
            With<Picking3dInteractable>,
            Without<XrTrackedRightGrip>,
            Without<XrTrackedLeftGrip>,
        ),
    >,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    res_scale: Res<RenderInformation>,
    mut state: ResMut<PickingState>,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.1 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.1 * scale);

    for p in controls_points {
        let test = if p.2.is_some() {
            BoundingSphere::new(p.0.transform_point(p.2.unwrap().0), 0.1 * scale)
        } else {
            BoundingSphere::new(p.0.translation(), 0.1 * scale)
        };

        if bb_sphere_left.intersects(&test) {
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
            state.ensure_inserted(p.1, HoveredBy::Left);
        }

        if bb_sphere_right.intersects(&test) {
            // event_writer.write(Intersection::Right(p.1));
            if !state.contains_entity_and_controller(&p.1, &HoveredBy::Right) {
                commands.trigger_targets(
                    Pointer3d {
                        position: right_tracked.0.translation(),
                        hit_entity: p.1,
                        event: MoveIn,
                        controler: HoveredBy::Right,
                    },
                    p.1,
                );
            }
            state.ensure_inserted(p.1, HoveredBy::Right);
        }
    }
}

/// Test if all previously hovered elements are no longer hovered.
/// This case can only occur, if picking is disabled for a controller (i.e. the controller is not picking an object).
#[allow(clippy::complexity)]
fn test_all_hovered(
    mut commands: Commands,
    controls_points: Query<
        (&GlobalTransform, Entity, Option<&Picking3dTranslation>),
        (
            With<Picking3dInteractable>,
            Without<XrTrackedLeftGrip>,
            Without<XrTrackedRightGrip>,
        ),
    >,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    res_scale: Res<RenderInformation>,
    mut res_picked: ResMut<PickingState>,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.1 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.1 * scale);

    let mut to_remove = Vec::new();

    for entity in &mut res_picked.iter_all() {
        let p = controls_points.get(*entity);
        if p.is_err() {
            to_remove.push((*entity, HoveredBy::Left));
            to_remove.push((*entity, HoveredBy::Right));
            continue;
        }
        let p = p.unwrap();

        let test = if p.2.is_some() {
            BoundingSphere::new(p.0.transform_point(p.2.unwrap().0), 0.1 * scale)
        } else {
            BoundingSphere::new(p.0.translation(), 0.1 * scale)
        };

        if !bb_sphere_left.intersects(&test) {
            to_remove.push((p.1, HoveredBy::Left));
        }

        if !bb_sphere_right.intersects(&test) {
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

pub fn create_hand_trackers(mut commands: Commands) {
    // Add left grip tracking
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        XrTrackedLeftGrip,
        XrTracker,
    ));

    // Add right grip Tracking
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        XrTrackedRightGrip,
        XrTracker,
    ));

    // Add head tracking
    commands.spawn((Transform::from_xyz(0.0, 0.0, 0.0), XrTrackedView, XrTracker));
}

pub fn setup_actions(mut commands: Commands) {
    let set = commands
        .spawn((
            XRUtilsActionSet {
                name: "picking".into(),
                pretty_name: "Picking 3D".into(),
                priority: u32::MIN,
            },
            ActiveSet,
        ))
        .id();

    let grab_action_left = commands
        .spawn((
            XRUtilsAction {
                action_name: "grab_left".into(),
                localized_name: "picking_grab_left".into(),
                action_type: ActionType::Float,
            },
            GrabActionMarker(HoveredBy::Left),
        ))
        .id();

    let grab_action_right = commands
        .spawn((
            XRUtilsAction {
                action_name: "grab_right".into(),
                localized_name: "picking_grab_right".into(),
                action_type: ActionType::Float,
            },
            GrabActionMarker(HoveredBy::Right),
        ))
        .id();

    let controller_left_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/left/input/trigger/value".into(),
        })
        .id();

    let controller_right_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/right/input/trigger/value".into(),
        })
        .id();

    commands
        .entity(grab_action_left)
        .add_child(controller_left_binding);

    commands
        .entity(grab_action_right)
        .add_child(controller_right_binding);

    commands
        .entity(set)
        .add_children(&[grab_action_left, grab_action_right]);
}

fn tick(mut pointer_state: ResMut<Pointer3dState>) {
    pointer_state.add_tick();
}

#[allow(clippy::too_many_arguments)]
fn handle_input_grab(
    mut commands: Commands,
    action_query: Query<(&XRUtilsActionState, &GrabActionMarker)>,
    mut pointer_state: ResMut<Pointer3dState>,
    transform_query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    mut picking_state: ResMut<PickingState>,
    mut moved_marked_query: Query<(&GlobalTransform, &mut MoveMarker, Entity)>,
) {
    for action in action_query.iter() {
        let state = action.0;
        let marker = action.1;
        let hover_by = marker.0;
        let tracked = match hover_by {
            HoveredBy::Left => *left_tracked,
            HoveredBy::Right => *right_tracked,
        };

        if let XRUtilsActionState::Float(gripped) = state {
            let current_state = gripped.current_state > 0.2;
            if gripped.is_active
                && !current_state
                && pointer_state.is_grabbing(&hover_by)
                && pointer_state.is_just_toggled(&hover_by)
            {
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
                }
            } else if gripped.is_active
                && !current_state
                && pointer_state.is_grabbing(&hover_by)
                && !pointer_state.is_just_toggled(&hover_by)
            {
                picking_state.set_dragging(false, &hover_by);

                for (_, _, entity) in moved_marked_query.iter() {
                    commands.get_entity(entity).unwrap().despawn();
                }

                // End Drag here, new state is false, old one was true for >n ticks
            } else if gripped.is_active
                && current_state
                && pointer_state.is_grabbing(&hover_by)
                && !pointer_state.is_just_toggled(&hover_by)
            {
                // Either start dragging here, since we crossed the n tick mark, or continue dragging
                if !picking_state.is_dragging_right && hover_by == HoveredBy::Right
                    || !picking_state.is_dragging_left && hover_by == HoveredBy::Left
                {
                    for entity in picking_state.iter(&hover_by) {
                        let transform = transform_query.get(*entity).unwrap();
                        let dist = transform.translation() - tracked.0.translation();
                        // The controller may be rotated. In order to properly spawn the child, use the inverse rotation.
                        let dist = tracked.0.rotation().inverse().mul_vec3(dist);
                        let dist = dist / tracked.0.scale();

                        commands.spawn((
                            ChildOf(tracked.1),
                            Transform::from_translation(dist),
                            // Mesh3d::from(meshes.add(Sphere::new(0.01))),
                            // MeshMaterial3d::from(materials.add(Color::from(BLUE_300))),
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
                }

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
                picking_state.set_dragging(true, &hover_by);
            }
            pointer_state.set_state(current_state, &hover_by);
        }
    }
}

fn suggest_action_bindings_hp_headset(
    actions: Res<ControllerActions>,
    mut bindings: EventWriter<OxrSuggestActionBinding>,
) {
    bindings.write(OxrSuggestActionBinding {
        action: actions.left.as_raw(),
        interaction_profile: "/interaction_profiles/oculus/touch_controller".into(),
        bindings: vec!["/user/hand/left/input/grip/pose".into()],
    });
    bindings.write(OxrSuggestActionBinding {
        action: actions.right.as_raw(),
        interaction_profile: "/interaction_profiles/oculus/touch_controller".into(),
        bindings: vec!["/user/hand/right/input/grip/pose".into()],
    });
}

pub struct ObjectPicking3d;

impl Plugin for ObjectPicking3d {
    fn build(&self, app: &mut App) {
        app.add_systems(XrSessionCreated, create_hand_trackers)
            //default bindings only use for prototyping
            .add_systems(OxrSendActionBindings, suggest_action_bindings_hp_headset)
            .add_systems(
                Startup,
                setup_actions.before(XRUtilsActionSystemSet::CreateEvents),
            )
            .add_systems(Update, handle_input_grab)
            .add_systems(PostUpdate, check_intersections)
            .add_systems(PostUpdate, test_all_hovered)
            .add_systems(PostUpdate, tick)
            .add_plugins(TrackingUtilitiesPlugin)
            .add_plugins(XRUtilsActionsPlugin)
            .add_event::<Pointer3d<Click>>()
            .add_event::<Pointer3d<MoveIn>>()
            .add_event::<Pointer3d<MoveOut>>()
            .add_event::<Pointer3d<DragStart>>()
            .add_event::<Pointer3d<Drag>>()
            .add_event::<Pointer3d<DragEnd>>()
            .insert_resource(PickingState::new())
            .insert_resource(Pointer3dState::new(15)); // On 90 FPS (VR Standard), 15 Ticks correspond to 0,1666666667 s
    }
}
