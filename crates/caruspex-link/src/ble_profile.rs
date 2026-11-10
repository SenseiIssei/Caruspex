//! Bluetooth LE adapters carry the ELM327 text protocol over two GATT
//! characteristics: one the computer writes to, one that notifies the answers.
//! Makers use different services for it. This picks the pair from what a
//! device offers, without any Bluetooth library, so it can be tested.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleCharacteristicDesc {
    /// Any form: "fff1", "0xFFF1", "{0000fff1-...}"
    pub uuid: String,
    pub notify: bool,
    pub indicate: bool,
    pub write: bool,
    pub write_no_response: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleServiceDesc {
    pub uuid: String,
    pub characteristics: Vec<BleCharacteristicDesc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleChannel {
    /// Index into the list that was passed in.
    pub service: Option<usize>,
    pub notify_characteristic: Option<usize>,
    pub write_characteristic: Option<usize>,
    pub write_without_response: bool,
    /// "FFF0", "FFE0", "vLinker", ... or "generic"
    pub profile: String,
}

impl BleChannel {
    pub fn valid(&self) -> bool {
        self.service.is_some()
            && self.notify_characteristic.is_some()
            && self.write_characteristic.is_some()
    }
}

const BASE_SUFFIX: &str = "-0000-1000-8000-00805f9b34fb";

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn short_uuid(hex16: &str) -> String {
    format!("0000{hex16}{BASE_SUFFIX}")
}

struct KnownProfile {
    name: &'static str,
    service: String,
    /// Empty: any notifying characteristic of the service.
    notify: String,
    /// Empty: any writable characteristic of the service.
    write: String,
}

// Collected from adapters on the market and the apps that talk to them.
fn known_profiles() -> Vec<KnownProfile> {
    let p = |name, service: String, notify: String, write: String| KnownProfile {
        name,
        service,
        notify,
        write,
    };
    vec![
        // Most ELM327 BLE clones, Vgate iCar Pro BLE, Veepeak OBDCheck BLE
        p(
            "FFF0",
            short_uuid("fff0"),
            short_uuid("fff1"),
            short_uuid("fff2"),
        ),
        // Vgate vLinker MC+, FS and BM+ in BLE mode
        p(
            "vLinker",
            short_uuid("18f0"),
            short_uuid("2af0"),
            short_uuid("2af1"),
        ),
        // HM-10 style modules, one characteristic for both directions
        p(
            "FFE0",
            short_uuid("ffe0"),
            short_uuid("ffe1"),
            short_uuid("ffe1"),
        ),
        // OBDLink CX, LELink and others with this vendor service
        p(
            "E781",
            "e7810a71-73ae-499d-8c15-faa9aef0c3f2".into(),
            "bef8d6c9-9c21-4c9e-b632-bd58c1009f9f".into(),
            "bef8d6c9-9c21-4c9e-b632-bd58c1009f9f".into(),
        ),
        // Nordic UART service
        p(
            "NUS",
            "6e400001-b5a3-f393-e0a9-e50e24dcca9e".into(),
            "6e400003-b5a3-f393-e0a9-e50e24dcca9e".into(),
            "6e400002-b5a3-f393-e0a9-e50e24dcca9e".into(),
        ),
    ]
}

fn is_standard_service(uuid: &str) -> bool {
    ["1800", "1801", "180a", "180f", "1805", "180d", "fe59"]
        .iter()
        .any(|s| uuid == short_uuid(s))
}

fn can_notify(c: &BleCharacteristicDesc) -> bool {
    c.notify || c.indicate
}

fn can_write(c: &BleCharacteristicDesc) -> bool {
    c.write || c.write_no_response
}

// Picks the characteristics inside one service; the wanted UUIDs win, any
// suitable one is the fallback.
fn pick_in_service(
    service: &BleServiceDesc,
    index: usize,
    notify_uuid: &str,
    write_uuid: &str,
) -> BleChannel {
    let mut ch = BleChannel {
        service: Some(index),
        ..BleChannel::default()
    };
    let chars = &service.characteristics;
    for (i, c) in chars.iter().enumerate() {
        let u = normalize_ble_uuid(&c.uuid);
        if ch.notify_characteristic.is_none()
            && can_notify(c)
            && !notify_uuid.is_empty()
            && u == notify_uuid
        {
            ch.notify_characteristic = Some(i);
        }
        if ch.write_characteristic.is_none()
            && can_write(c)
            && !write_uuid.is_empty()
            && u == write_uuid
        {
            ch.write_characteristic = Some(i);
        }
    }
    if ch.notify_characteristic.is_none() {
        ch.notify_characteristic = chars.iter().position(can_notify);
    }
    // A separate write characteristic is preferred over writing to the
    // notifying one, unless that is the only choice.
    if ch.write_characteristic.is_none() {
        ch.write_characteristic = chars
            .iter()
            .enumerate()
            .position(|(i, c)| can_write(c) && Some(i) != ch.notify_characteristic);
    }
    if ch.write_characteristic.is_none()
        && let Some(n) = ch.notify_characteristic
        && can_write(&chars[n])
    {
        ch.write_characteristic = Some(n);
    }
    if let Some(w) = ch.write_characteristic {
        ch.write_without_response = chars[w].write_no_response;
    }
    ch
}

/// Full lower-case 128-bit form without braces; 16- and 32-bit short forms
/// are expanded with the Bluetooth base UUID. Empty for garbage.
pub fn normalize_ble_uuid(uuid: &str) -> String {
    let mut s: String = uuid
        .chars()
        .filter(|&c| c != '{' && c != '}' && !c.is_ascii_whitespace() && c != '\x0B')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if let Some(rest) = s.strip_prefix("0x") {
        s = rest.to_string();
    }
    if s.len() == 4 && is_hex(&s) {
        return format!("0000{s}{BASE_SUFFIX}");
    }
    if s.len() == 8 && is_hex(&s) {
        return format!("{s}{BASE_SUFFIX}");
    }
    let b = s.as_bytes();
    if b.len() == 36 && b[8] == b'-' && b[13] == b'-' && b[18] == b'-' && b[23] == b'-' {
        let digits: String = s.chars().filter(|&c| c != '-').collect();
        if digits.len() == 32 && is_hex(&digits) {
            return s;
        }
    }
    String::new()
}

/// The best serial channel on the device: known adapter profiles first, then
/// any vendor service with a notify and a write characteristic. Standard
/// services (generic access, device information, battery) are never used.
pub fn choose_ble_channel(services: &[BleServiceDesc]) -> BleChannel {
    for profile in known_profiles() {
        for (i, service) in services.iter().enumerate() {
            if normalize_ble_uuid(&service.uuid) != profile.service {
                continue;
            }
            let mut ch = pick_in_service(service, i, &profile.notify, &profile.write);
            if ch.valid() {
                ch.profile = profile.name.to_string();
                return ch;
            }
        }
    }
    for (i, service) in services.iter().enumerate() {
        let u = normalize_ble_uuid(&service.uuid);
        if u.is_empty() || is_standard_service(&u) {
            continue;
        }
        let mut ch = pick_in_service(service, i, "", "");
        if ch.valid() {
            ch.profile = "generic".to_string();
            return ch;
        }
    }
    BleChannel::default()
}

/// How much a device name looks like an OBD adapter, 0 = not at all. Used to
/// sort the scan list and to pick the device when searching automatically.
pub fn obd_adapter_name_score(name: &str) -> i32 {
    let n = name.to_ascii_lowercase();
    if n.is_empty() {
        return 0;
    }
    // Strong hints first: names that only OBD adapters use.
    const STRONG: [&str; 16] = [
        "obd",
        "elm327",
        "elm 327",
        "vlink",
        "v-link",
        "vgate",
        "icar",
        "obdlink",
        "veepeak",
        "konnwei",
        "kiwi",
        "lelink",
        "carista",
        "vlinker",
        "scan tool",
        "obdii",
    ];
    if STRONG.iter().any(|s| n.contains(s)) {
        return 3;
    }
    const WEAK: [&str; 6] = ["elm", "car", "diag", "can", "ble", "spp"];
    if WEAK.iter().any(|w| n.contains(w)) {
        return 1;
    }
    0
}

/// Whether a device may be written to at all. A serial channel alone proves
/// nothing: lamps, sensors and toys use the same FFF0, FFE0 or Nordic UART
/// modules as cheap adapters, and an ATZ sent to them may switch or
/// reconfigure them. So the device has to look like an OBD adapter by its
/// name, or offer one of the vendor services only OBD adapters use (vLinker,
/// OBDLink/LELink). Nothing, not even switching on notifications, goes to
/// any other device.
pub fn is_obd_adapter(name: &str, channel: &BleChannel) -> bool {
    channel.valid()
        && (obd_adapter_name_score(name) >= 3
            || channel.profile == "vLinker"
            || channel.profile == "E781")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chr(uuid: &str, notify: bool, write: bool, write_no_resp: bool) -> BleCharacteristicDesc {
        BleCharacteristicDesc {
            uuid: uuid.to_string(),
            notify,
            write,
            write_no_response: write_no_resp,
            ..Default::default()
        }
    }

    fn svc(uuid: &str, characteristics: Vec<BleCharacteristicDesc>) -> BleServiceDesc {
        BleServiceDesc {
            uuid: uuid.to_string(),
            characteristics,
        }
    }

    fn device_info() -> BleServiceDesc {
        svc(
            "0x180A",
            vec![
                chr("2a29", false, false, false),
                chr("2a24", false, false, false),
            ],
        )
    }

    #[test]
    fn ble_uuids_are_normalised() {
        assert_eq!(
            normalize_ble_uuid("fff0"),
            "0000fff0-0000-1000-8000-00805f9b34fb"
        );
        assert_eq!(
            normalize_ble_uuid("0xFFF0"),
            "0000fff0-0000-1000-8000-00805f9b34fb"
        );
        assert_eq!(
            normalize_ble_uuid("{0000FFF0-0000-1000-8000-00805F9B34FB}"),
            "0000fff0-0000-1000-8000-00805f9b34fb"
        );
        assert_eq!(
            normalize_ble_uuid("E7810A71-73AE-499D-8C15-FAA9AEF0C3F2"),
            "e7810a71-73ae-499d-8c15-faa9aef0c3f2"
        );
        assert!(normalize_ble_uuid("nonsense").is_empty());
        assert!(normalize_ble_uuid("").is_empty());
    }

    #[test]
    fn the_common_fff0_adapter_uses_fff1_for_answers_and_fff2_for_commands() {
        let services = vec![
            device_info(),
            svc(
                "fff0",
                vec![
                    chr("fff1", true, false, false),
                    chr("fff2", false, true, true),
                ],
            ),
        ];
        let ch = choose_ble_channel(&services);
        assert!(ch.valid());
        assert_eq!(ch.profile, "FFF0");
        assert_eq!(ch.service, Some(1));
        assert_eq!(ch.notify_characteristic, Some(0));
        assert_eq!(ch.write_characteristic, Some(1));
        assert!(ch.write_without_response);
    }

    #[test]
    fn vlinker_and_hm10_style_adapters_are_recognised() {
        let vlinker = vec![svc(
            "18f0",
            vec![
                chr("2af1", false, true, false),
                chr("2af0", true, false, false),
            ],
        )];
        let ch = choose_ble_channel(&vlinker);
        assert!(ch.valid());
        assert_eq!(ch.profile, "vLinker");
        assert_eq!(ch.notify_characteristic, Some(1));
        assert_eq!(ch.write_characteristic, Some(0));
        assert!(!ch.write_without_response);

        // One characteristic for both directions.
        let hm10 = vec![svc("ffe0", vec![chr("ffe1", true, true, true)])];
        let ch = choose_ble_channel(&hm10);
        assert!(ch.valid());
        assert_eq!(ch.profile, "FFE0");
        assert_eq!(ch.notify_characteristic, Some(0));
        assert_eq!(ch.write_characteristic, Some(0));
    }

    #[test]
    fn a_known_profile_wins_over_an_unknown_vendor_service() {
        let services = vec![
            svc(
                "12345678-1234-1234-1234-123456789abc",
                vec![chr("aaaa", true, true, false)],
            ),
            svc(
                "6e400001-b5a3-f393-e0a9-e50e24dcca9e",
                vec![
                    chr("6e400002-b5a3-f393-e0a9-e50e24dcca9e", false, true, false),
                    chr("6e400003-b5a3-f393-e0a9-e50e24dcca9e", true, false, false),
                ],
            ),
        ];
        let ch = choose_ble_channel(&services);
        assert!(ch.valid());
        assert_eq!(ch.profile, "NUS");
        assert_eq!(ch.service, Some(1));
        assert_eq!(ch.notify_characteristic, Some(1));
        assert_eq!(ch.write_characteristic, Some(0));
    }

    #[test]
    fn unknown_vendor_services_work_as_long_as_they_can_notify_and_be_written() {
        let services = vec![
            device_info(),
            svc("0x1801", vec![chr("2a05", true, false, false)]),
            svc(
                "abcdef01-0000-4000-8000-000000000001",
                vec![
                    chr("abcdef02-0000-4000-8000-000000000001", true, false, false),
                    chr("abcdef03-0000-4000-8000-000000000001", false, false, true),
                ],
            ),
        ];
        let ch = choose_ble_channel(&services);
        assert!(ch.valid());
        assert_eq!(ch.profile, "generic");
        assert_eq!(ch.service, Some(2));
        assert!(ch.write_without_response);
    }

    #[test]
    fn devices_with_only_standard_services_give_no_channel() {
        assert!(
            !choose_ble_channel(&[
                device_info(),
                svc("1800", vec![chr("2a00", false, true, false)])
            ])
            .valid()
        );
        assert!(!choose_ble_channel(&[]).valid());
        // A known service without a notifying characteristic is not usable.
        assert!(!choose_ble_channel(&[svc("fff0", vec![chr("fff2", false, true, false)])]).valid());
    }

    #[test]
    fn adapter_names_are_ranked() {
        assert_eq!(obd_adapter_name_score("OBDII"), 3);
        assert_eq!(obd_adapter_name_score("IOS-Vlink"), 3);
        assert_eq!(obd_adapter_name_score("vLinker MC-IOS"), 3);
        assert_eq!(obd_adapter_name_score("OBDLink CX"), 3);
        assert_eq!(obd_adapter_name_score("Android-Vlink"), 3);
        assert_eq!(obd_adapter_name_score("V-LINK"), 3);
        assert_eq!(obd_adapter_name_score("Veepeak"), 3);
        assert_eq!(obd_adapter_name_score("BLE Device"), 1);
        assert_eq!(obd_adapter_name_score("Jakobs Kopfhörer"), 0);
        assert_eq!(obd_adapter_name_score(""), 0);
    }

    #[test]
    fn only_adapters_may_be_written_to() {
        let fff0 = choose_ble_channel(&[svc(
            "fff0",
            vec![
                chr("fff1", true, false, false),
                chr("fff2", false, true, true),
            ],
        )]);
        assert!(is_obd_adapter("OBDII", &fff0));
        // A lamp with the same serial module stays untouched.
        assert!(!is_obd_adapter("Govee_H6008_1A2B", &fff0));
        assert!(!is_obd_adapter("ihoment_H6159", &fff0));
        assert!(!is_obd_adapter("BLE Device", &fff0));
        assert!(!is_obd_adapter("", &fff0));
        // A vendor service of its own (Govee) gives "generic": not an adapter.
        let govee = choose_ble_channel(&[svc(
            "00010203-0405-0607-0809-0a0b0c0d1910",
            vec![
                chr("00010203-0405-0607-0809-0a0b0c0d2b10", true, false, false),
                chr("00010203-0405-0607-0809-0a0b0c0d2b11", false, true, true),
            ],
        )]);
        assert_eq!(govee.profile, "generic");
        assert!(!is_obd_adapter("Govee_H6008_1A2B", &govee));
        // Services only OBD adapters use count even without a name.
        let vlinker = choose_ble_channel(&[svc(
            "18f0",
            vec![
                chr("2af1", false, true, false),
                chr("2af0", true, false, false),
            ],
        )]);
        assert!(is_obd_adapter("", &vlinker));
        // Without a channel nothing is an adapter.
        assert!(!is_obd_adapter("OBDII", &BleChannel::default()));
    }
}
