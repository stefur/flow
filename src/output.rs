use wayland_client::protocol::wl_output::WlOutput;

use crate::protocols::river_protocols::zriver_output_status_v1::ZriverOutputStatusV1;

#[derive(Debug)]
pub struct Output {
    pub name: String,
    pub wloutput: WlOutput,
    pub status: Option<ZriverOutputStatusV1>,
    pub focused: bool,
    pub urgent_tags: Option<u32>,
    pub focused_tags: Option<u32>,
    pub occupied_tags: Vec<u8>,
}

impl Output {
    /// Set up state for an output
    pub fn new(name: String, wloutput: WlOutput) -> Self {
        Self {
            name,
            wloutput,
            status: None,
            focused: false,
            urgent_tags: None,
            focused_tags: None,
            occupied_tags: vec![],
        }
    }
    /// Cycle the tagmask in either next or previous direction
    pub fn cycle_tags(&self, direction: &str, n_tags: &u8, skip_unoccupied: bool) -> u32 {
        let tags: u32 = self.focused_tags.unwrap_or_default();
        let mut new_tags: u32 = tags;

        let occupied_tags = self.find_set_bits_positions(*n_tags);

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

        new_tags
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
    fn find_set_bits_positions(&self, n_tags: u8) -> Vec<u8> {
        let mut result = Vec::new();

        // Iterate over 4-byte chunks of the occupied tags
        for chunk in self.occupied_tags.chunks(4) {
            // Then iterate over bytes in the current chunk, along with their indices
            for (byte_index, &byte) in chunk.iter().enumerate() {
                // And iterate over bits in the current byte
                for bit_index in 0..8 {
                    // Check if the bit at bit_index is set = occupied tag
                    if (byte & (1 << bit_index)) != 0 {
                        // If set, calculate the overall bit index
                        let index = (byte_index * 8 + bit_index) as u8;
                        // Only keep it if it's within the number of tags
                        if index < n_tags {
                            result.push(index);
                        }
                    }
                }
            }
        }

        result
    }
    /// Checks if the requested tags are already focused
    pub fn toggle_tags(&self, to_tags: &u32) -> bool {
        self.focused_tags == Some(*to_tags)
    }
}
