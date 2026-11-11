// The report as a PDF (Report.cpp saveReportPdf / writeReport). The
// interface writes the HTML; here it is loaded into a hidden webview and
// printed to a file by the webview itself: WebView2's PrintToPdf on
// Windows, WKWebView's createPDF on macOS, a WebKitGTK print job to GTK's
// file printer on Linux. Should that fail on macOS or Linux, the page opens
// in a window of its own with the print dialog instead, where "Save as PDF"
// is one of the printers.
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, Runtime, UriSchemeContext, WebviewUrl, WebviewWindowBuilder};

pub const SCHEME: &str = "cxreport";

/// Pages waiting to be printed, by id.
#[derive(Default)]
pub struct ReportPages(Mutex<HashMap<String, String>>);

/// Serves a waiting page under cxreport://localhost/<id>.
pub fn serve<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let id = request.uri().path().trim_start_matches('/').to_string();
    let html = ctx
        .app_handle()
        .state::<ReportPages>()
        .0
        .lock()
        .ok()
        .and_then(|pages| pages.get(&id).cloned());
    let empty = || -> Cow<'static, [u8]> { Cow::Borrowed(&[]) };
    match html {
        Some(html) => Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Cow::<'static, [u8]>::Owned(html.into_bytes()))
            .unwrap_or_else(|_| Response::new(empty())),
        None => Response::builder()
            .status(404)
            .body(empty())
            .unwrap_or_else(|_| Response::new(empty())),
    }
}

fn page_url(id: &str) -> Result<tauri::Url, String> {
    // WebView2 reaches custom schemes as http://<scheme>.localhost.
    let text = if cfg!(windows) {
        format!("http://{SCHEME}.localhost/{id}")
    } else {
        format!("{SCHEME}://localhost/{id}")
    };
    text.parse::<tauri::Url>().map_err(|e| e.to_string())
}

fn is_html(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}

// The webview of each system, behind the same two functions.
#[cfg(target_os = "linux")]
use crate::webview_gtk as native;
#[cfg(target_os = "macos")]
use crate::webview_mac as native;
#[cfg(windows)]
use crate::webview_win as native;

/// Other systems: neither PDF nor picture, the dialog stays.
#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
mod native {
    use std::path::Path;
    use std::sync::mpsc::Sender;

    use tauri::WebviewWindow;

    pub fn print_to_pdf(_: &WebviewWindow, _: &Path, done: Sender<Result<(), String>>) {
        let _ = done.send(Err(String::from("PDF")));
    }

    pub fn capture_png(_: &WebviewWindow, _: &Path, done: Sender<Result<(), String>>) {
        let _ = done.send(Err(String::from("--shot")));
    }
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What happened to the report.
pub enum Written {
    /// The file is there.
    File,
    /// The print dialog is open instead (the PDF could not be written
    /// directly, macOS and Linux only).
    #[cfg_attr(windows, allow(dead_code))]
    PrintDialog,
}

/// Writes the report: HTML when the path ends in .html or .htm, PDF
/// otherwise. `dialog_fallback` opens the print dialog on macOS and Linux
/// when the PDF cannot be written directly (not in unattended runs).
pub async fn write_report(
    app: &AppHandle,
    html: String,
    path: PathBuf,
    dialog_fallback: bool,
) -> Result<Written, String> {
    if is_html(&path) {
        std::fs::write(&path, html.as_bytes()).map_err(|e| e.to_string())?;
        return Ok(Written::File);
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut pages) = app.state::<ReportPages>().0.lock() {
        pages.insert(id.clone(), html);
    }
    let result = print(app, &id, path, dialog_fallback).await;
    if let Ok(mut pages) = app.state::<ReportPages>().0.lock() {
        pages.remove(&id);
    }
    result
}

#[cfg(windows)]
async fn print(
    app: &AppHandle,
    id: &str,
    path: PathBuf,
    _dialog_fallback: bool,
) -> Result<Written, String> {
    print_native(app, id, path).await.map(|()| Written::File)
}

#[cfg(not(windows))]
async fn print(
    app: &AppHandle,
    id: &str,
    path: PathBuf,
    dialog_fallback: bool,
) -> Result<Written, String> {
    match print_native(app, id, path).await {
        Ok(()) => Ok(Written::File),
        Err(error) => {
            eprintln!("{error}");
            if dialog_fallback {
                print_dialog(app, id).await
            } else {
                Err(error)
            }
        }
    }
}

/// Loads the waiting page into a hidden window and lets the webview write
/// it to `path` as a PDF.
async fn print_native(app: &AppHandle, id: &str, path: PathBuf) -> Result<(), String> {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use tauri::webview::PageLoadEvent;

    let (tx, rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let label = format!("report-pdf-{id}");
    let started = Arc::new(AtomicBool::new(false));
    let target = path.clone();
    let window = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(page_url(id)?))
        .visible(false)
        .focused(false)
        .inner_size(820.0, 1160.0)
        .on_page_load(move |window, payload| {
            if payload.event() == PageLoadEvent::Finished && !started.swap(true, Ordering::SeqCst) {
                native::print_to_pdf(&window, &target, tx.clone());
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_secs(60))
            .unwrap_or_else(|_| Err(String::from("timeout")))
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = window.destroy();
    outcome?;
    if !file_written(&path).await {
        return Err(path.display().to_string());
    }
    Ok(())
}

#[cfg(windows)]
async fn file_written(path: &Path) -> bool {
    path.exists()
}

/// GTK's file printer may still be closing the file when WebKit reports the
/// job as done, so the file gets a moment to appear.
#[cfg(not(windows))]
async fn file_written(path: &Path) -> bool {
    let path = path.to_path_buf();
    tauri::async_runtime::spawn_blocking(move || {
        for _ in 0..50 {
            if path.metadata().is_ok_and(|m| m.len() > 0) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        false
    })
    .await
    .unwrap_or(false)
}

/// The fallback: the page in a window of its own with the print dialog.
#[cfg(not(windows))]
async fn print_dialog(app: &AppHandle, id: &str) -> Result<Written, String> {
    use tauri::webview::PageLoadEvent;

    let label = format!("report-print-{id}");
    WebviewWindowBuilder::new(app, &label, WebviewUrl::External(page_url(id)?))
        .title("Caruspex")
        .inner_size(820.0, 1000.0)
        .on_page_load(|window, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = window.eval("window.print()");
            }
        })
        .build()
        .map_err(|e| e.to_string())?;
    // The page has to stay until the dialog is done with it.
    tokio_sleep(std::time::Duration::from_secs(2)).await;
    Ok(Written::PrintDialog)
}

#[cfg(not(windows))]
async fn tokio_sleep(d: std::time::Duration) {
    let _ = tauri::async_runtime::spawn_blocking(move || std::thread::sleep(d)).await;
}

/// Saves a picture of the main window (--shot).
pub async fn screenshot(app: &AppHandle, path: PathBuf) -> Result<(), String> {
    let window = app.get_webview_window("main").ok_or("no window")?;
    let (tx, rx) = std::sync::mpsc::channel();
    native::capture_png(&window, &path, tx);
    tauri::async_runtime::spawn_blocking(move || {
        rx.recv_timeout(std::time::Duration::from_secs(20))
            .unwrap_or_else(|_| Err(String::from("timeout")))
    })
    .await
    .map_err(|e| e.to_string())?
}
