// The report as a PDF (Report.cpp saveReportPdf / writeReport). The
// interface writes the HTML; here it is loaded into a hidden webview and
// printed to a file by the webview itself: WebView2's PrintToPdf on
// Windows. On macOS and Linux the page opens in a window of its own with the
// print dialog, where "Save as PDF" is one of the printers.
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

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// What happened to the report.
pub enum Written {
    /// The file is there.
    File,
    /// The print dialog is open instead (no native PDF writer here).
    #[cfg_attr(windows, allow(dead_code))]
    PrintDialog,
}

/// Writes the report: HTML when the path ends in .html or .htm, PDF
/// otherwise.
pub async fn write_report(app: &AppHandle, html: String, path: PathBuf) -> Result<Written, String> {
    if is_html(&path) {
        std::fs::write(&path, html.as_bytes()).map_err(|e| e.to_string())?;
        return Ok(Written::File);
    }
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed).to_string();
    if let Ok(mut pages) = app.state::<ReportPages>().0.lock() {
        pages.insert(id.clone(), html);
    }
    let result = print(app, &id, path).await;
    if let Ok(mut pages) = app.state::<ReportPages>().0.lock() {
        pages.remove(&id);
    }
    result
}

#[cfg(windows)]
async fn print(app: &AppHandle, id: &str, path: PathBuf) -> Result<Written, String> {
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
                crate::webview_win::print_to_pdf(&window, &target, tx.clone());
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
    if !path.exists() {
        return Err(path.display().to_string());
    }
    Ok(Written::File)
}

#[cfg(not(windows))]
async fn print(app: &AppHandle, id: &str, _path: PathBuf) -> Result<Written, String> {
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
    #[cfg(windows)]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        crate::webview_win::capture_png(&window, &path, tx);
        tauri::async_runtime::spawn_blocking(move || {
            rx.recv_timeout(std::time::Duration::from_secs(20))
                .unwrap_or_else(|_| Err(String::from("timeout")))
        })
        .await
        .map_err(|e| e.to_string())?
    }
    #[cfg(not(windows))]
    {
        let _ = (window, path);
        Err(String::from(
            "--shot needs Windows (WebView2 CapturePreview)",
        ))
    }
}
