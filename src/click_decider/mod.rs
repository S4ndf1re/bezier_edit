use bevy::prelude::*;
use config::Config;
use tracer::{ButtonPressValue, ControllerTrace};

pub mod config;
pub mod tracer;

#[derive(Resource)]
pub struct Tracers {
    pub left_tracer: ControllerTrace,
    pub right_tracer: ControllerTrace,
}

#[derive(Event)]
pub struct AddLeftTrace {
    pub transform: Transform,
    pub click_value: ButtonPressValue,
}

#[derive(Event)]
pub struct AddRightTrace {
    pub transform: Transform,
    pub click_value: ButtonPressValue,
}

#[derive(Event)]
pub struct LogTrace {
    count_till_execution: usize,
}

impl Default for LogTrace {
    fn default() -> Self {
        Self {
            count_till_execution: 10,
        }
    }
}

fn handle_add_trace_events(
    mut left_events: EventReader<AddLeftTrace>,
    mut right_events: EventReader<AddRightTrace>,
    mut state: ResMut<Tracers>,
) {
    for evt in left_events.read() {
        state.left_tracer.update(evt.transform, evt.click_value);
    }

    for evt in right_events.read() {
        state.right_tracer.update(evt.transform, evt.click_value);
    }
}

fn handle_log_events(
    mut log_events: EventReader<LogTrace>,
    state: Res<Tracers>,
    mut log_event_writer: EventWriter<LogTrace>,
) {
    let mut execute_logging = false;

    // Collect up to "count_till_execution" frames, to completely catch the event
    for evt in log_events.read() {
        if evt.count_till_execution < 1 {
            execute_logging = true;
        } else {
            log_event_writer.write(LogTrace {
                count_till_execution: evt.count_till_execution - 1,
            });
        }
    }

    if !execute_logging {
        return;
    }

    log_events.clear();

    let _ = state.left_tracer.log_current_transforms();
    let _ = state.right_tracer.log_current_transforms();
}

pub struct TracingPlugin;

impl Plugin for TracingPlugin {
    fn build(&self, app: &mut App) {
        let config = Config::read_or_create_default("click_config.json");
        // using 90 FPS, 180 is equal to two seconds
        app.insert_resource(Tracers {
            left_tracer: ControllerTrace::new(
                config.clone(),
                0.0,
                tracer::ControllerSide::Left,
                180,
            ),
            right_tracer: ControllerTrace::new(config, 0.0, tracer::ControllerSide::Right, 180),
        });
        app.add_event::<AddLeftTrace>();
        app.add_event::<AddRightTrace>();
        app.add_event::<LogTrace>();
        app.add_systems(
            PostUpdate,
            handle_add_trace_events.before(handle_log_events),
        );
    }
}
