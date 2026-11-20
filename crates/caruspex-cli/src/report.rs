//! `report`: the inspection report of the app (app/src/report/report.ts)
//! as a PDF, written by caruspex-pdf without a web view.
//!
//! Languages: the PDF uses the standard fonts with WinAnsiEncoding (see
//! caruspex-pdf's encoding module for why nothing is embedded). Eight of
//! the 18 languages fit into it completely; for the others the report is
//! written in English with a one-line note on stderr, instead of a page
//! full of question marks. A test checks the eight against their catalogs.

use caruspex_analysis::common::Level;
use caruspex_data::dtc::lookup_dtc;
use caruspex_data::dtc_location::locate_dtc;
use caruspex_data::severity::{Severity, severity_label};
use caruspex_i18n::{Language, l, language_info, languages};
use caruspex_link::Elm327;
use caruspex_pdf::{A4_HEIGHT, A4_WIDTH, Cell, Color, Document, Flow, Font, Margins, Style, Table};
use caruspex_protocols::{MonitorResult, Obd2};
use caruspex_record::JsonWriter;
use caruspex_vehicles::catalog;

use crate::commands::{HealthRun, run_health};
use crate::out::{flush, say, say_err};

/// The languages whose letters WinAnsi has: German, English and the
/// Western European ones, and Indonesian (plain Latin letters).
pub const PDF_LANGUAGES: [Language; 8] = [
    Language::German,
    Language::English,
    Language::Spanish,
    Language::French,
    Language::Portuguese,
    Language::Italian,
    Language::Dutch,
    Language::Indonesian,
];

/// The report language for `--lang`: English if none is given, an error
/// for an unknown code, and English with a note for a language the PDF
/// fonts cannot show.
pub fn report_language(code: &str) -> Result<(Language, Option<String>), String> {
    if code.is_empty() {
        return Ok((Language::English, None));
    }
    let short = code.to_ascii_lowercase();
    let Some(info) = languages()
        .iter()
        .find(|i| i.code == short.split(['-', '_']).next().unwrap_or(""))
    else {
        let codes: Vec<&str> = languages().iter().map(|i| i.code).collect();
        return Err(format!(
            "Unknown language: {code} (one of {})",
            codes.join(", ")
        ));
    };
    if PDF_LANGUAGES.contains(&info.id) {
        return Ok((info.id, None));
    }
    Ok((
        Language::English,
        Some(format!(
            "Note: the PDF fonts cannot show {} ({}), so the report is in English.",
            info.native_name, info.code
        )),
    ))
}

// The colours of the app's report.
const INK: Color = Color::hex(0x141821);
const MUTED: Color = Color::hex(0x515B6B);
const ACCENT: Color = Color::hex(0x3B3FB8);
const BRAND: Color = Color::hex(0x5A54F0);
const HEAD_FILL: Color = Color::hex(0xEEF0F6);
const RULE: Color = Color::hex(0xE2E5EE);
const GREEN: Color = Color::hex(0x12875A);
const AMBER: Color = Color::hex(0xB97A06);
const RED: Color = Color::hex(0xC93636);
const GREY: Color = Color::hex(0x6B7280);

const BODY: Style = Style::new(Font::Helvetica, 9.5, INK);
const BOLD: Style = Style::new(Font::HelveticaBold, 9.5, INK);
const SMALL: Style = Style::new(Font::Helvetica, 8.5, MUTED);
const CODE: Style = Style::new(Font::CourierBold, 9.5, INK);
const HEADING: Style = Style::new(Font::HelveticaBold, 12.5, ACCENT);
const TH: Style = Style::new(Font::HelveticaBold, 8.5, MUTED);

fn muted(style: Style) -> Style {
    Style {
        color: MUTED,
        ..style
    }
}

fn coloured(style: Style, color: Color) -> Style {
    Style { color, ..style }
}

/// One stored, pending or permanent fault.
#[derive(Clone, Debug, Default)]
pub struct ReportFault {
    pub code: String,
    pub kind: String,
    pub text: String,
    pub severity: Severity,
    /// Where on the car, from locate_dtc.
    pub zone: String,
    /// The control unit, when the adapter told it.
    pub ecu: String,
}

#[derive(Clone, Debug)]
pub struct ReportFinding {
    pub level: Level,
    pub title: String,
    pub text: String,
}

#[derive(Clone, Debug, Default)]
pub struct ReportMonitor {
    pub name: String,
    pub component: String,
    pub value: String,
    pub limits: String,
    pub passed: bool,
}

/// Everything the report shows, read from the car first, so the layout can
/// be tested without one.
#[derive(Clone, Debug, Default)]
pub struct ReportData {
    /// "20.11.2026, 14:05" as in the app.
    pub created: String,
    /// "D:20261120140500" for the document info.
    pub creation_date: String,
    pub vehicle_name: String,
    pub description: String,
    pub vin: String,
    pub ecu_name: String,
    pub calibration: String,
    pub odometer_km: Option<f64>,
    pub adapter: String,
    pub connection: String,
    pub protocol: String,
    /// 0 to 100, None without a health check.
    pub score: Option<i32>,
    pub summary: String,
    pub findings: Vec<ReportFinding>,
    pub faults_read: bool,
    pub faults: Vec<ReportFault>,
    pub status_valid: bool,
    pub mil: bool,
    pub readiness: Vec<(String, bool)>,
    pub freeze_code: String,
    pub freeze: Vec<(String, String)>,
    pub monitors: Vec<ReportMonitor>,
}

/// QString::arg(v, 0, 'g', 5) as in the app: five significant digits.
fn g5(v: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    if v == 0.0 {
        return "0".to_string();
    }
    let magnitude = v.abs().log10().floor() as i32;
    let decimals = (4 - magnitude).clamp(0, 10) as usize;
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

fn monitor_row(m: &MonitorResult) -> ReportMonitor {
    let mut limits = Vec::new();
    // ASCII on purpose: WinAnsi has no "less or equal" sign.
    if m.has_minimum {
        limits.push(format!(">= {}", g5(m.minimum)));
    }
    if m.has_maximum {
        limits.push(format!("<= {}", g5(m.maximum)));
    }
    ReportMonitor {
        name: m.name.clone(),
        component: m.component.clone(),
        value: format!("{} {}", g5(m.value), m.unit).trim().to_string(),
        // One limit per line, so they read apart in the narrow column.
        limits: limits.join("\n"),
        passed: m.passed,
    }
}

/// Reads what the report needs. The order of the requests is the one of
/// `health`, then identity, odometer and the first freeze frame.
pub fn collect(obd: &mut Obd2, seconds: i32, progress: &dyn Fn(&str)) -> ReportData {
    let HealthRun {
        status,
        faults,
        faults_error,
        vin,
        monitors,
        report,
        ..
    } = run_health(obd, seconds, progress);

    progress("Reading identification, odometer and freeze frame ...");
    let identity = obd.read_identity();
    let odometer_km = if obd.supported_pids().contains(&0xA6) {
        obd.read_live(0xA6).map(|v| v.value)
    } else {
        None
    };
    let (frame, freeze_code) = obd.read_freeze_frame();

    let elm = obd.elm();
    let now = jiff::Zoned::now();
    let info = &vin.info;
    let vehicle_name = match vin.matched.generation {
        Some(g) => catalog::display_name(g),
        None => vin.brand.clone(),
    };
    let year = if info.model_year > 0 {
        info.model_year.to_string()
    } else {
        String::new()
    };
    let description = [
        info.manufacturer.as_str(),
        year.as_str(),
        info.country.as_str(),
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .copied()
    .collect::<Vec<_>>()
    .join(", ");

    ReportData {
        created: now.strftime("%d.%m.%Y, %H:%M").to_string(),
        creation_date: now.strftime("D:%Y%m%d%H%M%S").to_string(),
        vehicle_name,
        description,
        vin: if vin.vin.is_empty() {
            identity.vin.clone()
        } else {
            vin.vin.clone()
        },
        ecu_name: identity.ecu_name.clone(),
        calibration: identity.calibration_id.clone(),
        odometer_km,
        adapter: elm.version().to_string(),
        connection: elm.transport_name(),
        protocol: elm.protocol_text().to_string(),
        score: Some(report.score),
        summary: report.summary.text(),
        findings: report
            .findings
            .iter()
            .map(|f| {
                let mut text = f.explanation.text();
                let advice = f.advice.text();
                if !advice.is_empty() {
                    if !text.is_empty() {
                        text.push(' ');
                    }
                    text.push_str(&advice);
                }
                ReportFinding {
                    level: f.level,
                    title: f.title.text(),
                    text,
                }
            })
            .collect(),
        faults_read: faults_error.is_empty(),
        faults: faults
            .iter()
            .map(|f| {
                let info = lookup_dtc(&f.code, &vin.brand);
                // A meaning that only one source defines says so.
                let text = match info.source_note() {
                    "" => info.text.clone(),
                    note => format!("{} ({note})", info.text),
                };
                ReportFault {
                    code: f.code.clone(),
                    kind: f.kind.clone(),
                    text,
                    severity: info.severity,
                    zone: locate_dtc(&f.code).name,
                    ecu: f.ecu.clone(),
                }
            })
            .collect(),
        status_valid: status.valid,
        mil: status.mil_on,
        readiness: status
            .readiness
            .iter()
            .map(|r| (r.name.clone(), r.ready))
            .collect(),
        freeze_code,
        freeze: frame
            .iter()
            .map(|v| {
                let value = if v.text.is_empty() {
                    format!("{:.1} {}", v.value, v.unit).trim().to_string()
                } else {
                    v.text.clone()
                };
                (v.name.clone(), value)
            })
            .collect(),
        monitors: monitors.iter().map(monitor_row).collect(),
    }
}

fn score_colour(score: i32) -> Color {
    if score < 0 {
        GREY
    } else if score >= 85 {
        GREEN
    } else if score >= 60 {
        AMBER
    } else {
        RED
    }
}

fn severity_colour(s: Severity) -> Color {
    match s {
        Severity::Critical | Severity::High => RED,
        Severity::Medium => AMBER,
        Severity::Low | Severity::Info => MUTED,
    }
}

/// The level of a finding in the words of the app's report.
fn level_label(level: Level) -> (&'static str, Color) {
    match level {
        Level::Critical => (severity_label(Severity::High), RED),
        Level::Warning => (severity_label(Severity::Medium), AMBER),
        Level::Info => (severity_label(Severity::Info), MUTED),
        Level::Good => (l("In Ordnung", "OK"), GREEN),
    }
}

fn table(widths: &[f32], header: &[&str], rows: Vec<Vec<Cell>>) -> Table {
    Table {
        widths: widths.to_vec(),
        header: header.iter().map(|s| s.to_string()).collect(),
        header_style: TH,
        header_fill: HEAD_FILL,
        rule: RULE,
        rows,
    }
}

fn section(flow: &mut Flow, title: &str, keep: f32) {
    flow.space(14.0);
    flow.heading(title, HEADING, keep);
    flow.space(4.0);
}

fn section_with_table(flow: &mut Flow, title: &str, t: &Table) {
    let keep = flow.table_start_height(t);
    section(flow, title, keep);
    flow.table(t);
}

/// Lays out the report in the active language.
pub fn build(data: &ReportData) -> Document {
    let title = l("Prüfbericht", "Diagnostic report");
    let mut doc = Document::new(&if data.vin.is_empty() {
        format!("Caruspex: {title}")
    } else {
        format!("Caruspex: {title} {}", data.vin)
    });
    doc.producer = format!("Caruspex {}", env!("CARGO_PKG_VERSION"));
    doc.creation_date = data.creation_date.clone();
    let margins = Margins {
        top: 50.0,
        right: 48.0,
        bottom: 56.0,
        left: 48.0,
    };
    let mut flow = Flow::new(doc, A4_WIDTH, A4_HEIGHT, margins);
    let (left, right) = (flow.left(), flow.right());

    // Header: brand, title, date; the score on the right.
    let top = flow.cursor();
    flow.text_spaced(
        left,
        top,
        Style::new(Font::HelveticaBold, 9.0, BRAND),
        2.0,
        "CARUSPEX",
    );
    flow.text_at(
        left,
        top + 13.0,
        Style::new(Font::HelveticaBold, 20.0, INK),
        title,
    );
    flow.text_at(left, top + 41.0, muted(BODY), &data.created);
    if let Some(score) = data.score {
        flow.text_right(
            right,
            top,
            Style::new(Font::HelveticaBold, 30.0, score_colour(score)),
            &score.to_string(),
        );
        flow.text_right(
            right,
            top + 40.0,
            muted(BODY),
            l("Gesundheitscheck", "Health check"),
        );
    }
    flow.set_cursor(top + 56.0);

    // Vehicle
    let mut rows = Vec::new();
    let mut row = |k: &str, v: &str| {
        if !v.is_empty() {
            rows.push(vec![Cell::new(muted(BODY), k), Cell::new(BODY, v)]);
        }
    };
    row(l("Bezeichnung", "Name"), &data.vehicle_name);
    row(l("Beschreibung", "Description"), &data.description);
    row(l("Fahrgestellnummer", "VIN"), &data.vin);
    row(l("Steuergerät", "Control unit"), &data.ecu_name);
    row(l("Kalibrierung", "Calibration"), &data.calibration);
    let odometer = data.odometer_km.map(|km| format!("{km:.0} km"));
    row(
        l("Kilometerstand", "Mileage"),
        odometer.as_deref().unwrap_or(""),
    );
    row(l("Adapter", "Adapter"), &data.adapter);
    row(l("Verbindung", "Connection"), &data.connection);
    row(l("Protokoll", "Protocol"), &data.protocol);
    section_with_table(
        &mut flow,
        l("Fahrzeug", "Vehicle"),
        &table(&[0.32, 0.68], &[], rows),
    );

    // Findings of the health check, with its summary.
    if data.score.is_some() && (!data.summary.is_empty() || !data.findings.is_empty()) {
        let t = table(
            &[0.16, 0.84],
            &[l("Stufe", "Level"), l("Befund", "Finding")],
            data.findings
                .iter()
                .map(|f| {
                    let (label, colour) = level_label(f.level);
                    vec![
                        Cell::new(coloured(BOLD, colour), label),
                        Cell::new(BOLD, &f.title).then(muted(BODY), &f.text),
                    ]
                })
                .collect(),
        );
        section(
            &mut flow,
            l("Befunde", "Findings"),
            3.0 * BODY.line_height(),
        );
        if !data.summary.is_empty() {
            flow.paragraph(&data.summary, BODY);
            flow.space(6.0);
        }
        if !t.rows.is_empty() {
            flow.table(&t);
        }
    }

    // Fault memory
    let faults_title = l("Fehlerspeicher", "Fault memory");
    if !data.faults_read {
        section(&mut flow, faults_title, BODY.line_height());
        flow.paragraph(l("Nicht ausgelesen.", "Not read."), muted(BODY));
    } else if data.faults.is_empty() {
        section(&mut flow, faults_title, BODY.line_height());
        flow.paragraph(l("Kein Fehler gespeichert.", "No faults stored."), BODY);
    } else {
        let t = table(
            &[0.12, 0.15, 0.12, 0.41, 0.20],
            &[
                l("Code", "Code"),
                l("Art", "Type"),
                l("Stufe", "Level"),
                l("Bedeutung", "Meaning"),
                l("Wo", "Where"),
            ],
            data.faults
                .iter()
                .map(|f| {
                    vec![
                        Cell::new(CODE, &f.code),
                        Cell::new(BODY, &f.kind).then(muted(SMALL), &f.ecu),
                        Cell::new(
                            coloured(BODY, severity_colour(f.severity)),
                            severity_label(f.severity),
                        ),
                        Cell::new(BODY, &f.text),
                        Cell::new(muted(BODY), &f.zone),
                    ]
                })
                .collect(),
        );
        section_with_table(&mut flow, faults_title, &t);
    }
    if data.status_valid {
        flow.space(6.0);
        let mil = format!(
            "{}: {}",
            l("Motorkontrollleuchte", "Check engine light"),
            if data.mil {
                l("an", "on")
            } else {
                l("aus", "off")
            }
        );
        flow.paragraph(
            &mil,
            if data.mil {
                coloured(BOLD, AMBER)
            } else {
                BODY
            },
        );
    }

    // Readiness
    if !data.readiness.is_empty() {
        let t = table(
            &[0.6, 0.4],
            &[l("Test", "Test"), l("Stand", "Status")],
            data.readiness
                .iter()
                .map(|(name, ready)| {
                    let (text, colour) = if *ready {
                        (l("abgeschlossen", "complete"), GREEN)
                    } else {
                        (l("offen", "incomplete"), AMBER)
                    };
                    vec![
                        Cell::new(BODY, name),
                        Cell::new(coloured(BODY, colour), text),
                    ]
                })
                .collect(),
        );
        section_with_table(&mut flow, l("Bereitschaftstests", "Readiness monitors"), &t);
    }

    // Freeze frame
    if !data.freeze.is_empty() {
        let t = table(
            &[0.5, 0.5],
            &[],
            data.freeze
                .iter()
                .map(|(name, value)| vec![Cell::new(BODY, name), Cell::new(BODY, value)])
                .collect(),
        );
        let mut keep = flow.table_start_height(&t);
        if !data.freeze_code.is_empty() {
            keep += BODY.line_height();
        }
        section(
            &mut flow,
            l("Umgebungsdaten zum Fehler", "Freeze frame data"),
            keep,
        );
        if !data.freeze_code.is_empty() {
            flow.paragraph(
                &format!(
                    "{} {}",
                    l("Ausgelöst durch", "Triggered by"),
                    data.freeze_code
                ),
                muted(BODY),
            );
            flow.space(2.0);
        }
        flow.table(&t);
    }

    // Component monitoring (mode 06)
    if !data.monitors.is_empty() {
        let t = table(
            &[0.46, 0.18, 0.2, 0.16],
            &[
                l("Prüfung", "Check"),
                l("Wert", "Value"),
                l("Grenzen", "Limits"),
                l("Ergebnis", "Result"),
            ],
            data.monitors
                .iter()
                .map(|m| {
                    let (result, colour) = if m.passed {
                        (l("bestanden", "passed"), GREEN)
                    } else {
                        (l("nicht bestanden", "failed"), RED)
                    };
                    vec![
                        Cell::new(BODY, &m.name).then(muted(SMALL), &m.component),
                        Cell::new(BODY, &m.value),
                        Cell::new(BODY, &m.limits),
                        Cell::new(coloured(BODY, colour), result),
                    ]
                })
                .collect(),
        );
        section_with_table(&mut flow, l("Bauteilprüfung", "Component monitoring"), &t);
    }

    flow.space(18.0);
    flow.paragraph(
        l(
            "Erstellt mit Caruspex, freie Fahrzeugdiagnose. Der Bericht gibt wieder, was die Steuergeräte gemeldet haben. Er ersetzt keine Prüfung in der Werkstatt.",
            "Created with Caruspex, free vehicle diagnostics. The report shows what the control units reported. It does not replace an inspection at a workshop.",
        ),
        SMALL,
    );

    let footer_left = if data.vin.is_empty() {
        format!("Caruspex, {}", data.created)
    } else {
        format!("Caruspex, {}, {}", data.vin, data.created)
    };
    flow.finish(SMALL, RULE, |page, pages| {
        (
            footer_left.clone(),
            l("Seite %1 von %2", "Page %1 of %2")
                .replace("%1", &page.to_string())
                .replace("%2", &pages.to_string()),
        )
    })
}

/// The `report` command: reads the car, writes `path`.
pub fn run(elm: &mut Elm327, path: &str, seconds: i32, as_json: bool, language: Language) -> i32 {
    let progress = |text: &str| {
        if !as_json {
            say(&format!("{text}\n"));
            flush();
        }
    };
    let mut obd = Obd2::new(elm);
    let data = collect(&mut obd, seconds, &progress);
    let doc = build(&data);
    let pages = doc.pages.len();
    let unknown = doc.unknown_chars();
    if let Err(e) = std::fs::write(path, doc.to_bytes()) {
        return crate::commands::fail(as_json, &format!("Cannot write {path}: {e}"), 5);
    }
    if !unknown.is_empty() {
        let list: String = unknown.iter().collect();
        say_err(&format!(
            "Note: {} character(s) are not in the PDF fonts and were written as \"?\": {list}\n",
            unknown.len()
        ));
    }
    if as_json {
        let mut json = JsonWriter::new();
        json.begin_object()
            .field("file", path)
            .field("pages", &(pages as i32))
            .field("language", language_info(language).code)
            .field("faults", &(data.faults.len() as i32))
            .end_object();
        say(&json.text());
    } else {
        say(&format!(
            "\nReport     : {path} ({pages} {})\n",
            if pages == 1 { "page" } else { "pages" }
        ));
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use caruspex_i18n::with_language;
    use caruspex_pdf::encode;

    fn translations() -> std::path::PathBuf {
        [env!("CARGO_MANIFEST_DIR"), "..", "..", "translations"]
            .iter()
            .collect()
    }

    /// Every string value of the JSON files of one language (a tiny reader:
    /// the files are flat objects or lists of flat objects of strings).
    fn catalog_texts(code: &str) -> Vec<String> {
        let mut out = Vec::new();
        let dir = translations().join(code);
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "json") {
                let text = std::fs::read_to_string(&path).unwrap();
                let value: serde_json::Value = serde_json::from_str(&text).unwrap();
                collect_strings(&value, code, &mut out);
            }
        }
        out
    }

    fn collect_strings(value: &serde_json::Value, code: &str, out: &mut Vec<String>) {
        match value {
            serde_json::Value::String(s) => out.push(s.clone()),
            serde_json::Value::Array(items) => {
                for v in items {
                    collect_strings(v, code, out);
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map {
                    // The source files hold German and English side by side,
                    // with ids and contexts that are never printed.
                    if code == "source" && !matches!(k.as_str(), "de" | "en") {
                        continue;
                    }
                    collect_strings(v, code, out);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_pdf_languages_need_no_question_mark() {
        let mut codes = vec!["source"];
        codes.extend(
            PDF_LANGUAGES
                .iter()
                .map(|&l| language_info(l).code)
                .filter(|c| !matches!(*c, "de" | "en")),
        );
        for code in codes {
            let texts = catalog_texts(code);
            assert!(texts.len() > 1000, "{code}: {} texts", texts.len());
            for t in &texts {
                let e = encode(t);
                assert!(e.unknown.is_empty(), "{code}: {:?} in {t:?}", e.unknown);
            }
        }
    }

    #[test]
    fn the_other_languages_really_need_other_letters() {
        for info in languages() {
            if PDF_LANGUAGES.contains(&info.id) {
                continue;
            }
            let missing = catalog_texts(info.code)
                .iter()
                .flat_map(|t| t.chars())
                .filter(|&c| caruspex_pdf::winansi_byte(c).is_none() && !c.is_control())
                .count();
            assert!(
                missing > 100,
                "{}: only {missing} letters outside WinAnsi",
                info.code
            );
        }
    }

    #[test]
    fn language_choice() {
        assert_eq!(report_language("").unwrap(), (Language::English, None));
        assert_eq!(report_language("de").unwrap(), (Language::German, None));
        assert_eq!(
            report_language("pt-BR").unwrap(),
            (Language::Portuguese, None)
        );
        let (lang, note) = report_language("ja").unwrap();
        assert_eq!(lang, Language::English);
        assert!(note.unwrap().contains("(ja)"));
        assert!(report_language("pl").unwrap().1.is_some());
        assert!(
            report_language("xx")
                .unwrap_err()
                .starts_with("Unknown language: xx")
        );
    }

    #[test]
    fn five_significant_digits() {
        assert_eq!(g5(0.0), "0");
        assert_eq!(g5(0.60156), "0.60156");
        assert_eq!(g5(0.8203125), "0.82031");
        assert_eq!(g5(12.5), "12.5");
        assert_eq!(g5(123456.0), "123456");
        assert_eq!(g5(-1.5), "-1.5");
    }

    fn sample(faults: usize) -> ReportData {
        ReportData {
            created: "20.11.2026, 14:05".into(),
            creation_date: "D:20261120140500".into(),
            vehicle_name: "Volvo XC40 (2017)".into(),
            vin: "YV1XZ".into(),
            adapter: "ELM327 v1.5".into(),
            score: Some(42),
            summary: "Something to look at.".into(),
            findings: vec![ReportFinding {
                level: Level::Warning,
                title: "Mixture too lean (bank 1)".into(),
                text: "word ".repeat(80),
            }],
            faults_read: true,
            faults: (0..faults)
                .map(|i| ReportFault {
                    code: format!("P{:04}", 100 + i),
                    kind: "stored".into(),
                    text: "Catalyst system efficiency below threshold, a long text that wraps over several lines in its column".into(),
                    severity: Severity::High,
                    zone: "Exhaust".into(),
                    ecu: String::new(),
                })
                .collect(),
            status_valid: true,
            mil: true,
            readiness: vec![("Catalyst".into(), true), ("EGR system".into(), false)],
            freeze_code: "P0100".into(),
            freeze: vec![("Engine speed".into(), "2261.8 1/min".into())],
            monitors: vec![ReportMonitor {
                name: "Catalyst bank 1".into(),
                value: "0.82".into(),
                limits: "<= 0.602".into(),
                ..ReportMonitor::default()
            }],
            ..ReportData::default()
        }
    }

    #[test]
    fn many_faults_make_more_pages_with_every_code_once() {
        let doc = with_language(Language::English, || build(&sample(80)));
        assert!(doc.pages.len() >= 3, "{} pages", doc.pages.len());
        let all: String = doc.pages.iter().map(|p| p.content()).collect();
        for i in 0..80 {
            assert_eq!(all.matches(&format!("(P{:04}) Tj", 100 + i)).count(), 1);
        }
        let n = doc.pages.len();
        assert!(
            doc.pages[n - 1]
                .content()
                .contains(&format!("(Page {n} of {n}) Tj"))
        );
        assert!(doc.unknown_chars().is_empty());
    }

    #[test]
    fn every_pdf_language_writes_its_labels_without_loss() {
        for &language in &PDF_LANGUAGES {
            let doc = with_language(language, || build(&sample(3)));
            assert!(
                doc.unknown_chars().is_empty(),
                "{}: {:?}",
                language_info(language).code,
                doc.unknown_chars()
            );
        }
        let doc = with_language(Language::German, || build(&sample(1)));
        assert!(doc.pages[0].content().contains("(Pr\\374fbericht) Tj"));
    }
}
