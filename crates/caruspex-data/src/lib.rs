//! Static data: trouble codes, PIDs, control unit addresses and the plain
//! texts that belong to them. VIN decoding and the vehicle catalog live in
//! `caruspex-vehicles`.
#![forbid(unsafe_code)]

pub mod dtc;
pub mod dtc_location;
pub mod ecu_addresses;
pub mod failure_types;
#[rustfmt::skip]
mod generated;
pub mod monitor_tests;
pub mod pids;
pub mod severity;
pub mod vag_faults;

#[cfg(test)]
pub(crate) mod test_support;
