//! Health check, battery test, readiness, trip computer, performance timer,
//! used car check, and the health and used vehicle checks of trucks.
//!
//! Everything in this crate is pure computation: the caller collects the data
//! and hands it in, the modules only judge it. Texts are built as
//! [`common::Phrase`] values (both languages side by side) so a result can
//! follow a later language switch.
#![forbid(unsafe_code)]

pub mod battery_test;
pub mod common;
pub mod drive_cycle_guide;
pub mod health_check;
pub mod inputs;
pub mod performance_timer;
pub mod readiness;
mod series;
pub mod trip_computer;
pub mod truck_health;
pub mod truck_used;
pub mod used_car;
