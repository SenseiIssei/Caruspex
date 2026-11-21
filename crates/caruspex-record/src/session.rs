//! The extended diagnostic session for custom values that need it.
//!
//! Some manufacturer values are only given out in the extended diagnostic
//! session of ISO 14229 (10 03): OBDb reads the measured soot mass and the
//! distance and time since the last regeneration of the Volkswagen group
//! diesels that way. [`ExtendedSessions`] keeps that session per control
//! unit across the ticks of the live view and gives it back:
//!
//! - It is entered (10 03) only for a unit whose value is being read, and
//!   kept with 3E 00 only when more than [`KEEP_ALIVE_MS`] passed since the
//!   last request to it; reading the values resets the unit's S3 timer anyway.
//!   3E 80 (no answer) would make the ELM327 wait for its own timeout on every
//!   keep-alive, so the short answer of 3E 00 is cheaper.
//! - [`ExtendedSessions::leave_all`] sends 10 01 to every unit it put into the
//!   session. The worker calls it when no such value is read any more, on
//!   stop and disconnect; a request without an answer leaves the unit at once.
//! - A refusal (7F 10 22 "conditions not correct", for example while driving)
//!   is not asked again for [`RETRY_MS`]; until then the values say why they
//!   are missing.
//! - Every request of this path goes through
//!   [`caruspex_protocols::Uds::session_request`], whose allow list knows only
//!   10 01, 10 03, 3E and 22. Nothing that writes, unlocks or starts anything
//!   can be sent this way.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use caruspex_data::ecu_addresses::EcuDefinition;
use caruspex_i18n::l;
use caruspex_link::Elm327;
use caruspex_protocols::{Uds, UdsRefusal};

/// Without a request for this long a unit gets 3E 00 before the next values.
pub const KEEP_ALIVE_MS: u64 = 2000;
/// After this long without a request the unit has most likely left the
/// session by itself (S3 is 5 s); it is entered again instead of kept.
pub const ASSUME_LEFT_MS: u64 = 4500;
/// A refused session is not asked for again before this.
pub const RETRY_MS: u64 = 30_000;
/// Timeout of the session requests.
const SESSION_TIMEOUT_MS: u64 = 1000;

/// Milliseconds since some fixed point; injectable for tests.
pub type MsClock = Arc<dyn Fn() -> u64 + Send + Sync>;

#[derive(Clone)]
struct Unit {
    ecu: EcuDefinition,
    active: bool,
    last_ms: u64,
    /// Not asked again before this while refused.
    retry_at_ms: u64,
    reason: String,
}

/// The control units in the extended session, kept between ticks.
pub struct ExtendedSessions {
    enabled: bool,
    off_reason: String,
    units: BTreeMap<String, Unit>,
    clock: MsClock,
}

impl Default for ExtendedSessions {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtendedSessions {
    /// Enabled, with the real clock.
    pub fn new() -> Self {
        let start = Instant::now();
        Self::with_clock(Arc::new(move || start.elapsed().as_millis() as u64))
    }

    pub fn with_clock(clock: MsClock) -> Self {
        Self {
            enabled: true,
            off_reason: String::new(),
            units: BTreeMap::new(),
            clock,
        }
    }

    /// Switches the use of the session on or off. Off, the values that need
    /// it show `reason` instead.
    pub fn set_enabled(&mut self, enabled: bool, reason: impl Into<String>) {
        self.enabled = enabled;
        self.off_reason = reason.into();
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn off_reason(&self) -> String {
        if self.off_reason.is_empty() {
            l(
                "Dieser Wert braucht die erweiterte Diagnosesitzung, die ausgeschaltet ist.",
                "This value needs the extended diagnostic session, which is switched off.",
            )
            .to_string()
        } else {
            self.off_reason.clone()
        }
    }

    /// Whether some control unit is in the extended session now.
    pub fn any_active(&self) -> bool {
        self.units.values().any(|u| u.active)
    }

    /// Whether the unit with this address key ("7E0/7E8") is in it.
    pub fn is_active(&self, key: &str) -> bool {
        self.units.get(key).is_some_and(|u| u.active)
    }

    fn now(&self) -> u64 {
        (self.clock)()
    }

    /// Makes sure the selected unit is in the extended session: enters it,
    /// or keeps it with 3E 00 after a pause. `Err` with a plain reason when
    /// that is refused or was refused a short while ago.
    pub(crate) fn prepare(
        &mut self,
        uds: &mut Uds,
        key: &str,
        ecu: &EcuDefinition,
    ) -> Result<(), String> {
        let now = self.now();
        let unit = self.units.entry(key.to_string()).or_insert_with(|| Unit {
            ecu: ecu.clone(),
            active: false,
            last_ms: 0,
            retry_at_ms: 0,
            reason: String::new(),
        });
        if !unit.active && !unit.reason.is_empty() && now < unit.retry_at_ms {
            return Err(unit.reason.clone());
        }
        if unit.active && now.saturating_sub(unit.last_ms) > ASSUME_LEFT_MS {
            unit.active = false;
        }
        if unit.active && now.saturating_sub(unit.last_ms) > KEEP_ALIVE_MS {
            match uds.session_request(&[0x3E, 0x00], SESSION_TIMEOUT_MS) {
                Ok(_) => {
                    unit.last_ms = now;
                    return Ok(());
                }
                // Gone: entered again below.
                Err(_) => unit.active = false,
            }
        }
        if unit.active {
            return Ok(());
        }
        match uds.session_request(&[0x10, 0x03], SESSION_TIMEOUT_MS) {
            Ok(_) => {
                unit.active = true;
                unit.last_ms = now;
                unit.reason.clear();
                Ok(())
            }
            Err(refusal) => {
                if refusal.nrc.is_none() {
                    // No answer at all: make sure nothing stays behind.
                    let _ = uds.session_request(&[0x10, 0x01], SESSION_TIMEOUT_MS);
                }
                unit.active = false;
                unit.retry_at_ms = now + RETRY_MS;
                unit.reason = refusal_reason(&refusal);
                Err(unit.reason.clone())
            }
        }
    }

    /// The unit answered a request just now.
    pub(crate) fn touch(&mut self, key: &str) {
        let now = self.now();
        if let Some(unit) = self.units.get_mut(key) {
            unit.last_ms = now;
        }
    }

    /// The unit refused a value as if it had left the session (S3 ran out).
    pub(crate) fn mark_left(&mut self, key: &str) {
        if let Some(unit) = self.units.get_mut(key) {
            unit.active = false;
        }
    }

    /// A request in the session got no answer: back to the default session
    /// at once, and not again for a while.
    pub(crate) fn give_up(&mut self, uds: &mut Uds, key: &str) {
        let now = self.now();
        if let Some(unit) = self.units.get_mut(key) {
            let _ = uds.session_request(&[0x10, 0x01], SESSION_TIMEOUT_MS);
            unit.active = false;
            unit.retry_at_ms = now + RETRY_MS;
            unit.reason = l(
                "Das Steuergerät antwortet in der erweiterten Diagnosesitzung nicht mehr; sie ist beendet. Ein neuer Versuch folgt in 30 Sekunden.",
                "The control unit stopped answering in the extended diagnostic session, which is now ended. Another try follows in 30 seconds.",
            )
            .to_string();
        }
    }

    /// Sends 10 01 to every unit in the extended session and forgets them,
    /// refusals included, so the next reading starts fresh. The adapter is
    /// back on the default OBD addressing afterwards.
    pub fn leave_all(&mut self, elm: &mut Elm327) {
        let active: Vec<EcuDefinition> = self
            .units
            .values()
            .filter(|u| u.active)
            .map(|u| u.ecu.clone())
            .collect();
        self.units.clear();
        if active.is_empty() || !elm.connected() {
            return;
        }
        let mut uds = Uds::new(elm);
        for ecu in active {
            if uds.select(&ecu).is_ok() {
                let _ = uds.session_request(&[0x10, 0x01], SESSION_TIMEOUT_MS);
            }
        }
        uds.restore_default();
    }
}

fn refusal_reason(refusal: &UdsRefusal) -> String {
    let retry = l(
        "Ein neuer Versuch folgt in 30 Sekunden.",
        "Another try follows in 30 seconds.",
    );
    match refusal.nrc {
        Some(0x22) => format!(
            "{} {retry}",
            l(
                "Das Steuergerät lässt die erweiterte Diagnosesitzung gerade nicht zu (Bedingungen nicht erfüllt, etwa während der Fahrt).",
                "The control unit does not allow the extended diagnostic session right now (conditions not correct, for example while driving).",
            )
        ),
        Some(nrc) => format!(
            "{} {} {retry}",
            l(
                "Die erweiterte Diagnosesitzung wurde abgelehnt.",
                "The extended diagnostic session was refused.",
            ),
            caruspex_protocols::uds_refusal_text(nrc)
        ),
        None => format!(
            "{} {retry}",
            l(
                "Keine Antwort auf den Wechsel in die erweiterte Diagnosesitzung.",
                "No answer to the switch into the extended diagnostic session.",
            )
        ),
    }
}
