//! The connection assistant: when connecting fails, a short checklist for
//! the kind of connection instead of a single error line.
//!
//! The worker records what the attempt found out (was there a port at all,
//! did it open, did the adapter answer, which protocols were tried, what did
//! the adapter report from the bus). From that each point of the list is
//! either the likely problem, worth a look, or already known to be fine.

use caruspex_i18n::l;

use crate::text::args;

/// The way the computer talks to the adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum LinkKind {
    /// USB cable or a serial (COM) port, also Bluetooth paired as a COM port.
    #[default]
    Usb,
    /// Bluetooth LE or classic Bluetooth found by Caruspex itself.
    Bluetooth,
    /// A Wi-Fi adapter over TCP.
    Wifi,
    /// BMW ENET or DoIP over a network cable.
    Network,
    /// A raw CAN interface (J2534, SLCAN, SocketCAN).
    Can,
    /// A recording: nothing to check at the car.
    Replay,
}

/// What one connection attempt found out. `None` means: not got that far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectFacts {
    pub kind: LinkKind,
    /// Ports on the computer (USB search), `None` when not looked.
    pub ports_found: Option<usize>,
    /// Bluetooth: an adapter was found nearby.
    pub device_found: Option<bool>,
    /// The port, socket, channel or interface opened.
    pub port_opened: Option<bool>,
    /// Another program holds the port.
    pub port_busy: bool,
    /// The adapter answered the reset (ATZ).
    pub adapter_answered: Option<bool>,
    /// Its name, e.g. "ELM327 v1.5".
    pub adapter_id: String,
    /// Protocol numbers the adapter tried, as the ELM327 counts them.
    pub protocols_tried: String,
    /// The last error the adapter reported from the bus ("CAN ERROR").
    pub bus_error: String,
    /// Baud rates tried on the serial port.
    pub bauds_tried: Vec<u32>,
    /// CAN bit rate in bit/s, 0 if none.
    pub can_bit_rate: u32,
}

/// How one point of the checklist stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CheckState {
    /// Most likely the reason.
    Problem,
    /// Worth a look.
    Check,
    /// Known to be fine from this attempt.
    Fine,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckItem {
    /// Stable id, e.g. "driver" or "ignition".
    pub id: &'static str,
    pub state: CheckState,
    pub title: String,
    /// What to do; empty for points that are fine.
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConnectHelp {
    /// What the attempt found out, one plain line each.
    pub found: Vec<String>,
    /// Problems first, then the points to check, then what is fine.
    pub items: Vec<CheckItem>,
}

/// "CAN, KWP2000 (K-Line)" for the protocol numbers "6754".
pub fn protocol_families(numbers: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for c in numbers.chars() {
        let name = match c {
            '1' | '2' => "SAE J1850",
            '3' => "ISO 9141 (K-Line)",
            '4' | '5' => "KWP2000 (K-Line)",
            '6'..='9' => "CAN",
            'A' | 'B' | 'C' => "CAN (J1939)",
            _ => continue,
        };
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out.join(", ")
}

/// Adapters that call themselves v1.5 are clones (ELM Electronics never
/// released that version); an id without ELM, OBD or STN is odd too.
pub fn looks_like_clone(adapter_id: &str) -> bool {
    let up = adapter_id.to_ascii_uppercase();
    if up.trim().is_empty() || up.trim() == "?" {
        return false;
    }
    up.contains("V1.5") || !(up.contains("ELM") || up.contains("OBD") || up.contains("STN"))
}

fn check(id: &'static str, state: CheckState, title: &str, text: impl Into<String>) -> CheckItem {
    let text = if state == CheckState::Fine {
        String::new()
    } else {
        text.into()
    };
    CheckItem {
        id,
        state,
        title: title.to_string(),
        text,
    }
}

fn found_lines(f: &ConnectFacts) -> Vec<String> {
    let mut out = Vec::new();
    match f.ports_found {
        Some(0) => out.push(
            l(
                "Am Rechner ist kein Anschluss zu sehen.",
                "No port shows up on this computer.",
            )
            .to_string(),
        ),
        Some(n) => out.push(args(
            l("Anschlüsse am Rechner: %1", "Ports on this computer: %1"),
            &[&n.to_string()],
        )),
        None => {}
    }
    if f.device_found == Some(false) {
        out.push(
            l(
                "Kein Bluetooth-Adapter in der Nähe gefunden.",
                "No Bluetooth adapter found nearby.",
            )
            .to_string(),
        );
    }
    match f.port_opened {
        Some(false) if f.port_busy => out.push(
            l(
                "Der Anschluss ist von einem anderen Programm belegt.",
                "Another program is using the port.",
            )
            .to_string(),
        ),
        Some(false) => out.push(
            l(
                "Die Verbindung zum Adapter ließ sich nicht öffnen.",
                "The connection to the adapter could not be opened.",
            )
            .to_string(),
        ),
        Some(true) => out.push(
            l(
                "Die Verbindung zum Adapter ist offen.",
                "The connection to the adapter is open.",
            )
            .to_string(),
        ),
        None => {}
    }
    match f.adapter_answered {
        Some(true) => out.push(args(
            l("Der Adapter antwortet: %1", "The adapter answers: %1"),
            &[&f.adapter_id],
        )),
        Some(false) => out.push(
            l(
                "Der Adapter antwortet nicht.",
                "The adapter does not answer.",
            )
            .to_string(),
        ),
        None => {}
    }
    let families = protocol_families(&f.protocols_tried);
    if !families.is_empty() {
        out.push(args(
            l(
                "Probierte Protokolle: %1. Kein Steuergerät hat geantwortet.",
                "Protocols tried: %1. No control unit answered.",
            ),
            &[&families],
        ));
    }
    if !f.bus_error.is_empty() {
        out.push(args(
            l("Der Adapter meldet: %1", "The adapter reports: %1"),
            &[&f.bus_error],
        ));
    }
    out
}

/// The points between the adapter and the car, the same for every adapter
/// that speaks ELM327 (USB, Bluetooth, Wi-Fi).
fn adapter_and_car(f: &ConnectFacts, out: &mut Vec<CheckItem>) {
    let answered = f.adapter_answered == Some(true);
    if f.port_opened == Some(true) {
        out.push(check(
            "power",
            if answered {
                CheckState::Fine
            } else {
                CheckState::Problem
            },
            l("Adapter hat Strom", "Adapter has power"),
            l(
                "Die meisten Adapter leuchten, sobald sie in der Buchse stecken. Kein Licht: die Sicherung der Diagnosebuchse prüfen, oft ist es die des Zigarettenanzünders.",
                "Most adapters light up as soon as they sit in the socket. No light: check the fuse of the diagnostic socket, often the one of the cigarette lighter.",
            ),
        ));
    }
    if answered && looks_like_clone(&f.adapter_id) {
        out.push(check(
            "clone",
            CheckState::Check,
            l("Nachbau-Adapter", "Clone adapter"),
            args(
                l(
                    "Der Adapter nennt sich %1. Das steht fast nur auf billigen Nachbauten, die manche Protokolle nicht beherrschen; ein anderer Adapter hilft oft.",
                    "The adapter calls itself %1. That is found almost only on cheap clones that cannot handle some protocols; another adapter often helps.",
                ),
                &[&f.adapter_id],
            ),
        ));
    }
    if !f.bus_error.is_empty() {
        out.push(check(
            "bus",
            CheckState::Problem,
            l("Meldung vom Fahrzeugbus", "Message from the vehicle bus"),
            args(
                l(
                    "Der Adapter meldet %1. Das heißt meist: Zündung aus, falsches Protokoll oder ein Problem an der Leitung zum Auto.",
                    "The adapter reports %1. That usually means: ignition off, wrong protocol or a problem on the line to the car.",
                ),
                &[&f.bus_error],
            ),
        ));
    }
    out.push(check(
        "ignition",
        if answered {
            CheckState::Problem
        } else {
            CheckState::Check
        },
        l("Zündung an", "Ignition on"),
        l(
            "Ohne Zündung antwortet kein Steuergerät. Zündung einschalten, der Motor darf aus sein.",
            "Without the ignition no control unit answers. Switch the ignition on, the engine may stay off.",
        ),
    ));
    out.push(check(
        "plug",
        CheckState::Check,
        l("Stecker sitzt fest", "Plug firmly seated"),
        l(
            "Den Adapter abziehen, 10 Sekunden warten und fest in die OBD-Buchse stecken.",
            "Unplug the adapter, wait 10 seconds and push it firmly into the OBD socket.",
        ),
    ));
    let families = protocol_families(&f.protocols_tried);
    if answered && !families.is_empty() {
        out.push(check(
            "protocol",
            CheckState::Check,
            l("Protokoll", "Protocol"),
            args(
                l(
                    "Probiert: %1. Auf dieser Seite das Modell auswählen, dann probiert Caruspex zuerst das passende Protokoll; Autos vor etwa 2005 sprechen oft K-Line.",
                    "Tried: %1. Pick the model on this page, then Caruspex tries the matching protocol first; cars from before about 2005 often speak K-Line.",
                ),
                &[&families],
            ),
        ));
    }
}

/// The checklist for one failed attempt. Empty for a recording, which has
/// nothing to check at the car.
pub fn connect_help(f: &ConnectFacts) -> ConnectHelp {
    let mut items: Vec<CheckItem> = Vec::new();
    let opened = f.port_opened == Some(true);
    let answered = f.adapter_answered == Some(true);
    match f.kind {
        LinkKind::Replay => return ConnectHelp::default(),
        LinkKind::Usb => {
            let no_port = f.ports_found == Some(0);
            items.push(check(
                "driver",
                if no_port {
                    CheckState::Problem
                } else if opened || f.ports_found.is_some() {
                    CheckState::Fine
                } else {
                    CheckState::Check
                },
                l("Treiber", "Driver"),
                l(
                    "Ohne Treiber zeigt der Rechner keinen Anschluss. Meist fehlt der Treiber für den Chip im Kabel (CH340, FTDI oder CP210x), oder das USB-Kabel ist nur ein Ladekabel.",
                    "Without a driver the computer shows no port. Usually the driver for the chip in the cable is missing (CH340, FTDI or CP210x), or the USB cable is a charging cable only.",
                ),
            ));
            if !no_port {
                items.push(check(
                    "port",
                    if f.port_busy {
                        CheckState::Problem
                    } else if opened {
                        CheckState::Fine
                    } else {
                        CheckState::Check
                    },
                    l("Anschluss frei", "Port free"),
                    if f.port_busy {
                        l(
                            "Ein anderes Programm hält den Anschluss, etwa eine andere Diagnose-App oder ein zweites Caruspex. Schließen und noch einmal verbinden.",
                            "Another program holds the port, such as another diagnostic app or a second Caruspex. Close it and connect again.",
                        )
                    } else {
                        l(
                            "Steckt der Adapter noch, und ist es der richtige Anschluss? Im Geräte-Manager unter Anschlüsse steht, welcher zum Adapter gehört.",
                            "Is the adapter still plugged in, and is it the right port? Device Manager lists under Ports which one belongs to the adapter.",
                        )
                    },
                ));
            }
            if opened {
                let bauds: Vec<String> = f.bauds_tried.iter().map(u32::to_string).collect();
                items.push(check(
                    "baud",
                    if answered {
                        CheckState::Fine
                    } else {
                        CheckState::Check
                    },
                    l("Baudrate", "Baud rate"),
                    args(
                        l(
                            "Probiert: %1 Baud. Manche Adapter brauchen eine feste Rate, einstellbar in den Einstellungen unter Verbindung.",
                            "Tried: %1 baud. Some adapters need a fixed rate, which can be set in Settings under Connection.",
                        ),
                        &[&bauds.join(", ")],
                    ),
                ));
            }
            if opened || f.port_opened.is_none() {
                adapter_and_car(f, &mut items);
            }
        }
        LinkKind::Bluetooth => {
            items.push(check(
                "bluetooth",
                if f.device_found == Some(true) || opened {
                    CheckState::Fine
                } else {
                    CheckState::Check
                },
                l("Bluetooth am Rechner", "Bluetooth on this computer"),
                l(
                    "Bluetooth muss am Rechner eingeschaltet sein.",
                    "Bluetooth must be switched on on this computer.",
                ),
            ));
            items.push(check(
                "pairing",
                if opened {
                    CheckState::Fine
                } else {
                    CheckState::Problem
                },
                l("Kopplung", "Pairing"),
                if f.device_found == Some(false) {
                    l(
                        "Kein Adapter in der Nähe. Bluetooth-LE-Adapter brauchen keine Kopplung; klassische einmal in den Systemeinstellungen koppeln, die PIN ist meist 1234 oder 0000. Manche Adapter haben nur bei Zündung an Strom.",
                        "No adapter nearby. Bluetooth LE adapters need no pairing; pair classic ones once in the system settings, the PIN is usually 1234 or 0000. Some adapters only have power with the ignition on.",
                    )
                } else {
                    l(
                        "Der Adapter ließ keine Verbindung zu. Ist er noch mit einem Handy verbunden? Ein Adapter spricht nur mit einem Gerät.",
                        "The adapter refused the connection. Is it still connected to a phone? An adapter talks to one device only.",
                    )
                },
            ));
            if opened {
                adapter_and_car(f, &mut items);
            } else {
                items.push(check(
                    "ignition",
                    CheckState::Check,
                    l("Zündung an", "Ignition on"),
                    l(
                        "Ohne Zündung antwortet kein Steuergerät. Zündung einschalten, der Motor darf aus sein.",
                        "Without the ignition no control unit answers. Switch the ignition on, the engine may stay off.",
                    ),
                ));
            }
        }
        LinkKind::Wifi => {
            items.push(check(
                "wifi",
                if opened {
                    CheckState::Fine
                } else {
                    CheckState::Problem
                },
                l("WLAN des Adapters", "The adapter's Wi-Fi"),
                l(
                    "Der Rechner muss mit dem WLAN des Adapters verbunden sein (oft WiFi_OBDII), nicht mit dem Heimnetz.",
                    "The computer must be connected to the adapter's Wi-Fi (often WiFi_OBDII), not to the home network.",
                ),
            ));
            if !opened {
                items.push(check(
                    "address",
                    CheckState::Check,
                    l("Adresse und Port", "Address and port"),
                    l(
                        "Die meisten Adapter nutzen 192.168.0.10 und Port 35000, manche 192.168.4.1. Das steht in der Anleitung des Adapters.",
                        "Most adapters use 192.168.0.10 and port 35000, some 192.168.4.1. The adapter's manual says which.",
                    ),
                ));
            }
            adapter_and_car(f, &mut items);
        }
        LinkKind::Network => {
            items.push(check(
                "cable",
                CheckState::Check,
                l("Netzwerkkabel", "Network cable"),
                l(
                    "Ein ENET-Kabel (Ethernet zu OBD) braucht die Brücke für die Aktivierung; ein einfaches Netzwerkkabel ohne sie geht nicht.",
                    "An ENET cable (Ethernet to OBD) needs the activation bridge; a plain network cable without it does not work.",
                ),
            ));
            items.push(check(
                "address",
                CheckState::Check,
                l("Netzwerkadresse", "Network address"),
                l(
                    "Der Rechner sollte seine Adresse automatisch beziehen. Antwortet das Auto nicht darauf, vergibt der Rechner nach etwa einer Minute selbst eine; dann noch einmal verbinden.",
                    "The computer should obtain its address automatically. If the car does not answer, the computer assigns one itself after about a minute; then connect again.",
                ),
            ));
            items.push(check(
                "firewall",
                CheckState::Check,
                l("Firewall", "Firewall"),
                l(
                    "Die Firewall muss die Suche nach dem Auto zulassen (UDP 6811 für ENET, 13400 für DoIP).",
                    "The firewall must allow the search for the car (UDP 6811 for ENET, 13400 for DoIP).",
                ),
            ));
            items.push(check(
                "ignition",
                CheckState::Check,
                l("Zündung an", "Ignition on"),
                l(
                    "Ohne Zündung antwortet kein Steuergerät. Zündung einschalten, der Motor darf aus sein.",
                    "Without the ignition no control unit answers. Switch the ignition on, the engine may stay off.",
                ),
            ));
        }
        LinkKind::Can => {
            items.push(check(
                "driver",
                if opened {
                    CheckState::Fine
                } else {
                    CheckState::Problem
                },
                l("Treiber", "Driver"),
                l(
                    "Das CAN-Interface ließ sich nicht öffnen. Ist es eingesteckt und der Treiber installiert? Viele J2534-Treiber gibt es nur in 32 Bit, Caruspex braucht 64 Bit.",
                    "The CAN interface could not be opened. Is it plugged in and the driver installed? Many J2534 drivers exist in 32 bit only, Caruspex needs 64 bit.",
                ),
            ));
            if opened {
                if !f.bus_error.is_empty() {
                    items.push(check(
                        "bus",
                        CheckState::Problem,
                        l("Meldung vom Fahrzeugbus", "Message from the vehicle bus"),
                        args(
                            l(
                                "Der Adapter meldet %1. Das heißt meist: Zündung aus, falsches Protokoll oder ein Problem an der Leitung zum Auto.",
                                "The adapter reports %1. That usually means: ignition off, wrong protocol or a problem on the line to the car.",
                            ),
                            &[&f.bus_error],
                        ),
                    ));
                }
                items.push(check(
                    "bitrate",
                    CheckState::Check,
                    l("Bitrate", "Bit rate"),
                    args(
                        l(
                            "Probiert: %1 kbit/s. Fast alle Autos ab 2008 nutzen 500 kbit/s an der OBD-Buchse, manche ältere 250 kbit/s.",
                            "Tried: %1 kbit/s. Almost all cars from 2008 use 500 kbit/s at the OBD socket, some older ones 250 kbit/s.",
                        ),
                        &[&(f.can_bit_rate / 1000).to_string()],
                    ),
                ));
                items.push(check(
                    "ignition",
                    CheckState::Problem,
                    l("Zündung an", "Ignition on"),
                    l(
                        "Ohne Zündung antwortet kein Steuergerät. Zündung einschalten, der Motor darf aus sein.",
                        "Without the ignition no control unit answers. Switch the ignition on, the engine may stay off.",
                    ),
                ));
            }
        }
    }
    let rank = |s: CheckState| match s {
        CheckState::Problem => 0,
        CheckState::Check => 1,
        CheckState::Fine => 2,
    };
    items.sort_by_key(|i| rank(i.state));
    ConnectHelp {
        found: found_lines(f),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::german;

    fn ids(help: &ConnectHelp, state: CheckState) -> Vec<&'static str> {
        help.items
            .iter()
            .filter(|i| i.state == state)
            .map(|i| i.id)
            .collect()
    }

    #[test]
    fn no_port_at_all_points_to_the_driver() {
        let _lang = german();
        let help = connect_help(&ConnectFacts {
            kind: LinkKind::Usb,
            ports_found: Some(0),
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&help, CheckState::Problem), ["driver"]);
        assert_eq!(help.items[0].id, "driver");
        assert!(help.items[0].text.contains("CH340"));
        assert_eq!(help.found, ["Am Rechner ist kein Anschluss zu sehen."]);
    }

    #[test]
    fn a_busy_port_is_named() {
        let _lang = german();
        let help = connect_help(&ConnectFacts {
            kind: LinkKind::Usb,
            port_opened: Some(false),
            port_busy: true,
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&help, CheckState::Problem), ["port"]);
        assert!(help.items[0].text.contains("anderes Programm"));
    }

    #[test]
    fn a_silent_adapter_asks_for_power_and_baud_rate() {
        let _lang = german();
        let help = connect_help(&ConnectFacts {
            kind: LinkKind::Usb,
            port_opened: Some(true),
            adapter_answered: Some(false),
            bauds_tried: vec![38400, 9600],
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&help, CheckState::Problem), ["power"]);
        let baud = help.items.iter().find(|i| i.id == "baud").unwrap();
        assert_eq!(baud.state, CheckState::Check);
        assert!(baud.text.contains("38400, 9600"));
        assert!(ids(&help, CheckState::Fine).contains(&"port"));
    }

    #[test]
    fn adapter_answers_but_the_car_does_not() {
        let _lang = german();
        let help = connect_help(&ConnectFacts {
            kind: LinkKind::Usb,
            port_opened: Some(true),
            adapter_answered: Some(true),
            adapter_id: "ELM327 v1.5".into(),
            protocols_tried: "54312678".into(),
            bus_error: "UNABLE TO CONNECT".into(),
            bauds_tried: vec![38400],
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&help, CheckState::Problem), ["bus", "ignition"]);
        assert!(ids(&help, CheckState::Check).contains(&"clone"));
        assert!(ids(&help, CheckState::Fine).contains(&"power"));
        let protocol = help.items.iter().find(|i| i.id == "protocol").unwrap();
        assert!(
            protocol
                .text
                .contains("KWP2000 (K-Line), ISO 9141 (K-Line), SAE J1850, CAN")
        );
        assert!(help.found.iter().any(|l| l.contains("ELM327 v1.5")));
    }

    #[test]
    fn bluetooth_wifi_network_and_can_have_their_own_points() {
        let _lang = german();
        let bt = connect_help(&ConnectFacts {
            kind: LinkKind::Bluetooth,
            device_found: Some(false),
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&bt, CheckState::Problem), ["pairing"]);
        assert!(bt.items[0].text.contains("1234"));

        let wifi = connect_help(&ConnectFacts {
            kind: LinkKind::Wifi,
            port_opened: Some(false),
            ..ConnectFacts::default()
        });
        assert_eq!(ids(&wifi, CheckState::Problem), ["wifi"]);
        assert!(ids(&wifi, CheckState::Check).contains(&"address"));

        let enet = connect_help(&ConnectFacts {
            kind: LinkKind::Network,
            ..ConnectFacts::default()
        });
        assert_eq!(
            ids(&enet, CheckState::Check),
            ["cable", "address", "firewall", "ignition"]
        );

        let can = connect_help(&ConnectFacts {
            kind: LinkKind::Can,
            port_opened: Some(true),
            can_bit_rate: 250000,
            ..ConnectFacts::default()
        });
        let rate = can.items.iter().find(|i| i.id == "bitrate").unwrap();
        assert!(rate.text.contains("250 kbit/s"));
        assert!(
            connect_help(&ConnectFacts {
                kind: LinkKind::Replay,
                ..ConnectFacts::default()
            })
            .items
            .is_empty()
        );
    }

    #[test]
    fn clone_and_protocol_names() {
        assert!(looks_like_clone("ELM327 v1.5"));
        assert!(!looks_like_clone("ELM327 v2.2"));
        assert!(!looks_like_clone("OBDLink SX r4.2"));
        assert!(looks_like_clone("Interpreter v1.0"));
        assert!(!looks_like_clone(""));
        assert_eq!(protocol_families("6789"), "CAN");
        assert_eq!(protocol_families("x"), "");
    }
}
