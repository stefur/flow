mod client;
mod options;
mod protocols;
use crate::client::River;
use crate::options::parse_args;
use options::Arguments;
use std::error::Error;
use wayland_client::Connection;

fn main() {
    // Parse the options for use within the match rule for property changes
    let command: Result<Arguments, Box<dyn Error>> = match parse_args() {
        Ok(args) => match &args {
            Arguments::Global { help: _ } => {
                print!("{}", options::HELP);
                std::process::exit(0);
            }
            // Should probably check here that the provided arguments to the command are correct before proceeding
            Arguments::CycleTags { .. } => Ok(args),
            Arguments::ToggleTags { .. } => Ok(args),
        },
        Err(error) => {
            eprintln!("Error: {}", error);
            std::process::exit(1);
        }
    };

    // Create a Wayland connection by connecting to the server through the
    // environment-provided configuration.
    let conn = Connection::connect_to_env().unwrap();

    // Retrieve the WlDisplay Wayland object from the connection. This object is
    // the starting point of any Wayland program, from which all other objects will
    // be created.
    let display = conn.display();

    // Create an event queue for our event processing
    let mut event_queue = conn.new_event_queue();
    // An get its handle to associated new objects to it
    let qh = event_queue.handle();

    // Create a wl_registry object by sending the wl_display.get_registry request
    // This method takes two arguments: a handle to the queue the newly created
    // wl_registry will be assigned to, and the user-data that should be associated
    // with this registry (here it is () as we don't need user-data).
    let _registry = display.get_registry(&qh, ());

    // At this point everything is ready, and we just need to wait to receive the events
    // from the wl_registry, our callback will print the advertized globals.
    let mut river = River::default();

    event_queue.roundtrip(&mut river).unwrap();

    // Get the seat status
    river.status = Some(
        river
            .status_manager
            .as_ref()
            .unwrap()
            .get_river_seat_status(river.seat.as_ref().unwrap(), &qh, ()),
    );
    event_queue.roundtrip(&mut river).unwrap();

    // Get the focused output
    river
        .status_manager
        .as_ref()
        .unwrap()
        .get_river_output_status(river.focused_output.as_ref().unwrap(), &qh, ());

    event_queue.roundtrip(&mut river).unwrap();

    match command {
        Ok(Arguments::CycleTags { direction, n_tags }) => {
            let new_tags: &u32 = &river.clone().cycle_tags(direction, n_tags);

            river
                .control
                .as_ref()
                .unwrap()
                .add_argument(String::from("set-focused-tags"));
            river
                .control
                .as_ref()
                .unwrap()
                .add_argument(new_tags.to_string());

            river
                .control
                .as_ref()
                .unwrap()
                .run_command(river.seat.as_ref().unwrap(), &qh, ());
            event_queue.roundtrip(&mut river).unwrap();
        }
        Ok(Arguments::ToggleTags { to_tags }) => {
            if river.clone().toggle_tags(to_tags) {
                river
                    .control
                    .as_ref()
                    .unwrap()
                    .add_argument(String::from("focus-previous-tags"));
                river
                    .control
                    .as_ref()
                    .unwrap()
                    .run_command(river.seat.as_ref().unwrap(), &qh, ());
                event_queue.roundtrip(&mut river).unwrap();
            } else {
                river
                    .control
                    .as_ref()
                    .unwrap()
                    .add_argument(String::from("set-focused-tags"));
                river
                    .control
                    .as_ref()
                    .unwrap()
                    .add_argument(to_tags.to_string());

                river
                    .control
                    .as_ref()
                    .unwrap()
                    .run_command(river.seat.as_ref().unwrap(), &qh, ());
                event_queue.roundtrip(&mut river).unwrap();
            }
        }
        _ => (),
    }
}
