use crate::client::Flow;
use crate::options::{parse_args, Arguments};
use std::error::Error;
use wayland_client::Connection;

mod client;
mod options;
mod protocols;

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
    flow.seat_status = flow.status_manager.as_ref().map(|manager| {
        manager.get_river_seat_status(
            flow.seat.as_ref().expect("A seat should exist."),
            &queue_handle,
            (),
        )
    });

    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);

    // Setup the outputs, each one with an object and name
    for (object, name) in &flow.outputs {
        if let Some(status_manager) = flow.status_manager.as_ref() {
            let output_status = status_manager.get_river_output_status(
                object,
                &queue_handle,
                (object.to_owned(), name.to_owned()),
            );
            flow.output_status.push(output_status);
        }
    }

    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);

    match command {
        Ok(Arguments::CycleTags { direction, n_tags }) => {
            // If there are no n_tags assigned, or if unwrap fails, we assume default of 9
            flow.cycle_tags(&direction, &n_tags.unwrap_or(9), &queue_handle);
        }
        Ok(Arguments::ToggleTags { to_tags }) => {
            if flow.toggle_tags(&to_tags) {
                flow.send_command(vec![String::from("focus-previous-tags")], &queue_handle);
            } else {
                flow.send_command(
                    vec![String::from("set-focused-tags"), to_tags.to_string()],
                    &queue_handle,
                );
            }
        }
        Ok(Arguments::FocusUrgentTags) => {
            // Make sure there is an output as well as tags that are urgent
            if let (Some(urgent_output), Some(urgent_tags)) =
                (flow.urgent.keys().next(), flow.urgent.values().next())
            {
                flow.send_command(
                    vec![String::from("focus-output"), urgent_output.to_owned()],
                    &queue_handle,
                );
                flow.send_command(
                    vec![String::from("set-focused-tags"), urgent_tags.to_string()],
                    &queue_handle,
                )
            }
        }
        _ => (),
    }
    event_queue.roundtrip(&mut flow).expect(ROUNDTRIP_EXPECT);
    flow.destroy();
}
