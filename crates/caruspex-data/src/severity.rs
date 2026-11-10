//! How urgent something is, shared by the fault code database and the health
//! check so both rate a finding the same way.

use caruspex_i18n::l;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Severity {
    /// Worth knowing, nothing to do.
    Info,
    /// Fix it some time, the car is fine to drive.
    Low,
    /// Fix it soon, consumption or emissions suffer.
    #[default]
    Medium,
    /// Get it looked at in the next days, damage can follow.
    High,
    /// Stop driving, risk of engine damage or to safety.
    Critical,
}

/// Short label for the traffic light, in the active language.
pub fn severity_label(s: Severity) -> &'static str {
    match s {
        Severity::Info => l("Hinweis", "Note"),
        Severity::Low => l("Gering", "Low"),
        Severity::Medium => l("Mittel", "Medium"),
        Severity::High => l("Hoch", "High"),
        Severity::Critical => l("Kritisch", "Critical"),
    }
}
