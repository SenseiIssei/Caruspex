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
use tauri::{AppHandle, Emitter, Manager, State, Window};
#[cfg(desktop)]
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::AppState;
use crate::advice::{self, AdviceInputArg};
use crate::garage::{self, GarageCar};
use crate::measure::{BatteryInputArg, SpeedSampleArg, TripConfigArg};
use crate::rows::ConnectRoute;
use crate::settings::{self, AppSettings};
use crate::texts::{args, mw};
use crate::worker::Job;
use crate::{knowledge, measure, pdf, update};

type Result<T> = std::result::Result<T, String>;

fn job(state: &State<'_, AppState>, job: Job) {
    state.worker.send(job);
}

/// A connect job, after the way that worked last time for this car when
/// the interface knows one. `truck`: the J1939 bit rate in kbit/s (0 for
/// both in turn) when the interface connects to a truck, `None` for cars.
fn connect_job(
    state: &State<'_, AppState>,
    remembered: Option<ConnectRoute>,
    truck: Option<u32>,
    then: Job,
) {
    job(state, Job::SetTruck(truck));
    match remembered {
        Some(route) => job(
            state,
            Job::OpenRemembered {
                route,
                then: Box::new(then),
            },
        ),
        None => job(state, then),
    }
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
        } else if cfg!(target_os = "android") {
            "android"
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

/// Size of everything in the window (the font size of the settings), the
/// way the zoom of a browser does it, so charts and the X-ray view keep
/// their mouse positions.
#[tauri::command]
pub fn set_zoom(app: AppHandle, scale: f64) -> Result<()> {
    let scale = if scale.is_finite() {
        scale.clamp(0.8, 1.6)
    } else {
        1.0
    };
    match app.get_webview_window("main") {
        Some(window) => window.set_zoom(scale).map_err(|e| e.to_string()),
        None => Ok(()),
    }
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
    let found = tauri::async_runtime::spawn_blocking(|| update::check(env!("CARGO_PKG_VERSION")))
        .await
        .map_err(|e| e.to_string())?;
    let Some(found) = found else {
        return Ok(Value::Null);
    };
    let can_install = found.package.is_some() && update::this_method() != update::Method::PageOnly;
    let german = caruspex_i18n::language() == caruspex_i18n::Language::German;
    let info = json!({
        "version": found.version,
        "url": if can_install { found.page() } else { update::fallback_url(&found) },
        "notes": update::notes_in(&found.notes, german),
        "date": found.date,
        "canInstall": can_install,
    });
    // install_update takes the package from here, never from the interface.
    if let Ok(mut slot) = state.update.lock() {
        *slot = Some(found);
    }
    Ok(info)
}

/// Downloads the release found by check_for_update, checks its signature
/// and installs it. Answers "quit" (the app ends a moment later and the new
/// version starts) or "page" (this copy cannot be replaced, the download
/// is open in the browser). Progress comes as update_progress events.
#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, AppState>) -> Result<String> {
    let found = state
        .update
        .lock()
        .ok()
        .and_then(|slot| slot.clone())
        .ok_or_else(|| {
            caruspex_i18n::l(
                "Es wurde noch keine neue Version gefunden.",
                "No new version has been found yet.",
            )
            .to_string()
        })?;
    if state.installing.swap(true, Ordering::SeqCst) {
        return Err(caruspex_i18n::l(
            "Die Aktualisierung läuft schon.",
            "The update is already running.",
        )
        .to_string());
    }
    let emitter = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut last = (0u64, std::time::Instant::now());
        let mut progress = |received: u64, total: u64| {
            // A few updates per second are enough for the bar.
            let done = total > 0 && received >= total;
            if done || received == 0 || last.1.elapsed() >= Duration::from_millis(100) {
                last = (received, std::time::Instant::now());
                let _ = emitter.emit(
                    "update_progress",
                    json!({ "received": received, "total": total }),
                );
            }
        };
        update::install(&found, &mut progress).map(|outcome| (outcome, found))
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    state.installing.store(false, Ordering::SeqCst);
    let (outcome, found) = result?;
    match outcome {
        update::Outcome::Quit => {
            // Give the interface a moment to say so, then make room for the
            // installer or the new version.
            if let Some(window) = app.get_webview_window("main") {
                save_geometry(&window.as_ref().window());
            }
            let handle = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(800));
                handle.exit(0);
            });
            Ok("quit".into())
        }
        update::Outcome::Page => {
            app.opener()
                .open_url(update::fallback_url(&found), None::<&str>)
                .map_err(|e| e.to_string())?;
            Ok("page".into())
        }
    }
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
    #[cfg(desktop)]
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.set_parent(&window);
    }
    for f in filters {
        let extensions: Vec<&str> = f.extensions.iter().map(String::as_str).collect();
        dialog = dialog.add_filter(&f.name, &extensions);
    }
    dialog
}

#[cfg(not(target_os = "android"))]
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

#[cfg(not(target_os = "android"))]
fn ask_open_path(app: &AppHandle, title: &str, filters: &[FileFilter]) -> Option<PathBuf> {
    file_dialog(app, title, filters)
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
}

/// After the file at a path of ask_save_path is written: what to show.
#[cfg(not(target_os = "android"))]
fn finish_save(path: &Path) -> Result<String> {
    Ok(path.display().to_string())
}

// Android's file dialog hands out content:// addresses instead of paths.
// A save is written to a file in the cache first and then copied to the
// address (finish_save); a picked file is copied into the cache, so
// everything behind it (replays, models, backups) reads a normal path.
#[cfg(target_os = "android")]
static PENDING_SAVES: std::sync::Mutex<Vec<(PathBuf, String)>> = std::sync::Mutex::new(Vec::new());

#[cfg(target_os = "android")]
fn android_cache(app: &AppHandle, name: &str) -> Option<PathBuf> {
    let dir = app.path().app_cache_dir().ok()?.join("files");
    std::fs::create_dir_all(&dir).ok()?;
    let name: String = name
        .chars()
        .map(|c| if c == '/' || c == '\\' { '_' } else { c })
        .collect();
    Some(dir.join(if name.is_empty() { "file".into() } else { name }))
}

#[cfg(target_os = "android")]
fn ask_save_path(
    app: &AppHandle,
    title: &str,
    suggested: &str,
    filters: &[FileFilter],
) -> Option<PathBuf> {
    use tauri_plugin_dialog::FilePath;
    match file_dialog(app, title, filters)
        .set_file_name(suggested)
        .blocking_save_file()?
    {
        FilePath::Path(path) => Some(path),
        FilePath::Url(url) => {
            let path = android_cache(app, suggested)?;
            if let Ok(mut pending) = PENDING_SAVES.lock() {
                pending.retain(|(p, _)| *p != path);
                pending.push((path.clone(), url.to_string()));
            }
            Some(path)
        }
    }
}

#[cfg(target_os = "android")]
fn ask_open_path(app: &AppHandle, title: &str, filters: &[FileFilter]) -> Option<PathBuf> {
    use tauri_plugin_dialog::FilePath;
    match file_dialog(app, title, filters).blocking_pick_file()? {
        FilePath::Path(path) => Some(path),
        FilePath::Url(url) => {
            let (name, data) = crate::android::read_uri(url.as_str()).ok()?;
            let path = android_cache(app, &name)?;
            std::fs::write(&path, data).ok()?;
            Some(path)
        }
    }
}

#[cfg(target_os = "android")]
fn finish_save(path: &Path) -> Result<String> {
    let uri = PENDING_SAVES.lock().ok().and_then(|mut pending| {
        let at = pending.iter().position(|(p, _)| p == path)?;
        Some(pending.remove(at).1)
    });
    let Some(uri) = uri else {
        return Ok(path.display().to_string());
    };
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(path);
    crate::android::write_uri(&uri, &data)?;
    Ok(path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(uri))
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
    finish_save(&path).map(Some)
}

#[tauri::command]
pub async fn save_pdf(
    app: AppHandle,
    title: String,
    suggested_name: String,
    html: String,
) -> Result<Option<String>> {
    #[cfg(desktop)]
    let pdf_filter = [FileFilter {
        name: "PDF".into(),
        extensions: vec!["pdf".into()],
    }];
    // A phone has no hidden window to print from: the report is saved as
    // an HTML page, which every browser there shows and prints.
    #[cfg(mobile)]
    let (suggested_name, pdf_filter) = (
        Path::new(&suggested_name)
            .with_extension("html")
            .display()
            .to_string(),
        [FileFilter {
            name: "HTML".into(),
            extensions: vec!["html".into()],
        }],
    );
    let Some(mut path) = ask_save_path(&app, &title, &suggested_name, &pdf_filter) else {
        return Ok(None);
    };
    if path.extension().is_none() {
        path.set_extension(if cfg!(mobile) { "html" } else { "pdf" });
    }
    match pdf::write_report(&app, html, path.clone(), true).await? {
        pdf::Written::File => finish_save(&path).map(Some),
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
    if state.cli.report.as_deref() == Some(path.as_str()) {
        state.report_stage.store(REPORT_WRITING, Ordering::Relaxed);
    }
    let ok = matches!(
        pdf::write_report(&app, html, PathBuf::from(&path), false).await,
        Ok(pdf::Written::File)
    );
    if state.cli.report.as_deref() == Some(path.as_str()) {
        state.report_stage.store(REPORT_WRITTEN, Ordering::Relaxed);
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
    // On Android these files (log, recording) are written while the app
    // runs, which a content:// address cannot take: they go into the app's
    // own documents folder, which a computer sees over USB.
    #[cfg(target_os = "android")]
    {
        let _ = (title, filters);
        let dir = app.path().document_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(Some(dir.join(&suggested_name).display().to_string()))
    }
    #[cfg(not(target_os = "android"))]
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

/// The bytes of a 3D model the user picked for the X-ray view (a .glb, or a
/// .gltf with the buffers and pictures it names next to it). Only these file
/// types, and not more than a large model needs.
#[tauri::command]
pub async fn read_model_file(path: String) -> Result<tauri::ipc::Response> {
    const TYPES: [&str; 8] = ["glb", "gltf", "bin", "png", "jpg", "jpeg", "webp", "ktx2"];
    const LIMIT: u64 = 512 * 1024 * 1024;
    let path = PathBuf::from(path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if !TYPES.contains(&ext.as_str()) {
        return Err(format!("{}: not a glTF model", path.display()));
    }
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > LIMIT {
        return Err(format!("{}: larger than 512 MB", path.display()));
    }
    tauri::async_runtime::spawn_blocking(move || std::fs::read(&path))
        .await
        .map_err(|e| e.to_string())?
        .map(tauri::ipc::Response::new)
        .map_err(|e| e.to_string())
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
    #[cfg(target_os = "android")]
    use crate::android::enumerate_ports;
    #[cfg(not(target_os = "android"))]
    use caruspex_link::enumerate_ports;
    tauri::async_runtime::spawn_blocking(|| {
        enumerate_ports()
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
    bridged: bool,
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
            bridged: d.bridged,
        })
        .collect()
}

#[tauri::command]
pub fn list_socketcan_interfaces() -> Vec<String> {
    caruspex_net::socket_can_interfaces()
}

#[derive(Serialize)]
pub struct VendorCanRow {
    label: String,
    spec: String,
}

/// The channels of PEAK PCAN-Basic and Kvaser CANlib, when installed.
#[tauri::command]
pub fn list_can_vendor_channels() -> Vec<VendorCanRow> {
    caruspex_net::can_vendor_channels()
        .into_iter()
        .map(|c| VendorCanRow {
            label: c.label,
            spec: c.spec,
        })
        .collect()
}

#[tauri::command]
pub fn open_serial(
    state: State<'_, AppState>,
    device: String,
    baud: i32,
    protocol_order: String,
    remembered: Option<ConnectRoute>,
    truck: Option<u32>,
) {
    connect_job(
        &state,
        remembered,
        truck,
        Job::OpenSerial {
            device,
            baud,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn open_tcp(
    state: State<'_, AppState>,
    host: String,
    port: i32,
    protocol_order: String,
    remembered: Option<ConnectRoute>,
    truck: Option<u32>,
) {
    connect_job(
        &state,
        remembered,
        truck,
        Job::OpenTcp {
            host,
            port,
            protocol_order,
        },
    );
}

#[tauri::command]
pub fn open_demo(state: State<'_, AppState>, protocol_order: String, truck: Option<u32>) {
    let _ = protocol_order;
    connect_job(&state, None, truck, Job::OpenDemo);
}

#[tauri::command]
pub fn open_enet(
    state: State<'_, AppState>,
    host: String,
    protocol_order: String,
    remembered: Option<ConnectRoute>,
) {
    let _ = protocol_order;
    connect_job(&state, remembered, None, Job::OpenEnet { host });
}

#[tauri::command]
pub fn open_can(
    state: State<'_, AppState>,
    spec: String,
    protocol_order: String,
    remembered: Option<ConnectRoute>,
    truck: Option<u32>,
) {
    let _ = protocol_order;
    connect_job(&state, remembered, truck, Job::OpenCan { spec });
}

#[tauri::command]
pub fn scan_ble(state: State<'_, AppState>, milliseconds: i32) {
    job(&state, Job::ScanBle { milliseconds });
}

/// Whether Bluetooth may be used. Android 12 and newer ask the user at run
/// time, after the interface has explained why; elsewhere there is nothing
/// to ask.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BluetoothAccess {
    pub available: bool,
    pub needs_permission: bool,
    pub granted: bool,
    pub enabled: bool,
}

impl Default for BluetoothAccess {
    fn default() -> Self {
        Self {
            available: true,
            needs_permission: false,
            granted: true,
            enabled: true,
        }
    }
}

#[tauri::command]
pub async fn bluetooth_access() -> Result<BluetoothAccess> {
    #[cfg(target_os = "android")]
    return tauri::async_runtime::spawn_blocking(crate::android::bluetooth_access)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "android"))]
    Ok(BluetoothAccess::default())
}

/// Shows the system question for Bluetooth (Android 12 and newer).
#[tauri::command]
pub async fn request_bluetooth_access() -> Result<BluetoothAccess> {
    #[cfg(target_os = "android")]
    return tauri::async_runtime::spawn_blocking(crate::android::request_bluetooth_access)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "android"))]
    Ok(BluetoothAccess::default())
}

#[tauri::command]
pub fn open_ble(
    state: State<'_, AppState>,
    id: String,
    name: String,
    kinds: i32,
    protocol_order: String,
    remembered: Option<ConnectRoute>,
    truck: Option<u32>,
) {
    connect_job(
        &state,
        remembered,
        truck,
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
    job(&state, Job::SetTruck(None));
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

/// "Mitschnitt teilen": the recording without the last VIN digits, the port
/// or address and the time of day, with a preview of what is in it. `vin`
/// is the VIN of the connected car, empty if none; the ones in the traffic
/// are found anyway. Nothing is written or sent: the interface shows the
/// preview and saves the file only when asked.
#[tauri::command]
pub fn share_recording(path: String, vin: String) -> Result<Value> {
    let recording = caruspex_record::load_recording(&path)?;
    let shared = caruspex_record::share_recording(&recording, &[vin.as_str()]);
    Ok(share_json(&shared))
}

/// The newest crash report the user has not decided about, if any. Runs
/// for a screenshot or a report show none, unless they bring their own
/// folder (CARUSPEX_CRASH_DIR).
#[tauri::command]
pub fn crash_report_pending(state: State<'_, AppState>) -> Option<crate::crash::CrashReport> {
    let automated = state.cli.shot.is_some() || state.cli.report.is_some();
    if automated && std::env::var_os("CARUSPEX_CRASH_DIR").is_none() {
        return None;
    }
    crate::crash::dir().and_then(|dir| crate::crash::pending(dir))
}

/// The user decided about a report: it stays on the computer but is not
/// offered again.
#[tauri::command]
pub fn crash_report_done(path: String) -> Result<()> {
    let dir = crate::crash::dir().ok_or("no report folder")?;
    crate::crash::mark_seen(dir, &path)
}

pub fn share_json(shared: &caruspex_record::SharedRecording) -> Value {
    let p = &shared.preview;
    json!({
        "file": String::from_utf8_lossy(&shared.file),
        "preview": {
            "durationSeconds": p.duration_seconds,
            "exchanges": p.exchanges,
            "adapter": p.adapter,
            "protocol": p.protocol,
            "created": p.created,
            "vins": p.vins,
            "vinPlaces": p.vin_places,
            "transportRemoved": p.transport_removed,
            "requests": p.requests.iter().map(|(kind, count)| json!({ "kind": kind, "count": count })).collect::<Vec<_>>(),
            "sample": p.sample,
            "bytes": p.bytes,
            "hiddenDigits": caruspex_record::VIN_HIDDEN,
        }
    })
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

/// The used car check; `papers_vin` is the VIN from the papers, may be empty.
#[tauri::command]
pub fn run_used_car_check(state: State<'_, AppState>, papers_vin: String) {
    job(&state, Job::UsedCarCheck(papers_vin));
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
// The stages of --report, for the message when it gives up.
const REPORT_WAITING: u8 = 0;
const REPORT_CONNECTED: u8 = 1;
const REPORT_ASKED: u8 = 2;
const REPORT_WRITING: u8 = 3;
const REPORT_WRITTEN: u8 = 4;

fn report_stage_name(stage: u8) -> &'static str {
    match stage {
        REPORT_WAITING => "no connection event",
        REPORT_CONNECTED => "connected, waiting for the readout",
        REPORT_ASKED => "interface asked for the report, no answer",
        REPORT_WRITING => "writing the PDF",
        _ => "written",
    }
}

/// The interface could not build the report for --report: end the run with
/// the reason instead of waiting for the two minute limit.
#[tauri::command]
pub fn cli_report_failed(app: AppHandle, state: State<'_, AppState>, message: String) {
    if let Some(path) = state.cli.report.clone() {
        let text = args(
            &mw(
                "Prüfbericht ließ sich nicht schreiben: %1",
                "The diagnostic report could not be written: %1",
            ),
            &[&format!("{path} ({message})")],
        );
        finish(&app, 1, &text);
    }
}

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
/// The protocol order for a setting: "can" tries CAN first, anything else
/// the K-line first, like the Connect page does. The setting itself is a
/// word, not an order; handed over as it is, the adapter was asked for the
/// protocols "k", "l", "i", "n" and "e".
pub fn order_from_setting(setting: &str) -> String {
    if setting.eq_ignore_ascii_case("can") {
        caruspex_link::elm327::PROTOCOL_ORDER_CAN_FIRST.to_string()
    } else {
        caruspex_link::elm327::PROTOCOL_ORDER_KLINE_FIRST.to_string()
    }
}

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
            protocol_order: order_from_setting(&values.protocol_order),
        })
    } else if let Some(address) = &cli.tcp {
        let (host, port) = address
            .rsplit_once(':')
            .unwrap_or((address.as_str(), "35000"));
        Some(Job::OpenTcp {
            host: host.to_string(),
            port: port.parse().unwrap_or(35000),
            protocol_order: order_from_setting(&values.protocol_order),
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
            protocol_order: order_from_setting(&values.protocol_order),
        })
    } else {
        cli.replay.as_ref().map(|path| Job::OpenReplay {
            path: path.clone(),
            real_time: false,
            protocol_order: order_from_setting(&values.protocol_order),
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
            let stage = app
                .try_state::<AppState>()
                .map_or(REPORT_WAITING, |s| s.report_stage.load(Ordering::Relaxed));
            let text = mw(
                "Keine Verbindung nach zwei Minuten, abgebrochen.",
                "No connection after two minutes, cancelled.",
            );
            finish(&app, 3, &format!("{text} [{}]", report_stage_name(stage)));
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
            state
                .report_stage
                .store(REPORT_CONNECTED, Ordering::Relaxed);
            let app = app.clone();
            later(state.cli.report_wait_ms, move || {
                if let Some(state) = app.try_state::<AppState>() {
                    state.report_stage.store(REPORT_ASKED, Ordering::Relaxed);
                }
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

/// The desktop window only; on a phone the app fills the screen.
#[cfg(desktop)]
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

#[cfg(test)]
mod order_tests {
    use super::order_from_setting;

    #[test]
    fn the_setting_becomes_a_protocol_order() {
        assert_eq!(order_from_setting("can"), "67891543");
        assert_eq!(order_from_setting("kline"), "54312678");
        assert_eq!(order_from_setting(""), "54312678");
        assert!(
            order_from_setting("kline")
                .chars()
                .all(|c| c.is_ascii_digit())
        );
    }
}
