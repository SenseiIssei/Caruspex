//! Internal layout of the generated fault code tables (tools/gen_dtc.py).
//!
//! The tables are plain constant slices sorted by code, so a lookup is a
//! binary search and nothing has to be built at startup. Only the database
//! and the generated files use these types.

use super::{CostClass, DtcSystem};
use crate::generated::{dtc_brands, dtc_details, dtc_generic_cbu, dtc_generic_p};
use crate::severity::Severity;
use caruspex_i18n::Text;

pub const NO_DETAIL: u16 = 0xFFFF;

#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub code: &'static str,
    pub text: Text,
    pub severity: Severity,
    pub system: DtcSystem,
    /// Index into `details()`, or `NO_DETAIL`.
    pub detail: u16,
    pub throttle: bool,
}

/// Shorthand the generated tables use, one call per code.
pub const fn r(
    code: &'static str,
    de: &'static str,
    en: &'static str,
    severity: Severity,
    system: DtcSystem,
    detail: u16,
    throttle: bool,
) -> Record {
    Record {
        code,
        text: Text::new(de, en),
        severity,
        system,
        detail,
        throttle,
    }
}

/// Plain-language help shared by many codes. Causes and checks hold one
/// entry per line, causes ordered by likelihood.
#[derive(Clone, Copy, Debug)]
pub struct Detail {
    pub advice: Text,
    pub causes: Text,
    pub checks: Text,
    pub cost: CostClass,
}

#[derive(Clone, Copy, Debug)]
pub struct Family {
    pub key: &'static str,
    pub label: Text,
    /// Normalised brand names separated by '|'.
    pub brands: &'static str,
    pub records: &'static [Record],
}

/// Meaning of a number range for codes that are in no table. `lo` and `hi`
/// are the hex value of the four digits after the letter.
#[derive(Clone, Copy, Debug)]
pub struct RangeRule {
    pub letter: char,
    pub lo: i32,
    pub hi: i32,
    pub text: Text,
    pub severity: Severity,
    pub system: DtcSystem,
    pub detail: u16,
}

pub fn generic_powertrain() -> &'static [Record] {
    dtc_generic_p::POWERTRAIN
}

pub fn generic_chassis() -> &'static [Record] {
    dtc_generic_cbu::CHASSIS
}

pub fn generic_body() -> &'static [Record] {
    dtc_generic_cbu::BODY
}

pub fn generic_network() -> &'static [Record] {
    dtc_generic_cbu::NETWORK
}

pub fn families() -> &'static [Family] {
    dtc_brands::FAMILIES
}

pub fn details() -> &'static [Detail] {
    dtc_details::DETAILS
}

pub fn range_rules() -> &'static [RangeRule] {
    dtc_details::RANGE_RULES
}
