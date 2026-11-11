// "Was soll ich jetzt tun?" and the connection assistant (crate
// caruspex-advice): the arguments from the interface in, the answers out as
// the JSON of AdviceReport and ConnectHelp in api/types.ts.
use caruspex_advice::connect_help::{CheckState, ConnectHelp};
use caruspex_advice::next_steps::{
    AdviceInput, AdviceReport, Cost, FaultInput, FindingInput, ReadinessInput, Source, Urgency,
    what_to_do as compute,
};
use caruspex_analysis::common::Level;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdviceFaultArg {
    pub code: String,
    pub kind: String,
    pub stored: bool,
    pub ecu: String,
    pub text: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdviceFindingArg {
    pub id: String,
    /// analysis::Level: 0 good, 1 info, 2 warning, 3 critical.
    pub level: i32,
    pub title: String,
    pub explanation: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdviceReadinessArg {
    pub name: String,
    pub ready: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdviceLiveArg {
    pub pid: i32,
    pub value: f64,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AdviceInputArg {
    pub brand: String,
    pub faults_read: bool,
    pub faults: Vec<AdviceFaultArg>,
    pub mil_on: Option<bool>,
    pub readiness: Vec<AdviceReadinessArg>,
    pub findings: Vec<AdviceFindingArg>,
    pub live: Vec<AdviceLiveArg>,
    pub mil_flashing: bool,
    pub red_warning_lamp: bool,
}

fn level(value: i32) -> Level {
    match value {
        3 => Level::Critical,
        2 => Level::Warning,
        1 => Level::Info,
        _ => Level::Good,
    }
}

fn urgency(u: Urgency) -> i32 {
    match u {
        Urgency::StopNow => 0,
        Urgency::Soon => 1,
        Urgency::CanWait => 2,
    }
}

fn cost(c: Cost) -> i32 {
    match c {
        Cost::Unknown => 0,
        Cost::Free => 1,
        Cost::Cheap => 2,
        Cost::Medium => 3,
        Cost::Expensive => 4,
    }
}

fn source(s: Source) -> &'static str {
    match s {
        Source::Observation => "observation",
        Source::Fault => "fault",
        Source::Finding => "finding",
        Source::LiveValue => "live",
        Source::WarningLamp => "lamp",
        Source::Readiness => "readiness",
    }
}

pub fn input(arg: &AdviceInputArg) -> AdviceInput {
    AdviceInput {
        brand: arg.brand.clone(),
        faults_read: arg.faults_read,
        faults: arg
            .faults
            .iter()
            .map(|f| FaultInput {
                code: f.code.clone(),
                kind: f.kind.clone(),
                stored: f.stored,
                ecu: f.ecu.clone(),
                text: f.text.clone(),
            })
            .collect(),
        mil_on: arg.mil_on,
        readiness: arg
            .readiness
            .iter()
            .map(|r| ReadinessInput {
                name: r.name.clone(),
                ready: r.ready,
            })
            .collect(),
        findings: arg
            .findings
            .iter()
            .map(|f| FindingInput {
                id: f.id.clone(),
                level: level(f.level),
                title: f.title.clone(),
                explanation: f.explanation.clone(),
            })
            .collect(),
        live: arg.live.iter().map(|v| (v.pid, v.value)).collect(),
        mil_flashing: arg.mil_flashing,
        red_warning_lamp: arg.red_warning_lamp,
    }
}

pub fn report_json(r: &AdviceReport) -> Value {
    json!({
        "headline": r.headline,
        "summary": r.summary,
        "nothingRead": r.nothing_read,
        "items": r.items.iter().map(|i| json!({
            "urgency": urgency(i.urgency),
            "source": source(i.source),
            "code": i.code,
            "title": i.title,
            "why": i.why,
            "cost": cost(i.cost),
            "costText": i.cost_text,
            "workshop": i.workshop,
            "evidence": i.evidence,
            "page": i.page,
        })).collect::<Vec<_>>(),
    })
}

pub fn what_to_do(arg: &AdviceInputArg) -> Value {
    report_json(&compute(&input(arg)))
}

pub fn help_json(h: &ConnectHelp) -> Value {
    json!({
        "found": h.found,
        "items": h.items.iter().map(|i| json!({
            "id": i.id,
            "state": match i.state {
                CheckState::Problem => "problem",
                CheckState::Check => "check",
                CheckState::Fine => "fine",
            },
            "title": i.title,
            "text": i.text,
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_arguments_arrive() {
        let arg: AdviceInputArg = serde_json::from_str(
            r#"{"faultsRead": true, "milOn": true,
                "faults": [{"code": "P0171", "kind": "gespeichert", "stored": true}],
                "readiness": [{"name": "Katalysator", "ready": false}],
                "live": [{"pid": 5, "value": 90}]}"#,
        )
        .unwrap();
        let input = input(&arg);
        assert_eq!(input.faults[0].code, "P0171");
        assert_eq!(input.mil_on, Some(true));
        assert_eq!(input.live, [(5, 90.0)]);
        let out = report_json(&compute(&input));
        assert_eq!(out["items"][0]["code"], "P0171");
        assert_eq!(out["items"][0]["urgency"], 1);
        assert_eq!(out["items"][1]["source"], "readiness");
        assert_eq!(out["items"][1]["cost"], 1);
    }
}
