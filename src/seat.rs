use crate::protocols::river_protocols::zriver_seat_status_v1::ZriverSeatStatusV1;
use wayland_client::protocol::wl_seat::WlSeat;

#[derive(Debug)]
pub struct Seat {
    pub wlseat: WlSeat,
    pub seat_status: Option<ZriverSeatStatusV1>,
}

impl Seat {
    pub fn new(wlseat: WlSeat) -> Self {
        Self {
            wlseat,
            seat_status: None,
        }
    }
}
