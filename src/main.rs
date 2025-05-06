mod advanced_orbit_controls;
mod bezier_curve_renderer;
mod nurbs;
mod thirdparty_copy;
mod translation_controller;
pub mod util;

use crate::advanced_orbit_controls::AdvancedOrbitControls;
use crate::thirdparty_copy::transform_util_copy::{
    handle_transform_events, SnapToPosition, SnapToRotation,
};
use crate::translation_controller::TranslationController;
use bevy::color::palettes::tailwind::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::math::bounding::{BoundingSphere, BoundingVolume, IntersectsVolume};
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy_mod_openxr::action_binding::OxrSendActionBindings;
use bevy_mod_openxr::add_xr_plugins;
use bevy_mod_openxr::init::OxrInitPlugin;
use bevy_mod_openxr::resources::{OxrInstance, OxrSessionConfig};
use bevy_mod_openxr::types::EnvironmentBlendMode;
use bevy_mod_xr::session::{XrRootTransform, XrSessionCreated, XrTracker, XrTrackingRoot};
use bevy_mod_xr::spaces::XrVelocity;
use bevy_xr_utils::tracking_utils::{
    suggest_action_bindings, TrackingUtilitiesPlugin, XrTrackedLeftGrip, XrTrackedRightGrip,
    XrTrackedView,
};
use bevy_xr_utils::transform_utils;
use bevy_xr_utils::xr_utils_actions::{XRUtilsActionSystemSet, XRUtilsActionsPlugin};
use bezier_curve_renderer::*;

#[cfg(not(feature = "vr_enable"))]
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));
}

#[derive(Component)]
struct Test;

#[derive(Event)]
pub enum Intersection {
    Left(Entity),
    Right(Entity),
}
fn check_intersections(
    mut event_writer: EventWriter<Intersection>,
    controls_points: Query<(&GlobalTransform, Entity), With<Test>>,
    left_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity), With<XrTrackedRightGrip>>,
    res_scale: Res<ScaleInformation>,
) {
    let scale = res_scale.scale;
    let bb_sphere_left = BoundingSphere::new(left_tracked.0.translation(), 0.3 * scale);
    let bb_sphere_right = BoundingSphere::new(right_tracked.0.translation(), 0.3 * scale);

    for p in controls_points {
        let test = BoundingSphere::new(p.0.translation(), 0.1 * scale);

        if bb_sphere_left.intersects(&test) {
            event_writer.write(Intersection::Left(p.1));
        }

        if bb_sphere_right.intersects(&test) {
            event_writer.write(Intersection::Right(p.1));
        }
    }
}

#[derive(Component)]
struct MoveMarker {
    entity: Entity,
    starting_global_translation: Vec3,
}

pub fn log_intersections(
    mut commands: Commands,
    mut event_reader: EventReader<Intersection>,
    query: Query<&GlobalTransform>,
    left_tracked: Single<(&GlobalTransform, Entity, &Transform), With<XrTrackedLeftGrip>>,
    right_tracked: Single<(&GlobalTransform, Entity, &Transform), With<XrTrackedRightGrip>>,
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

                commands.spawn((
                    ChildOf(left_tracked.1),
                    Transform::from_translation(left_tracked.2.translation + dist),
                    MoveMarker {
                        entity: *entity,
                        starting_global_translation: transform.translation(),
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

                commands.spawn((
                    ChildOf(right_tracked.1),
                    Transform::from_translation(right_tracked.2.translation + dist),
                    MoveMarker {
                        entity: *entity,
                        starting_global_translation: transform.translation(),
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

#[cfg(feature = "vr_enable")]
fn setup(
    mut commands: Commands,
    mut rotation_writer: EventWriter<SnapToRotation>,
    mut position_writer: EventWriter<SnapToPosition>,
    mut scale: ResMut<ScaleInformation>,
) {
    scale.scale = 0.3;

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, -10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 10.0, 10.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
    ));

    let position =
        Transform::from_xyz(-0.7, -1.7, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
    position_writer.write(SnapToPosition(position.translation));
    rotation_writer.write(SnapToRotation(position.rotation));
}

#[cfg(not(feature = "vr_enable"))]
fn create_app() -> App {
    info!("Creating Non-VR App");
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugins(MeshPickingPlugin)
        .add_plugins(BezierRender)
        .add_plugins(AdvancedOrbitControls)
        .add_plugins(TranslationController)
        .add_systems(Startup, setup);

    app
}

#[cfg(feature = "vr_enable")]
fn create_actions() {}
#[cfg(feature = "vr_enable")]
fn create_app() -> App {
    info!("Creating VR App");
    let mut app = App::new();
    app.add_plugins(add_xr_plugins(
        DefaultPlugins.build().disable::<PipelinedRenderingPlugin>(),
    ))
    .insert_resource(OxrSessionConfig {
        blend_modes: Some(vec![
            EnvironmentBlendMode::ALPHA_BLEND,
            EnvironmentBlendMode::OPAQUE,
        ]),
        ..default()
    })
    .add_plugins(bevy_mod_xr::hand_debug_gizmos::HandGizmosPlugin)
    .add_plugins(thirdparty_copy::transform_util_copy::TransformUtilitiesPlugin)
    .add_plugins(TrackingUtilitiesPlugin)
    .add_plugins(XRUtilsActionsPlugin)
    .add_plugins(MeshPickingPlugin)
    //default bindings only use for prototyping
    .add_systems(OxrSendActionBindings, suggest_action_bindings)
    // .add_plugins(BezierRender)
    //.add_plugins(AdvancedOrbitControls)
    // .add_plugins(TranslationController)
    .add_systems(
        Startup,
        (
            setup.before(generate_default_curve),
            create_actions.before(XRUtilsActionSystemSet::CreateEvents),
        ),
    )
    .add_systems(Update, check_intersections)
    .add_systems(PostUpdate, log_intersections)
    .add_systems(XrSessionCreated, create_hand_trackers)
    .add_event::<Intersection>()
    .init_resource::<ScaleInformation>()
    .insert_resource(ClearColor(Color::NONE));

    app
}

fn main() {
    create_app().run();
}
