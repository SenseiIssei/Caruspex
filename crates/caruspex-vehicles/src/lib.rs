//! VIN decoding and the vehicle catalog.

#![forbid(unsafe_code)]

pub mod anatomy;
pub mod catalog;
#[rustfmt::skip]
mod generated {
    pub mod catalog;
}
pub mod vehicles;
pub mod vin;
