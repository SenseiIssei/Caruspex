// The Tauri commands of app/src/api/contract.ts. Commands that talk to the
// car put a job into the worker's channel and return at once; the answer
// comes as events. Lookups answer directly.
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, Window,
};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::advice::{self, AdviceInputArg};
use crate::garage::{self, GarageCar};
use crate::measure::{BatteryInputArg, SpeedSampleArg, TripConfigArg};
use crate::settings::{self, AppSettings};
use crate::texts::{args, mw};
use crate::worker::Job;
use crate::{knowledge, measure, pdf, update};

type Result<T> = std::result::Result<T, String>;

fn job(state: &State<'_, AppState>, job: Job) {
    state.worker.send(job);
}

// --- Language -----------------------------------------------------------------------

/// "auto" follows the system language when Caruspex has it, English
/// otherwise (Settings.cpp effectiveLanguage).
pub fn effective_language(code: &str) -> String {
    if caruspex_i18n::languages().iter().any(|i| i.code == code) {
        return code.to_string();
    }
    let system = std::env::var("LC_ALL")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let guess = caruspex_i18n::language_info(caruspex_i18n::language_from_code(&system)).code;
    if system.starts_with(guess) {
        guess.to_string()
    } else {
        "en".to_string()
    }
}

pub fn apply_language(code: &str) {
    use caruspex_i18n::{Language, language_from_code, set_language};
    set_language(if code == "de" {
        Language::German
    } else {
        language_from_code(code)
    });
}

// --- App and window ---------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: String,
    has_ble: bool,
    platform: &'static str,
    documents_dir: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<'_, AppState>) -> AppInfo {
    if !state.ready.swap(true, Ordering::SeqCst) {
        start_cli_tasks(&app, &state);
    }
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        has_ble: true,
        platform: if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else {
            "linux"
        },
        documents_dir: app
            .path()
            .document_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
    }
}

#[tauri::command]
pub fn set_language(code: String) {
    apply_language(&code);
}

#[tauri::command]
pub async fn check_for_update(state: State<'_, AppState>) -> Result<Value> {
    let allowed = state
        .settings
        .lock()
        .map(|s| s.check_updates)
        .unwrap_or(false)
        && state.cli.allows_update_check();
    if !allowed {
        return Ok(Value::Null);
    }
    tauri::async_runtime::spawn_blocking(|| update::check(env!("CARGO_PKG_VERSION")))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<()> {
    if !(url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:")) {
        return Err(url);
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

// --- Settings ----------------------------------------------------------------------------

#[tauri::command]
pub fn load_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.lock().map(|s| s.clone()).unwrap_or_default()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    values: AppSettings,
) -> Result<()> {
    let values = values.sanitized();
    settings::save_settings(&state.config_dir, &values)?;
    if let Ok(mut current) = state.settings.lock() {
        *current = values.clone();
    }
    open_log_file(&app, &values);
    job(&state, Job::ApplySettings(values));
    Ok(())
}

#[tauri::command]
pub fn open_config_location(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let path = settings::settings_path(&state.config_dir);
    if path.exists() {
        app.opener()
            .reveal_item_in_dir(path)
            .map_err(|e| e.to_string())
    } else {
        app.opener()
            .open_path(state.config_dir.display().to_string(), None::<&str>)
            .map_err(|e| e.to_string())
    }
}

/// A log line of the interface itself into the log file. The worker's lines
/// (log_line) are written on their way out, so the interface only sends the
/// ones it makes; in Qt all of them went through MainWindow::appendLog.
#[tauri::command]
pub fn write_log(state: State<'_, AppState>, line: String) {
    crate::write_log_line(&state.log_file, &line);
}

/// Opens, switches or closes the log file (MainWindow::applySettings).
pub fn open_log_file(app: &AppHandle, values: &AppSettings) {
    let state = app.state::<AppState>();
    let Ok(mut slot) = state.log_file.lock() else {
        return;
    };
    let want = values.log_to_file && !values.log_path.is_empty();
    let path = PathBuf::from(&values.log_path);
    let changed = slot.as_ref().is_some_and(|(open, _)| *open != path);
    if slot.is_some() && (!want || changed) {
        *slot = None;
    }
    if want && slot.is_none() {
        match OpenOptions::new().create(true).append(true).open(&path) {
            Ok(mut file) => {
                let header = format!(
                    "\n=== Caruspex, {} ===\n",
                    chrono::Local::now().format("%d.%m.%Y %H:%M")
                );
                let _ = file.write_all(header.as_bytes());
                *slot = Some((path, file));
            }
            Err(_) => {
                // Swallowing this silently would be the worst option: the
                // user would otherwise believe the log is being written.
                let line = args(
                    &mw(
                        "Protokolldatei %1 lässt sich nicht schreiben. Es wird nur hier im Fenster protokolliert.",
                        "Cannot write log file %1. Logging only here in the window.",
                    ),
                    &[&values.log_path],
                );
                let _ = app.emit("log_line", json!({ "line": line }));
            }
        }
    }
}

// --- Files -----------------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct FileFilter {
    name: String,
    extensions: Vec<String>,
}

fn file_dialog(
    app: &AppHandle,
    title: &str,
    filters: &[FileFilter],
) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    let mut dialog = app.dialog().file().set_title(title);
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    for f in filters {
        let extensions: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&f.name, &extensions);
    }
    dialog
}

fn ask_save_path(
    app: &AppHandle,
    title: &str,
    suggested: &str,
    filters: &[FileFilter],
) -> Option<PathBuf> {
    let mut dialog = file_dialog(app, title, filters).set_file_name(suggested);
    if let Ok(dir) = app.path().document_dir() {
        dialog = dialog.set_directory(dir);
    }
    dialog.blocking_save_file().and_then(|p| p.into_path().ok())
}

fn ask_open_path(app: &AppHandle, title: &str, filters: &[FileFilter]) -> Option<PathBuf> {
    file_dialog(app, title, filters)
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
}

#[tauri::command]
pub async fn save_text_file(
    app: AppHandle,
    title: String,
    suggested_name: String,
    filters: Vec<FileFilter>,
    content: String,
) -> Result<Option<String>> {
    let Some(path) = ask_save_path(&app, &title, &suggested_name, &filters) else {
        return Ok(None);
    };
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
pub async fn save_pdf(
    app: AppHandle,
    title: String,
    suggested_name: String,
    html: String,
) -> Result<Option<String>> {
    let pdf_filter = [FileFilter {
        name: "PDF".into(),
        extensions: vec!["pdf".into()],
    }];
    let Some(mut path) = ask_save_path(&app, &title, &suggested_name, &pdf_filter) else {
        return Ok(None);
    };
    if path.extension().is_none() {
        path.set_extension("pdf");
    }
    match pdf::write_report(&app, html, path.clone(), true).await? {
        pdf::Written::File => Ok(Some(path.display().to_string())),
        pdf::Written::PrintDialog => Ok(None),
    }
}

/// Writes the report without asking (--report): PDF, or HTML when the path
/// ends in .html or .htm. During a --report run the program ends afterwards.
#[tauri::command]
pub async fn write_report(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
    html: String,
) -> Result<bool> {
    let ok = matches!(
        pdf::write_report(&app, html, PathBuf::from(&path), false).await,
        Ok(pdf::Written::File)
    );
    if state.cli.report.as_deref() == Some(path.as_str()) {
        let text = if ok {
            args(
                &mw("Prüfbericht gespeichert: %1", "Diagnostic report saved: %1"),
                &[&path],
            )
        } else {
            args(
                &mw(
                    "Prüfbericht ließ sich nicht schreiben: %1",
                    "The diagnostic report could not be written: %1",
                ),
                &[&path],
            )
        };
        finish(&app, if ok { 0 } else { 1 }, &text);
    }
    Ok(ok)
}

#[derive(Serialize)]
pub struct OpenedTextFile {
    path: String,
    content: String,
}

#[tauri::command]
pub async fn open_text_file(
    app: AppHandle,
    title: String,
    filters: Vec<FileFilter>,
) -> Result<Option<OpenedTextFile>> {
    let Some(path) = ask_open_path(&app, &title, &filters) else {
        return Ok(None);
    };
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(Some(OpenedTextFile {
        path: path.display().to_string(),
        content: String::from_utf8_lossy(&data).into_owned(),
    }))
}

#[tauri::command]
pub async fn pick_save_path(
    app: AppHandle,
    title: String,
    suggested_name: String,
    filters: Vec<FileFilter>,
) -> Result<Option<String>> {
    Ok(ask_save_path(&app, &title, &suggested_name, &filters).map(|p| p.display().to_string()))
}

#[tauri::command]
pub async fn pick_open_path(
    app: AppHandle,
    title: String,
    filters: Vec<FileFilter>,
) -> Result<Option<String>> {
    Ok(ask_open_path(&app, &title, &filters).map(|p| p.display().to_string()))
}

// --- Garage ----------------------------------------------------------------------------------

#[tauri::command]
pub fn garage_load(state: State<'_, AppState>) -> Vec<GarageCar> {
    garage::load(&state.garage_path)
}

#[tauri::command]
pub fn garage_save(state: State<'_, AppState>, cars: Vec<GarageCar>) -> Result<()> {
    garage::save(&state.garage_path, &cars)
}

// --- Connecting ------------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortInfo {
    device: String,
    description: String,
    likely_adapter: bool,
    chip_guess: String,
}

#[tauri::command]
pub async fn list_serial_ports() -> Result<Vec<SerialPortInfo>> {
    tauri::async_runtime::spawn_blocking(|| {
        caruspex_link::enumerate_ports()
            .into_iter()
            .map(|p| SerialPortInfo {
                device: p.device,
                description: p.description,
                likely_adapter: p.likely_adapter,
                chip_guess: p.chip_guess,
            })
            .collect()
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct J2534DeviceRow {
    name: String,
    vendor: String,
    library: String,
    usable: bool,
}

#[tauri::command]
pub fn list_j2534_devices() -> Vec<J2534DeviceRow> {
    caruspex_net::j2534_devices()
        .into_iter()
        .map(|d| J2534DeviceRow {
            name: d.name,
            vendor: d.vendor,
            library: d.library,
            usable: d.usable,
        })
        .collect()
}

#[tauri::command]
pub fn list_socketcan_interfaces() -> Vec<String> {
    caruspex_net::socket_can_interfaces()
}

#[tauri::command]
pub fn open_serial(state: State<'_, AppState>, device: String, baud: i32, protocol_order: String) {
    job(
        &state,
        Job::OpenSerial {
            device,
            baud,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn open_tcp(state: State<'_, AppState>, host: String, port: i32, protocol_order: String) {
    job(
        &state,
        Job::OpenTcp {
            host,
            port,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn open_demo(state: State<'_, AppState>, protocol_order: String) {
    let _ = protocol_order;
    job(&state, Job::OpenDemo);
}

#[tauri::command]
pub fn open_enet(state: State<'_, AppState>, host: String, protocol_order: String) {
    let _ = protocol_order;
    job(&state, Job::OpenEnet { host });
}

#[tauri::command]
pub fn open_can(state: State<'_, AppState>, spec: String, protocol_order: String) {
    let _ = protocol_order;
    job(&state, Job::OpenCan { spec });
}

#[tauri::command]
pub fn scan_ble(state: State<'_, AppState>, milliseconds: i32) {
    job(&state, Job::ScanBle { milliseconds });
}

#[tauri::command]
pub fn open_ble(
    state: State<'_, AppState>,
    id: String,
    name: String,
    kinds: i32,
    protocol_order: String,
) {
    job(
        &state,
        Job::OpenBle {
            id,
            name,
            kinds,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn open_replay(
    state: State<'_, AppState>,
    path: String,
    real_time: bool,
    protocol_order: String,
) {
    job(
        &state,
        Job::OpenReplay {
            path,
            real_time,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn close_adapter(state: State<'_, AppState>) {
    job(&state, Job::Close);
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>, path: String) {
    job(&state, Job::StartRecording(path));
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) {
    job(&state, Job::StopRecording);
}

// --- Reading -----------------------------------------------------------------------------------

#[tauri::command]
pub fn read_faults(state: State<'_, AppState>) {
    job(&state, Job::ReadFaults);
}

#[tauri::command]
pub fn clear_faults(state: State<'_, AppState>) {
    job(&state, Job::ClearFaults);
}

#[tauri::command]
pub fn read_details(state: State<'_, AppState>) {
    job(&state, Job::ReadDetails);
}

#[tauri::command]
pub fn start_live(state: State<'_, AppState>, pids: Vec<i32>) {
    job(&state, Job::StartLive(pids));
}

#[tauri::command]
pub fn stop_live(state: State<'_, AppState>) {
    job(&state, Job::StopLive);
}

#[tauri::command]
pub fn send_raw(state: State<'_, AppState>, command: String) {
    job(&state, Job::SendRaw(command));
}

#[tauri::command]
pub fn read_monitors(state: State<'_, AppState>) {
    job(&state, Job::ReadMonitors);
}

#[tauri::command]
pub fn run_health_check(state: State<'_, AppState>, seconds: i32) {
    job(&state, Job::HealthCheck(seconds));
}

#[tauri::command]
pub fn probe_adapter(state: State<'_, AppState>) {
    job(&state, Job::ProbeAdapter);
}

#[tauri::command]
pub fn bruteforce_clear(state: State<'_, AppState>, attempts: i32) {
    job(&state, Job::BruteForceClear(attempts));
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.worker.cancel_scan();
}

#[tauri::command]
pub fn start_voltage_poll(state: State<'_, AppState>, interval_ms: i32) {
    job(&state, Job::StartVoltagePoll(interval_ms));
}

#[tauri::command]
pub fn stop_voltage_poll(state: State<'_, AppState>) {
    job(&state, Job::StopVoltagePoll);
}

// --- Manufacturer and user values ------------------------------------------------------------

#[tauri::command]
pub fn read_custom(state: State<'_, AppState>, keys: Vec<i32>) {
    job(&state, Job::ReadCustom(keys));
}

#[tauri::command]
pub fn start_custom_poll(state: State<'_, AppState>, keys: Vec<i32>) {
    job(&state, Job::StartCustomPoll(keys));
}

#[tauri::command]
pub fn import_custom_csv(state: State<'_, AppState>, path: String) {
    job(&state, Job::ImportCustomCsv(path));
}

// --- Measuring -------------------------------------------------------------------------------

#[tauri::command]
pub fn trip_reset(state: State<'_, AppState>, config: TripConfigArg) {
    if let Ok(mut trip) = state.trip.lock() {
        trip.reset(&config);
    }
}

#[tauri::command]
pub fn trip_start(state: State<'_, AppState>) {
    if let Ok(mut trip) = state.trip.lock() {
        trip.start();
    }
}

#[tauri::command]
pub fn trip_stop(state: State<'_, AppState>) -> Value {
    state
        .trip
        .lock()
        .map(|mut t| t.stop())
        .unwrap_or(Value::Null)
}

#[tauri::command]
pub fn trip_summary(state: State<'_, AppState>) -> Value {
    state
        .trip
        .lock()
        .map(|t| t.summary())
        .unwrap_or(Value::Null)
}

#[tauri::command]
pub fn analyze_performance(samples: Vec<SpeedSampleArg>) -> Value {
    measure::analyze_performance(&samples)
}

#[tauri::command]
pub fn evaluate_battery(input: BatteryInputArg) -> Value {
    measure::evaluate_battery(&input)
}

#[tauri::command]
pub fn battery_test_steps() -> Value {
    measure::battery_test_steps()
}

// --- UDS ---------------------------------------------------------------------------------------

#[tauri::command]
pub fn ecu_families() -> Value {
    knowledge::ecu_families()
}

#[tauri::command]
pub fn scan_uds(state: State<'_, AppState>, family: String) {
    job(&state, Job::ScanUds(family));
}

#[tauri::command]
pub fn clear_uds_ecu(
    state: State<'_, AppState>,
    request: u32,
    response: u32,
    extended29: bool,
    name: String,
) {
    job(
        &state,
        Job::ClearUdsEcu {
            request,
            response,
            extended29,
            name,
        },
    );
}

// --- K-line ----------------------------------------------------------------------------------

#[tauri::command]
pub fn open_kkl(state: State<'_, AppState>, device: String, ecu_address: i32) {
    job(
        &state,
        Job::OpenKkl {
            device,
            ecu_address,
        },
    );
}

#[tauri::command]
pub fn close_kkl(state: State<'_, AppState>) {
    job(&state, Job::CloseKkl);
}

#[tauri::command]
pub fn start_basic_setting(state: State<'_, AppState>, group: i32) {
    job(&state, Job::StartBasicSetting(group));
}

#[tauri::command]
pub fn stop_basic_setting(state: State<'_, AppState>) {
    job(&state, Job::StopBasicSetting);
}

#[tauri::command]
pub fn set_basic_setting_session(state: State<'_, AppState>, mode: i32) {
    job(&state, Job::SetBasicSettingSession(mode));
}

#[tauri::command]
pub fn scan_all_ecus(state: State<'_, AppState>, device: String, clear_attempts: i32) {
    job(
        &state,
        Job::ScanAllEcus {
            device,
            clear_attempts,
        },
    );
}

#[tauri::command]
pub fn read_adaptation_channel(state: State<'_, AppState>, channel: i32) {
    job(&state, Job::ReadAdaptation(channel));
}

#[tauri::command]
pub fn run_actuator_test(state: State<'_, AppState>) {
    job(&state, Job::ActuatorTest);
}

#[tauri::command]
pub fn kkl_ecus() -> Value {
    knowledge::kkl_ecus()
}

// --- Knowledge --------------------------------------------------------------------------------

#[tauri::command]
pub fn lookup_dtc(code: String, brand: String) -> Value {
    knowledge::lookup_dtc(&code, &brand)
}

#[tauri::command]
pub fn locate_dtc(code: String) -> Value {
    knowledge::locate_dtc(&code)
}

/// "Was soll ich jetzt tun?" from what the interface knows about the car.
#[tauri::command]
pub fn what_to_do(input: AdviceInputArg) -> Value {
    advice::what_to_do(&input)
}

#[tauri::command]
pub async fn dtc_lexicon() -> Result<Value> {
    Ok(knowledge::dtc_lexicon())
}

#[tauri::command]
pub fn brand_families() -> Value {
    knowledge::brand_families()
}

#[tauri::command]
pub fn brand_family(brand: String) -> String {
    knowledge::brand_family(&brand)
}

#[tauri::command]
pub fn pid_table() -> Value {
    knowledge::pid_table()
}

#[tauri::command]
pub fn default_live_pids() -> Vec<i32> {
    knowledge::default_live_pids()
}

#[tauri::command]
pub fn pid_packs() -> Value {
    knowledge::pid_packs()
}

/// The OBDb packs for the detected car (catalog generation, model year).
#[tauri::command]
pub fn obdb_packs(generation_id: Option<usize>, year: i32) -> Value {
    knowledge::obdb_packs(generation_id, year)
}

#[tauri::command]
pub fn catalog_brands() -> Vec<String> {
    knowledge::catalog_brands()
}

#[tauri::command]
pub fn catalog_models(brand: String) -> Vec<String> {
    knowledge::catalog_models(&brand)
}

#[tauri::command]
pub fn catalog_generations(brand: String, model: String) -> Value {
    knowledge::catalog_generations(&brand, &model)
}

#[tauri::command]
pub fn catalog_search(query: String, limit: usize) -> Value {
    knowledge::catalog_search(&query, limit)
}

#[tauri::command]
pub fn catalog_size() -> usize {
    knowledge::catalog_size()
}

#[tauri::command]
pub fn catalog_diagnosis(generation_id: usize, year: i32) -> Result<Value> {
    knowledge::catalog_diagnosis(generation_id, year)
}

#[tauri::command]
pub fn vehicle_anatomy(
    generation_id: i64,
    year: i32,
    body: i32,
    brand: String,
    model: String,
    powertrain: u32,
) -> Value {
    knowledge::vehicle_anatomy(generation_id, year, body, &brand, &model, powertrain)
}

#[tauri::command]
pub fn decode_vin(vin: String) -> Value {
    knowledge::decode_vin(&vin)
}

#[tauri::command]
pub fn match_vin(vin: String) -> Value {
    knowledge::match_vin(&vin)
}

// --- Command line -------------------------------------------------------------------------------

/// Prints the outcome of an unattended run and ends the program.
fn finish(app: &AppHandle, code: i32, message: &str) {
    if code == 0 {
        println!("{message}");
    } else {
        eprintln!("{message}");
    }
    app.exit(code);
}

fn later(delay_ms: u64, f: impl FnOnce() + Send + 'static) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(delay_ms));
        f();
    });
}

/// What main.cpp did once the window was up: connect, show a page, take a
/// picture, write the report. Runs when the interface first asks for
/// app_info, because by then it listens to the events.
fn start_cli_tasks(app: &AppHandle, state: &State<'_, AppState>) {
    let cli = state.cli.clone();
    let values = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let worker = state.worker.clone();

    // --demo goes through the page (?demo), like in the browser preview.
    let connect = if let Some(device) = &cli.port {
        let device = if device.eq_ignore_ascii_case("auto") {
            String::new()
        } else {
            device.clone()
        };
        Some(Job::OpenSerial {
            device,
            baud: values.baud_override,
            protocol_order: values.protocol_order.clone(),
        })
    } else if let Some(address) = &cli.tcp {
        let (host, port) = address
            .rsplit_once(':')
            .unwrap_or((address.as_str(), "35000"));
        Some(Job::OpenTcp {
            host: host.to_string(),
            port: port.parse().unwrap_or(35000),
            protocol_order: values.protocol_order.clone(),
        })
    } else if let Some(host) = &cli.enet {
        let host = if host.eq_ignore_ascii_case("auto") {
            String::new()
        } else {
            host.clone()
        };
        Some(Job::OpenEnet { host })
    } else if let Some(spec) = &cli.can {
        let _ = app.emit(
            "log_line",
            json!({ "line": args(&mw("Verbinde über das CAN-Interface %1 ...", "Connecting over the CAN interface %1 ..."), &[spec]) }),
        );
        Some(Job::OpenCan { spec: spec.clone() })
    } else if let Some(id) = &cli.bluetooth {
        let id = if id.eq_ignore_ascii_case("auto") {
            String::new()
        } else {
            id.clone()
        };
        // A device given by address: offer both ways, Bluetooth LE first,
        // then classic RFCOMM (ConnectPage::selectBluetooth). The search
        // (auto) finds both kinds itself.
        Some(Job::OpenBle {
            id,
            name: String::new(),
            kinds: 3,
            protocol_order: values.protocol_order.clone(),
        })
    } else {
        cli.replay.as_ref().map(|path| Job::OpenReplay {
            path: path.clone(),
            real_time: false,
            protocol_order: values.protocol_order.clone(),
        })
    };
    if let Some(connect) = connect {
        let w = worker.clone();
        later(300, move || w.send(connect));
    }

    if let Some(page) = cli.page.clone()
        && cli.connects()
    {
        // Give a connection a moment, it jumps to the start page when done.
        let app = app.clone();
        later(2600, move || {
            let _ = app.emit("cli_page", json!({ "page": page }));
        });
    }

    if let Some(file) = cli.shot.clone() {
        let app = app.clone();
        later(cli.shot_delay_ms, move || {
            let result =
                tauri::async_runtime::block_on(pdf::screenshot(&app, PathBuf::from(&file)));
            match result {
                Ok(()) => finish(&app, 0, &file),
                Err(error) => finish(&app, 1, &error),
            }
        });
    }

    if cli.report.is_some() {
        let app = app.clone();
        later(120_000, move || {
            finish(
                &app,
                3,
                &mw(
                    "Keine Verbindung nach zwei Minuten, abgebrochen.",
                    "No connection after two minutes, cancelled.",
                ),
            );
        });
    }
}

/// The worker's events that the command line waits for (reportAndQuit).
pub fn on_worker_event(app: &AppHandle, event: &str, payload: &Value) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Some(path) = state.cli.report.clone() else {
        return;
    };
    match event {
        "connected" => {
            // The readout (status, faults, vehicle data) follows the
            // connection; give it time before the report is written.
            let app = app.clone();
            later(state.cli.report_wait_ms, move || {
                let _ = app.emit("cli_report", json!({ "path": path }));
            });
        }
        "failed" => finish(app, 2, payload["error"].as_str().unwrap_or("")),
        _ => {}
    }
}

// --- Window geometry ------------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    maximized: bool,
}

fn geometry_path(config_dir: &Path) -> PathBuf {
    config_dir.join("window.json")
}

pub fn restore_geometry(
    window: &WebviewWindow,
    config_dir: &Path,
    values: &AppSettings,
    cli: &crate::cli::Cli,
) {
    if cli.size.is_some() || cli.window_pos.is_some() {
        // Placed by the command line already (tauri.conf.json overridden).
        return;
    }
    if !values.remember_geometry {
        return;
    }
    let Some(g) = std::fs::read_to_string(geometry_path(config_dir))
        .ok()
        .and_then(|t| serde_json::from_str::<Geometry>(&t).ok())
    else {
        return;
    };
    // Only where a screen still is: a monitor may have gone since.
    let visible = window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .any(|m| {
            let p = m.position();
            let s = m.size();
            g.x >= p.x && g.y >= p.y && g.x < p.x + s.width as i32 && g.y < p.y + s.height as i32
        });
    if !visible {
        return;
    }
    let _ = window.set_position(PhysicalPosition::new(g.x, g.y));
    let _ = window.set_size(PhysicalSize::new(g.width, g.height));
    if g.maximized {
        let _ = window.maximize();
    }
}

pub fn save_geometry(window: &Window) {
    let app = window.app_handle();
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let remember = state
        .settings
        .lock()
        .map(|s| s.remember_geometry)
        .unwrap_or(false);
    if !remember || state.cli.size.is_some() || state.cli.window_pos.is_some() {
        return;
    }
    let maximized = window.is_maximized().unwrap_or(false);
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let g = Geometry {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
        maximized,
    };
    if let Ok(text) = serde_json::to_string(&g) {
        let _ = std::fs::create_dir_all(&state.config_dir);
        let _ = std::fs::write(geometry_path(&state.config_dir), text);
    }
}
