// What WebKitGTK can do beyond Tauri's own API, the Linux counterpart of
// webview_win.rs: print the page to a PDF file without the dialog (a
// PrintOperation aimed at GTK's "Print to File" printer) and take a picture
// of it (webkit_web_view_get_snapshot, saved as PNG through cairo).
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use gtk::glib;
use tauri::WebviewWindow;
use webkit2gtk::{PrintOperationExt, SnapshotOptions, SnapshotRegion, WebViewExt};

/// The name GTK gives its file printer. WebKit looks the printer up by
/// name, and GTK translates that name, so it is asked the same way GTK
/// does (domain gtk30, msgid "Print to File").
fn file_printer_name() -> String {
    let name = glib::dgettext(Some("gtk30"), FILE_PRINTER).to_string();
    if name.is_empty() {
        FILE_PRINTER.to_string()
    } else {
        name
    }
}

const FILE_PRINTER: &str = "Print to File";

/// The print settings that send the job to `path` as a PDF.
fn pdf_settings(uri: &str) -> gtk::PrintSettings {
    let settings = gtk::PrintSettings::new();
    settings.set_printer(&file_printer_name());
    settings.set(gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT, Some("pdf"));
    settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(uri));
    settings
}

/// Prints the page in `webview` to `path` as a PDF. The answer comes
/// through `done` once WebKit is done with the job.
pub fn print_to_pdf(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target: PathBuf = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let unreachable = done.clone();
    let result = webview.with_webview(move |platform| {
        let uri = match glib::filename_to_uri(&target, None) {
            Ok(uri) => uri.to_string(),
            Err(e) => {
                let _ = done.send(Err(e.to_string()));
                return;
            }
        };
        let page = gtk::PageSetup::new();
        page.set_paper_size(&gtk::PaperSize::new(Some(gtk::PAPER_NAME_A4)));
        let operation = webkit2gtk::PrintOperation::new(&platform.inner());
        operation.set_print_settings(&pdf_settings(&uri));
        operation.set_page_setup(&page);
        // "failed" comes before "finished", and only the first answer
        // counts.
        let failed = done.clone();
        operation.connect_failed(move |_, error| {
            let _ = failed.send(Err(error.to_string()));
        });
        // The operation keeps itself alive until it has finished.
        let keep = RefCell::new(Some(operation.clone()));
        operation.connect_finished(move |_| {
            keep.borrow_mut().take();
            let _ = done.send(Ok(()));
        });
        operation.print();
    });
    if let Err(e) = result {
        let _ = unreachable.send(Err(e.to_string()));
    }
}

/// Saves a PNG picture of what `webview` shows to `path`.
pub fn capture_png(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target: PathBuf = path.to_path_buf();
    let unreachable = done.clone();
    let result = webview.with_webview(move |platform| {
        platform.inner().snapshot(
            SnapshotRegion::Visible,
            SnapshotOptions::NONE,
            None::<&webkit2gtk::gio::Cancellable>,
            move |result| {
                let outcome = result.map_err(|e| e.to_string()).and_then(|surface| {
                    // WebKitGTK hands over a surface of the cairo-rs that gtk
                    // 0.18 uses; the PNG is written by our newer cairo-rs.
                    // Both wrap the same C object of the one libcairo, so the
                    // surface moves over as its C pointer, with a reference of
                    // its own (from_raw_none adds one).
                    let raw = surface.to_raw_none().cast::<cairo::ffi::cairo_surface_t>();
                    // SAFETY: `raw` is a live cairo_surface_t, kept alive by
                    // `surface` until the new wrapper holds its own reference.
                    let surface = unsafe { cairo::Surface::from_raw_none(raw) };
                    let image = cairo::ImageSurface::try_from(surface)
                        .map_err(|_| String::from("snapshot is no image"))?;
                    let mut file = std::fs::File::create(&target).map_err(|e| e.to_string())?;
                    image.write_to_png(&mut file).map_err(|e| e.to_string())
                });
                let _ = done.send(outcome);
            },
        );
    });
    if let Err(e) = result {
        let _ = unreachable.send(Err(e.to_string()));
    }
}
