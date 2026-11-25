//! Trucks and buses: diagnosis over SAE J1939 instead of OBD-II.
//!
//! Heavy vehicles put every control unit on a J1939 bus (29-bit CAN). In
//! North America the diagnostic socket is the round 9-pin Deutsch plug of
//! SAE J1939-13: the black type 1 runs at 250 kbit/s, the green type 2
//! (from about 2016) at 500 kbit/s and also takes the older black plug.
//! European trucks since Euro 6 mostly have the 16-pin socket of the cars,
//! with J1939 on its CAN pins, and 24 V on pin 16.
//!
//! Vans and light commercial vehicles (Mercedes Sprinter, Ford Transit,
//! VW Crafter, Fiat Ducato, Renault Master and the like) are cars as far
//! as diagnosis goes: they speak normal OBD-II on the 16-pin socket.

use caruspex_i18n::l;

use crate::catalog::{BodyType, Connector, VehicleGeneration, connector_for};

/// True when the generation is diagnosed over J1939 (a truck or bus, or
/// anything with the 9-pin socket).
pub fn speaks_j1939(g: &VehicleGeneration, year: i32) -> bool {
    g.body == BodyType::Truck || connector_for(g, year) == Connector::J1939
}

/// The two 9-pin sockets and the cable, in a few sentences.
pub fn plug_text() -> &'static str {
    l(
        "Runde 9-polige Diagnosebuchse (Deutsch, SAE J1939-13) im Fahrerhaus, meist unter dem Armaturenbrett auf der Fahrerseite. Schwarze Buchse (Typ 1): 250 kbit/s. Grüne Buchse (Typ 2, ab etwa 2016): 500 kbit/s, sie nimmt auch den schwarzen Stecker. Ein Adapterkabel von 9 Polen auf eine 16-polige OBD-Kupplung verbindet den Adapter.",
        "Round 9-pin diagnostic socket (Deutsch, SAE J1939-13) in the cab, usually under the dashboard on the driver side. Black socket (type 1): 250 kbit/s. Green socket (type 2, from about 2016): 500 kbit/s, it also takes the black plug. An adapter cable from 9 pins to a 16-pin OBD female connects the adapter.",
    )
}

/// The warning every truck user needs: 24 V.
pub(crate) fn voltage_text() -> &'static str {
    l(
        "Lkw haben meist 24 V. Viele billige ELM327-Adapter sind nur für 12 V gebaut und gehen daran kaputt: nur einen Adapter nehmen, der 24 V ausdrücklich verträgt.",
        "Trucks mostly run on 24 V. Many cheap ELM327 adapters are built for 12 V only and break on it: use only an adapter that is rated for 24 V.",
    )
}

/// The diagnosis sentence of a truck.
pub fn diagnosis_text(g: &VehicleGeneration, year: i32) -> String {
    let mut out = l(
        "Lkw mit SAE J1939: Caruspex hört die Werte mit, die die Steuergeräte ständig senden, liest aktive und frühere Fehler (DM1, DM2) mit SPN, FMI und Warnlampen, die Fahrgestellnummer und die Kennungen der Steuergeräte.",
        "Truck with SAE J1939: Caruspex listens to the values the control units send all the time, reads active and previous faults (DM1, DM2) with SPN, FMI and warning lamps, the VIN and the identification of the control units.",
    )
    .to_string();
    out.push(' ');
    if connector_for(g, year) == Connector::J1939 {
        out.push_str(plug_text());
    } else {
        out.push_str(l(
            "Die Diagnose läuft über die 16-polige Buchse, J1939 liegt dort auf den CAN-Pins.",
            "Diagnosis runs through the 16-pin socket, with J1939 on its CAN pins.",
        ));
    }
    out.push(' ');
    out.push_str(voltage_text());
    out
}

/// Where the 9-pin socket sits.
pub fn location_text() -> &'static str {
    l(
        "Im Fahrerhaus, meist unter dem Armaturenbrett auf der Fahrerseite oder hinter einer Abdeckung am Knie des Fahrers.",
        "In the cab, usually under the dashboard on the driver side or behind a cover at the driver's knee.",
    )
}
