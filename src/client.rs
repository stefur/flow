use std::collections::HashMap;
use wayland_client::{
    protocol::{
        wl_output::{Event::Name, WlOutput},
        wl_registry::{Event::Global, WlRegistry},
        wl_seat::WlSeat,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use crate::protocols::river_protocols::{
    zriver_command_callback_v1::ZriverCommandCallbackV1,
    zriver_control_v1::ZriverControlV1,
    zriver_output_status_v1::{
        Event::{FocusedTags, UrgentTags},
        ZriverOutputStatusV1,
    },
    zriver_seat_status_v1::{self, ZriverSeatStatusV1},
    zriver_status_manager_v1::{self, ZriverStatusManagerV1},
};

type OutputName = String;

#[derive(Debug)]
pub struct Flow {
    pub status_manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub seat_status: Option<zriver_seat_status_v1::ZriverSeatStatusV1>,
    pub seat: Option<WlSeat>,
    pub output_status: Vec<ZriverOutputStatusV1>,
    pub outputs: HashMap<WlOutput, OutputName>,
    pub focused_output: Option<WlOutput>,
    pub focused_tags: Option<u32>,
    pub urgent: HashMap<String, u32>,
    pub control: Option<ZriverControlV1>,
}

impl Flow {
    pub fn new() -> Flow {
        Flow {
            status_manager: None,
            seat: None,
            seat_status: None,
            output_status: vec![],
            outputs: HashMap::new(),
            focused_output: None,
            focused_tags: None,
            urgent: HashMap::new(),
            control: None,
        }
    }

    /// Send a command to river
    pub fn send_command(&self, arguments: Vec<String>, queue_handle: &QueueHandle<Self>) {
        if let (Some(control), Some(seat)) = (&self.control, &self.seat) {
            for arg in &arguments {
                control.add_argument(arg.to_owned());
            }
            control.run_command(seat, queue_handle, ());
        }
    }

    /// Destroy objects when no longer needed
    pub fn destroy(&mut self) {
        self.status_manager.take().map(|manager| manager.destroy());
        self.output_status
            .iter()
            .for_each(|output_status| output_status.destroy());
        self.seat_status.take().map(|status| status.destroy());
        self.control.take().map(|control| control.destroy());
    }

    /// Checks if the requested tags are already focused
    pub fn toggle_tags(&self, to_tags: &u32) -> bool {
        self.focused_tags == Some(*to_tags)
    }

    pub fn cycle_tags(&self, direction: &str, n_tags: &u32, queue_handle: &QueueHandle<Self>) {
        let last_tag: u32 = 1 << (n_tags - 1);
        let mut new_tags = 0;
        let mut tags = self.focused_tags.unwrap_or_default();

        match direction {
            "next" => {
                if tags & last_tag != 0 {
                    tags ^= last_tag;
                    new_tags = 1;
                }

                new_tags |= tags << 1;
            }
            "previous" => {
                if (tags & 1) != 0 {
                    tags ^= 1;
                    new_tags = last_tag;
                }
                new_tags |= tags >> 1;
            }
            _ => (),
        }

        self.send_command(
            vec![String::from("set-focused-tags"), new_tags.to_string()],
            queue_handle,
        );
    }
}

impl Dispatch<WlRegistry, ()> for Flow {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _: &(),
        _: &Connection,
        queue_handle: &QueueHandle<Self>,
    ) {
        if let Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_output" => {
                    registry.bind::<WlOutput, _, Self>(name, version, queue_handle, ());
                }
                "zriver_status_manager_v1" => {
                    state.status_manager = Some(registry.bind::<ZriverStatusManagerV1, _, Self>(
                        name,
                        version,
                        queue_handle,
                        (),
                    ));
                }
                "zriver_control_v1" => {
                    state.control = Some(registry.bind::<ZriverControlV1, _, Self>(
                        name,
                        version,
                        queue_handle,
                        (),
                    ));
                }
                "wl_seat" => {
                    state.seat =
                        Some(registry.bind::<WlSeat, _, Self>(name, version, queue_handle, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZriverOutputStatusV1, (WlOutput, String)> for Flow {
    fn event(
        state: &mut Self,
        _: &ZriverOutputStatusV1,
        event: <ZriverOutputStatusV1 as Proxy>::Event,
        output: &(WlOutput, String),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            FocusedTags { tags } => {
                // Ignore the tags that are not on the focused output
                if &output.0
                    != state
                        .focused_output
                        .as_ref()
                        .expect("There should be a focused output.")
                {
                    return;
                }
                // Set the focused tags
                state.focused_tags = Some(tags);
            }
            UrgentTags { tags } => {
                // If urgent tags are not 0 (e.g. none are urgent) ,we add the output name and tags to state
                if tags != 0 {
                    state.urgent.insert(output.1.to_owned(), tags);
                }
            }
            _ => (),
        }
    }
}

impl Dispatch<ZriverSeatStatusV1, ()> for Flow {
    fn event(
        state: &mut Self,
        _: &ZriverSeatStatusV1,
        event: <ZriverSeatStatusV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zriver_seat_status_v1::Event::FocusedOutput { output } = event {
            state.focused_output = Some(output);
        }
    }
}

impl Dispatch<WlOutput, ()> for Flow {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Name { name } = event {
            state.outputs.insert(output.to_owned(), name);
        }
    }
}

impl Dispatch<WlSeat, ()> for Flow {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZriverStatusManagerV1, ()> for Flow {
    fn event(
        _: &mut Self,
        _: &ZriverStatusManagerV1,
        _: <ZriverStatusManagerV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZriverCommandCallbackV1, ()> for Flow {
    fn event(
        _: &mut Self,
        _: &ZriverCommandCallbackV1,
        _: <ZriverCommandCallbackV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZriverControlV1, ()> for Flow {
    fn event(
        _: &mut Self,
        _: &ZriverControlV1,
        _: <ZriverControlV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
