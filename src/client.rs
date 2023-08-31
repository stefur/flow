use crate::protocols::river_protocols::zriver_command_callback_v1::ZriverCommandCallbackV1;
use crate::protocols::river_protocols::zriver_control_v1::ZriverControlV1;
use crate::protocols::river_protocols::zriver_output_status_v1::{
    Event::FocusedTags, Event::UrgentTags, ZriverOutputStatusV1,
};
use crate::protocols::river_protocols::zriver_seat_status_v1;
use crate::protocols::river_protocols::zriver_status_manager_v1;
use std::collections::HashMap;
use wayland_client::{
    protocol::{
        wl_output::{Event::Name as OutputName, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_seat::WlSeat,
    },
    Dispatch, Proxy, QueueHandle,
};

#[derive(Debug, Clone)]
pub struct River {
    pub status_manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub status: Option<zriver_seat_status_v1::ZriverSeatStatusV1>,
    pub seat: Option<WlSeat>,
    pub outputs: HashMap<WlOutput, String>,
    pub focused_output: Option<WlOutput>,
    pub focused_tags: Option<u32>,
    pub urgent: HashMap<String, u32>,
    pub control: Option<ZriverControlV1>,
}

impl Default for River {
    fn default() -> Self {
        Self::new()
    }
}

impl River {
    pub fn new() -> River {
        River {
            status_manager: None,
            seat: None,
            status: None,
            outputs: HashMap::new(),
            focused_output: None,
            focused_tags: None,
            urgent: HashMap::new(),
            control: None,
        }
    }

    pub fn focus_previous_tags(&self, queue_handle: &QueueHandle<Self>) {
        self.control
            .as_ref()
            .unwrap()
            .add_argument(String::from("focus-previous-tags"));

        self.control
            .as_ref()
            .unwrap()
            .run_command(self.seat.as_ref().unwrap(), queue_handle, ());
    }

    // Focus output
    pub fn focus_output(&self, output: &String, queue_handle: &QueueHandle<Self>) {
        self.control
            .as_ref()
            .unwrap()
            .add_argument(String::from("focus-output"));
        self.control
            .as_ref()
            .unwrap()
            .add_argument(output.to_owned());
        self.control
            .as_ref()
            .unwrap()
            .run_command(self.seat.as_ref().unwrap(), queue_handle, ());
    }

    pub fn toggle_tags(self, to_tags: u32) -> bool {
        self.focused_tags.unwrap_or_default() == to_tags
    }

    pub fn set_focused_tags(&self, tags: &u32, queue_handle: &QueueHandle<Self>) {
        self.control
            .as_ref()
            .unwrap()
            .add_argument(String::from("set-focused-tags"));
        self.control
            .as_ref()
            .unwrap()
            .add_argument(tags.to_string());

        self.control
            .as_ref()
            .unwrap()
            .run_command(self.seat.as_ref().unwrap(), queue_handle, ());
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

        self.control
            .as_ref()
            .unwrap()
            .add_argument(String::from("set-focused-tags"));
        self.control
            .as_ref()
            .unwrap()
            .add_argument(new_tags.to_string());

        self.control
            .as_ref()
            .unwrap()
            .run_command(self.seat.as_ref().unwrap(), queue_handle, ());
    }
}

impl Dispatch<WlRegistry, ()> for River {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: <WlRegistry as Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        qh: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_output" => {
                    registry.bind::<WlOutput, _, Self>(name, version, qh, ());
                }
                "zriver_status_manager_v1" => {
                    state.status_manager = Some(
                        registry.bind::<zriver_status_manager_v1::ZriverStatusManagerV1, _, Self>(
                            name,
                            version,
                            qh,
                            (),
                        ),
                    );
                }
                "zriver_control_v1" => {
                    state.control =
                        Some(registry.bind::<ZriverControlV1, _, Self>(name, version, qh, ()));
                }
                "wl_seat" => {
                    state.seat = Some(registry.bind::<WlSeat, _, Self>(name, version, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZriverOutputStatusV1, (WlOutput, String)> for River {
    fn event(
        state: &mut Self,
        _seat_status: &ZriverOutputStatusV1,
        event: <ZriverOutputStatusV1 as wayland_client::Proxy>::Event,
        output: &(WlOutput, String),
        _: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            FocusedTags { tags } => {
                if &output.0 != state.focused_output.as_ref().unwrap() {
                    return;
                }
                state.focused_tags = Some(tags);
            }
            UrgentTags { tags } => {
                if tags != 0 {
                    state.urgent.insert(output.1.to_owned(), tags);
                }
            }
            _ => (),
        }
    }
}

impl Dispatch<zriver_seat_status_v1::ZriverSeatStatusV1, ()> for River {
    fn event(
        state: &mut Self,
        _seat_status: &zriver_seat_status_v1::ZriverSeatStatusV1,
        event: <zriver_seat_status_v1::ZriverSeatStatusV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
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
        event: <WlOutput as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
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
        _: <WlSeat as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zriver_status_manager_v1::ZriverStatusManagerV1, ()> for River {
    fn event(
        _: &mut Self,
        _: &zriver_status_manager_v1::ZriverStatusManagerV1,
        _: <zriver_status_manager_v1::ZriverStatusManagerV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZriverCommandCallbackV1, ()> for River {
    fn event(
        _: &mut Self,
        _: &ZriverCommandCallbackV1,
        _: <ZriverCommandCallbackV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZriverControlV1, ()> for River {
    fn event(
        _: &mut Self,
        _: &ZriverControlV1,
        _: <ZriverControlV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}
