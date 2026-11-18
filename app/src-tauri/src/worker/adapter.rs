// The adapter itself: raw commands, the adapter test and the voltage on
// pin 16.
use caruspex_link::adapter_probe::version_text;
use caruspex_link::probe_adapter;
use serde_json::json;

use super::{Worker, app_seconds};
use crate::rows::*;
use crate::texts::w;

impl Worker {
    pub(super) fn send_raw(&mut self, command: &str) {
        let result = self.elm.send(command, 8000);
        let reply = if result.lines.is_empty() {
            w("(keine Antwort)", "(no reply)")
        } else {
            result.lines.join("\n")
        };
        self.emit("raw_reply", json!({ "reply": reply }));
    }

    pub(super) fn probe_adapter(&mut self) {
        if !self.elm.connected() {
            self.failed(w(
                "Für den Adaptertest erst verbinden.",
                "Connect first to run the adapter test.",
            ));
            return;
        }
        self.stop_live();
        self.voltage_timer.stop();
        self.busy(true);
        let report = probe_adapter(&mut self.elm);
        let row = AdapterRow {
            reachable: report.reachable,
            version: report.version_text.clone(),
            description: report.description.clone(),
            is_stn: report.is_stn,
            stn_id: report.stn_id.clone(),
            voltage: report.voltage,
            has_voltage: report.has_voltage,
            round_trip_ms: report.round_trip_ms,
            round_trip_with_hint_ms: report.round_trip_with_hint_ms,
            behaves_like: if report.behaves_like_level > 0 {
                version_text(report.behaves_like_level)
            } else {
                String::new()
            },
            accepts_unknown: report.accepts_unknown_commands,
            verdict: report.verdict.get(),
            facts: report.facts.iter().map(|f| f.get()).collect(),
            speed_tips: report.speed_tips.iter().map(|t| t.get()).collect(),
            checks: report
                .checks
                .iter()
                .map(|c| AdapterCheckRow {
                    command: c.command.clone(),
                    version: version_text(c.introduced_in),
                    supported: c.supported,
                    purpose: c.purpose.get(),
                })
                .collect(),
        };
        self.emit("adapter_report", json!({ "row": row }));
        self.busy(false);
    }

    pub(super) fn start_voltage_poll(&mut self, interval_ms: i32) {
        if !self.elm.connected() {
            return;
        }
        self.stop_live();
        self.voltage_timer.set_interval(interval_ms.max(20) as u64);
        self.voltage_timer.start();
    }

    pub(super) fn voltage_tick(&mut self) {
        let result = self.elm.send("ATRV", 1000);
        for line in &result.lines {
            let text: String = line.trim().chars().filter(|&c| c != 'V').collect();
            if let Ok(volts) = text.trim().parse::<f64>()
                && volts > 3.0
                && volts < 32.0
            {
                self.emit(
                    "adapter_voltage",
                    json!({ "volts": volts, "time": app_seconds() }),
                );
                break;
            }
        }
    }
}
