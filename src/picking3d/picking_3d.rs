use crate::bezier_curve_renderer::{RenderPoint, ScaleInformation};
use bevy::asset::Assets;
use bevy::color::palettes::tailwind::{BLUE_300, BLUE_600, BLUE_800, RED_800};
use bevy::color::Color;
use bevy::ecs::query::QueryData;
use bevy::ecs::traversal::Traversal;
use bevy::log::info;
use bevy::math::bounding::{BoundingSphere, IntersectsVolume};
use bevy::math::Vec3;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{
    App, ChildOf, Commands, Component, Entity, Event, EventReader, EventWriter, GlobalTransform,
    IntoScheduleConfigs, Mesh, Mesh3d, Or, Plugin, PostUpdate, Query, Reflect, Res, ResMut,
    Resource, Single, Sphere, Startup, Transform, Update, Visibility, Window, With, Without,
};
use bevy::render::camera::NormalizedRenderTarget;
use bevy_mod_openxr::action_binding::{OxrSendActionBindings, OxrSuggestActionBinding};
use bevy_mod_xr::actions::ActionType;
use bevy_mod_xr::session::{XrSessionCreated, XrTracker};
use bevy_xr_utils::tracking_utils::{
    suggest_action_bindings, ControllerActions, TrackingUtilitiesPlugin, XrTrackedLeftGrip,
    XrTrackedRightGrip, XrTrackedView,
};
use bevy_xr_utils::xr_utils_actions::{
    ActiveSet, XRUtilsAction, XRUtilsActionSet, XRUtilsActionState, XRUtilsActionSystemSet,
    XRUtilsActionsPlugin, XRUtilsBinding,
};
use std::collections::hash_set::Iter;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt::Debug;

/// Component to shift picking center according the the later found global transpose
#[derive(Component, Clone, Copy)]
pub struct Picking3dTranslation(pub Vec3);

#[derive(Component, Clone, Copy)]
pub struct Picking3dInteractable;

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
    fn traverse(item: Self::Item<'_>, pointer: &Pointer3d<E>) -> Option<Entity> {
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

#[derive(Component)]
struct MoveMarker {
    entity: Entity,
    global_start: Vec3,
    current_position: Vec3,
}

#[derive(Component)]
struct GrabMarker(HoveredBy);

#[derive(Resource)]
struct PickingState {
    hovered_entities: HashMap<Entity, HashSet<HoveredBy>>,
    hovered_by_left: HashSet<Entity>,
    hovered_by_right: HashSet<Entity>,
    is_dragging_left: bool,
    is_dragging_right: bool,
}

impl PickingState {
    pub fn new() -> Self {
        Self {
            hovered_entities: HashMap::new(),
            hovered_by_left: HashSet::new(),
            hovered_by_right: HashSet::new(),
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

    pub fn ensure_inserted(&mut self, entity: Entity, controller: HoveredBy) {
        if self.check_is_dragging(&controller) {
            return;
        }

        self.hovered_entities
            .entry(entity)
            .or_insert(HashSet::new())
            .insert(controller);

        match controller {
            HoveredBy::Left => {
                self.hovered_by_left.insert(entity);
            }
            HoveredBy::Right => {
                self.hovered_by_right.insert(entity);
            }
        }
    }

    pub fn remove_from_entity(&mut self, entity: &Entity, controller: &HoveredBy) -> bool {
        if self.check_is_dragging(&controller) {
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
}

#[derive(Resource)]
struct Pointer3dState {
    grab_left_prev_state: bool,
    grab_right_prev_state: bool,
    toggled_left_since: u32,
    toggled_right_since: u32,
    max_ticks: u32,
}

impl Pointer3dState {
    pub fn new(max_ticks: u32) -> Self {
        Self {
            grab_left_prev_state: false,
            grab_right_prev_state: false,
            toggled_left_since: 0,
            toggled_right_since: 0,
            max_ticks,
        }
    }

    pub fn set_state(&mut self, state: bool, controller: &HoveredBy) {
        match controller {
            HoveredBy::Left => {
                if self.grab_left_prev_state != state {
                    self.toggled_left_since = 0;
                }
                self.grab_left_prev_state = state
            }
            HoveredBy::Right => {
                if self.grab_right_prev_state != state {
                    self.toggled_right_since = 0;
                }
                self.grab_right_prev_state = state
            }
        }
    }

    pub fn add_tick(&mut self) {
        if self.grab_left_prev_state {
            self.toggled_left_since += 1;
        }

        if self.grab_right_prev_state {
            self.toggled_right_since += 1;
        }
    }

    pub fn is_grabbing(&self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.grab_left_prev_state,
            HoveredBy::Right => self.grab_right_prev_state,
        }
    }

    /// Test if the controller is toggled in shorter than n ticks (<)
    pub fn is_just_toggled(&self, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.toggled_left_since < self.max_ticks,
            HoveredBy::Right => self.toggled_right_since < self.max_ticks,
        }
    }
}

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
    res_scale: Res<ScaleInformation>,
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
    res_scale: Res<ScaleInformation>,
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

fn create_hand_trackers(mut commands: Commands) {
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
            GrabMarker(HoveredBy::Left),
        ))
        .id();

    let grab_action_right = commands
        .spawn((
            XRUtilsAction {
                action_name: "grab_right".into(),
                localized_name: "picking_grab_right".into(),
                action_type: ActionType::Float,
            },
            GrabMarker(HoveredBy::Right),
        ))
        .id();

    let controller_left_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/hp/mixed_reality_controller".into(),
            binding: "/user/hand/left/input/trigger/value".into(),
        })
        .id();

    let controller_right_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/hp/mixed_reality_controller".into(),
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

fn handle_input_grab(
    mut commands: Commands,
    action_query: Query<(&XRUtilsActionState, &GrabMarker)>,
    mut pointer_state: ResMut<Pointer3dState>,
    transform_query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    mut picking_state: ResMut<PickingState>,
    mut moved_marked_query: Query<(&GlobalTransform, &mut MoveMarker, Entity)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for action in action_query.iter() {
        let state = action.0;
        let marker = action.1;
        let hover_by = marker.0;
        let tracked = match hover_by {
            HoveredBy::Left => left_tracked.clone(),
            HoveredBy::Right => right_tracked.clone(),
        };

        match state {
            XRUtilsActionState::Float(gripped) => {
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
                            let entity_global_position =
                                transform_query.get(marker.entity).unwrap();

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
            _ => {}
        }
    }
}

fn suggest_action_bindings_hp_headset(
    actions: Res<ControllerActions>,
    mut bindings: EventWriter<OxrSuggestActionBinding>,
) {
    bindings.write(OxrSuggestActionBinding {
        action: actions.left.as_raw(),
        interaction_profile: "/interaction_profiles/hp/mixed_reality_controller".into(),
        bindings: vec!["/user/hand/left/input/grip/pose".into()],
    });
    bindings.write(OxrSuggestActionBinding {
        action: actions.right.as_raw(),
        interaction_profile: "/interaction_profiles/hp/mixed_reality_controller".into(),
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
