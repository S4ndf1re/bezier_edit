use bevy::prelude::*;
use bevy_mod_openxr::action_binding::OxrSendActionBindings;
use bevy_mod_openxr::action_set_syncing::OxrActionSetSyncSet;
use bevy_mod_openxr::{
    action_binding::OxrSuggestActionBinding, action_set_attaching::OxrAttachActionSet,
    action_set_syncing::OxrSyncActionSet, openxr_session_running, resources::OxrInstance,
    session::OxrSession,
};
use bevy_mod_xr::session::{session_available, XrSessionCreated};
use openxr::Posef;

pub struct AimTrackingPlugin;

impl Plugin for AimTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(XrSessionCreated, spawn_hands);
        app.add_systems(XrSessionCreated, attach_set);
        app.add_systems(
            PreUpdate,
            sync_actions
                .before(OxrActionSetSyncSet)
                .run_if(openxr_session_running),
        );
        app.add_systems(OxrSendActionBindings, suggest_action_bindings);
        app.add_systems(Startup, create_actions.run_if(session_available));
    }
}

fn attach_set(actions: Res<ControllerAimActions>, mut attach: EventWriter<OxrAttachActionSet>) {
    attach.write(OxrAttachActionSet(actions.set.clone()));
}

#[derive(Resource)]
struct ControllerAimActions {
    set: openxr::ActionSet,
    left: openxr::Action<Posef>,
    right: openxr::Action<Posef>,
}
fn sync_actions(actions: Res<ControllerAimActions>, mut sync: EventWriter<OxrSyncActionSet>) {
    sync.write(OxrSyncActionSet(actions.set.clone()));
}

fn suggest_action_bindings(
    actions: Res<ControllerAimActions>,
    mut bindings: EventWriter<OxrSuggestActionBinding>,
) {
    bindings.write(OxrSuggestActionBinding {
        action: actions.left.as_raw(),
        interaction_profile: "/interaction_profiles/oculus/touch_controller".into(),
        bindings: vec!["/user/hand/left/input/aim/pose".into()],
    });
    bindings.write(OxrSuggestActionBinding {
        action: actions.right.as_raw(),
        interaction_profile: "/interaction_profiles/oculus/touch_controller".into(),
        bindings: vec!["/user/hand/right/input/aim/pose".into()],
    });
}
fn create_actions(instance: Res<OxrInstance>, mut cmds: Commands) {
    let set = instance.create_action_set("aim_pose", "Aim", 0).unwrap();
    let left = set
        .create_action("left_aim", "Left Hand Aim Pose", &[])
        .unwrap();
    let right = set
        .create_action("right_aim", "Right Hand Aim Pose", &[])
        .unwrap();

    cmds.insert_resource(ControllerAimActions { set, left, right })
}

fn spawn_hands(actions: Res<ControllerAimActions>, mut cmds: Commands, session: Res<OxrSession>) {
    let left_space = session
        .create_action_space(&actions.left, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    let right_space = session
        .create_action_space(&actions.right, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    cmds.spawn((left_space, Aim));
    cmds.spawn((right_space, Aim));
}

#[derive(Component)]
#[require(Transform)]
struct Aim;
