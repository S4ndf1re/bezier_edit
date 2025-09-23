pub mod config;
pub mod thumbstick3d;
pub mod trigger;
pub mod vibrate;

use bevy::prelude::*;
use bevy_mod_openxr::{
    action_binding::{OxrSendActionBindings, OxrSuggestActionBinding},
    action_set_attaching::OxrAttachActionSet,
    action_set_syncing::{OxrActionSetSyncSet, OxrSyncActionSet},
    openxr_session_running,
    resources::OxrInstance,
    session::OxrSession,
};
use bevy_mod_xr::session::{XrSessionCreated, XrTracker, session_available};
use bevy_xr_utils::tracking_utils::{TrackingUtilitiesPlugin, XrTrackedView};
use config::Config;
use openxr::{Haptic, Posef};
use thumbstick3d::ThumbstickPlugin;
use trigger::TriggerPlugin;
use vibrate::VibrationPlugin;

struct ControllerActionSet {
    set: openxr::ActionSet,
    aim: openxr::Action<Posef>,
    trigger: openxr::Action<f32>,
    squeeze: openxr::Action<f32>,
    grip: openxr::Action<Posef>,
    thumbstick_x: openxr::Action<f32>,
    thumbstick_y: openxr::Action<f32>,
    output: openxr::Action<Haptic>,
}

#[derive(Resource)]
struct ControllerActions {
    left: ControllerActionSet,
    right: ControllerActionSet,
}

#[derive(Component)]
#[require(Transform)]
pub struct AimLeft;

#[derive(Component)]
#[require(Transform)]
pub struct AimRight;

#[derive(Component)]
#[require(Transform)]
pub struct GripLeft;

#[derive(Component)]
#[require(Transform)]
pub struct GripRight;

pub fn create_hand_trackers(mut commands: Commands) {
    // Add head tracking
    commands.spawn((Transform::from_xyz(0.0, 0.0, 0.0), XrTrackedView, XrTracker));
}

fn attach_set(actions: Res<ControllerActions>, mut attach: EventWriter<OxrAttachActionSet>) {
    attach.write(OxrAttachActionSet(actions.left.set.clone()));
    attach.write(OxrAttachActionSet(actions.right.set.clone()));
}

fn sync_actions(actions: Res<ControllerActions>, mut sync: EventWriter<OxrSyncActionSet>) {
    sync.write(OxrSyncActionSet(actions.left.set.clone()));
    sync.write(OxrSyncActionSet(actions.right.set.clone()));
}

fn suggest_action_bindings(
    actions: Res<ControllerActions>,
    mut bindings: EventWriter<OxrSuggestActionBinding>,
    config: Res<Config>,
) {
    for profile in &config.profiles {
        let interaction_profile = profile.interaction_profile.clone();

        for (action, config) in [&actions.left, &actions.right]
            .iter()
            .zip([&profile.left, &profile.right].iter())
        {
            bindings.write(OxrSuggestActionBinding {
                action: action.aim.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.aim.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.grip.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.grip.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.trigger.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.trigger.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.squeeze.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.squeeze.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.thumbstick_x.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.thumbstick_x.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.thumbstick_y.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.thumbstick_y.clone().into()],
            });

            bindings.write(OxrSuggestActionBinding {
                action: action.output.as_raw(),
                interaction_profile: interaction_profile.clone().into(),
                bindings: vec![config.output.clone().into()],
            });
        }
    }
}
fn create_actions(instance: Res<OxrInstance>, mut cmds: Commands) {
    let set = instance
        .create_action_set("left_set", "Left Set", 0)
        .unwrap();
    let aim = set
        .create_action("aim_left", "Left Hand Aim Pose", &[])
        .unwrap();

    let trigger = set
        .create_action("trigger_left", "Left Hand Trigger", &[])
        .unwrap();

    let squeeze = set
        .create_action("squeeze_left", "Left Hand Squeeze", &[])
        .unwrap();

    let grip = set
        .create_action("pose_left", "Left Hand Pose", &[])
        .unwrap();

    let thumbstick_x = set
        .create_action("thumbstick_x_left", "Left Hand Thumbstick_x", &[])
        .unwrap();

    let thumbstick_y = set
        .create_action("thumbstick_y_left", "Left Hand Thumbstick_y", &[])
        .unwrap();

    let output = set
        .create_action("output_left", "Left Hand Output", &[])
        .unwrap();

    let left = ControllerActionSet {
        set,
        aim,
        trigger,
        squeeze,
        grip,
        thumbstick_x,
        thumbstick_y,
        output,
    };

    let set = instance
        .create_action_set("right_set", "Right Set", 0)
        .unwrap();
    let aim = set
        .create_action("aim_right", "Right Hand Aim Pose", &[])
        .unwrap();

    let trigger = set
        .create_action("trigger_right", "Right Hand Trigger", &[])
        .unwrap();

    let squeeze = set
        .create_action("squeeze_right", "Right Hand Squeeze", &[])
        .unwrap();

    let grip = set
        .create_action("pose_right", "Right Hand Pose", &[])
        .unwrap();

    let thumbstick_x = set
        .create_action("thumbstick_x_right", "Right Hand Thumbstick_x", &[])
        .unwrap();

    let thumbstick_y = set
        .create_action("thumbstick_y_right", "Right Hand Thumbstick_y", &[])
        .unwrap();

    let output = set
        .create_action("output_right", "Right Hand Output", &[])
        .unwrap();

    let right = ControllerActionSet {
        set,
        aim,
        trigger,
        squeeze,
        grip,
        thumbstick_x,
        thumbstick_y,
        output,
    };

    cmds.insert_resource(ControllerActions { left, right })
}

fn spawn_hand_tracking(
    actions: Res<ControllerActions>,
    mut cmds: Commands,
    session: Res<OxrSession>,
) {
    // SPAWN AIM
    // =========================================================================================================
    let left_space = session
        .create_action_space(&actions.left.aim, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    let right_space = session
        .create_action_space(&actions.right.aim, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    cmds.spawn((left_space, AimLeft));
    cmds.spawn((right_space, AimRight));

    // SPAWN GRIP
    // =========================================================================================================
    let left_space = session
        .create_action_space(&actions.left.grip, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    let right_space = session
        .create_action_space(
            &actions.right.grip,
            openxr::Path::NULL,
            Isometry3d::IDENTITY,
        )
        .unwrap();

    cmds.spawn((left_space, GripLeft));
    cmds.spawn((right_space, GripRight));
}

pub struct VrControlPlugin;

impl Plugin for VrControlPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Config::read_or_create_default("vr_config.json"));
        app.add_systems(XrSessionCreated, spawn_hand_tracking);
        app.add_systems(XrSessionCreated, create_hand_trackers);
        app.add_systems(XrSessionCreated, attach_set);
        app.add_systems(
            PreUpdate,
            sync_actions
                .before(OxrActionSetSyncSet)
                .run_if(openxr_session_running),
        );
        app.add_systems(OxrSendActionBindings, suggest_action_bindings);
        app.add_systems(Startup, create_actions.run_if(session_available));
        app.add_plugins(ThumbstickPlugin);
        app.add_plugins(TriggerPlugin);
        app.add_plugins(VibrationPlugin);
        app.add_plugins(TrackingUtilitiesPlugin);
    }
}
