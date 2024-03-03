use crate::client::Flow;
use crate::options::{parse_args, Arguments};
use std::error::Error;
use wayland_client::{Connection, Proxy};

mod client;
mod options;
mod output;
mod protocols;
mod seat;

static ROUNDTRIP_EXPECT: &str = "All requests in queue must be sent and handled before proceeding.";

fn main() {
    // Parse the options
    let command: Result<Arguments, Box<dyn Error>> = match parse_args() {
        Ok(args) => match &args {
            Arguments::Global { help: _ } => {
                print!("{}", options::HELP);
                std::process::exit(0);
            }
            // Should probably check here that the provided arguments to the command are correct before proceeding
            Arguments::CycleTags { .. } => Ok(args),
            Arguments::ToggleTags { .. } => Ok(args),
            Arguments::FocusUrgentTags => Ok(args),
            Arguments::FocusSetViewTags { .. } => Ok(args),
        },
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    let conn = Connection::connect_to_env().expect("Failed to connect to the Wayland server!");

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let queue_handle = event_queue.handle();

    let _registry = display.get_registry(&queue_handle, ());

    let mut flow = Flow::new();

    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);

    // Get the seat status
    if let Some(seat) = flow.seat.as_mut() {
        seat.seat_status = Some(
            flow.status_manager
                .as_ref()
                .expect("A status manager should exist.")
                .get_river_seat_status(&seat.wlseat, &queue_handle, ()),
        );
    } else {
        panic!("Failed to get the seat status. A seat should exist but was None.")
    }

    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);

    // Setup the outputs, pass on the wloutput id so we bind the state correctly to each output
    for output in &mut flow.outputs {
        if let Some(status_manager) = flow.status_manager.as_ref() {
            output.status = Some(status_manager.get_river_output_status(
                &output.wloutput,
                &queue_handle,
                output.wloutput.id(),
            ));
        }
    }

    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);

    match command {
        Ok(Arguments::CycleTags {
            direction,
            n_tags,
            skip_unoccupied,
        }) => {
            // Find the focused output state
            if let Some(focused_output_state) = flow.find_focused_output() {
                // If there are no n_tags assigned, or if unwrap fails, we assume default of 9
                let new_tags = focused_output_state.cycle_tags(
                    &direction,
                    &n_tags.unwrap_or(9),
                    skip_unoccupied,
                );

                flow.send_command(
                    vec![String::from("set-focused-tags"), new_tags.to_string()],
                    &queue_handle,
                );
            }
        }
        Ok(Arguments::ToggleTags { to_tags }) => {
            if let Some(output) = flow.find_focused_output() {
                if output.toggle_tags(&to_tags) {
                    flow.send_command(vec![String::from("focus-previous-tags")], &queue_handle);
                } else {
                    flow.send_command(
                        vec![String::from("set-focused-tags"), to_tags.to_string()],
                        &queue_handle,
                    );
                }
            }
        }
        Ok(Arguments::FocusUrgentTags) => {
            // Make sure there is an output as well as tags that are urgent
            if let Some(output) = flow.find_focused_output() {
                if let Some(urgent_tags) = output.urgent_tags {
                    flow.send_command(
                        vec![String::from("focus-output"), output.name.to_owned()],
                        &queue_handle,
                    );
                    flow.send_command(
                        vec![String::from("set-focused-tags"), urgent_tags.to_string()],
                        &queue_handle,
                    )
                }
            }
        }
        Ok(Arguments::FocusSetViewTags { to_tags }) => {
            flow.send_command(
                vec![String::from("set-view-tags"), to_tags.to_string()],
                &queue_handle,
            );
            flow.send_command(
                vec![String::from("set-focused-tags"), to_tags.to_string()],
                &queue_handle,
            );
        }
        _ => (),
    }
    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);
    flow.destroy();
}
