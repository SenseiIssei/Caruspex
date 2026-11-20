//! Internal layout of the generated fault code tables (tools/gen_dtc.py).
//!
//! The tables are plain constant slices sorted by code, so a lookup is a
//! binary search and nothing has to be built at startup. Only the database
//! and the generated files use these types.

use super::{CostClass, DtcSystem};
use crate::generated::{dtc_brands, dtc_details, dtc_generic_cbu, dtc_generic_p, dtc_phrases};
use crate::severity::Severity;
use caruspex_i18n::Text;

pub const NO_DETAIL: u16 = 0xFFFF;
/// `how` of a composed text without a fault type.
pub const NO_PHRASE: u16 = 0xFFFF;

/// How a record names its fault.
#[derive(Clone, Copy, Debug)]
pub enum Desc {
    /// The whole text, written out.
    Plain(Text),
    /// Composed from the phrase table: the part (`what`, with `%1`, `%2`
    /// filled from `args`, separated by '|') and the fault type (`how`),
    /// as "part - fault type", or with `lead` as "fault type: part" (lost
    /// communication and the like). Most codes of a family share both
    /// phrases, so each phrase is translated once instead of once per code.
    Parts {
        what: u16,
        args: &'static str,
        how: u16,
        lead: bool,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub code: &'static str,
    pub desc: Desc,
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
        desc: Desc::Plain(Text::new(de, en)),
        severity,
        system,
        detail,
        throttle,
    }
}

/// Shorthand for a composed record, see `Desc::Parts`.
#[allow(clippy::too_many_arguments)]
pub const fn q(
    code: &'static str,
    what: u16,
    args: &'static str,
    how: u16,
    lead: bool,
    severity: Severity,
    system: DtcSystem,
    detail: u16,
) -> Record {
    Record {
        code,
        desc: Desc::Parts {
            what,
            args,
            how,
            lead,
        },
        severity,
        system,
        detail,
        throttle: false,
    }
}

impl Record {
    /// The description in the active language.
    pub fn text(&self) -> String {
        match self.desc {
            Desc::Plain(text) => text.get().to_string(),
            Desc::Parts {
                what,
                args,
                how,
                lead,
            } => {
                let table = phrases();
                let part = fill(table[what as usize].get(), args);
                if how == NO_PHRASE {
                    return part;
                }
                let how = table[how as usize].get();
                if lead {
                    format!("{how}: {part}")
                } else {
                    format!("{part} - {how}")
                }
            }
        }
    }
}

/// Puts the arguments into `%1` to `%9` of a phrase.
fn fill(phrase: &str, args: &str) -> String {
    if args.is_empty() {
        return phrase.to_string();
    }
    let args: Vec<&str> = args.split('|').collect();
    let mut out = String::with_capacity(phrase.len() + 8);
    let mut chars = phrase.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%'
            && let Some(d) = chars.peek().and_then(|d| d.to_digit(10))
            && (1..=args.len() as u32).contains(&d)
        {
            chars.next();
            out.push_str(args[d as usize - 1]);
            continue;
        }
        out.push(c);
    }
    out
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

/// Parts and fault types the composed records are built from.
pub fn phrases() -> &'static [Text] {
    dtc_phrases::PHRASES
}
