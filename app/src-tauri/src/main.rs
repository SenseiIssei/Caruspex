// Caruspex desktop app: the frameless main window with the web interface,
// and behind it the worker thread that talks to the car (worker.rs, ported from the Qt
// version). The commands and events are the ones of app/src/api/contract.ts.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_code)]

mod advice;
mod cli;
mod commands;
mod garage;
mod knowledge;
mod measure;
mod pdf;
mod rows;
mod settings;
mod texts;
mod update;
#[cfg(target_os = "linux")]
mod webview_gtk;
#[cfg(target_os = "macos")]
mod webview_mac;
#[cfg(windows)]
mod webview_win;
mod worker;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};

use crate::cli::Cli;
use crate::measure::{Trip, TripState};
use crate::rows::LiveRow;
use crate::settings::AppSettings;
use crate::worker::{Sink, WorkerHandle};

/// Everything the commands share.
pub struct AppState {
    pub worker: WorkerHandle,
    pub config_dir: PathBuf,
    pub garage_path: PathBuf,
    pub settings: Mutex<AppSettings>,
    pub trip: Arc<TripState>,
    pub log_file: Arc<Mutex<Option<(PathBuf, File)>>>,
    pub cli: Cli,
    /// Set by the first app_info: the interface is up and listening.
    pub ready: AtomicBool,
}

/// The worker's events into the window, the live values into the trip
/// computer, the log lines into the log file.
struct TauriSink {
    app: AppHandle,
    trip: Arc<TripState>,
    log_file: Arc<Mutex<Option<(PathBuf, File)>>>,
}

/// One line into the log file, if one is open, stamped like
/// MainWindow::appendLog. Used for the worker's lines and for the ones the
/// interface logs itself (the write_log command).
pub fn write_log_line(log_file: &Mutex<Option<(PathBuf, File)>>, line: &str) {
    if let Ok(mut file) = log_file.lock()
        && let Some((_, file)) = file.as_mut()
    {
        let stamped = format!("{}  {line}\n", chrono::Local::now().format("%H:%M:%S"));
        let _ = file.write_all(stamped.as_bytes());
        let _ = file.flush();
    }
}

impl Sink for TauriSink {
    fn emit(&self, event: &str, payload: Value) {
        if event == "log_line"
            && let Some(line) = payload["line"].as_str()
        {
            write_log_line(&self.log_file, line);
        }
        commands::on_worker_event(&self.app, event, &payload);
        let _ = self.app.emit(event, payload);
    }

    fn live(&self, row: &LiveRow) {
        if let Ok(mut trip) = self.trip.lock() {
            trip.feed(row);
        }
    }
}

/// The WebView2 folder of a command-line driven run, see main().
static WEBVIEW_DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Gives a window the run's own WebView2 folder when there is one; every
/// window of one process has to use the same folder.
pub fn isolate<'a, R: tauri::Runtime, M: Manager<R>>(
    builder: WebviewWindowBuilder<'a, R, M>,
) -> WebviewWindowBuilder<'a, R, M> {
    match WEBVIEW_DIR.get() {
        Some(dir) => builder.data_directory(dir.clone()),
        None => builder,
    }
}

fn print_and_exit(text: &str, code: i32) -> ! {
    if code == 0 {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
    std::process::exit(code);
}

fn main() {
    let _ = worker::app_seconds();
    let cli = match cli::parse(std::env::args().skip(1)) {
        Ok(cli) => cli,
        Err(error) => print_and_exit(&error, 2),
    };

    // Answers that need no window come first: on a machine without a display
    // (a server, the CI) the window system cannot even start.
    if cli.help || cli.version || (cli.report.is_some() && !cli.connects()) {
        // The same folder Tauri uses for app_config_dir: <config>/<identifier>.
        let config_dir = dirs::config_dir()
            .unwrap_or_default()
            .join("dev.senseiissei.caruspex");
        let values = settings::load_settings(&config_dir);
        let code = cli
            .language
            .clone()
            .unwrap_or_else(|| values.language.clone());
        commands::apply_language(&commands::effective_language(&code));
        if cli.help {
            print_and_exit(&cli::help_text(), 0);
        }
        if cli.version {
            print_and_exit(&format!("Caruspex {}", env!("CARGO_PKG_VERSION")), 0);
        }
        print_and_exit(
            &texts::main_text(
                "--report braucht eine Verbindung: --demo, --port, --tcp, --enet, --bluetooth oder --replay",
                "--report needs a connection: --demo, --port, --tcp, --enet, --bluetooth or --replay",
            ),
            2,
        );
    }

    // Runs driven by the command line (a screenshot, a report) get their own
    // WebView2 folder, so they work next to an open window and next to each
    // other. A normal second start brings the open window to the front
    // instead, like most programs do.
    let automated = cli.shot.is_some() || cli.report.is_some();
    if automated {
        let dir = std::env::temp_dir().join(format!("caruspex-webview-{}", std::process::id()));
        let _ = WEBVIEW_DIR.set(dir);
    }
    let mut builder = tauri::Builder::default();
    if !automated {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }));
    }
    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(pdf::ReportPages::default())
        .register_uri_scheme_protocol(pdf::SCHEME, pdf::serve)
        .setup(move |app| {
            let handle = app.handle().clone();
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().data_dir()?;
            let values = settings::load_settings(&config_dir);

            // The language of the command line and of the settings, for the
            // help text and everything the backend says before the interface
            // has sent its own.
            let code = cli.language.clone().unwrap_or_else(|| values.language.clone());
            commands::apply_language(&commands::effective_language(&code));
            // English tables for the other languages, built before any other
            // thread looks at the language.
            knowledge::init_english_names();

            let trip: Arc<TripState> = Arc::new(Mutex::new(Trip::default()));
            let log_file = Arc::new(Mutex::new(None));
            let sink = Arc::new(TauriSink {
                app: handle.clone(),
                trip: trip.clone(),
                log_file: log_file.clone(),
            });
            let worker = worker::spawn(sink, values.clone());
            app.manage(AppState {
                worker,
                config_dir: config_dir.clone(),
                garage_path: garage::garage_path(&data_dir),
                settings: Mutex::new(values.clone()),
                trip,
                log_file,
                cli: cli.clone(),
                ready: AtomicBool::new(false),
            });
            commands::open_log_file(&handle, &values);

            // The main window, from the configuration, with the start
            // options of the command line in its address.
            let mut config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .cloned()
                .ok_or("main window missing in tauri.conf.json")?;
            if let Some((w, h)) = cli.size {
                config.width = w;
                config.height = h;
            }
            if let Some((x, y)) = cli.window_pos {
                // Placed before it is shown, and without taking the focus.
                config.center = false;
                config.x = Some(x);
                config.y = Some(y);
                config.focus = false;
            }
            let query = cli::start_query(&cli);
            let mut builder = isolate(WebviewWindowBuilder::from_config(&handle, &config)?);
            if !query.is_empty() {
                // Set before the page's own scripts run, so main.tsx finds the
                // options in location.search like in a browser.
                let script = format!(
                    "try {{ if (window.top === window && !location.search) history.replaceState(null, '', location.pathname + {} + location.hash); }} catch (e) {{}}",
                    serde_json::to_string(&query).unwrap_or_default()
                );
                builder = builder.initialization_script(&script);
            }
            let window = builder.build()?;
            commands::restore_geometry(&window, &config_dir, &values, &cli);
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { .. } = event
            {
                commands::save_geometry(window);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::set_language,
            commands::check_for_update,
            commands::open_url,
            commands::load_settings,
            commands::save_settings,
            commands::open_config_location,
            commands::write_log,
            commands::save_text_file,
            commands::save_pdf,
            commands::write_report,
            commands::open_text_file,
            commands::pick_save_path,
            commands::pick_open_path,
            commands::garage_load,
            commands::garage_save,
            commands::list_serial_ports,
            commands::list_j2534_devices,
            commands::list_socketcan_interfaces,
            commands::open_serial,
            commands::open_tcp,
            commands::open_demo,
            commands::open_enet,
            commands::open_can,
            commands::scan_ble,
            commands::open_ble,
            commands::open_replay,
            commands::close_adapter,
            commands::start_recording,
            commands::stop_recording,
            commands::read_faults,
            commands::clear_faults,
            commands::read_details,
            commands::start_live,
            commands::stop_live,
            commands::send_raw,
            commands::read_monitors,
            commands::run_health_check,
            commands::probe_adapter,
            commands::bruteforce_clear,
            commands::cancel_scan,
            commands::start_voltage_poll,
            commands::stop_voltage_poll,
            commands::read_custom,
            commands::start_custom_poll,
            commands::import_custom_csv,
            commands::trip_reset,
            commands::trip_start,
            commands::trip_stop,
            commands::trip_summary,
            commands::analyze_performance,
            commands::evaluate_battery,
            commands::battery_test_steps,
            commands::ecu_families,
            commands::scan_uds,
            commands::clear_uds_ecu,
            commands::open_kkl,
            commands::close_kkl,
            commands::start_basic_setting,
            commands::stop_basic_setting,
            commands::set_basic_setting_session,
            commands::scan_all_ecus,
            commands::read_adaptation_channel,
            commands::run_actuator_test,
            commands::kkl_ecus,
            commands::lookup_dtc,
            commands::locate_dtc,
            commands::what_to_do,
            commands::dtc_lexicon,
            commands::brand_families,
            commands::brand_family,
            commands::pid_table,
            commands::default_live_pids,
            commands::pid_packs,
            commands::obdb_packs,
            commands::catalog_brands,
            commands::catalog_models,
            commands::catalog_generations,
            commands::catalog_search,
            commands::catalog_size,
            commands::catalog_diagnosis,
            commands::decode_vin,
            commands::match_vin,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Caruspex")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.worker.cancel_scan();
                    state.worker.send(worker::Job::Shutdown);
                }
                // The WebView2 folder of a command-line run; what the browser
                // processes still hold is left to the temp folder cleanup.
                if let Some(dir) = WEBVIEW_DIR.get() {
                    let _ = std::fs::remove_dir_all(dir);
                }
            }
        });
}
