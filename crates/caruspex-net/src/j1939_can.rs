//! SAE J1939 straight on a raw CAN interface (SLCAN, SocketCAN, J2534,
//! PCAN, Kvaser): every 29-bit frame on the bus goes to the J1939 session,
//! frames from the session go out as they are. Open the interface at 250
//! kbit/s for the black type 1 socket, at 500 kbit/s for the green type 2.

use caruspex_link::timing::now_ms;
use caruspex_link::{BusFrame, FrameLink};

use crate::can_bus::{CanFrame, CanLink, SharedCanLink};

/// A [`FrameLink`] on a raw CAN interface. Frames with 11-bit ids (other
/// traffic on the same wires) are left out.
pub struct J1939Can {
    link: SharedCanLink,
}

impl J1939Can {
    pub fn new(link: SharedCanLink) -> Self {
        J1939Can { link }
    }
}

impl FrameLink for J1939Can {
    fn exchange(
        &mut self,
        send: Option<&BusFrame>,
        listen_ms: u64,
        seen: &mut dyn FnMut(&BusFrame) -> bool,
    ) -> Result<(), String> {
        if let Some(frame) = send {
            let out = CanFrame {
                id: frame.id & 0x1FFF_FFFF,
                extended: true,
                data: frame.data.clone(),
            };
            if !self.link.send(&out) {
                return Err(caruspex_i18n::l(
                    "Die CAN-Schnittstelle nimmt den Rahmen nicht an.",
                    "The CAN interface does not take the frame.",
                )
                .to_string());
            }
        }
        let deadline = now_ms() + listen_ms;
        loop {
            let now = now_ms();
            if now >= deadline {
                return Ok(());
            }
            let Some(frame) = self.link.receive((deadline - now).min(50)) else {
                continue;
            };
            if !frame.extended || frame.data.len() > 8 {
                continue;
            }
            let bus = BusFrame {
                id: frame.id,
                data: frame.data,
            };
            if seen(&bus) {
                return Ok(());
            }
        }
    }

    fn name(&self) -> String {
        self.link.name()
    }
}
