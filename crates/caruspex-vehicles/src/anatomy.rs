//! The shape of a car for the 3D X-ray view: outer dimensions and where the
//! engine, the driven axles, the motors and the batteries sit.
//!
//! Nothing here comes from construction data (that belongs to the makers).
//! The dimensions are typical values per body type and era, which is close
//! enough for a recognisable model; the drivetrain follows a short rule:
//!
//! - A pure electric car has its battery in the floor and one motor on the
//!   axle its platform drives (rear for Tesla, BMW, the Volkswagen ID family
//!   and the like, otherwise front), two motors for large electric SUVs and
//!   the dual motor Teslas.
//! - A combustion engine sits in front and across (transverse) with front
//!   wheel drive, the layout of most cars since the 1980s.
//! - A short table lists the exceptions: brands and models with the engine
//!   along the car (BMW, Mercedes outside the compact class, Audi from the
//!   A4 upwards, Jaguar, Land Rover, Subaru's boxer, pick-ups and so on),
//!   mid engines (Boxster, Cayman, most Ferraris, R8, Elise) and rear
//!   engines (911, Beetle, Smart Fortwo, Twingo III).
//! - A hybrid keeps that layout and adds a motor at the gearbox and a small
//!   battery under the rear seat; a plug-in hybrid a larger one.
//!
//! Real per-generation dimensions are the next step: once the catalog rows
//! carry length, width, height and wheelbase (from Wikidata, CC0), they are
//! returned by [`dimensions_for`] with [`DimensionSource::Catalog`].

use crate::catalog::{
    BodyType, PT_DIESEL, PT_ELECTRIC, PT_GAS, PT_HYBRID, PT_PETROL, PT_PLUG_IN, VehicleGeneration,
};
use caruspex_i18n::l;

/// Outer dimensions in millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimensions {
    pub length: u32,
    pub width: u32,
    pub height: u32,
    pub wheelbase: u32,
    /// Distance between the centres of the left and right wheels.
    pub track: u32,
    /// Outer diameter of a wheel with its tyre.
    pub wheel: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DimensionSource {
    /// Typical values for the body type and era.
    Typical,
    /// Measured values of this generation (not filled yet, see the module docs).
    Catalog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineLayout {
    /// No combustion engine.
    None,
    FrontTransverse,
    FrontLongitudinal,
    /// Between the cabin and the rear axle.
    Mid,
    /// Behind the rear axle.
    Rear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrivenAxles {
    Front,
    Rear,
    All,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TractionBattery {
    None,
    /// Hybrid without a plug: a small pack under the rear seat.
    Hybrid,
    /// Plug-in hybrid: a larger pack under the rear seat and boot floor.
    PlugIn,
    /// Electric car: the whole floor between the axles.
    Floor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drivetrain {
    pub engine: EngineLayout,
    pub driven: DrivenAxles,
    pub front_motor: bool,
    pub rear_motor: bool,
    pub battery: TractionBattery,
    /// Petrol, diesel or gas tank.
    pub fuel_tank: bool,
}

// --- Dimensions ------------------------------------------------------------------

/// Length, width, height, wheelbase, track of a car from about 1995 to 2009.
fn base_dimensions(body: BodyType) -> [u32; 5] {
    match body {
        BodyType::Hatchback => [4120, 1720, 1470, 2540, 1490],
        BodyType::Sedan => [4620, 1770, 1440, 2710, 1530],
        BodyType::Estate => [4600, 1770, 1490, 2690, 1530],
        BodyType::Suv => [4520, 1850, 1690, 2670, 1580],
        BodyType::Van => [4900, 1910, 1950, 3000, 1630],
        BodyType::Coupe => [4450, 1790, 1360, 2620, 1530],
        BodyType::Convertible => [4380, 1770, 1390, 2580, 1510],
        BodyType::Pickup => [5250, 1860, 1790, 3090, 1570],
        BodyType::Mpv => [4560, 1800, 1640, 2760, 1540],
        BodyType::Microcar => [2700, 1560, 1550, 1870, 1360],
        BodyType::Roadster => [4020, 1740, 1270, 2420, 1480],
    }
}

/// Typical outer dimensions of a car of this body type and year. Cars grew
/// over the decades, so older ones come out a little smaller and newer ones
/// a little larger, with bigger wheels.
pub fn typical_dimensions(body: BodyType, year: i32) -> Dimensions {
    let [length, width, height, wheelbase, track] = base_dimensions(body);
    // Per mille: length, width, wheelbase and track; height stays.
    let (grow, wide, wheel) = if year > 0 && year < 1995 {
        (960, 950, 590)
    } else if year >= 2010 {
        (1030, 1035, 670)
    } else {
        (1000, 1000, 630)
    };
    let wheel = match body {
        BodyType::Suv | BodyType::Pickup => wheel + 80,
        BodyType::Van => wheel + 30,
        BodyType::Microcar => wheel - 60,
        _ => wheel,
    };
    let scale = |v: u32, per_mille: u32| (v * per_mille + 500) / 1000;
    Dimensions {
        length: scale(length, grow),
        width: scale(width, wide),
        height,
        wheelbase: scale(wheelbase, grow),
        track: scale(track, wide),
        wheel,
    }
}

/// The dimensions for one generation. Measured values per generation are
/// not in the catalog yet; this is the place where they will be returned
/// once `data/vehicles` carries them (Wikidata, checked by the data check).
pub fn dimensions_for(g: &VehicleGeneration, year: i32) -> (Dimensions, DimensionSource) {
    let year = if year > 0 { year } else { g.year_from };
    (typical_dimensions(g.body, year), DimensionSource::Typical)
}

pub fn dimension_source_text(source: DimensionSource) -> &'static str {
    match source {
        DimensionSource::Typical => l(
            "Typische Maße für diesen Aufbau und dieses Baujahr, nicht die genauen des Modells.",
            "Typical dimensions for this body type and year, not the exact ones of the model.",
        ),
        DimensionSource::Catalog => l("Maße dieses Modells.", "Dimensions of this model."),
    }
}

// --- Drivetrain -------------------------------------------------------------------

/// The engine variant the model shows by default: the first combustion
/// engine the generation has, electric only when there is no other.
pub fn default_powertrain(powertrains: u32) -> u32 {
    for flag in [
        PT_PETROL,
        PT_DIESEL,
        PT_GAS,
        PT_HYBRID,
        PT_PLUG_IN,
        PT_ELECTRIC,
    ] {
        if powertrains & flag != 0 {
            return flag;
        }
    }
    PT_PETROL
}

/// Every single variant of a flag set, in the order of [`default_powertrain`].
pub fn powertrain_variants(powertrains: u32) -> Vec<u32> {
    [
        PT_PETROL,
        PT_DIESEL,
        PT_GAS,
        PT_HYBRID,
        PT_PLUG_IN,
        PT_ELECTRIC,
    ]
    .into_iter()
    .filter(|flag| powertrains & flag != 0)
    .collect()
}

struct Exception {
    brand: &'static str,
    /// Prefix of the model name, "" for every model of the brand.
    model: &'static str,
    from: i32,
    to: i32,
    engine: EngineLayout,
    driven: DrivenAxles,
}

const fn ex(
    brand: &'static str,
    model: &'static str,
    engine: EngineLayout,
    driven: DrivenAxles,
) -> Exception {
    Exception {
        brand,
        model,
        from: 0,
        to: 9999,
        engine,
        driven,
    }
}

const fn ex_years(
    brand: &'static str,
    model: &'static str,
    from: i32,
    to: i32,
    engine: EngineLayout,
    driven: DrivenAxles,
) -> Exception {
    Exception {
        brand,
        model,
        from,
        to,
        engine,
        driven,
    }
}

use DrivenAxles::{All, Front, Rear};
use EngineLayout::{FrontLongitudinal as Long, FrontTransverse as Across, Mid, Rear as Back};

/// The first matching row wins, so the specific rows come before the rule
/// for the whole brand.
const EXCEPTIONS: &[Exception] = &[
    // Rear and mid engines.
    ex("Porsche", "911", Back, Rear),
    ex("Porsche", "718", Mid, Rear),
    ex("Porsche", "Boxster", Mid, Rear),
    ex("Porsche", "Cayman", Mid, Rear),
    ex("Porsche", "Carrera GT", Mid, Rear),
    ex("Porsche", "918", Mid, All),
    ex("Porsche", "Cayenne", Long, All),
    ex("Porsche", "Macan", Long, All),
    ex("Porsche", "", Long, Rear),
    ex("Volkswagen", "Käfer", Back, Rear),
    ex("Smart", "Fortwo", Back, Rear),
    ex("Smart", "Roadster", Back, Rear),
    ex_years("Smart", "Forfour", 2014, 9999, Back, Rear),
    ex_years("Renault", "Twingo", 2014, 2024, Back, Rear),
    ex("DeLorean", "", Back, Rear),
    ex("Škoda", "105", Back, Rear),
    ex("Ferrari", "456", Long, Rear),
    ex("Ferrari", "550", Long, Rear),
    ex("Ferrari", "575", Long, Rear),
    ex("Ferrari", "599", Long, Rear),
    ex("Ferrari", "612", Long, Rear),
    ex("Ferrari", "812", Long, Rear),
    ex("Ferrari", "California", Long, Rear),
    ex("Ferrari", "F12", Long, Rear),
    ex("Ferrari", "FF", Long, All),
    ex("Ferrari", "Portofino", Long, Rear),
    ex("Ferrari", "Purosangue", Long, All),
    ex("Ferrari", "Roma", Long, Rear),
    ex("Ferrari", "", Mid, Rear),
    ex("Lamborghini", "Urus", Long, All),
    ex("Lamborghini", "", Mid, All),
    ex("McLaren", "", Mid, Rear),
    ex("Lotus", "Elise", Mid, Rear),
    ex("Lotus", "Exige", Mid, Rear),
    ex("Lotus", "Evora", Mid, Rear),
    ex("Lotus", "Emira", Mid, Rear),
    ex("Lotus", "Esprit", Mid, Rear),
    ex("Audi", "R8", Mid, All),
    ex("Honda", "NSX", Mid, Rear),
    ex("Alpine", "A110", Mid, Rear),
    ex("Alpine", "A610", Back, Rear),
    ex("Toyota", "MR2", Mid, Rear),
    ex("Alfa Romeo", "4C", Mid, Rear),
    // Engine along the car.
    ex("Audi", "A1", Across, Front),
    ex("Audi", "A2", Across, Front),
    ex("Audi", "A3", Across, Front),
    ex("Audi", "Q2", Across, Front),
    ex("Audi", "Q3", Across, Front),
    ex("Audi", "TT", Across, Front),
    ex("Audi", "Q5", Long, All),
    ex("Audi", "Q7", Long, All),
    ex("Audi", "Q8", Long, All),
    ex("Audi", "A8", Long, All),
    ex("Audi", "A6 allroad", Long, All),
    ex("Audi", "Quattro", Long, All),
    ex("Audi", "", Long, Front),
    ex_years("BMW", "1er", 2019, 9999, Across, Front),
    ex("BMW", "2er Active", Across, Front),
    ex("BMW", "2er Gran Tourer", Across, Front),
    ex("BMW", "X2", Across, Front),
    ex_years("BMW", "X1", 2015, 9999, Across, Front),
    ex("BMW", "X", Long, All),
    ex("BMW", "", Long, Rear),
    ex("Alpina", "", Long, Rear),
    ex("Mercedes-Benz", "A-Klasse", Across, Front),
    ex("Mercedes-Benz", "B-Klasse", Across, Front),
    ex("Mercedes-Benz", "CLA", Across, Front),
    ex("Mercedes-Benz", "GLA", Across, Front),
    ex("Mercedes-Benz", "GLB", Across, Front),
    ex("Mercedes-Benz", "Vaneo", Across, Front),
    ex("Mercedes-Benz", "Citan", Across, Front),
    ex("Mercedes-Benz", "G-Klasse", Long, All),
    ex("Mercedes-Benz", "GL", Long, All),
    ex("Mercedes-Benz", "M-Klasse", Long, All),
    ex("Mercedes-Benz", "X-Klasse", Long, All),
    ex("Mercedes-Benz", "", Long, Rear),
    ex("Jaguar", "", Long, Rear),
    ex("Land Rover", "Discovery Sport", Across, All),
    ex("Land Rover", "Range Rover Evoque", Across, All),
    ex("Land Rover", "Freelander", Across, All),
    ex("Land Rover", "", Long, All),
    ex("Maserati", "", Long, Rear),
    ex("Aston Martin", "", Long, Rear),
    ex("Bentley", "", Long, All),
    ex("Rolls-Royce", "", Long, Rear),
    ex("Subaru", "BRZ", Long, Rear),
    ex("Subaru", "Trezia", Across, Front),
    ex("Subaru", "Justy", Across, Front),
    ex("Subaru", "", Long, All),
    ex("Alfa Romeo", "75", Long, Rear),
    ex("Alfa Romeo", "Alfetta", Long, Rear),
    ex_years("Alfa Romeo", "Giulia", 2015, 9999, Long, Rear),
    ex("Alfa Romeo", "Stelvio", Long, All),
    ex("Lexus", "GS", Long, Rear),
    ex("Lexus", "IS", Long, Rear),
    ex("Lexus", "LS", Long, Rear),
    ex("Lexus", "LC", Long, Rear),
    ex("Lexus", "SC", Long, Rear),
    ex("Lexus", "LFA", Long, Rear),
    ex("Lexus", "GX", Long, All),
    ex("Lexus", "LX", Long, All),
    ex("Volvo", "240", Long, Rear),
    ex("Volvo", "740", Long, Rear),
    ex("Volvo", "760", Long, Rear),
    ex("Volvo", "940", Long, Rear),
    ex("Volvo", "960", Long, Rear),
    ex("Toyota", "Supra", Long, Rear),
    ex("Toyota", "GT86", Long, Rear),
    ex("Toyota", "GR86", Long, Rear),
    ex("Toyota", "Land Cruiser", Long, All),
    ex("Toyota", "4Runner", Long, All),
    ex("Toyota", "FJ Cruiser", Long, All),
    ex("Toyota", "Sequoia", Long, All),
    ex("Nissan", "GT-R", Long, All),
    ex("Nissan", "350Z", Long, Rear),
    ex("Nissan", "370Z", Long, Rear),
    ex("Nissan", "Patrol", Long, All),
    ex("Mazda", "MX-5", Long, Rear),
    ex("Mazda", "RX-", Long, Rear),
    ex("Mazda", "CX-60", Long, Rear),
    ex("Mazda", "CX-80", Long, Rear),
    ex("Mazda", "CX-90", Long, Rear),
    ex("Jeep", "Wrangler", Long, All),
    ex("Jeep", "Grand Cherokee", Long, All),
    ex("Jeep", "Gladiator", Long, All),
    ex("Jeep", "Commander", Long, All),
    ex("Jeep", "Wagoneer", Long, All),
    ex("Dodge", "Challenger", Long, Rear),
    ex("Dodge", "Charger", Long, Rear),
    ex("Dodge", "Viper", Long, Rear),
    ex("Chevrolet", "Camaro", Long, Rear),
    ex("Chevrolet", "Corvette", Long, Rear),
    ex("Chevrolet", "Tahoe", Long, Rear),
    ex("Ford", "Mustang", Long, Rear),
    ex("Ford", "Sierra", Long, Rear),
    ex("Ford", "Scorpio", Long, Rear),
    ex("Ford", "Granada", Long, Rear),
    ex("Ford", "Capri", Long, Rear),
    ex("Volkswagen", "Touareg", Long, All),
    ex("Volkswagen", "Phaeton", Long, All),
    ex_years("Volkswagen", "Passat", 1996, 2005, Long, Front),
    ex_years("Škoda", "Superb", 2001, 2008, Long, Front),
];

fn exception(brand: &str, model: &str, year: i32) -> Option<&'static Exception> {
    EXCEPTIONS.iter().find(|e| {
        e.brand.eq_ignore_ascii_case(brand)
            && model.starts_with(e.model)
            && (year <= 0 || (e.from..=e.to).contains(&year))
    })
}

/// Electric platforms that drive the rear wheels with a single motor.
fn rear_motor_platform(brand: &str, model: &str) -> bool {
    let rear_brands = ["Tesla", "BMW", "Mercedes-Benz", "Porsche", "Smart"];
    if rear_brands.iter().any(|b| b.eq_ignore_ascii_case(brand)) {
        return !(brand == "Mercedes-Benz" && (model == "EQA" || model == "EQB"));
    }
    match brand {
        "Volkswagen" => model.starts_with("ID."),
        "Škoda" => model.starts_with("Enyaq") || model.starts_with("Elroq"),
        "Cupra" => model.starts_with("Born") || model.starts_with("Tavascan"),
        "Audi" => model.starts_with("Q4"),
        "Renault" => model.starts_with("Twingo"),
        _ => false,
    }
}

/// Drivetrain of a car of this brand, model and body with one engine
/// variant (`PT_*`, one flag; 0 takes petrol).
pub fn drivetrain(brand: &str, model: &str, body: BodyType, year: i32, variant: u32) -> Drivetrain {
    let variant = if variant == 0 { PT_PETROL } else { variant };
    if variant & PT_ELECTRIC != 0 {
        let dual = body == BodyType::Suv && !matches!(brand, "Volkswagen" | "Škoda" | "Cupra")
            || (brand == "Tesla" && (model.contains('S') || model.contains('X')))
            || model.contains("GT");
        let rear = rear_motor_platform(brand, model);
        return Drivetrain {
            engine: EngineLayout::None,
            driven: if dual {
                All
            } else if rear {
                Rear
            } else {
                Front
            },
            front_motor: dual || !rear,
            rear_motor: dual || rear,
            battery: TractionBattery::Floor,
            fuel_tank: false,
        };
    }
    let (engine, mut driven) = match exception(brand, model, year) {
        Some(e) => (e.engine, e.driven),
        None if body == BodyType::Pickup => (Long, Rear),
        None => (Across, Front),
    };
    if body == BodyType::Pickup && engine == Long {
        driven = if driven == Front { All } else { driven };
    }
    let battery = match variant {
        PT_HYBRID => TractionBattery::Hybrid,
        PT_PLUG_IN => TractionBattery::PlugIn,
        _ => TractionBattery::None,
    };
    let hybrid = battery != TractionBattery::None;
    Drivetrain {
        engine,
        driven,
        // The motor of a hybrid sits at the gearbox, on the axle the engine drives.
        front_motor: hybrid && matches!(engine, Across | Long) && driven != Rear,
        rear_motor: hybrid && (driven == Rear || matches!(engine, Mid | Back)),
        battery,
        fuel_tank: variant & (PT_PETROL | PT_DIESEL | PT_GAS | PT_HYBRID | PT_PLUG_IN) != 0,
    }
}

pub fn generation_drivetrain(g: &VehicleGeneration, year: i32, variant: u32) -> Drivetrain {
    let variant = if variant == 0 {
        default_powertrain(g.powertrains)
    } else {
        variant
    };
    drivetrain(g.brand, g.model, g.body, year, variant)
}

pub fn engine_layout_name(engine: EngineLayout) -> &'static str {
    match engine {
        EngineLayout::None => l("Kein Verbrennungsmotor", "No combustion engine"),
        EngineLayout::FrontTransverse => l("Motor vorn quer", "Front engine, transverse"),
        EngineLayout::FrontLongitudinal => l("Motor vorn längs", "Front engine, longitudinal"),
        EngineLayout::Mid => l("Mittelmotor", "Mid engine"),
        EngineLayout::Rear => l("Heckmotor", "Rear engine"),
    }
}

pub fn driven_axles_name(driven: DrivenAxles) -> &'static str {
    match driven {
        DrivenAxles::Front => l("Frontantrieb", "Front-wheel drive"),
        DrivenAxles::Rear => l("Hinterradantrieb", "Rear-wheel drive"),
        DrivenAxles::All => l("Allradantrieb", "All-wheel drive"),
    }
}

/// One sentence on why the model is laid out like this.
pub fn drivetrain_reason(d: &Drivetrain) -> &'static str {
    match (d.engine, d.battery) {
        // One line per text: tools/i18n_rust.py reads the literals as JSON
        // strings, which have no line continuation.
        (EngineLayout::None, _) => l(
            "Elektroauto: der Akku liegt flach im Boden zwischen den Achsen, die Motoren sitzen direkt an den angetriebenen Achsen.",
            "Electric car: the battery lies flat in the floor between the axles, the motors sit right at the driven axles.",
        ),
        (_, TractionBattery::Hybrid | TractionBattery::PlugIn) => l(
            "Hybrid: der Verbrenner bleibt, wo er bei diesem Modell sitzt, dazu ein Elektromotor am Getriebe und ein Akku unter der Rückbank.",
            "Hybrid: the combustion engine stays where it sits in this model, plus an electric motor at the gearbox and a battery under the rear seat.",
        ),
        (EngineLayout::FrontTransverse, _) => l(
            "Motor vorn quer mit Frontantrieb: so sind die meisten Autos seit den 1980ern gebaut.",
            "Front engine across the car with front-wheel drive: the layout of most cars since the 1980s.",
        ),
        (EngineLayout::FrontLongitudinal, _) => l(
            "Motor vorn längs: typisch für diese Marke oder dieses Modell, meist mit Hinterrad- oder Allradantrieb.",
            "Front engine along the car: typical for this brand or model, mostly with rear or all-wheel drive.",
        ),
        (EngineLayout::Mid | EngineLayout::Rear, _) => l(
            "Motor hinten: bei diesem Modell sitzt er hinter dem Fahrer und treibt die Hinterräder an.",
            "Engine at the back: in this model it sits behind the driver and drives the rear wheels.",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn every_generation_has_a_sensible_shape() {
        for g in catalog::all() {
            for year in [g.year_from, if g.year_to > 0 { g.year_to } else { 2026 }] {
                let (d, _) = dimensions_for(g, year);
                assert!(d.wheelbase < d.length, "{} {}", g.brand, g.model);
                assert!(d.track < d.width);
                assert!(
                    d.wheel * 2 < d.height + d.wheel,
                    "wheels fit under the roof"
                );
                assert!(d.length > d.width && d.width > 1000 && d.height > 1000);
                for variant in powertrain_variants(g.powertrains) {
                    let t = generation_drivetrain(g, year, variant);
                    // Something has to drive the car.
                    assert!(
                        t.engine != EngineLayout::None || t.front_motor || t.rear_motor,
                        "{} {}",
                        g.brand,
                        g.model
                    );
                    assert_eq!(t.engine == EngineLayout::None, !t.fuel_tank);
                }
            }
        }
    }

    #[test]
    fn layouts_follow_the_rule() {
        let golf = drivetrain("Volkswagen", "Golf", BodyType::Hatchback, 2005, PT_PETROL);
        assert_eq!(golf.engine, EngineLayout::FrontTransverse);
        assert_eq!(golf.driven, DrivenAxles::Front);
        let e46 = drivetrain("BMW", "3er", BodyType::Sedan, 2002, PT_DIESEL);
        assert_eq!((e46.engine, e46.driven), (Long, Rear));
        let f40 = drivetrain("BMW", "1er", BodyType::Hatchback, 2020, PT_PETROL);
        assert_eq!((f40.engine, f40.driven), (Across, Front));
        let e87 = drivetrain("BMW", "1er", BodyType::Hatchback, 2008, PT_PETROL);
        assert_eq!((e87.engine, e87.driven), (Long, Rear));
        let a4 = drivetrain("Audi", "A4", BodyType::Estate, 2010, PT_DIESEL);
        assert_eq!((a4.engine, a4.driven), (Long, Front));
        let p911 = drivetrain("Porsche", "911", BodyType::Coupe, 2015, PT_PETROL);
        assert_eq!(p911.engine, EngineLayout::Rear);
        let id3 = drivetrain("Volkswagen", "ID.3", BodyType::Hatchback, 2021, PT_ELECTRIC);
        assert_eq!(id3.battery, TractionBattery::Floor);
        assert!(id3.rear_motor && !id3.front_motor);
        let zoe = drivetrain("Renault", "Zoe", BodyType::Hatchback, 2019, PT_ELECTRIC);
        assert!(zoe.front_motor && !zoe.rear_motor);
        let prius = drivetrain("Toyota", "Prius", BodyType::Hatchback, 2018, PT_HYBRID);
        assert_eq!(prius.battery, TractionBattery::Hybrid);
        assert!(prius.front_motor && prius.fuel_tank);
        let hilux = drivetrain("Toyota", "Hilux", BodyType::Pickup, 2018, PT_DIESEL);
        assert_eq!(hilux.engine, Long);
    }

    #[test]
    fn newer_cars_are_larger() {
        let old = typical_dimensions(BodyType::Hatchback, 1988);
        let new = typical_dimensions(BodyType::Hatchback, 2020);
        assert!(new.length > old.length && new.width > old.width && new.wheel > old.wheel);
        assert_eq!(default_powertrain(PT_HYBRID | PT_PETROL), PT_PETROL);
        assert_eq!(default_powertrain(PT_ELECTRIC), PT_ELECTRIC);
        assert_eq!(
            powertrain_variants(PT_PETROL | PT_ELECTRIC),
            vec![PT_PETROL, PT_ELECTRIC]
        );
    }
}
