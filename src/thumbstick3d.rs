use bevy::app::Startup;
use bevy::prelude::{
    App, Commands, Component, IntoScheduleConfigs, Plugin, PostUpdate, Query, ResMut, Resource,
};
use bevy_mod_xr::actions::ActionType;
use bevy_xr_utils::xr_utils_actions::{
    ActiveSet, XRUtilsAction, XRUtilsActionSet, XRUtilsActionState, XRUtilsActionSystemSet,
    XRUtilsBinding,
};

enum ThumbstickDirection {
    X,
    Y,
}
#[derive(Component)]
struct ThumbstickMarker(ThumbstickDirection);

#[derive(Resource)]
pub struct AccumulatedThumbstickInfo {
    x: f32,
    y: f32,
}
impl AccumulatedThumbstickInfo {
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }

    #[allow(unused)]
    pub fn x(&self) -> f32 {
        self.x
    }

    #[allow(unused)]
    pub fn y(&self) -> f32 {
        self.y
    }
}

fn setup_thumstick_actions(mut commands: Commands) {
    let set = commands
        .spawn((
            XRUtilsActionSet {
                name: "thumbstick".into(),
                pretty_name: "Thumbstick 3D".into(),
                priority: u32::MIN,
            },
            ActiveSet,
        ))
        .id();

    let thumbstick_action_x = commands
        .spawn((
            XRUtilsAction {
                action_name: "thumbstick_x".into(),
                localized_name: "thumbstick_x".into(),
                action_type: ActionType::Float,
            },
            ThumbstickMarker(ThumbstickDirection::X),
        ))
        .id();

    let thumbstick_action_y = commands
        .spawn((
            XRUtilsAction {
                action_name: "thumbstick_y".into(),
                localized_name: "thumbstick_y".into(),
                action_type: ActionType::Float,
            },
            ThumbstickMarker(ThumbstickDirection::Y),
        ))
        .id();

    let controller_left_x_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/left/input/thumbstick/x".into(),
        })
        .id();

    let controller_left_y_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/left/input/thumbstick/y".into(),
        })
        .id();

    let controller_right_x_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/right/input/thumbstick/x".into(),
        })
        .id();

    let controller_right_y_binding = commands
        .spawn(XRUtilsBinding {
            profile: "/interaction_profiles/oculus/touch_controller".into(),
            binding: "/user/hand/right/input/thumbstick/y".into(),
        })
        .id();

    commands
        .entity(thumbstick_action_x)
        .add_child(controller_left_x_binding)
        .add_child(controller_right_x_binding);

    commands
        .entity(thumbstick_action_y)
        .add_child(controller_left_y_binding)
        .add_child(controller_right_y_binding);

    commands
        .entity(set)
        .add_children(&[thumbstick_action_x, thumbstick_action_y]);
}

fn update_thumbstick_events(
    events: Query<(&XRUtilsActionState, &ThumbstickMarker)>,
    mut thumbstick_state: ResMut<AccumulatedThumbstickInfo>,
) {
    thumbstick_state.x = 0.0;
    thumbstick_state.y = 0.0;
    for (event, marker) in events.iter() {
        if let XRUtilsActionState::Float(state) = event {
            if !state.is_active {
                continue;
            }
            match marker.0 {
                ThumbstickDirection::X => thumbstick_state.x += state.current_state,
                ThumbstickDirection::Y => thumbstick_state.y += state.current_state,
            }
        }
    }
}

pub struct ThumbstickPlugin;

impl Plugin for ThumbstickPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AccumulatedThumbstickInfo::new());
        app.add_systems(
            Startup,
            setup_thumstick_actions.before(XRUtilsActionSystemSet::CreateEvents),
        );
        app.add_systems(PostUpdate, update_thumbstick_events);
    }
}
