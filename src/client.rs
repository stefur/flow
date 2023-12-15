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
        Event::{FocusedTags, UrgentTags, ViewTags},
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
    pub urgent: HashMap<OutputName, u32>,
    pub control: Option<ZriverControlV1>,
    pub occupied_tags: Vec<u8>,
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
            occupied_tags: vec![],
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
        if let Some(manager) = self.status_manager.take() {
            manager.destroy()
        };

        self.output_status
            .iter()
            .for_each(|output_status| output_status.destroy());

        if let Some(status) = self.seat_status.take() {
            status.destroy()
        };

        if let Some(control) = self.control.take() {
            control.destroy()
        };
    }

    /// Checks if the requested tags are already focused
    pub fn toggle_tags(&self, to_tags: &u32) -> bool {
        self.focused_tags == Some(*to_tags)
    }

    /// Find the next occupied tag in the direction
    fn find_next_occupied(
        &self,
        occupied_tags: &[u8],
        direction: &str,
        tag_index: u8,
    ) -> Option<u8> {
        let mut result: Option<u8> = None;

        // Go over each tag in the occupied tags
        for &occupied_tag_index in occupied_tags.iter() {
            // If we cycle next the tag index should be greater than the one we're looking up.
            // Otherwise it should be smaller.
            let condition = if direction == "next" {
                occupied_tag_index > tag_index
            } else {
                occupied_tag_index < tag_index
            };

            // Then find the closest occupied tag according to direction by using min/max function on each tag and replace the value.
            // This could be None, which is fine, and will be unhandled with an unwrap to a default tag position.
            if condition {
                result = Some(result.map_or(occupied_tag_index, |next_occupied| {
                    if direction == "next" {
                        u8::min(next_occupied, occupied_tag_index)
                    } else {
                        u8::max(next_occupied, occupied_tag_index)
                    }
                }));
            }
        }

        result
    }

    /// Find the indices of set bits
    fn find_set_bits_positions(&self) -> Vec<u8> {
        let mut result = Vec::new();

        // Iterate over 4-byte chunks of the occupied tags
        for chunk in self.occupied_tags.chunks(4) {
            // Then iterate over bytes in the current chunk, along with their indices
            for (byte_index, &byte) in chunk.iter().enumerate() {
                // And iterate over bits in the current byte
                for bit_index in 0..8 {
                    // Check if the bit at bit_index is set = occupied tag
                    if (byte & (1 << bit_index)) != 0 {
                        // If set, calculate the overall bit index and push it to the result vector
                        result.push((byte_index * 8 + bit_index) as u8);
                    }
                }
            }
        }

        result
    }

    /// Cycle the tagmask in either next or previous direction
    pub fn cycle_tags(
        &self,
        direction: &str,
        n_tags: &u8,
        skip_unoccupied: bool,
        queue_handle: &QueueHandle<Self>,
    ) {
        let tags: u32 = self.focused_tags.unwrap_or_default();
        let mut new_tags: u32 = tags;

        let occupied_tags = self.find_set_bits_positions();

        match direction {
            // Only skip unoccupied on user flag and if there are more than one occupied tag
            "next" | "previous" if skip_unoccupied && occupied_tags.len() > 1 => {
                let mut old_bits: Vec<u8> = Vec::new();
                let mut new_bits: Vec<u8> = Vec::new();

                let wrap_around = if direction == "next" { 0 } else { n_tags - 1 };

                for tag_index in (0..*n_tags).filter(|&i| (tags >> i) & 1 == 1) {
                    // Find the next occupied position
                    let mut next_occupied = self
                        .find_next_occupied(&occupied_tags, direction, tag_index)
                        .unwrap_or(wrap_around);

                    // Handle the cases where we hit the wrap_around and need to go again to find the next occupied
                    if next_occupied == wrap_around && !occupied_tags.contains(&wrap_around) {
                        next_occupied = self
                            .find_next_occupied(&occupied_tags, direction, next_occupied)
                            .unwrap_or(wrap_around);
                    }

                    // Set the next occupied tag
                    new_tags |= 1 << next_occupied;

                    // Add each old and new bit to vector
                    old_bits.push(tag_index);
                    new_bits.push(next_occupied);
                }

                // Go over the old bits and unset those that are no longer part of the tagmask
                for bit in old_bits {
                    if !new_bits.contains(&bit) {
                        new_tags &= !(1 << bit);
                    }
                }
            }

            "next" => {
                let mut tags = tags;
                new_tags = 0;
                let last_tag: u32 = 1 << (n_tags - 1);

                if tags & last_tag != 0 {
                    tags ^= last_tag;
                    new_tags = 1;
                }

                new_tags |= tags << 1;
            }
            "previous" => {
                let mut tags = tags;
                new_tags = 0;
                let last_tag: u32 = 1 << (n_tags - 1);

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
            ViewTags { tags } => {
                state.occupied_tags = tags;
            }
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
