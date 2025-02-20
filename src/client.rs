use wayland_client::{
    backend::ObjectId,
    protocol::{
        wl_output::{Event::Name, WlOutput},
        wl_registry::{Event::Global, WlRegistry},
        wl_seat::WlSeat,
    },
    Connection, Dispatch, Proxy, QueueHandle,
};

use crate::output::Output;
use crate::seat::Seat;

use crate::protocols::river_protocols::{
    zriver_command_callback_v1::ZriverCommandCallbackV1,
    zriver_control_v1::ZriverControlV1,
    zriver_output_status_v1::{
        Event::{FocusedTags, UrgentTags, ViewTags},
        ZriverOutputStatusV1,
    },
    zriver_seat_status_v1::{self, ZriverSeatStatusV1},
    zriver_status_manager_v1::{self, ZriverStatusManagerV1},
};

#[derive(Debug)]
pub struct Flow {
    pub status_manager: Option<zriver_status_manager_v1::ZriverStatusManagerV1>,
    pub seat: Option<Seat>,
    pub outputs: Vec<Output>,
    pub control: Option<ZriverControlV1>,
}

impl Flow {
    pub fn new() -> Self {
        Self {
            status_manager: None,
            seat: None,
            outputs: vec![],
            control: None,
        }
    }

    /// Send a command to river
    pub fn send_command(&self, arguments: Vec<String>, queue_handle: &QueueHandle<Self>) {
        if let (Some(control), Some(seat)) = (&self.control, &self.seat) {
            for arg in &arguments {
                control.add_argument(arg.to_owned());
            }
            control.run_command(&seat.wlseat, queue_handle, ());
        }
    }

    /// Destroy all objects when no longer needed
    pub fn destroy(&mut self) {
        if let Some(manager) = self.status_manager.take() {
            manager.destroy()
        };

        for output in &self.outputs {
            if let Some(status) = &output.status {
                status.destroy();
            }
        }

        if let Some(status) = self.seat.as_mut().and_then(|seat| seat.seat_status.take()) {
            status.destroy()
        }

        if let Some(control) = self.control.take() {
            control.destroy()
        };
    }

    /// Identify an output based on a specific state
    pub fn find_output(&self, state: &str) -> Option<&Output> {
        match state {
            "focused" => self.outputs.iter().find(|output| output.focused),
            "urgent" => self
                .outputs
                .iter()
                .find(|output| output.urgent_tags.is_some()),
            &_ => None,
        }
    }

    /// Get a mutable output matching the wloutput id. This is used to update state.
    pub fn get_output(&mut self, wloutput_id: &ObjectId) -> Option<&mut Output> {
        self.outputs
            .iter_mut()
            .find(|output| output.wloutput.id() == *wloutput_id)
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
                    state.seat = Some(Seat::new(registry.bind::<WlSeat, _, Self>(
                        name,
                        version,
                        queue_handle,
                        (),
                    )));
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZriverOutputStatusV1, ObjectId> for Flow {
    fn event(
        state: &mut Self,
        _: &ZriverOutputStatusV1,
        event: <ZriverOutputStatusV1 as Proxy>::Event,
        wloutput_id: &ObjectId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ViewTags { tags } => {
                if let Some(output) = state.get_output(wloutput_id) {
                    output.occupied_tags = tags;
                }
            }
            FocusedTags { tags } => {
                if let Some(output) = state.get_output(wloutput_id) {
                    output.focused_tags = Some(tags);
                }
            }
            UrgentTags { tags } => {
                // If urgent tags are not 0 (e.g. none are urgent), we add the tags
                if tags != 0 {
                    if let Some(output) = state.get_output(wloutput_id) {
                        output.urgent_tags = Some(tags);
                    }
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
            if let Some(output) = state.get_output(&output.id()) {
                output.focused = true;
            }
        }
    }
}

impl Dispatch<WlOutput, ()> for Flow {
    fn event(
        state: &mut Self,
        wloutput: &WlOutput,
        event: <WlOutput as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Name { name } = event {
            let output = Output::new(name, wloutput.to_owned());
            state.outputs.push(output);
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
