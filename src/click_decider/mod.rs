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

#[derive(Event, Clone)]
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
    state: Res<Tracers>,
    mut set: ParamSet<(EventReader<LogTrace>, EventWriter<LogTrace>)>,
) {
    let mut execute_logging = false;

    // Collect up to "count_till_execution" frames, to completely catch the event
    let event_list = {
        let mut reader = set.p0();
        reader.read().map(|e| e.clone()).collect::<Vec<LogTrace>>()
    };

    for evt in event_list {
        if evt.count_till_execution < 1 {
            execute_logging = true;
        } else {
            set.p1().write(LogTrace {
                count_till_execution: evt.count_till_execution - 1,
            });
        }
    }

    if !execute_logging {
        return;
    }

    let res = state.left_tracer.log_current_transforms();
    if res.is_err() {
        println!("{}", res.err().unwrap());
    }
    let res = state.right_tracer.log_current_transforms();
    if res.is_err() {
        println!("{}", res.err().unwrap());
    }
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
        app.add_systems(PostUpdate, handle_add_trace_events);
        app.add_systems(PostUpdate, handle_log_events.after(handle_add_trace_events));
    }
}
