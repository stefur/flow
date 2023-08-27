use crate::protocols::river_protocols::zriver_command_callback_v1::ZriverCommandCallbackV1;
use crate::protocols::river_protocols::zriver_control_v1::ZriverControlV1;
use crate::protocols::river_protocols::zriver_output_status_v1::{
    Event::FocusedTags, ZriverOutputStatusV1,
};
use crate::protocols::river_protocols::zriver_seat_status_v1;
use crate::protocols::river_protocols::zriver_status_manager_v1;
use wayland_client::{
    protocol::{
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
    Dispatch, Proxy,
};
// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.

#[derive(Debug, Clone)]
pub struct River {
    pub status_manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub status: Option<zriver_seat_status_v1::ZriverSeatStatusV1>,
    pub seat: Option<String>,
    pub wl_seat: Option<WlSeat>,
    pub focused_output: Option<WlOutput>,
    pub focused_tags: Option<u32>,
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
            wl_seat: None,
            status: None,
            focused_output: None,
            focused_tags: None,
            control: None,
        }
    }

    pub fn cycle_tags(self, direction: String, mut n_tags: Option<u32>) -> u32 {
        if n_tags.is_none() {
            n_tags = Some(9);
        }

        let last_tag: u32 = 1 << (n_tags.unwrap() - 1);
        let mut new_tags = 0;
        let mut tags = self.focused_tags.unwrap();

        match direction.to_lowercase().as_str() {
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
        new_tags
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
                    registry.bind::<wl_output::WlOutput, _, Self>(name, version, qh, ());
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
                    state.wl_seat = Some(registry.bind::<WlSeat, _, Self>(name, version, qh, ()));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZriverOutputStatusV1, ()> for River {
    fn event(
        state: &mut Self,
        _seat_status: &ZriverOutputStatusV1,
        event: <ZriverOutputStatusV1 as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let FocusedTags { tags } = event {
            state.focused_tags = Some(tags);
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
        _: &mut Self,
        _: &WlOutput,
        _: <WlOutput as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _: &wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for River {
    fn event(
        state: &mut Self,
        _wl_seat: &WlSeat,
        event: <WlSeat as wayland_client::Proxy>::Event,
        _: &(),
        _: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Name { name } = event {
            state.seat = Some(name);
        }
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
