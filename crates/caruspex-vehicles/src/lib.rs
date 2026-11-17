//! VIN decoding and the vehicle catalog.

#![forbid(unsafe_code)]

pub mod anatomy;
pub mod catalog;
pub mod engines;
pub mod moto;
pub mod truck;
#[rustfmt::skip]
mod generated {
    pub mod catalog;
    pub mod dimensions;
    pub mod engines;
}
pub mod vehicles;
pub mod vin;
