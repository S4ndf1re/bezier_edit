use crate::bezier_curve::bezier_curve_renderer::RenderInformation;
use bevy::color::palettes::tailwind::{BLUE_600, RED_600};
use bevy::prelude::*;
use bevy_mod_openxr::action_binding::OxrSendActionBindings;
use bevy_mod_openxr::action_set_syncing::OxrActionSetSyncSet;
use bevy_mod_openxr::{
    action_binding::OxrSuggestActionBinding, action_set_attaching::OxrAttachActionSet,
    action_set_syncing::OxrSyncActionSet, openxr_session_running, resources::OxrInstance,
    session::OxrSession,
};
use bevy_mod_xr::session::{session_available, XrSessionCreated};
use openxr::{Haptic, Posef};

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
    left_trigger: openxr::Action<Haptic>,
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

    let left_trigger = set
        .create_action("left_trigger", "Left Hand Trigger", &[])
        .unwrap();

    cmds.insert_resource(ControllerAimActions {
        set,
        left,
        right,
        left_trigger,
    })
}

fn spawn_hands(
    actions: Res<ControllerAimActions>,
    mut cmds: Commands,
    session: Res<OxrSession>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    scaleinfo: Res<RenderInformation>,
) {
    let left_space = session
        .create_action_space(&actions.left, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    let right_space = session
        .create_action_space(&actions.right, openxr::Path::NULL, Isometry3d::IDENTITY)
        .unwrap();

    let cuboid = meshes.add(Cuboid::new(
        0.01 * scaleinfo.scale,
        0.01 * scaleinfo.scale,
        10.0 * scaleinfo.scale,
    ));
    let red = Color::from(RED_600);
    let blue = Color::from(BLUE_600);

    cmds.spawn((left_space, Aim)).with_children(|ui| {
        ui.spawn((
            Mesh3d(cuboid.clone()),
            MeshMaterial3d(materials.add(red)),
            Transform::from_xyz(0.0, 0.0, -5.0 * scaleinfo.scale),
        ));
    });
    cmds.spawn((right_space, Aim)).with_children(|ui| {
        ui.spawn((
            Mesh3d(cuboid.clone()),
            MeshMaterial3d(materials.add(blue)),
            Transform::from_xyz(0.0, 0.0, -5.0 * scaleinfo.scale),
        ));
    });
}

#[derive(Component)]
#[require(Transform)]
struct Aim;
