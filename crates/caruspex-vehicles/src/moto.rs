//! Motorcycles: OBD after Euro 5 through the maker's diagnostic plug.
//!
//! Motorcycles of the EU have OBD since Euro 4 (2016/2017, stage I) and must
//! speak the standard OBD services since Euro 5 (new types from 2020, every
//! new registration from 2021), over ISO 15765-4 (CAN) or ISO 14230-4
//! (KWP2000 on the K-line). Most makers keep their own plug for that, usually
//! under the seat; an adapter cable leads from it to a 16-pin socket, into
//! which the ELM327 or STN adapter goes. Older motorcycles often answer only
//! the maker's own protocol.
//!
//! The plug of each maker is described as the adapter cables for that maker
//! name it (number of pins, where the plug sits). There is no free list with
//! the plug of every model, so the texts say "usually" and the help asks to
//! look at the plug before buying a cable. The motorcycles themselves are rows
//! of the vehicle catalog (data/vehicles/moto.csv, body `moto`, connector
//! `moto`).

use caruspex_i18n::{Text, l};

use crate::catalog::{Connector, VehicleGeneration};

/// First model year in which every new motorcycle must meet Euro 5.
pub(crate) const EURO5_ALL: i32 = 2021;
/// First model year of new types under Euro 5.
pub(crate) const EURO5_NEW_TYPES: i32 = 2020;

/// The diagnostic plug of one maker and the cable that fits it.
#[derive(Clone, Copy, Debug)]
pub struct MotoPlug {
    /// Brands of the catalog this plug is for.
    pub brands: &'static [&'static str],
    /// Plug, where it sits and the adapter cable, one sentence each.
    pub text: Text,
}

const PLUGS: &[MotoPlug] = &[
    MotoPlug {
        brands: &["Honda"],
        text: Text::new(
            "Honda: 4-poliger Diagnosestecker, meist rot, unter dem Sitz. Adapterkabel Honda 4-polig auf OBD-II.",
            "Honda: 4-pin diagnostic plug, usually red, under the seat. Adapter cable Honda 4-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["Yamaha"],
        text: Text::new(
            "Yamaha: 3-poliger (ältere Modelle) oder 4-poliger Diagnosestecker unter dem Sitz. Adapterkabel Yamaha 3- oder 4-polig auf OBD-II.",
            "Yamaha: 3-pin (older models) or 4-pin diagnostic plug under the seat. Adapter cable Yamaha 3-pin or 4-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["Kawasaki"],
        text: Text::new(
            "Kawasaki: 6-poliger Diagnosestecker (ältere Modelle 4-polig) unter dem Sitz. Adapterkabel Kawasaki 6-polig auf OBD-II.",
            "Kawasaki: 6-pin diagnostic plug (older models 4-pin) under the seat. Adapter cable Kawasaki 6-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["Suzuki"],
        text: Text::new(
            "Suzuki: 6-poliger Diagnosestecker (SDS) unter dem Sitz. Adapterkabel Suzuki 6-polig auf OBD-II.",
            "Suzuki: 6-pin diagnostic plug (SDS) under the seat. Adapter cable Suzuki 6-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["KTM", "Husqvarna"],
        text: Text::new(
            "KTM und Husqvarna: 6-poliger Diagnosestecker unter dem Sitz. Adapterkabel KTM 6-polig auf OBD-II.",
            "KTM and Husqvarna: 6-pin diagnostic plug under the seat. Adapter cable KTM 6-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["BMW"],
        text: Text::new(
            "BMW Motorrad: runder 10-poliger Diagnosestecker, meist unter dem Sitz. Adapterkabel BMW 10-polig rund auf OBD-II; einige neuere Modelle haben einen anderen Stecker.",
            "BMW Motorrad: round 10-pin diagnostic plug, usually under the seat. Adapter cable BMW 10-pin round to OBD-II; some newer models have a different plug.",
        ),
    },
    MotoPlug {
        brands: &["Ducati"],
        text: Text::new(
            "Ducati: 4-poliger Diagnosestecker unter dem Sitz, neuere Modelle teils mit eigenem Stecker. Das Adapterkabel nach dem Stecker am Motorrad wählen.",
            "Ducati: 4-pin diagnostic plug under the seat, newer models partly with a plug of their own. Choose the adapter cable by the plug on the motorcycle.",
        ),
    },
    MotoPlug {
        brands: &["Triumph"],
        text: Text::new(
            "Triumph: 6-poliger Diagnosestecker unter dem Sitz. Adapterkabel Triumph 6-polig auf OBD-II.",
            "Triumph: 6-pin diagnostic plug under the seat. Adapter cable Triumph 6-pin to OBD-II.",
        ),
    },
    MotoPlug {
        brands: &["Harley-Davidson"],
        text: Text::new(
            "Harley-Davidson: 4-poliger Deutsch-Stecker bis etwa 2013, 6-poliger ab 2014, meist hinter der Seitenverkleidung. Adapterkabel Deutsch 4- oder 6-polig auf OBD-II.",
            "Harley-Davidson: 4-pin Deutsch plug up to about 2013, 6-pin from 2014, usually behind the side cover. Adapter cable Deutsch 4-pin or 6-pin to OBD-II.",
        ),
    },
];

const OTHER: Text = Text::new(
    "Diagnosestecker des Herstellers, meist unter dem Sitz. Manche Motorräder haben die 16-polige Buchse, sonst braucht es das passende Adapterkabel.",
    "The maker's diagnostic plug, usually under the seat. Some motorcycles have the 16-pin socket, otherwise it takes the matching adapter cable.",
);

/// Whether the generation is a motorcycle of the catalog.
pub fn is_motorcycle(g: &VehicleGeneration) -> bool {
    g.connector == Connector::Moto || g.late_connector == Connector::Moto
}

/// The plug of a maker, if Caruspex knows it.
pub(crate) fn moto_plug(brand: &str) -> Option<&'static MotoPlug> {
    PLUGS
        .iter()
        .find(|p| p.brands.iter().any(|b| b.eq_ignore_ascii_case(brand)))
}

/// Plug, place and cable for a motorcycle brand, in the active language.
pub fn plug_text(brand: &str) -> &'static str {
    moto_plug(brand).map_or_else(|| OTHER.get(), |p| p.text.get())
}

/// Whether a motorcycle of this generation and model year falls under
/// Euro 5 and so speaks the standard OBD services.
pub(crate) fn euro5(g: &VehicleGeneration, year: i32) -> bool {
    let year = if year <= 0 { g.year_from } else { year };
    year >= EURO5_ALL || (year >= EURO5_NEW_TYPES && g.year_from >= EURO5_NEW_TYPES)
}

/// The diagnosis sentence of a motorcycle: what to expect, then the plug.
pub fn diagnosis_text(g: &VehicleGeneration, year: i32) -> String {
    let mut out = if euro5(g, year) {
        l(
            "Motorrad nach Euro 5: Es spricht die genormte OBD-Diagnose über ISO 15765 (CAN) oder ISO 14230 (K-Line). Mit dem Adapterkabel am Diagnosestecker liest ein ELM327 Fehlercodes und Messwerte.",
            "Motorcycle under Euro 5: it speaks the standard OBD diagnosis over ISO 15765 (CAN) or ISO 14230 (K-line). With the adapter cable on the diagnostic plug an ELM327 reads trouble codes and live values.",
        )
    } else {
        l(
            "Motorrad vor Euro 5: Viele sprechen nur das Protokoll des Herstellers. Mit dem Adapterkabel antworten manche trotzdem über ISO 14230 oder ISO 15765, andere nur dem Werkstattgerät des Herstellers.",
            "Motorcycle before Euro 5: many speak only the maker's protocol. With the adapter cable some answer over ISO 14230 or ISO 15765 anyway, others only the maker's workshop tool.",
        )
    }
    .to_string();
    out.push(' ');
    out.push_str(plug_text(g.brand));
    out
}

/// The steps of the connect help for a motorcycle, in the active language.
pub fn connect_steps() -> Vec<&'static str> {
    vec![
        l(
            "Das Adapterkabel auf den Diagnosestecker des Motorrads stecken und den OBD-Adapter in dessen 16-polige Buchse.",
            "Plug the adapter cable onto the motorcycle's diagnostic plug and the OBD adapter into the cable's 16-pin socket.",
        ),
        l(
            "Zündung an und den Not-Aus-Schalter auf Betrieb. Manche Modelle wollen dazu den Leerlauf oder den eingeklappten Seitenständer.",
            "Ignition on and the kill switch on run. Some models also want neutral or the side stand up.",
        ),
        l(
            "Antwortet nichts: Vor Euro 5 sprechen viele Motorräder nur das Protokoll des Herstellers, das ein ELM327 nicht versteht.",
            "If nothing answers: before Euro 5 many motorcycles speak only the maker's protocol, which an ELM327 does not understand.",
        ),
        l(
            "Den Motor für Live-Werte nur im Freien laufen lassen und das Motorrad sicher abstellen.",
            "Run the engine for live values only outdoors, with the motorcycle parked safely.",
        ),
    ]
}
