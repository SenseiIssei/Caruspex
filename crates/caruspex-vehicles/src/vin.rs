//! Vehicle identification number (VIN) decoder.
//!
//! A 17 character VIN after ISO 3779 consists of three parts:
//!   positions 1-3   WMI, world manufacturer identifier
//!   positions 4-9   VDS, vehicle descriptor section (maker specific,
//!                   position 9 is the check digit in North America and China)
//!   positions 10-17 VIS, vehicle identifier section (10 model year,
//!                   11 plant, 12-17 serial number)
//!
//! Only the WMI and the overall layout are standardised worldwide. The model
//! year at position 10 is mandatory in North America and China, but optional
//! in Europe, and several European makers put something else there. The
//! decoder therefore says how sure it is instead of pretending.
//!
//! Texts in `VinInfo` (country, region, problems, notes) come out in the
//! language that is active at decode time (see `caruspex_i18n`).

use caruspex_i18n::{Text, l};

/// How a maker uses position 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VinYearRule {
    /// Model year after the norm.
    Standard,
    /// Not known for this maker, the value is only a guess.
    Unknown,
    /// The maker does not put the model year there.
    None,
}

/// Where the maker writes its own model code into the VIN, if we know it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VinModelCodeRule {
    #[default]
    None,
    /// Volkswagen group: positions 7 and 8 (8Z = Audi A2, 1J = Golf IV).
    Vag78,
    /// Mercedes-Benz and smart, European layout: positions 4 to 6 (203 = W203).
    Mercedes456,
    /// Porsche: positions 7 and 8 (99 = 911, Y1 = Taycan).
    Porsche78,
    /// Tesla: position 4 is the model (S, X, 3, Y).
    Tesla4,
}

#[derive(Clone, Copy, Debug)]
pub struct WmiEntry {
    /// Two or three characters; two means "any third character".
    pub wmi: &'static str,
    /// Company behind the code.
    pub manufacturer: &'static str,
    /// Catalog brand names, several separated by '|'.
    pub brands: &'static str,
    pub year_rule: VinYearRule,
    pub code_rule: VinModelCodeRule,
    /// Only set where the plant country differs from the code range.
    pub country: Text,
}

#[derive(Clone, Debug, Default)]
pub struct VinInfo {
    /// 17 characters, none of them forbidden.
    pub valid: bool,
    /// Normalised: upper case, no spaces or dashes.
    pub vin: String,
    /// Positions 1-3.
    pub wmi: String,
    /// Empty if the WMI is unknown.
    pub manufacturer: String,
    /// Catalog brand, empty if unknown or ambiguous.
    pub brand: String,
    /// All brands the WMI stands for.
    pub brand_candidates: Vec<String>,
    /// From the first two characters.
    pub country: String,
    /// Continent.
    pub region: String,
    /// Built for or in North America (position 1 = 1..5, 7).
    pub north_america: bool,

    /// Candidates, most likely first; empty if not encoded.
    pub model_years: Vec<i32>,
    /// `model_years.first()`, or 0.
    pub model_year: i32,
    /// False: several candidates or the maker is unknown.
    pub model_year_certain: bool,
    /// Position 10 as written, '\0' if absent.
    pub year_code: char,

    /// The check digit is mandatory for this VIN.
    pub check_digit_applies: bool,
    /// Position 9 matches the calculated digit.
    pub check_digit_ok: bool,
    /// Position 9 as written.
    pub check_digit: char,
    /// Calculated, '\0' if the VIN is not complete.
    pub expected_check_digit: char,

    /// Position 11.
    pub plant: char,
    /// Positions 12-17.
    pub serial: String,
    /// Maker specific model code, see `VinModelCodeRule`.
    pub model_code: String,
    pub model_code_rule: VinModelCodeRule,

    /// Things that are wrong with the number.
    pub problems: Vec<String>,
    /// Things worth knowing, not errors.
    pub notes: Vec<String>,
}

// Short names keep the table below readable.
const STD: VinYearRule = VinYearRule::Standard;
const UNK: VinYearRule = VinYearRule::Unknown;
const NO_YEAR: VinYearRule = VinYearRule::None;
const NO_CODE: VinModelCodeRule = VinModelCodeRule::None;
const VAG: VinModelCodeRule = VinModelCodeRule::Vag78;
const MB: VinModelCodeRule = VinModelCodeRule::Mercedes456;
const POR: VinModelCodeRule = VinModelCodeRule::Porsche78;
const TESLA: VinModelCodeRule = VinModelCodeRule::Tesla4;

// Plant countries that differ from the ISO range of the code.
const IN_AUSTRIA: Text = Text::new("Österreich", "Austria");
const IN_GERMANY: Text = Text::new("Deutschland", "Germany");
const IN_FRANCE: Text = Text::new("Frankreich", "France");
#[allow(dead_code)]
const IN_SPAIN: Text = Text::new("Spanien", "Spain");
const NO_COUNTRY: Text = Text::new("", "");

const fn w(
    wmi: &'static str,
    manufacturer: &'static str,
    brands: &'static str,
    year_rule: VinYearRule,
    code_rule: VinModelCodeRule,
    country: Text,
) -> WmiEntry {
    WmiEntry {
        wmi,
        manufacturer,
        brands,
        year_rule,
        code_rule,
        country,
    }
}

// World manufacturer identifiers. Three character entries are exact, two
// character entries catch the rest of a range that belongs to one maker
// (JT is Toyota unless a more specific entry says otherwise).
//
// Sources are the public WMI lists (SAE, NHTSA, ISO 3780 ranges). Where a
// code is shared by several brands of one group, all of them are listed and
// the catalog decides by the model code or leaves it open.
#[rustfmt::skip]
static WMI: &[WmiEntry] = &[
    // --- Germany ---------------------------------------------------------
    w("WAU", "Audi AG", "Audi", STD, VAG, NO_COUNTRY),
    w("WA1", "Audi AG (SUV)", "Audi", STD, VAG, NO_COUNTRY),
    w("WUA", "Audi Sport GmbH", "Audi", STD, VAG, NO_COUNTRY),
    w("TRU", "Audi Hungaria", "Audi", STD, VAG, NO_COUNTRY),
    w("WVW", "Volkswagen AG", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("WVG", "Volkswagen AG (SUV)", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("WV1", "Volkswagen Nutzfahrzeuge", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("WV2", "Volkswagen Nutzfahrzeuge", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("1VW", "Volkswagen of America", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("1V1", "Volkswagen of America", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("1V2", "Volkswagen of America (SUV)", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("3VW", "Volkswagen de México", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("3VV", "Volkswagen de México (SUV)", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("9BW", "Volkswagen do Brasil", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("8AW", "Volkswagen Argentina", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("AAV", "Volkswagen South Africa", "Volkswagen", STD, VAG, NO_COUNTRY),
    w("LSV", "SAIC Volkswagen", "Volkswagen|Škoda", STD, VAG, NO_COUNTRY),
    w("LFV", "FAW-Volkswagen", "Volkswagen|Audi", STD, VAG, NO_COUNTRY),
    w("XW8", "Volkswagen Group Rus", "Volkswagen|Škoda", STD, VAG, NO_COUNTRY),
    w("TMB", "Škoda Auto", "Škoda", STD, VAG, NO_COUNTRY),
    w("VSS", "SEAT", "SEAT|Cupra", STD, VAG, NO_COUNTRY),
    w("WP0", "Porsche AG", "Porsche", STD, POR, NO_COUNTRY),
    w("WP1", "Porsche AG (SUV)", "Porsche", STD, POR, NO_COUNTRY),
    w("WBA", "BMW AG", "BMW", UNK, NO_CODE, NO_COUNTRY),
    w("WBS", "BMW M GmbH", "BMW", UNK, NO_CODE, NO_COUNTRY),
    w("WBX", "BMW AG (SUV)", "BMW", UNK, NO_CODE, NO_COUNTRY),
    w("WBY", "BMW AG (BMW i)", "BMW", UNK, NO_CODE, NO_COUNTRY),
    w("4US", "BMW Manufacturing USA", "BMW", STD, NO_CODE, NO_COUNTRY),
    w("5UX", "BMW Manufacturing USA (SUV)", "BMW", STD, NO_CODE, NO_COUNTRY),
    w("5YM", "BMW Manufacturing USA (M SUV)", "BMW", STD, NO_CODE, NO_COUNTRY),
    w("3MW", "BMW de México", "BMW", STD, NO_CODE, NO_COUNTRY),
    w("LBV", "BMW Brilliance", "BMW", STD, NO_CODE, NO_COUNTRY),
    w("WMW", "BMW AG (MINI)", "Mini", UNK, NO_CODE, NO_COUNTRY),
    w("WAP", "Alpina", "Alpina", UNK, NO_CODE, NO_COUNTRY),
    w("WDB", "Mercedes-Benz", "Mercedes-Benz|Maybach", UNK, MB, NO_COUNTRY),
    w("WDD", "Mercedes-Benz (Pkw)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("WDC", "Mercedes-Benz (SUV)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("WDF", "Mercedes-Benz (Transporter)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("W1K", "Mercedes-Benz (Pkw)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("W1N", "Mercedes-Benz (SUV)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("W1V", "Mercedes-Benz (Transporter)", "Mercedes-Benz", UNK, MB, NO_COUNTRY),
    w("W1W", "Mercedes-Benz (Transporter, Nordamerika)", "Mercedes-Benz", STD, NO_CODE, NO_COUNTRY),
    w("4JG", "Mercedes-Benz U.S. International (SUV)", "Mercedes-Benz", STD, NO_CODE, NO_COUNTRY),
    w("55S", "Mercedes-Benz U.S. International", "Mercedes-Benz", STD, NO_CODE, NO_COUNTRY),
    w("LE4", "Beijing Benz", "Mercedes-Benz", STD, NO_CODE, NO_COUNTRY),
    w("9BM", "Mercedes-Benz do Brasil", "Mercedes-Benz", STD, NO_CODE, NO_COUNTRY),
    w("WME", "smart", "Smart", UNK, MB, NO_COUNTRY),
    w("TCC", "Micro Compact Car", "Smart", UNK, MB, NO_COUNTRY),
    w("W1A", "smart", "Smart", UNK, NO_CODE, NO_COUNTRY),
    w("W0L", "Adam Opel AG", "Opel", STD, NO_CODE, NO_COUNTRY),
    w("W0V", "Opel Automobile GmbH", "Opel", STD, NO_CODE, NO_COUNTRY),
    w("VXK", "Opel Automobile GmbH (Stellantis)", "Opel", UNK, NO_CODE, IN_FRANCE),
    w("VXE", "Opel Automobile GmbH (Stellantis, Transporter)", "Opel", UNK, NO_CODE, IN_FRANCE),
    w("VSX", "Opel España", "Opel", STD, NO_CODE, NO_COUNTRY),
    w("WF0", "Ford-Werke (Ford of Europe)", "Ford", UNK, NO_CODE, NO_COUNTRY),
    w("WF1", "Ford-Werke (Merkur)", "Merkur", STD, NO_CODE, NO_COUNTRY),
    w("WMA", "MAN Truck & Bus", "MAN", UNK, NO_CODE, NO_COUNTRY),
    w("WZ1", "Toyota Supra, gebaut von Magna Steyr", "Toyota", STD, NO_CODE, IN_AUSTRIA),
    w("XP7", "Tesla Gigafactory Berlin-Brandenburg", "Tesla", STD, TESLA, IN_GERMANY),

    // --- France ----------------------------------------------------------
    w("VF1", "Renault", "Renault|Dacia|Alpine", UNK, NO_CODE, NO_COUNTRY),
    w("VF2", "Renault", "Renault", UNK, NO_CODE, NO_COUNTRY),
    w("VF6", "Renault Trucks", "Renault", UNK, NO_CODE, NO_COUNTRY),
    w("VS5", "Renault España", "Renault", UNK, NO_CODE, NO_COUNTRY),
    w("VFA", "Alpine", "Alpine", UNK, NO_CODE, NO_COUNTRY),
    w("VF3", "Peugeot", "Peugeot", UNK, NO_CODE, NO_COUNTRY),
    w("VR3", "Peugeot (Stellantis)", "Peugeot", UNK, NO_CODE, NO_COUNTRY),
    w("VF7", "Citroën", "Citroën|DS", UNK, NO_CODE, NO_COUNTRY),
    w("VR7", "Citroën (Stellantis)", "Citroën", UNK, NO_CODE, NO_COUNTRY),
    w("VS7", "Citroën España", "Citroën", UNK, NO_CODE, NO_COUNTRY),
    w("VR1", "DS Automobiles", "DS", UNK, NO_CODE, NO_COUNTRY),
    w("VF8", "Matra Automobile", "Matra", UNK, NO_CODE, NO_COUNTRY),
    w("VF9", "Bugatti Automobiles", "Bugatti", UNK, NO_CODE, NO_COUNTRY),
    w("VNK", "Toyota Motor Manufacturing France", "Toyota", UNK, NO_CODE, NO_COUNTRY),

    // --- Italy -----------------------------------------------------------
    w("ZFA", "Fiat", "Fiat|Abarth", UNK, NO_CODE, NO_COUNTRY),
    w("ZAR", "Alfa Romeo", "Alfa Romeo", UNK, NO_CODE, NO_COUNTRY),
    w("ZAS", "Alfa Romeo", "Alfa Romeo", UNK, NO_CODE, NO_COUNTRY),
    w("ZLA", "Lancia", "Lancia", UNK, NO_CODE, NO_COUNTRY),
    w("ZFF", "Ferrari", "Ferrari", UNK, NO_CODE, NO_COUNTRY),
    w("ZHW", "Lamborghini", "Lamborghini", UNK, NO_CODE, NO_COUNTRY),
    w("ZPB", "Lamborghini (SUV)", "Lamborghini", UNK, NO_CODE, NO_COUNTRY),
    w("ZAM", "Maserati", "Maserati", UNK, NO_CODE, NO_COUNTRY),
    w("ZAC", "FCA Italy (Jeep, Dodge)", "Jeep|Dodge", UNK, NO_CODE, NO_COUNTRY),
    w("ZCF", "Iveco", "Iveco", UNK, NO_CODE, NO_COUNTRY),
    w("3C3", "FCA México", "Fiat|Chrysler", STD, NO_CODE, NO_COUNTRY),
    w("9BD", "Fiat Automóveis (Brasilien)", "Fiat", STD, NO_CODE, NO_COUNTRY),
    w("8AP", "Fiat Auto Argentina", "Fiat", STD, NO_CODE, NO_COUNTRY),
    w("SUF", "Fiat Auto Poland", "Fiat", UNK, NO_CODE, NO_COUNTRY),
    w("NM4", "Tofaş", "Fiat", UNK, NO_CODE, NO_COUNTRY),

    // --- United Kingdom --------------------------------------------------
    w("SAJ", "Jaguar Cars", "Jaguar", STD, NO_CODE, NO_COUNTRY),
    w("SAD", "Jaguar Land Rover (Jaguar SUV)", "Jaguar", STD, NO_CODE, NO_COUNTRY),
    w("SAL", "Land Rover", "Land Rover", STD, NO_CODE, NO_COUNTRY),
    w("SC6", "INEOS Automotive (SUV)", "INEOS", STD, NO_CODE, NO_COUNTRY),
    w("SH7", "INEOS Automotive (pick-up)", "INEOS", STD, NO_CODE, NO_COUNTRY),
    w("ZS1", "Leapmotor International", "Leapmotor", STD, NO_CODE, NO_COUNTRY),
    w("SAR", "Rover Group / MG Rover", "Rover|MG", UNK, NO_CODE, NO_COUNTRY),
    w("SAX", "Austin Rover Group", "Austin|Rover", UNK, NO_CODE, NO_COUNTRY),
    w("SCA", "Rolls-Royce", "Rolls-Royce", STD, NO_CODE, NO_COUNTRY),
    w("SCB", "Bentley", "Bentley", STD, NO_CODE, NO_COUNTRY),
    w("SCC", "Lotus Cars", "Lotus", STD, NO_CODE, NO_COUNTRY),
    w("SCE", "DeLorean Motor Company", "DeLorean", STD, NO_CODE, NO_COUNTRY),
    w("SCF", "Aston Martin", "Aston Martin", STD, NO_CODE, NO_COUNTRY),
    w("SBM", "McLaren Automotive", "McLaren", STD, NO_CODE, NO_COUNTRY),
    w("SFZ", "Tesla Roadster, gebaut von Lotus", "Tesla", STD, NO_CODE, NO_COUNTRY),
    w("SFA", "Ford Motor Company UK", "Ford", UNK, NO_CODE, NO_COUNTRY),
    w("SHH", "Honda of the UK Manufacturing", "Honda", UNK, NO_CODE, NO_COUNTRY),
    w("SHS", "Honda of the UK Manufacturing (SUV)", "Honda", UNK, NO_CODE, NO_COUNTRY),
    w("SJN", "Nissan Motor Manufacturing UK", "Nissan", UNK, NO_CODE, NO_COUNTRY),
    w("SJK", "Nissan Motor Manufacturing UK (Infiniti)", "Infiniti", UNK, NO_CODE, NO_COUNTRY),
    w("SB1", "Toyota Motor Manufacturing UK", "Toyota", UNK, NO_CODE, NO_COUNTRY),

    // --- Sweden, Finland, Netherlands -----------------------------------
    w("YV1", "Volvo Cars", "Volvo", STD, NO_CODE, NO_COUNTRY),
    w("YV4", "Volvo Cars (SUV)", "Volvo", STD, NO_CODE, NO_COUNTRY),
    w("7JR", "Volvo Car USA", "Volvo", STD, NO_CODE, NO_COUNTRY),
    w("LVY", "Volvo Cars Daqing", "Volvo", STD, NO_CODE, NO_COUNTRY),
    w("LYV", "Volvo Cars Chengdu / Luqiao", "Volvo|Polestar", STD, NO_CODE, NO_COUNTRY),
    w("LPS", "Polestar", "Polestar", STD, NO_CODE, NO_COUNTRY),
    w("YSM", "Polestar", "Polestar", STD, NO_CODE, NO_COUNTRY),
    w("YSR", "Polestar (SUV)", "Polestar", STD, NO_CODE, NO_COUNTRY),
    w("YS3", "Saab Automobile", "Saab", STD, NO_CODE, NO_COUNTRY),
    w("YK1", "Saab, gebaut von Valmet Automotive", "Saab", STD, NO_CODE, NO_COUNTRY),
    w("3G0", "Saab 9-4X (GM México)", "Saab", STD, NO_CODE, NO_COUNTRY),
    w("YH4", "Fisker Automotive (Karma), gebaut von Valmet", "Fisker", STD, NO_CODE, NO_COUNTRY),
    w("XMC", "NedCar (Mitsubishi)", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("XLR", "DAF Trucks", "DAF", UNK, NO_CODE, NO_COUNTRY),
    w("YAR", "Toyota Motor Europe", "Toyota", UNK, NO_CODE, NO_COUNTRY),

    // --- Central and Eastern Europe, Russia, Turkey ---------------------
    w("TMA", "Hyundai Motor Manufacturing Czech", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("TMC", "Hyundai Motor Manufacturing Czech (SUV)", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("TMT", "Tatra", "Tatra", UNK, NO_CODE, NO_COUNTRY),
    w("TSM", "Magyar Suzuki", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("TW1", "Toyota Caetano Portugal", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("U5Y", "Kia Slovakia", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("U6Y", "Kia Slovakia (SUV)", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("UU1", "Dacia", "Dacia", UNK, NO_CODE, NO_COUNTRY),
    w("UU6", "Daewoo Automobile Romania", "Daewoo", UNK, NO_CODE, NO_COUNTRY),
    w("VSK", "Nissan Motor Ibérica", "Nissan", UNK, NO_CODE, NO_COUNTRY),
    w("VSE", "Santana Motor (Suzuki)", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("VS6", "Ford España", "Ford", UNK, NO_CODE, NO_COUNTRY),
    w("VCF", "Fisker Ocean, gebaut von Magna Steyr", "Fisker", STD, NO_CODE, NO_COUNTRY),
    w("SUP", "FSO / Daewoo-FSO", "Daewoo|FSO", UNK, NO_CODE, NO_COUNTRY),
    w("XTA", "AvtoVAZ", "Lada", UNK, NO_CODE, NO_COUNTRY),
    w("XTT", "UAZ", "UAZ", UNK, NO_CODE, NO_COUNTRY),
    w("X9L", "GM-AvtoVAZ", "Chevrolet", UNK, NO_CODE, NO_COUNTRY),
    w("X7L", "Renault Russia", "Renault", UNK, NO_CODE, NO_COUNTRY),
    w("NM0", "Ford Otosan", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("NMT", "Toyota Motor Manufacturing Turkey", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("NLH", "Hyundai Assan", "Hyundai", STD, NO_CODE, NO_COUNTRY),

    // --- Japan -----------------------------------------------------------
    w("JT", "Toyota Motor Corporation", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("JTH", "Toyota Motor Corporation (Lexus)", "Lexus", UNK, NO_CODE, NO_COUNTRY),
    w("JTJ", "Toyota Motor Corporation (Lexus SUV)", "Lexus", UNK, NO_CODE, NO_COUNTRY),
    w("JTK", "Toyota Motor Corporation (Scion)", "Scion", STD, NO_CODE, NO_COUNTRY),
    w("JTL", "Toyota Motor Corporation (Scion)", "Scion", STD, NO_CODE, NO_COUNTRY),
    w("JHM", "Honda Motor Co.", "Honda", UNK, NO_CODE, NO_COUNTRY),
    w("JHL", "Honda Motor Co. (SUV)", "Honda", UNK, NO_CODE, NO_COUNTRY),
    w("JHG", "Honda Motor Co.", "Honda", UNK, NO_CODE, NO_COUNTRY),
    w("JH4", "Honda Motor Co. (Acura)", "Acura", STD, NO_CODE, NO_COUNTRY),
    w("JN", "Nissan Motor Co.", "Nissan", UNK, NO_CODE, NO_COUNTRY),
    w("JNK", "Nissan Motor Co. (Infiniti)", "Infiniti", UNK, NO_CODE, NO_COUNTRY),
    w("JNR", "Nissan Motor Co. (Infiniti SUV)", "Infiniti", UNK, NO_CODE, NO_COUNTRY),
    w("JM1", "Mazda Motor Corporation", "Mazda", UNK, NO_CODE, NO_COUNTRY),
    w("JM3", "Mazda Motor Corporation (SUV)", "Mazda", UNK, NO_CODE, NO_COUNTRY),
    w("JMZ", "Mazda Motor Corporation (Europa)", "Mazda", UNK, NO_CODE, NO_COUNTRY),
    w("JA3", "Mitsubishi Motors", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("JA4", "Mitsubishi Motors (SUV)", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("JMB", "Mitsubishi Motors (Europa)", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("JMY", "Mitsubishi Motors", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("JF1", "Subaru (Fuji Heavy Industries)", "Subaru", UNK, NO_CODE, NO_COUNTRY),
    w("JF2", "Subaru (Fuji Heavy Industries, SUV)", "Subaru", UNK, NO_CODE, NO_COUNTRY),
    w("JS2", "Suzuki Motor Corporation", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("JS3", "Suzuki Motor Corporation (SUV)", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("JSA", "Suzuki Motor Corporation (Europa)", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("JDA", "Daihatsu Motor Co.", "Daihatsu", UNK, NO_CODE, NO_COUNTRY),
    w("JAA", "Isuzu Motors", "Isuzu", UNK, NO_CODE, NO_COUNTRY),
    w("JAC", "Isuzu Motors (SUV)", "Isuzu", UNK, NO_CODE, NO_COUNTRY),

    // --- Korea -----------------------------------------------------------
    w("KMH", "Hyundai Motor Company", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("KM8", "Hyundai Motor Company (SUV)", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("KMF", "Hyundai Motor Company (Transporter)", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("KMT", "Genesis Motor", "Genesis", STD, NO_CODE, NO_COUNTRY),
    w("KMU", "Genesis Motor (SUV)", "Genesis", STD, NO_CODE, NO_COUNTRY),
    w("KNA", "Kia Corporation", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("KND", "Kia Corporation (SUV)", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("KNE", "Kia Corporation (Europa)", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("KLA", "Daewoo Motor", "Daewoo", UNK, NO_CODE, NO_COUNTRY),
    w("KL1", "GM Daewoo / GM Korea", "Chevrolet|Daewoo", STD, NO_CODE, NO_COUNTRY),
    w("KL4", "GM Korea (Buick)", "Buick", STD, NO_CODE, NO_COUNTRY),
    w("KL7", "GM Korea (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("KL8", "GM Korea (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("KPT", "SsangYong Motor", "SsangYong", UNK, NO_CODE, NO_COUNTRY),
    w("KPA", "SsangYong Motor (Pick-up)", "SsangYong", UNK, NO_CODE, NO_COUNTRY),
    w("KNM", "Renault Samsung Motors", "Renault|Nissan", STD, NO_CODE, NO_COUNTRY),

    // --- China, Vietnam, India, South East Asia -------------------------
    w("LRW", "Tesla Gigafactory Shanghai", "Tesla", STD, TESLA, NO_COUNTRY),
    w("LSJ", "SAIC Motor (MG, Roewe)", "MG", STD, NO_CODE, NO_COUNTRY),
    w("LSG", "SAIC-GM", "Buick|Chevrolet|Cadillac", STD, NO_CODE, NO_COUNTRY),
    w("LGX", "BYD Auto", "BYD", STD, NO_CODE, NO_COUNTRY),
    w("LC0", "BYD Auto", "BYD", STD, NO_CODE, NO_COUNTRY),
    w("LPE", "BYD Auto", "BYD", STD, NO_CODE, NO_COUNTRY),
    w("LA9", "BYD Auto", "BYD", STD, NO_CODE, NO_COUNTRY),
    w("L6T", "Zhejiang Geely (Geely, Zeekr, Lynk & Co)", "Geely|Zeekr|Lynk & Co", STD, NO_CODE, NO_COUNTRY),
    w("LB3", "Geely Automobile", "Geely", STD, NO_CODE, NO_COUNTRY),
    w("HJN", "NIO", "NIO", STD, NO_CODE, NO_COUNTRY),
    w("LGW", "Great Wall Motor (GWM, Haval, ORA, Tank, WEY)", "Great Wall|ORA", STD, NO_CODE, NO_COUNTRY),
    w("LVV", "Chery Automobile (Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", STD, NO_CODE, NO_COUNTRY),
    w("PRB", "Chery (Jaecoo)", "Jaecoo", STD, NO_CODE, NO_COUNTRY),
    w("LVX", "Aiways Automobiles", "Aiways", STD, NO_CODE, NO_COUNTRY),
    w("SDB", "Talbot", "Talbot", UNK, NO_CODE, NO_COUNTRY),
    w("VF4", "Talbot", "Talbot", UNK, NO_CODE, NO_COUNTRY),
    w("VSC", "Talbot", "Talbot", UNK, NO_CODE, NO_COUNTRY),
    w("SNT", "Trabant (VEB Sachsenring)", "Trabant", UNK, NO_CODE, NO_COUNTRY),
    w("SNE", "Wartburg (Automobilwerk Eisenach)", "Wartburg", UNK, NO_CODE, NO_COUNTRY),
    w("LNN", "Chery Automobile (Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", STD, NO_CODE, NO_COUNTRY),
    w("LVU", "Chery Automobile (Jetour, Omoda, Jaecoo)", "Chery|Omoda|Jaecoo", STD, NO_CODE, NO_COUNTRY),
    w("L1N", "XPeng Motors", "Xpeng", STD, NO_CODE, NO_COUNTRY),
    w("LMV", "XPeng Motors (G3, made by Haima)", "Xpeng", STD, NO_CODE, NO_COUNTRY),
    w("LJ1", "NIO", "NIO", STD, NO_CODE, NO_COUNTRY),
    w("HLX", "Li Auto", "Li Auto", STD, NO_CODE, NO_COUNTRY),
    w("LW4", "Li Auto", "Li Auto", STD, NO_CODE, NO_COUNTRY),
    w("HXM", "Xiaomi Auto", "Xiaomi", STD, NO_CODE, NO_COUNTRY),
    w("LFZ", "Leapmotor", "Leapmotor", STD, NO_CODE, NO_COUNTRY),
    w("LFB", "FAW Group (Hongqi, Bestune)", "Hongqi", STD, NO_CODE, NO_COUNTRY),
    w("LFP", "FAW Car (Hongqi, Bestune)", "Hongqi", STD, NO_CODE, NO_COUNTRY),
    w("LSF", "SAIC Maxus", "Maxus", STD, NO_CODE, NO_COUNTRY),
    w("LSH", "SAIC Maxus", "Maxus", STD, NO_CODE, NO_COUNTRY),
    w("LSK", "SAIC Maxus", "Maxus", STD, NO_CODE, NO_COUNTRY),
    w("LM8", "Seres (AITO)", "Aito", STD, NO_CODE, NO_COUNTRY),
    w("LDP", "Dongfeng (Voyah)", "Voyah", STD, NO_CODE, NO_COUNTRY),
    w("LS6", "Changan (Deepal, Avatr)", "Avatr", STD, NO_CODE, NO_COUNTRY),
    w("LK6", "SAIC-GM-Wuling", "Wuling", STD, NO_CODE, NO_COUNTRY),
    w("LPA", "Changan PSA (DS)", "DS", STD, NO_CODE, NO_COUNTRY),
    w("LBE", "Beijing Hyundai", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("LFM", "FAW Toyota", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("LHG", "GAC Honda", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("LVH", "Dongfeng Honda", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("LDC", "Dongfeng Peugeot Citroën", "Peugeot|Citroën", STD, NO_CODE, NO_COUNTRY),
    w("LGB", "Dongfeng Nissan", "Nissan", STD, NO_CODE, NO_COUNTRY),
    w("LZW", "SAIC-GM-Wuling", "Wuling", STD, NO_CODE, NO_COUNTRY),
    w("RLL", "VinFast", "VinFast", STD, NO_CODE, NO_COUNTRY),
    w("RLN", "VinFast", "VinFast", STD, NO_CODE, NO_COUNTRY),
    w("MA1", "Mahindra & Mahindra", "Mahindra", UNK, NO_CODE, NO_COUNTRY),
    w("MA3", "Maruti Suzuki", "Suzuki", UNK, NO_CODE, NO_COUNTRY),
    w("MAT", "Tata Motors", "Tata", UNK, NO_CODE, NO_COUNTRY),
    w("MAL", "Hyundai Motor India", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("MAJ", "Ford India", "Ford", UNK, NO_CODE, NO_COUNTRY),
    w("MR0", "Toyota Motor Thailand", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("MMB", "Mitsubishi Motors Thailand", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
    w("MPA", "Isuzu Motors Thailand", "Isuzu", UNK, NO_CODE, NO_COUNTRY),
    w("MRX", "BYD Auto Thailand", "BYD", STD, NO_CODE, NO_COUNTRY),
    w("PL1", "Proton", "Proton", UNK, NO_CODE, NO_COUNTRY),

    // --- USA: General Motors --------------------------------------------
    w("1G", "General Motors USA", "Chevrolet|GMC|Buick|Cadillac|Pontiac|Oldsmobile|Saturn", STD, NO_CODE, NO_COUNTRY),
    w("1G1", "General Motors (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("1GC", "General Motors (Chevrolet Truck)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("1GN", "General Motors (Chevrolet SUV)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("1GB", "General Motors (Chevrolet Nutzfahrzeug)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("1G2", "General Motors (Pontiac)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("1GM", "General Motors (Pontiac MPV)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("1G3", "General Motors (Oldsmobile)", "Oldsmobile", STD, NO_CODE, NO_COUNTRY),
    w("1G4", "General Motors (Buick)", "Buick", STD, NO_CODE, NO_COUNTRY),
    w("1G6", "General Motors (Cadillac)", "Cadillac", STD, NO_CODE, NO_COUNTRY),
    w("1GY", "General Motors (Cadillac SUV)", "Cadillac", STD, NO_CODE, NO_COUNTRY),
    w("1G8", "General Motors (Saturn)", "Saturn", STD, NO_CODE, NO_COUNTRY),
    w("1GT", "General Motors (GMC Truck)", "GMC", STD, NO_CODE, NO_COUNTRY),
    w("1GK", "General Motors (GMC SUV)", "GMC", STD, NO_CODE, NO_COUNTRY),
    w("1GD", "General Motors (GMC Nutzfahrzeug)", "GMC", STD, NO_CODE, NO_COUNTRY),
    w("2G", "General Motors of Canada", "Chevrolet|Pontiac|Buick|GMC", STD, NO_CODE, NO_COUNTRY),
    w("2G1", "General Motors of Canada (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("2G2", "General Motors of Canada (Pontiac)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("2G4", "General Motors of Canada (Buick)", "Buick", STD, NO_CODE, NO_COUNTRY),
    w("2GN", "General Motors of Canada (Chevrolet SUV)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("2GK", "General Motors of Canada (GMC SUV)", "GMC", STD, NO_CODE, NO_COUNTRY),
    w("2CN", "CAMI Automotive (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("3G", "General Motors de México", "Chevrolet|GMC|Buick|Cadillac|Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("3G1", "General Motors de México (Chevrolet)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("3GN", "General Motors de México (Chevrolet SUV)", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("3G2", "General Motors de México (Pontiac)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("3G7", "General Motors de México (Pontiac SUV)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("3GS", "General Motors de México (Saturn SUV)", "Saturn", STD, NO_CODE, NO_COUNTRY),
    w("5GA", "General Motors (Buick SUV)", "Buick", STD, NO_CODE, NO_COUNTRY),
    w("5GR", "General Motors (Hummer H2)", "Hummer", STD, NO_CODE, NO_COUNTRY),
    w("5GT", "General Motors (Hummer H3)", "Hummer", STD, NO_CODE, NO_COUNTRY),
    w("5GN", "General Motors (Hummer H3T)", "Hummer", STD, NO_CODE, NO_COUNTRY),
    w("5GZ", "General Motors (Saturn SUV)", "Saturn", STD, NO_CODE, NO_COUNTRY),
    w("5Y2", "NUMMI (Pontiac Vibe)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("137", "AM General (Hummer H1)", "Hummer", STD, NO_CODE, NO_COUNTRY),
    w("6G1", "General Motors-Holden", "Holden|Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("6G2", "General Motors-Holden (Pontiac)", "Pontiac", STD, NO_CODE, NO_COUNTRY),
    w("6H8", "General Motors-Holden", "Holden", UNK, NO_CODE, NO_COUNTRY),
    w("9BG", "General Motors do Brasil", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("8AG", "General Motors de Argentina", "Chevrolet", STD, NO_CODE, NO_COUNTRY),
    w("KL", "GM Daewoo / GM Korea", "Chevrolet|Daewoo", STD, NO_CODE, NO_COUNTRY),

    // --- USA: Ford --------------------------------------------------------
    w("1F", "Ford Motor Company", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1FA", "Ford Motor Company (Pkw)", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1FM", "Ford Motor Company (SUV, MPV)", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1FT", "Ford Motor Company (Truck)", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1FD", "Ford Motor Company (Nutzfahrzeug)", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1FU", "Freightliner", "Freightliner", STD, NO_CODE, NO_COUNTRY),
    w("1FV", "Freightliner", "Freightliner", STD, NO_CODE, NO_COUNTRY),
    w("1ZV", "AutoAlliance International (Ford)", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("2F", "Ford Motor Company of Canada", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("3F", "Ford de México", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("1L", "Ford Motor Company (Lincoln)", "Lincoln", STD, NO_CODE, NO_COUNTRY),
    w("5L", "Ford Motor Company (Lincoln SUV)", "Lincoln", STD, NO_CODE, NO_COUNTRY),
    w("2LM", "Ford Motor Company of Canada (Lincoln)", "Lincoln", STD, NO_CODE, NO_COUNTRY),
    w("1ME", "Ford Motor Company (Mercury)", "Mercury", STD, NO_CODE, NO_COUNTRY),
    w("2ME", "Ford Motor Company of Canada (Mercury)", "Mercury", STD, NO_CODE, NO_COUNTRY),
    w("4M2", "Ford Motor Company (Mercury SUV)", "Mercury", STD, NO_CODE, NO_COUNTRY),
    w("9BF", "Ford do Brasil", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("8AF", "Ford Argentina", "Ford", STD, NO_CODE, NO_COUNTRY),
    w("6FP", "Ford Australia", "Ford", UNK, NO_CODE, NO_COUNTRY),
    w("AFA", "Ford South Africa", "Ford", UNK, NO_CODE, NO_COUNTRY),

    // --- USA: Chrysler, Dodge, Jeep, Plymouth, Ram ----------------------
    w("1C3", "Chrysler (Pkw)", "Chrysler|Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1C4", "Chrysler (MPV, SUV)", "Jeep|Chrysler|Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1C6", "Chrysler (Ram Truck)", "Ram|Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1C8", "Chrysler (MPV)", "Chrysler", STD, NO_CODE, NO_COUNTRY),
    w("1B3", "Dodge (Pkw)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1B4", "Dodge (MPV)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1B7", "Dodge (Truck)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1D3", "Dodge (Ram Truck)", "Dodge|Ram", STD, NO_CODE, NO_COUNTRY),
    w("1D4", "Dodge (SUV)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("1D7", "Dodge (Ram Truck)", "Dodge|Ram", STD, NO_CODE, NO_COUNTRY),
    w("1J4", "Jeep (SUV)", "Jeep", STD, NO_CODE, NO_COUNTRY),
    w("1J8", "Jeep (SUV)", "Jeep", STD, NO_CODE, NO_COUNTRY),
    w("1P3", "Plymouth (Pkw)", "Plymouth", STD, NO_CODE, NO_COUNTRY),
    w("1P4", "Plymouth (MPV)", "Plymouth", STD, NO_CODE, NO_COUNTRY),
    w("2B3", "Chrysler Canada (Dodge)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("2B4", "Chrysler Canada (Dodge MPV)", "Dodge", STD, NO_CODE, NO_COUNTRY),
    w("2C3", "Chrysler Canada (Pkw)", "Chrysler|Dodge", STD, NO_CODE, NO_COUNTRY),
    w("2C4", "Chrysler Canada (MPV)", "Chrysler|Dodge", STD, NO_CODE, NO_COUNTRY),
    w("2P4", "Chrysler Canada (Plymouth MPV)", "Plymouth", STD, NO_CODE, NO_COUNTRY),
    w("2A4", "Chrysler Canada (MPV)", "Chrysler", STD, NO_CODE, NO_COUNTRY),
    w("3C4", "Chrysler de México (MPV)", "Chrysler|Dodge|Jeep", STD, NO_CODE, NO_COUNTRY),
    w("3C6", "Chrysler de México (Ram)", "Ram", STD, NO_CODE, NO_COUNTRY),
    w("3D7", "Chrysler de México (Ram)", "Dodge|Ram", STD, NO_CODE, NO_COUNTRY),

    // --- USA, Canada, Mexico: other makers ------------------------------
    w("1HG", "Honda of America", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("19X", "Honda Manufacturing of Indiana", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("19U", "Honda of America (Acura)", "Acura", STD, NO_CODE, NO_COUNTRY),
    w("2HG", "Honda of Canada", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("2HK", "Honda of Canada (SUV)", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("2HN", "Honda of Canada (Acura SUV)", "Acura", STD, NO_CODE, NO_COUNTRY),
    w("3HG", "Honda de México", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("5FN", "Honda Manufacturing of Alabama", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("5J6", "Honda of America (SUV)", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("5J8", "Honda of America (Acura SUV)", "Acura", STD, NO_CODE, NO_COUNTRY),
    w("7FA", "Honda Manufacturing of Indiana (SUV)", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("4T1", "Toyota Motor Manufacturing Kentucky", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("4T3", "Toyota Motor Manufacturing (SUV)", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("4T4", "Toyota Motor Manufacturing", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("5TD", "Toyota Motor Manufacturing Indiana", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("5TF", "Toyota Motor Manufacturing Texas", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("5TB", "Toyota Motor Manufacturing (Truck)", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("5YF", "Toyota Motor Manufacturing Mississippi", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("1NX", "NUMMI (Toyota)", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("2T1", "Toyota Motor Manufacturing Canada", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("2T2", "Toyota Motor Manufacturing Canada (Lexus)", "Lexus", STD, NO_CODE, NO_COUNTRY),
    w("2T3", "Toyota Motor Manufacturing Canada (SUV)", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("3TM", "Toyota Motor Manufacturing de Baja California", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("58A", "Toyota Motor Manufacturing Kentucky (Lexus)", "Lexus", STD, NO_CODE, NO_COUNTRY),
    w("7MU", "Mazda Toyota Manufacturing (Toyota)", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("7MM", "Mazda Toyota Manufacturing (Mazda)", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("1YV", "Mazda (AutoAlliance)", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("4F2", "Mazda (gebaut von Ford, SUV)", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("4F4", "Mazda (gebaut von Ford, Truck)", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("3MZ", "Mazda de México", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("3MD", "Mazda de México", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("3MV", "Mazda de México (SUV)", "Mazda", STD, NO_CODE, NO_COUNTRY),
    w("1N4", "Nissan North America (Pkw)", "Nissan", STD, NO_CODE, NO_COUNTRY),
    w("1N6", "Nissan North America (Truck)", "Nissan", STD, NO_CODE, NO_COUNTRY),
    w("5N1", "Nissan North America (SUV)", "Nissan|Infiniti", STD, NO_CODE, NO_COUNTRY),
    w("3N1", "Nissan Mexicana", "Nissan", STD, NO_CODE, NO_COUNTRY),
    w("3PC", "Nissan Mexicana (Infiniti)", "Infiniti", STD, NO_CODE, NO_COUNTRY),
    w("4A3", "Mitsubishi Motors North America", "Mitsubishi", STD, NO_CODE, NO_COUNTRY),
    w("4A4", "Mitsubishi Motors North America (SUV)", "Mitsubishi", STD, NO_CODE, NO_COUNTRY),
    w("4S3", "Subaru of Indiana", "Subaru", STD, NO_CODE, NO_COUNTRY),
    w("4S4", "Subaru of Indiana (SUV)", "Subaru", STD, NO_CODE, NO_COUNTRY),
    w("2S3", "CAMI Automotive (Suzuki)", "Suzuki", STD, NO_CODE, NO_COUNTRY),
    w("5NP", "Hyundai Motor Manufacturing Alabama", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("5NM", "Hyundai Motor Manufacturing Alabama (SUV)", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("5XX", "Kia Motors Manufacturing Georgia", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("5XY", "Kia Motors Manufacturing Georgia (SUV)", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("3KP", "Kia Motors México", "Kia", STD, NO_CODE, NO_COUNTRY),
    w("5YJ", "Tesla", "Tesla", STD, TESLA, NO_COUNTRY),
    w("7SA", "Tesla", "Tesla", STD, TESLA, NO_COUNTRY),
    w("7G2", "Tesla (Cybertruck, Semi)", "Tesla", STD, NO_CODE, NO_COUNTRY),
    w("7PD", "Rivian Automotive", "Rivian", STD, NO_CODE, NO_COUNTRY),
    w("50E", "Lucid Motors", "Lucid", STD, NO_CODE, NO_COUNTRY),
    w("1M8", "Motor Coach Industries", "MCI", STD, NO_CODE, NO_COUNTRY),

    // --- South America, Africa, Australia: other makers -----------------
    w("9BR", "Toyota do Brasil", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("8AJ", "Toyota Argentina", "Toyota", STD, NO_CODE, NO_COUNTRY),
    w("93H", "Honda Automóveis do Brasil", "Honda", STD, NO_CODE, NO_COUNTRY),
    w("93Y", "Renault do Brasil", "Renault", STD, NO_CODE, NO_COUNTRY),
    w("8A1", "Renault Argentina", "Renault", STD, NO_CODE, NO_COUNTRY),
    w("936", "Peugeot Citroën do Brasil (Peugeot)", "Peugeot", STD, NO_CODE, NO_COUNTRY),
    w("935", "Peugeot Citroën do Brasil (Citroën)", "Citroën", STD, NO_CODE, NO_COUNTRY),
    w("8AD", "Peugeot Citroën Argentina (Peugeot)", "Peugeot", STD, NO_CODE, NO_COUNTRY),
    w("94D", "Nissan do Brasil", "Nissan", STD, NO_CODE, NO_COUNTRY),
    w("9BH", "Hyundai Motor Brasil", "Hyundai", STD, NO_CODE, NO_COUNTRY),
    w("AHT", "Toyota South Africa", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("6T1", "Toyota Motor Corporation Australia", "Toyota", UNK, NO_CODE, NO_COUNTRY),
    w("6MM", "Mitsubishi Motors Australia", "Mitsubishi", UNK, NO_CODE, NO_COUNTRY),
];

// ISO 3780 sorts the second character in this order; ranges like "AA-AH"
// are meant along it.
const RANGE_ORDER: &str = "ABCDEFGHJKLMNPRSTUVWXYZ1234567890";

fn range_index(c: u8) -> i32 {
    if c == 0 {
        return -1;
    }
    RANGE_ORDER
        .bytes()
        .position(|x| x == c)
        .map_or(-1, |p| p as i32)
}

struct CountryRange {
    first: u8,
    from: u8,
    to: u8,
    name: Text,
}

const fn cr(first: char, from: char, to: char, de: &'static str, en: &'static str) -> CountryRange {
    CountryRange {
        first: first as u8,
        from: from as u8,
        to: to as u8,
        name: Text::new(de, en),
    }
}

#[rustfmt::skip]
static COUNTRIES: &[CountryRange] = &[
    cr('A', 'A', 'H', "Südafrika", "South Africa"),
    cr('A', 'J', 'N', "Elfenbeinküste", "Ivory Coast"),
    cr('C', 'L', 'R', "Tunesien", "Tunisia"),
    cr('D', 'A', 'E', "Ägypten", "Egypt"),
    cr('D', 'F', 'K', "Marokko", "Morocco"),
    cr('F', 'F', 'K', "Nigeria", "Nigeria"),
    cr('H', 'A', '0', "China", "China"),
    cr('J', 'A', '0', "Japan", "Japan"),
    cr('K', 'A', 'E', "Sri Lanka", "Sri Lanka"),
    cr('K', 'F', 'K', "Israel", "Israel"),
    cr('K', 'L', 'R', "Südkorea", "South Korea"),
    cr('K', 'S', '0', "Kasachstan", "Kazakhstan"),
    cr('L', 'A', '0', "China", "China"),
    cr('M', 'A', 'E', "Indien", "India"),
    cr('M', 'F', 'K', "Indonesien", "Indonesia"),
    cr('M', 'L', 'R', "Thailand", "Thailand"),
    cr('N', 'A', 'E', "Iran", "Iran"),
    cr('N', 'F', 'K', "Pakistan", "Pakistan"),
    cr('N', 'L', 'R', "Türkei", "Turkey"),
    cr('P', 'A', 'E', "Philippinen", "Philippines"),
    cr('P', 'F', 'K', "Singapur", "Singapore"),
    cr('P', 'L', 'R', "Malaysia", "Malaysia"),
    cr('R', 'A', 'E', "Vereinigte Arabische Emirate", "United Arab Emirates"),
    cr('R', 'F', 'K', "Taiwan", "Taiwan"),
    cr('R', 'L', 'R', "Vietnam", "Vietnam"),
    cr('R', 'S', '0', "Saudi-Arabien", "Saudi Arabia"),
    cr('S', 'A', 'M', "Vereinigtes Königreich", "United Kingdom"),
    cr('S', 'N', 'T', "Deutschland (ehemals DDR)", "Germany (former East Germany)"),
    cr('S', 'U', 'Z', "Polen", "Poland"),
    cr('S', '1', '4', "Lettland", "Latvia"),
    cr('T', 'A', 'H', "Schweiz", "Switzerland"),
    cr('T', 'J', 'P', "Tschechien", "Czech Republic"),
    cr('T', 'R', 'V', "Ungarn", "Hungary"),
    cr('T', 'W', '1', "Portugal", "Portugal"),
    cr('U', 'H', 'M', "Dänemark", "Denmark"),
    cr('U', 'N', 'T', "Irland", "Ireland"),
    cr('U', 'U', 'X', "Rumänien", "Romania"),
    cr('U', '5', '7', "Slowakei", "Slovakia"),
    cr('V', 'A', 'E', "Österreich", "Austria"),
    cr('V', 'F', 'R', "Frankreich", "France"),
    cr('V', 'S', 'W', "Spanien", "Spain"),
    cr('V', 'X', '2', "Frankreich oder Serbien", "France or Serbia"),
    cr('V', '3', '5', "Kroatien", "Croatia"),
    cr('V', '6', '0', "Estland", "Estonia"),
    cr('W', 'A', '0', "Deutschland", "Germany"),
    cr('X', 'A', 'E', "Bulgarien", "Bulgaria"),
    cr('X', 'F', 'K', "Griechenland", "Greece"),
    cr('X', 'L', 'R', "Niederlande", "Netherlands"),
    cr('X', 'S', 'W', "Russland", "Russia"),
    cr('X', 'X', '2', "Luxemburg", "Luxembourg"),
    cr('X', '3', '0', "Russland", "Russia"),
    cr('Y', 'A', 'E', "Belgien", "Belgium"),
    cr('Y', 'F', 'K', "Finnland", "Finland"),
    cr('Y', 'S', 'W', "Schweden", "Sweden"),
    cr('Y', 'X', '2', "Norwegen", "Norway"),
    cr('Y', '3', '5', "Belarus", "Belarus"),
    cr('Y', '6', '0', "Ukraine", "Ukraine"),
    cr('Z', 'A', 'R', "Italien", "Italy"),
    cr('Z', 'X', '2', "Slowenien", "Slovenia"),
    cr('Z', '3', '5', "Litauen", "Lithuania"),
    cr('Z', '6', '0', "Russland", "Russia"),
    cr('1', 'A', '0', "USA", "USA"),
    cr('2', 'A', '0', "Kanada", "Canada"),
    cr('3', 'A', 'W', "Mexiko", "Mexico"),
    cr('3', 'X', '7', "Costa Rica", "Costa Rica"),
    cr('4', 'A', '0', "USA", "USA"),
    cr('5', 'A', '0', "USA", "USA"),
    cr('6', 'A', 'W', "Australien", "Australia"),
    cr('7', 'A', 'E', "Neuseeland", "New Zealand"),
    cr('7', 'F', '0', "USA", "USA"),
    cr('8', 'A', 'E', "Argentinien", "Argentina"),
    cr('8', 'F', 'K', "Chile", "Chile"),
    cr('8', 'L', 'R', "Ecuador", "Ecuador"),
    cr('8', 'S', 'W', "Peru", "Peru"),
    cr('8', 'X', '2', "Venezuela", "Venezuela"),
    cr('9', 'A', 'E', "Brasilien", "Brazil"),
    cr('9', 'F', 'J', "Kolumbien", "Colombia"),
    cr('9', 'S', 'V', "Uruguay", "Uruguay"),
    cr('9', '3', '9', "Brasilien", "Brazil"),
];

// Values of the characters for the check digit (49 CFR 565.15).
fn transliterate(c: u8) -> i32 {
    match c {
        b'0'..=b'9' => i32::from(c - b'0'),
        b'A' | b'J' => 1,
        b'B' | b'K' | b'S' => 2,
        b'C' | b'L' | b'T' => 3,
        b'D' | b'M' | b'U' => 4,
        b'E' | b'N' | b'V' => 5,
        b'F' | b'W' => 6,
        b'G' | b'P' | b'X' => 7,
        b'H' | b'Y' => 8,
        b'R' | b'Z' => 9,
        _ => -1, // I, O, Q and everything else
    }
}

const WEIGHTS: [i32; 17] = [8, 7, 6, 5, 4, 3, 2, 10, 0, 9, 8, 7, 6, 5, 4, 3, 2];

/// The calendar year of the system clock (UTC).
fn system_year() -> i32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(month <= 2)) as i32
}

fn split_brands(list: &str) -> Vec<String> {
    list.split('|')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

/// Bytes `start..start + len` of `text` as a string (shorter at the end).
fn substr(text: &str, start: usize, len: usize) -> String {
    let b = text.as_bytes();
    if start >= b.len() {
        return String::new();
    }
    let end = start.saturating_add(len).min(b.len());
    String::from_utf8_lossy(&b[start..end]).into_owned()
}

/// The whole WMI table, for statistics and tests.
pub fn wmi_table() -> &'static [WmiEntry] {
    WMI
}

/// Looks up the WMI table: three characters first, then the two character
/// prefix entries. `None` if unknown.
pub fn find_wmi(vin_or_wmi: &str) -> Option<&'static WmiEntry> {
    let text = vin_or_wmi.as_bytes();
    if text.len() < 2 {
        return None;
    }
    let mut key = [0u8; 3];
    for (k, t) in key.iter_mut().zip(text) {
        *k = t.to_ascii_uppercase();
    }
    let mut prefix = None;
    for e in WMI {
        let wmi = e.wmi.as_bytes();
        if wmi.len() == 3 && key[2] != 0 && wmi == key {
            return Some(e);
        }
        if wmi.len() == 2 && wmi == &key[..2] {
            prefix = Some(e);
        }
    }
    prefix
}

/// Country from the first two characters (ISO 3780 ranges).
pub fn vin_country(vin: &str) -> String {
    let b = vin.as_bytes();
    if b.len() < 2 {
        return String::new();
    }
    let first = b[0].to_ascii_uppercase();
    let second = range_index(b[1].to_ascii_uppercase());
    if second < 0 {
        return String::new();
    }
    for r in COUNTRIES {
        if r.first != first {
            continue;
        }
        if second >= range_index(r.from) && second <= range_index(r.to) {
            return r.name.get().to_string();
        }
    }
    l("nicht zugeordnet", "not assigned").to_string()
}

/// Continent from the first character.
pub fn vin_region(vin: &str) -> String {
    let b = vin.as_bytes();
    if b.is_empty() {
        return String::new();
    }
    let c = b[0].to_ascii_uppercase();
    let text = match c {
        b'A'..=b'H' => l("Afrika", "Africa"),
        b'J'..=b'R' => l("Asien", "Asia"),
        b'S'..=b'Z' => l("Europa", "Europe"),
        b'1'..=b'5' => l("Nordamerika", "North America"),
        b'6' => l("Ozeanien", "Oceania"),
        b'7' => {
            // 7A to 7E is New Zealand, the rest of the range went to the USA.
            let s = b.get(1).map_or(0, u8::to_ascii_uppercase);
            if (b'A'..=b'E').contains(&s) {
                l("Ozeanien", "Oceania")
            } else {
                l("Nordamerika", "North America")
            }
        }
        b'8' | b'9' => l("Südamerika", "South America"),
        _ => l("unbekannt", "unknown"),
    };
    text.to_string()
}

/// Model year for a position 10 code: 1980-2009 or the same year 30 later.
/// Returns 0 for characters that are no year code (0, I, O, Q, U, Z).
pub fn vin_base_year(code: char) -> i32 {
    const CODES: &str = "ABCDEFGHJKLMNPRSTVWXY123456789";
    if code == '\0' {
        return 0;
    }
    CODES.find(code).map_or(0, |p| 1980 + p as i32)
}

/// Calculated check digit after the North American rule ('0'-'9' or 'X'),
/// '\0' if the text is not 17 valid characters.
pub fn vin_check_digit(vin: &str) -> char {
    let b = vin.as_bytes();
    if b.len() != 17 {
        return '\0';
    }
    let mut sum = 0;
    for (c, weight) in b.iter().zip(WEIGHTS) {
        let v = transliterate(c.to_ascii_uppercase());
        if v < 0 {
            return '\0';
        }
        sum += v * weight;
    }
    let rest = sum % 11;
    if rest == 10 {
        'X'
    } else {
        char::from(b'0' + rest as u8)
    }
}

/// Decodes a VIN. `current_year` limits the model year guess (a car cannot be
/// newer than next year's model); 0 takes the system clock.
pub fn decode_vin(input: &str, current_year: i32) -> VinInfo {
    let mut info = VinInfo::default();
    let current_year = if current_year <= 0 {
        system_year()
    } else {
        current_year
    };

    // Normalise: upper case, drop the separators people type or copy along.
    let mut bad_chars = false;
    let mut ioq = false;
    let mut normalised = String::new();
    for raw in input.chars() {
        let c = raw.to_ascii_uppercase();
        if matches!(c, ' ' | '-' | '.' | '\t' | '*' | '\r' | '\n') {
            continue;
        }
        if matches!(c, 'I' | 'O' | 'Q') {
            ioq = true;
        } else if !c.is_ascii_alphanumeric() {
            bad_chars = true;
        }
        normalised.push(c);
    }
    info.vin = normalised;
    let v = info.vin.clone();
    let vb = v.as_bytes();

    if v.is_empty() {
        info.problems
            .push(l("Keine Fahrgestellnummer angegeben.", "No VIN given.").to_string());
        return info;
    }
    if v.len() != 17 {
        let text = if v.len() < 17 {
            l(
                "Zu kurz: eine Fahrgestellnummer hat 17 Zeichen. Autos vor 1981 hatten oft kürzere Nummern, \
                 die sich nicht nach dieser Norm deuten lassen.",
                "Too short: a VIN has 17 characters. Cars built before 1981 often had shorter numbers \
                 that cannot be decoded after this standard.",
            )
        } else {
            l(
                "Zu lang: eine Fahrgestellnummer hat 17 Zeichen.",
                "Too long: a VIN has 17 characters.",
            )
        };
        info.problems.push(text.to_string());
    }
    if ioq {
        info.problems.push(
            l(
                "Enthält I, O oder Q. Diese Buchstaben kommen in Fahrgestellnummern nicht vor, \
                 gemeint ist wohl 1 oder 0.",
                "Contains I, O or Q. These letters never appear in a VIN, \
                 probably 1 or 0 is meant.",
            )
            .to_string(),
        );
    }
    if bad_chars {
        info.problems.push(
            l(
                "Enthält Zeichen, die keine Buchstaben oder Ziffern sind.",
                "Contains characters that are neither letters nor digits.",
            )
            .to_string(),
        );
    }
    info.valid = v.len() == 17 && !ioq && !bad_chars;

    // Manufacturer and origin work with the first three characters alone.
    if v.len() >= 3 {
        info.wmi = substr(&v, 0, 3);
    }
    info.country = vin_country(&v);
    info.region = vin_region(&v);
    let first = vb[0];
    let second = vb.get(1).copied().unwrap_or(0);
    let chinese = first == b'L' || first == b'H';
    info.north_america = (b'1'..=b'5').contains(&first)
        || (first == b'7' && range_index(second) >= range_index(b'F'));

    let wmi = if v.len() >= 2 { find_wmi(&v) } else { None };
    if let Some(wmi) = wmi {
        info.manufacturer = wmi.manufacturer.to_string();
        info.brand_candidates = split_brands(wmi.brands);
        if info.brand_candidates.len() == 1 {
            info.brand = info.brand_candidates[0].clone();
        }
        if !wmi.country.de.is_empty() {
            info.country = wmi.country.get().to_string();
        }
    } else if v.len() >= 3 {
        info.notes.push(
            l(
                "Der Hersteller ist in der Tabelle nicht verzeichnet.",
                "The manufacturer is not in the table.",
            )
            .to_string(),
        );
    }

    if v.len() != 17 {
        return info;
    }

    // Check digit. Mandatory in North America and China, optional elsewhere.
    info.check_digit = char::from(vb[8]);
    info.expected_check_digit = vin_check_digit(&v);
    info.check_digit_ok =
        info.expected_check_digit != '\0' && info.check_digit == info.expected_check_digit;
    info.check_digit_applies = info.north_america || chinese;
    if info.check_digit_applies && !info.check_digit_ok && info.valid {
        info.problems.push(
            l(
                "Die Prüfziffer an Stelle 9 stimmt nicht. Wahrscheinlich ist ein Zeichen \
                 falsch abgeschrieben.",
                "The check digit at position 9 does not match. Probably a character \
                 was copied wrongly.",
            )
            .to_string(),
        );
    } else if !info.check_digit_applies && info.valid {
        if info.check_digit_ok {
            info.notes.push(
                l(
                    "Die Prüfziffer an Stelle 9 stimmt. Das spricht für ein Fahrzeug, das für \
                     Nordamerika gebaut wurde.",
                    "The check digit at position 9 matches. That suggests a vehicle built \
                     for North America.",
                )
                .to_string(),
            );
        } else {
            info.notes.push(
                l(
                    "Die Prüfziffer passt nicht, ist bei Fahrzeugen für Europa aber nicht \
                     vorgeschrieben. Das ist kein Fehler.",
                    "The check digit does not match, but it is not required for vehicles \
                     built for Europe. That is not an error.",
                )
                .to_string(),
            );
        }
    }
    // A matching check digit outside North America almost always means a
    // car built to US rules, including the position 7 rule below.
    let us_rules = info.north_america || (!chinese && info.check_digit_ok);

    info.plant = char::from(vb[10]);
    info.serial = substr(&v, 11, usize::MAX);

    // Maker specific model codes.
    if let Some(wmi) = wmi {
        match wmi.code_rule {
            VinModelCodeRule::Vag78 | VinModelCodeRule::Porsche78 => {
                if substr(&v, 6, 2) != "ZZ" {
                    info.model_code = substr(&v, 6, 2);
                }
            }
            VinModelCodeRule::Tesla4 => info.model_code = substr(&v, 3, 1),
            VinModelCodeRule::Mercedes456 => {
                // European layout: three digits of the model series. North
                // American VINs use letter codes there, which we leave alone.
                if is_digit(vb[3]) && is_digit(vb[4]) && is_digit(vb[5]) {
                    info.model_code = substr(&v, 3, 3);
                }
            }
            VinModelCodeRule::None => {}
        }
        if !info.model_code.is_empty() {
            info.model_code_rule = wmi.code_rule;
        }
    }

    // Model year from position 10.
    info.year_code = char::from(vb[9]);
    let mut rule = match wmi {
        Some(w) => w.year_rule,
        None if info.north_america || chinese => STD,
        None => UNK,
    };
    if us_rules || chinese {
        rule = STD;
    }
    if wmi.is_some_and(|w| w.code_rule == MB) && !info.model_code.is_empty() && !us_rules {
        // Mercedes-Benz writes no model year into European VINs.
        rule = NO_YEAR;
    }

    let base = vin_base_year(info.year_code);
    if rule == NO_YEAR {
        info.notes.push(
            l(
                "Dieser Hersteller schreibt bei Fahrzeugen für Europa kein Baujahr in die \
                 Fahrgestellnummer. Es steht im Fahrzeugschein.",
                "This manufacturer writes no model year into VINs for Europe. It is in \
                 the registration papers.",
            )
            .to_string(),
        );
        return info;
    }
    if base == 0 {
        info.notes.push(
            l(
                "Stelle 10 enthält kein Baujahr. In Europa ist das erlaubt, das Baujahr steht \
                 dann nur im Fahrzeugschein.",
                "Position 10 holds no model year. That is allowed in Europe, the year is \
                 then only in the registration papers.",
            )
            .to_string(),
        );
        return info;
    }

    let max_year = current_year + 1;
    if us_rules {
        // US rule since model year 2010: a letter at position 7 marks the
        // second cycle (2010-2039), a digit the first one (1980-2009).
        let second_cycle = !is_digit(vb[6]);
        info.model_years
            .push(if second_cycle { base + 30 } else { base });
        info.model_year_certain = true;
    } else {
        if base + 30 <= max_year {
            info.model_years.push(base + 30);
        }
        info.model_years.push(base);
        info.model_year_certain = rule == STD && info.model_years.len() == 1;
        if rule == UNK {
            info.notes.push(
                l(
                    "Ob dieser Hersteller an Stelle 10 das Baujahr schreibt, ist nicht sicher. \
                     Das Baujahr ist daher nur eine Vermutung.",
                    "It is not certain that this manufacturer puts the model year at \
                     position 10. The year is therefore only a guess.",
                )
                .to_string(),
            );
        }
    }
    info.model_year = info.model_years[0];
    info
}
