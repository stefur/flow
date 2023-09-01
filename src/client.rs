use crate::protocols::river_protocols::zriver_command_callback_v1::ZriverCommandCallbackV1;
use crate::protocols::river_protocols::zriver_control_v1::ZriverControlV1;
use crate::protocols::river_protocols::zriver_output_status_v1::{
    Event::FocusedTags, Event::UrgentTags, ZriverOutputStatusV1,
};
use crate::protocols::river_protocols::zriver_seat_status_v1;
use crate::protocols::river_protocols::zriver_seat_status_v1::ZriverSeatStatusV1;
use crate::protocols::river_protocols::zriver_status_manager_v1;
use crate::protocols::river_protocols::zriver_status_manager_v1::ZriverStatusManagerV1;
use std::collections::HashMap;
use wayland_client::{
    protocol::{
        wl_output::{Event::Name as OutputName, WlOutput},
        wl_registry::{Event::Global, WlRegistry},
        wl_seat::WlSeat,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

#[derive(Debug)]
pub struct River {
    pub status_manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub seat_status: Option<zriver_seat_status_v1::ZriverSeatStatusV1>,
    pub seat: Option<WlSeat>,
    pub output_status: Vec<ZriverOutputStatusV1>,
    pub outputs: HashMap<WlOutput, String>,
    pub focused_output: Option<WlOutput>,
    pub focused_tags: Option<u32>,
    pub urgent: HashMap<String, u32>,
    pub control: Option<ZriverControlV1>,
}

impl River {
    pub fn new() -> River {
        River {
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

    pub fn send_command(&self, arguments: Vec<String>, queue_handle: &QueueHandle<Self>) {
        for arg in arguments.iter() {
            self.control.as_ref().unwrap().add_argument(arg.to_string());
        }
        self.control
            .as_ref()
            .unwrap()
            .run_command(self.seat.as_ref().unwrap(), queue_handle, ());
    }

    pub fn destroy(&self) {
        self.status_manager.as_ref().unwrap().destroy();
        for output_status in self.output_status.iter() {
            output_status.destroy();
        }
        self.seat_status.as_ref().unwrap().destroy();
        self.control.as_ref().unwrap().destroy();
    }

    pub fn toggle_tags(&self, to_tags: &u32) -> bool {
        self.focused_tags.unwrap_or_default() == *to_tags
    }

    pub fn cycle_tags(&self, direction: &str, n_tags: &u32, queue_handle: &QueueHandle<Self>) {
        let last_tag: u32 = 1 << (n_tags - 1);
        let mut new_tags = 0;
        let mut tags = self.focused_tags.unwrap();

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

impl Dispatch<WlRegistry, ()> for River {
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

impl Dispatch<ZriverOutputStatusV1, (WlOutput, String)> for River {
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
                if &output.0 != state.focused_output.as_ref().unwrap() {
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

impl Dispatch<ZriverSeatStatusV1, ()> for River {
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

impl Dispatch<WlOutput, ()> for River {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let OutputName { name } = event {
            state.outputs.insert(output.to_owned(), name);
        }
    }
}

impl Dispatch<WlSeat, ()> for River {
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

impl Dispatch<ZriverStatusManagerV1, ()> for River {
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

impl Dispatch<ZriverCommandCallbackV1, ()> for River {
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

impl Dispatch<ZriverControlV1, ()> for River {
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
