//! Plain advice for car owners, built on top of the data and the analysis
//! crates.
//!
//! - [`next_steps`]: "What should I do now?" Faults, readiness, health check
//!   findings and live values become one list in three steps: stop now,
//!   visit a workshop soon, can wait. Each item says why, roughly what it
//!   costs and what to tell the workshop.
//! - [`connect_help`]: when connecting fails, a short checklist for the kind
//!   of connection, built from what the attempt found out.
//!
//! Like the analysis crate this is pure computation: the caller collects the
//! facts and hands them in. All texts come out in the active language.
#![forbid(unsafe_code)]

pub mod connect_help;
pub mod next_steps;

mod text;

#[cfg(test)]
pub(crate) mod test_support;
