mod client;
mod options;
mod protocols;
use crate::client::River;
use crate::options::parse_args;
use options::Arguments;
use std::error::Error;
use wayland_client::Connection;

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
            Arguments::FocusUrgentTags { .. } => Ok(args),
        },
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let queue_handle = event_queue.handle();

    let _registry = display.get_registry(&queue_handle, ());

    let mut river = River::default();

    event_queue.roundtrip(&mut river).unwrap();

    // Get the seat status
    river.status = Some(
        river
            .status_manager
            .as_ref()
            .unwrap()
            .get_river_seat_status(river.seat.as_ref().unwrap(), &queue_handle, ()),
    );
    event_queue.roundtrip(&mut river).unwrap();

    // Setup the outputs
    for (object, name) in &river.outputs {
        river
            .status_manager
            .as_ref()
            .unwrap()
            .get_river_output_status(object, &queue_handle, (object.to_owned(), name.to_string()));
    }

    event_queue.roundtrip(&mut river).unwrap();

    match command {
        Ok(Arguments::CycleTags { direction, n_tags }) => {
            // If there are no n_tags assigned, or if unwrap fails, we assume default of 9
            river.cycle_tags(&direction, &n_tags.unwrap_or(9), &queue_handle);
        }
        Ok(Arguments::ToggleTags { to_tags }) => {
            if river.clone().toggle_tags(to_tags) {
                river.focus_previous_tags(&queue_handle);
            } else {
                river.set_focused_tags(&to_tags, &queue_handle);
            }
        }
        Ok(Arguments::FocusUrgentTags) => {
            // If there are no empty tags there is nothing to do
            if river.urgent.is_empty() {
                return;
            }
            river.focus_output(river.urgent.keys().next().unwrap(), &queue_handle);
            river.set_focused_tags(river.urgent.values().next().unwrap(), &queue_handle);
        }
        _ => (),
    }
    event_queue.roundtrip(&mut river).unwrap();
}
