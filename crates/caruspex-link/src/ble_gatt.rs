//! Bluetooth LE through a GATT connection that the system holds open for
//! us: on Android the Kotlin plugin with BluetoothGatt, where btleplug would
//! need its own Java classes. The native side connects and lists the
//! services; this side picks the channel and decides whether the device is
//! an OBD adapter at all ([`plan_channel`]). Only then does the native side
//! switch notifications on and ask for a larger MTU. From there on it
//! writes single values and hands the notifications over.
//!
//! Everything here is free of any Bluetooth library, so it is tested with
//! fakes: the property bits, the plan, cutting writes to the MTU and the
//! [`Transport`] the ELM327 driver reads.

use caruspex_i18n::l;

use crate::ble_profile::{
    BleCharacteristicDesc, BleServiceDesc, choose_ble_channel, is_obd_adapter,
};
use crate::transport::Transport;

/// The property bits of a characteristic as Android reports them
/// (BluetoothGattCharacteristic.PROPERTY_*), the same as in the
/// Bluetooth core specification.
pub const PROPERTY_WRITE_NO_RESPONSE: u32 = 0x04;
pub const PROPERTY_WRITE: u32 = 0x08;
pub const PROPERTY_NOTIFY: u32 = 0x10;
pub const PROPERTY_INDICATE: u32 = 0x20;

/// A characteristic described by its property bits.
pub fn characteristic(uuid: &str, properties: u32) -> BleCharacteristicDesc {
    BleCharacteristicDesc {
        uuid: uuid.to_string(),
        notify: properties & PROPERTY_NOTIFY != 0,
        indicate: properties & PROPERTY_INDICATE != 0,
        write: properties & PROPERTY_WRITE != 0,
        write_no_response: properties & PROPERTY_WRITE_NO_RESPONSE != 0,
    }
}

/// The smallest MTU every device supports.
pub const DEFAULT_MTU: u32 = 23;
/// What we ask for: the most Android allows.
pub const WANTED_MTU: u32 = 517;

/// How many bytes fit into one write at this MTU: the ATT header takes 3,
/// and no attribute value is longer than 512. Below the minimum MTU (a
/// stack that reported nothing) the 20 bytes of the minimum are used.
pub fn payload_size(mtu: u32) -> usize {
    (mtu.saturating_sub(3) as usize).clamp(20, 512)
}

/// What to switch on, on a connected device: indices into the service list
/// that was passed to [`plan_channel`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GattPlan {
    pub service: usize,
    pub notify: usize,
    pub write: usize,
    pub write_without_response: bool,
    /// The answer characteristic only indicates (each value confirmed)
    /// instead of notifying; the descriptor gets the other value then.
    pub indicate: bool,
    /// "FFF0", "vLinker", ... for the log.
    pub profile: String,
}

/// The channel to use on a device with this name and these services, or why
/// nothing may be sent to it. Devices that are not recognised as OBD
/// adapters are refused here, before anything is written to them, not even
/// the descriptor that switches notifications on.
pub fn plan_channel(name: &str, services: &[BleServiceDesc]) -> Result<GattPlan, String> {
    let channel = choose_ble_channel(services);
    let (Some(service), Some(notify), Some(write)) = (
        channel.service,
        channel.notify_characteristic,
        channel.write_characteristic,
    ) else {
        return Err(l(
            "Auf diesem Gerät wurde kein serieller Kanal für OBD gefunden. Ist es ein OBD-Adapter mit Bluetooth LE?",
            "No serial channel for OBD was found on this device. Is it an OBD adapter with Bluetooth LE?",
        )
        .to_string());
    };
    if !is_obd_adapter(name, &channel) {
        return Err(l(
            "Das Gerät sieht nicht wie ein OBD-Adapter aus. An unbekannte Geräte sendet Caruspex nichts.",
            "The device does not look like an OBD adapter. Caruspex sends nothing to unknown devices.",
        )
        .to_string());
    }
    let answers = &services[service].characteristics[notify];
    Ok(GattPlan {
        service,
        notify,
        write,
        write_without_response: channel.write_without_response,
        indicate: !answers.notify && answers.indicate,
        profile: channel.profile,
    })
}

/// The native half of an open, set up GATT connection.
pub trait GattLink: Send {
    /// One write to the write characteristic, at most the payload size long.
    fn write_value(&mut self, value: &[u8]) -> bool;
    /// The notifications received since the last call, joined in order;
    /// waits up to `timeout_ms` when there are none yet.
    fn notifications(&mut self, timeout_ms: u64) -> Vec<u8>;
    /// Drops notifications that were received and not read yet.
    fn clear(&mut self);
    /// The device has gone away.
    fn closed(&self) -> bool;
}

/// An ELM327 behind a Bluetooth LE serial channel, on a [`GattLink`].
pub struct GattTransport<L: GattLink> {
    link: L,
    chunk: usize,
    name: String,
}

impl<L: GattLink> GattTransport<L> {
    /// `mtu` is what the device agreed to.
    pub fn new(link: L, mtu: u32, name: String) -> Self {
        Self {
            link,
            chunk: payload_size(mtu),
            name,
        }
    }

    /// Bytes per write.
    pub fn chunk(&self) -> usize {
        self.chunk
    }

    pub fn link(&self) -> &L {
        &self.link
    }
}

impl<L: GattLink> Transport for GattTransport<L> {
    fn write(&mut self, data: &[u8]) -> bool {
        if self.link.closed() {
            return false;
        }
        // An ELM327 collects the characters up to the CR itself, so a
        // command may arrive in several pieces.
        data.chunks(self.chunk)
            .all(|piece| self.link.write_value(piece))
    }

    fn read(&mut self, timeout_ms: u64) -> Vec<u8> {
        self.link.notifications(timeout_ms)
    }

    fn flush_input(&mut self) {
        self.link.clear();
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Default)]
    struct FakeLink {
        written: Vec<Vec<u8>>,
        incoming: VecDeque<Vec<u8>>,
        fail_after: Option<usize>,
        gone: bool,
    }

    impl GattLink for FakeLink {
        fn write_value(&mut self, value: &[u8]) -> bool {
            if self.fail_after.is_some_and(|n| self.written.len() >= n) {
                return false;
            }
            self.written.push(value.to_vec());
            true
        }
        fn notifications(&mut self, _timeout_ms: u64) -> Vec<u8> {
            self.incoming.drain(..).flatten().collect()
        }
        fn clear(&mut self) {
            self.incoming.clear();
        }
        fn closed(&self) -> bool {
            self.gone
        }
    }

    fn svc(uuid: &str, chars: &[(&str, u32)]) -> BleServiceDesc {
        BleServiceDesc {
            uuid: uuid.into(),
            characteristics: chars.iter().map(|(u, p)| characteristic(u, *p)).collect(),
        }
    }

    #[test]
    fn property_bits() {
        let c = characteristic("fff1", PROPERTY_NOTIFY | 0x02);
        assert!(c.notify && !c.indicate && !c.write && !c.write_no_response);
        let c = characteristic("fff2", PROPERTY_WRITE | PROPERTY_WRITE_NO_RESPONSE);
        assert!(c.write && c.write_no_response && !c.notify);
        assert!(characteristic("x", PROPERTY_INDICATE).indicate);
    }

    #[test]
    fn the_payload_follows_the_mtu() {
        assert_eq!(payload_size(DEFAULT_MTU), 20);
        assert_eq!(payload_size(185), 182);
        assert_eq!(payload_size(WANTED_MTU), 512);
        assert_eq!(payload_size(0), 20);
        assert_eq!(payload_size(5000), 512);
    }

    #[test]
    fn writes_are_cut_to_the_mtu() {
        let mut t = GattTransport::new(FakeLink::default(), 23, "x".into());
        let line = b"STCAFCP 7E0,7E8,7DF,7E9,7E1\r";
        assert!(t.write(line));
        let pieces = &t.link().written;
        assert_eq!(pieces.len(), 2);
        assert_eq!(pieces[0].len(), 20);
        assert_eq!(pieces.concat(), line);

        let mut big = GattTransport::new(FakeLink::default(), 247, "x".into());
        assert!(big.write(line));
        assert_eq!(big.link().written.len(), 1);
    }

    #[test]
    fn a_failed_piece_fails_the_write_and_stops() {
        let link = FakeLink {
            fail_after: Some(1),
            ..FakeLink::default()
        };
        let mut t = GattTransport::new(link, 23, "x".into());
        assert!(!t.write(&[b'A'; 50]));
        assert_eq!(t.link().written.len(), 1);
    }

    #[test]
    fn nothing_goes_out_after_the_device_left() {
        let link = FakeLink {
            gone: true,
            ..FakeLink::default()
        };
        let mut t = GattTransport::new(link, 23, "x".into());
        assert!(!t.write(b"ATZ\r"));
        assert!(t.link().written.is_empty());
    }

    #[test]
    fn notifications_become_one_stream_and_flush_drops_them() {
        let mut link = FakeLink::default();
        link.incoming.push_back(b"ELM327 v1".to_vec());
        link.incoming.push_back(b".5\r\r>".to_vec());
        let mut t = GattTransport::new(link, 23, "x".into());
        assert_eq!(t.read(100), b"ELM327 v1.5\r\r>");
        t.link.incoming.push_back(b"stale".to_vec());
        t.flush_input();
        assert!(t.read(10).is_empty());
    }

    #[test]
    fn the_plan_of_a_vgate_with_fff0() {
        let services = vec![
            svc("1800", &[("2a00", 0x02)]),
            svc(
                "0000fff0-0000-1000-8000-00805f9b34fb",
                &[
                    ("0000fff1-0000-1000-8000-00805f9b34fb", PROPERTY_NOTIFY),
                    (
                        "0000fff2-0000-1000-8000-00805f9b34fb",
                        PROPERTY_WRITE | PROPERTY_WRITE_NO_RESPONSE,
                    ),
                ],
            ),
        ];
        let plan = plan_channel("IOS-Vlink", &services).expect("an adapter");
        assert_eq!(
            plan,
            GattPlan {
                service: 1,
                notify: 0,
                write: 1,
                write_without_response: true,
                indicate: false,
                profile: "FFF0".into(),
            }
        );
    }

    #[test]
    fn an_indicating_adapter_gets_indications() {
        let services = vec![svc(
            "e7810a71-73ae-499d-8c15-faa9aef0c3f2",
            &[(
                "bef8d6c9-9c21-4c9e-b632-bd58c1009f9f",
                PROPERTY_INDICATE | PROPERTY_WRITE,
            )],
        )];
        // OBDLink CX: the vendor service alone makes it an adapter.
        let plan = plan_channel("", &services).expect("an adapter");
        assert!(plan.indicate);
        assert!(!plan.write_without_response);
        assert_eq!((plan.notify, plan.write), (0, 0));
        assert_eq!(plan.profile, "E781");
    }

    #[test]
    fn a_lamp_with_the_same_module_is_refused() {
        let lamp = vec![svc(
            "fff0",
            &[
                ("fff1", PROPERTY_NOTIFY),
                ("fff2", PROPERTY_WRITE_NO_RESPONSE),
            ],
        )];
        caruspex_i18n::with_language(caruspex_i18n::Language::English, || {
            let refused = plan_channel("Govee_H6008_1A2B", &lamp).unwrap_err();
            assert!(refused.contains("does not look like an OBD adapter"));
            let nothing = plan_channel("OBDII", &[svc("180f", &[("2a19", PROPERTY_NOTIFY)])]);
            assert!(nothing.unwrap_err().contains("No serial channel"));
        });
    }
}
