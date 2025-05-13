use crate::bezier_curve_renderer::ScaleInformation;
use crate::Test;
use bevy::asset::Assets;
use bevy::color::palettes::tailwind::{BLUE_800, RED_800};
use bevy::color::Color;
use bevy::log::info;
use bevy::math::bounding::{BoundingSphere, IntersectsVolume};
use bevy::math::Vec3;
use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy::prelude::{
    App, ChildOf, Commands, Component, Entity, Event, EventReader, GlobalTransform, Mesh, Mesh3d,
    Plugin, PostUpdate, Query, Reflect, Res, ResMut, Resource, Single, Sphere, Startup, Transform,
    Update, With,
};
use bevy_mod_openxr::action_binding::OxrSendActionBindings;
use bevy_mod_xr::actions::ActionType;
use bevy_mod_xr::session::{XrSessionCreated, XrTracker};
use bevy_xr_utils::tracking_utils::{
    suggest_action_bindings, TrackingUtilitiesPlugin, XrTrackedLeftGrip, XrTrackedRightGrip,
    XrTrackedView,
};
use bevy_xr_utils::xr_utils_actions::{
    ActiveSet, XRUtilsAction, XRUtilsActionSet, XRUtilsActionState, XRUtilsBinding,
};
use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Reflect)]
pub struct Click;

#[derive(Clone, Copy, Reflect)]
pub struct MoveIn;

#[derive(Clone, Copy, Reflect)]
pub struct MoveOut;

#[derive(Clone, Copy, Reflect)]
pub struct DragStart;

#[derive(Clone, Copy, Reflect)]
pub struct Drag {
    current_entity_position: Vec3,
}

#[derive(Clone, Copy, Reflect)]
pub struct DragEnd;

#[derive(Clone, Copy, Hash, PartialOrd, PartialEq, Eq)]
pub enum HoveredBy {
    Left,
    Right,
}

/// The Pointer3d Structure represents any picking event
#[derive(Component, Event, Clone, Copy)]
pub struct Pointer3d<E>
where
    E: Clone + Copy + Reflect,
{
    /// The 3d Position of the controlelr that triggered the event
    pub position: Vec3,
    /// The entity, that triggered the event, i.e. the controller
    pub hit_entity: Entity,
    /// The event type itself. This may contain additional information
    pub event: E,
    pub controler: HoveredBy,
}

#[derive(Component)]
struct MoveMarker {
    entity: Entity,
    global_start: Vec3,
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
            self.hovered_entities
                .get_mut(entity)
                .unwrap()
                .remove(controller)
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

        assert_eq!(
            removed, removed_inverse,
            "This case should never ever happen. If this case happens, a programming error can be assumed"
        );
        removed_inverse && removed
    }

    pub fn iter(&self, controller: &HoveredBy) -> Iter<Entity> {
        match controller {
            HoveredBy::Left => self.hovered_by_left.iter(),
            HoveredBy::Right => self.hovered_by_right.iter(),
        }
    }

    pub fn contains_entity(&self, entity: &Entity, controller: &HoveredBy) -> bool {
        match controller {
            HoveredBy::Left => self.hovered_by_left.contains(entity),
            HoveredBy::Right => self.hovered_by_left.contains(entity),
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

    pub fn toggle_state(&mut self, controller: &HoveredBy) {
        match controller {
            HoveredBy::Left => {
                self.toggled_left_since = 0;
                self.grab_left_prev_state = !self.grab_left_prev_state
            }
            HoveredBy::Right => {
                self.toggled_right_since = 0;
                self.grab_right_prev_state = !self.grab_right_prev_state
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

#[derive(Event)]
pub enum Intersection {
    Left(Entity),
    Right(Entity),
}

fn check_intersections(
    mut commands: Commands,
    // mut event_writer: EventWriter<Intersection>,
    controls_points: Query<(&GlobalTransform, Entity), With<Test>>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    res_scale: Res<ScaleInformation>,
    mut state: ResMut<PickingState>,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.3 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.3 * scale);

    for p in controls_points {
        let test = BoundingSphere::new(p.0.translation(), 0.1 * scale);

        if bb_sphere_left.intersects(&test) {
            // event_writer.write(Intersection::Left(p.1));
            if !state.contains_entity_and_controller(&p.1, &HoveredBy::Left) {
                commands.get_entity(p.1).unwrap().trigger(Pointer3d {
                    position: left_tracked.0.translation(),
                    hit_entity: left_tracked.1,
                    event: MoveIn,
                    controler: HoveredBy::Left,
                });
            }
            state.ensure_inserted(p.1, HoveredBy::Left);
        }

        if bb_sphere_right.intersects(&test) {
            // event_writer.write(Intersection::Right(p.1));
            if !state.contains_entity_and_controller(&p.1, &HoveredBy::Right) {
                commands.get_entity(p.1).unwrap().trigger(Pointer3d {
                    position: right_tracked.0.translation(),
                    hit_entity: p.1,
                    event: MoveIn,
                    controler: HoveredBy::Right,
                });
            }
            state.ensure_inserted(p.1, HoveredBy::Right);
        }
    }
}

/// Test if all previously hovered elements are no longer hovered.
/// This case can only occur, if picking is disabled for a controller (i.e. the controller is not picking an object).
fn test_all_hovered(
    mut commands: Commands,
    controls_points: Query<(&GlobalTransform, Entity)>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    res_scale: Res<ScaleInformation>,
    mut res_picked: ResMut<PickingState>,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.3 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.3 * scale);

    let mut to_remove = Vec::new();

    for entity in &mut res_picked.as_mut().hovered_entities.keys() {
        let p = controls_points.get(*entity).unwrap();
        let test = BoundingSphere::new(p.0.translation(), 0.1 * scale);

        if !bb_sphere_left.intersects(&test) {
            to_remove.push((p.1, HoveredBy::Left));
        }

        if !bb_sphere_right.intersects(&test) {
            to_remove.push((p.1, HoveredBy::Right));
        }
    }

    for removable in to_remove {
        if res_picked.remove_from_entity(&removable.0, &removable.1) {
            commands
                .get_entity(removable.0)
                .unwrap()
                .trigger(Pointer3d {
                    position: left_tracked.0.translation(),
                    hit_entity: match removable.1 {
                        HoveredBy::Left => left_tracked.1,
                        HoveredBy::Right => right_tracked.1,
                    },
                    event: MoveOut,
                    controler: removable.1,
                });
        }
    }
}

fn log_intersections(
    mut commands: Commands,
    mut event_reader: EventReader<Intersection>,
    query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
) {
    for evt in event_reader.read() {
        match evt {
            Intersection::Left(entity) => {
                info!(
                    "Received Intersection between left controller and {:?}",
                    entity
                );
                // TODO: Only to this, when action is pressed
                let transform = query.get(*entity).unwrap();
                let dist = transform.translation() - left_tracked.0.translation();
                let dist = left_tracked.0.affine().inverse().transform_point3(dist);

                commands.spawn((
                    ChildOf(left_tracked.1),
                    Transform::from_translation(dist),
                    MoveMarker {
                        entity: *entity,
                        global_start: transform.translation(),
                    },
                ));
            }
            Intersection::Right(entity) => {
                info!(
                    "Received Intersection between right controller and {:?}",
                    entity
                );

                // TODO: Only to this, when action is pressed

                let transform = query.get(*entity).unwrap();
                let dist = transform.translation() - right_tracked.0.translation();
                let dist = right_tracked.0.affine().inverse().transform_point3(dist);

                commands.spawn((
                    ChildOf(right_tracked.1),
                    Transform::from_translation(dist),
                    MoveMarker {
                        entity: *entity,
                        global_start: transform.translation(),
                    },
                ));
            }
        }
    }
}

#[cfg(feature = "vr_enable")]
fn create_hand_trackers(
    mut commands: Commands,
    scale_res: Res<ScaleInformation>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let scale = scale_res.scale;
    let height = scale_res.height;
    info!("Creating trackers");
    // Add left grip tracking
    commands.spawn((
        Mesh3d::from(meshes.add(Sphere::new(0.3 * scale))),
        MeshMaterial3d::from(materials.add(Color::from(BLUE_800))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        XrTrackedLeftGrip,
        XrTracker,
    ));

    // Add right grip Tracking
    commands.spawn((
        Mesh3d::from(meshes.add(Sphere::new(0.3 * scale))),
        MeshMaterial3d::from(materials.add(Color::from(BLUE_800))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        XrTrackedRightGrip,
        XrTracker,
    ));

    // Add right grip Tracking
    commands.spawn((
        Mesh3d::from(meshes.add(Sphere::new(0.3 * scale))),
        MeshMaterial3d::from(materials.add(Color::from(RED_800))),
        Transform::from_xyz(0.0, 0.0 + height, 0.0),
        Test,
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
                action_name: "grab".into(),
                localized_name: "picking_grab".into(),
                action_type: ActionType::Bool,
            },
            GrabMarker(HoveredBy::Left),
        ))
        .id();

    let grab_action_right = commands
        .spawn((
            XRUtilsAction {
                action_name: "grab".into(),
                localized_name: "picking_grab".into(),
                action_type: ActionType::Bool,
            },
            GrabMarker(HoveredBy::Right),
        ))
        .id();

    let controller_left_binding = commands
        .spawn(XRUtilsBinding {
            profile: "interaction_profiles/hp/mixed_reality_controller".into(),
            binding: "/user/hand/left/input/grip/pose".into(),
        })
        .id();

    let controller_right_binding = commands
        .spawn(XRUtilsBinding {
            profile: "interaction_profiles/hp/mixed_reality_controller".into(),
            binding: "/user/hand/right/input/grip/pose".into(),
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

fn handle_input_grab(
    mut commands: Commands,
    action_query: Query<(&XRUtilsActionState, &GrabMarker)>,
    pointer_state: ResMut<Pointer3dState>,
    transform_query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    picking_state: Res<PickingState>,
    moved_marked_query: Query<(&GlobalTransform, &MoveMarker)>,
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
            XRUtilsActionState::Bool(gripped) => {
                if gripped.is_active
                    && !gripped.current_state
                    && pointer_state.is_grabbing(&hover_by)
                    && pointer_state.is_just_toggled(&hover_by)
                {
                    // Click event here, since the new state is false, the old state was true and the state change lasted only <n ticks
                    for entity in picking_state.iter(&hover_by) {
                        let entity_global_position = transform_query.get(*entity).unwrap();
                        commands.get_entity(*entity).unwrap().trigger(Pointer3d {
                            hit_entity: tracked.1,
                            controler: hover_by,
                            position: entity_global_position.translation(),
                            event: DragEnd,
                        });
                    }
                } else if gripped.is_active
                    && !gripped.current_state
                    && pointer_state.is_grabbing(&hover_by)
                    && !pointer_state.is_just_toggled(&hover_by)
                {
                    // End Drag here, new state is false, old one was true for >n ticks
                } else if gripped.is_active
                    && gripped.current_state
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
                            let dist = tracked.0.affine().inverse().transform_point3(dist);

                            commands.spawn((
                                ChildOf(tracked.1),
                                Transform::from_translation(dist),
                                MoveMarker {
                                    entity: *entity,
                                    global_start: transform.translation(),
                                },
                            ));

                            commands.get_entity(*entity).unwrap().trigger(Pointer3d {
                                controler: hover_by,
                                hit_entity: tracked.1,
                                event: DragStart,
                                position: transform.translation(),
                            });
                        }
                    } else {
                        for (transform, marker) in moved_marked_query.iter() {
                            if picking_state.contains_entity(&marker.entity, &hover_by) {
                                let entity_global_position =
                                    transform_query.get(marker.entity).unwrap();
                                commands
                                    .get_entity(marker.entity)
                                    .unwrap()
                                    .trigger(Pointer3d {
                                        controler: hover_by,
                                        hit_entity: tracked.1,
                                        event: Drag {
                                            current_entity_position: transform.translation(),
                                        },
                                        position: entity_global_position.translation(),
                                    });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct ObjectPicking3d;

impl Plugin for ObjectPicking3d {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_actions)
            .add_systems(Update, handle_input_grab)
            .add_systems(PostUpdate, check_intersections)
            .add_systems(PostUpdate, test_all_hovered)
            // .add_systems(PostUpdate, log_intersections)
            .add_systems(XrSessionCreated, create_hand_trackers)
            .add_event::<Intersection>()
            .add_plugins(TrackingUtilitiesPlugin)
            //default bindings only use for prototyping
            .add_systems(OxrSendActionBindings, suggest_action_bindings)
            .add_event::<Pointer3d<Click>>()
            .add_event::<Pointer3d<MoveIn>>()
            .add_event::<Pointer3d<MoveOut>>()
            .add_event::<Pointer3d<DragStart>>()
            .add_event::<Pointer3d<Drag>>()
            .add_event::<Pointer3d<DragEnd>>()
            .insert_resource(PickingState::new())
            .insert_resource(Pointer3dState::new(10)); // On 60 FPS, 10 Ticks correspond to 0,1666666667 s
    }
}
