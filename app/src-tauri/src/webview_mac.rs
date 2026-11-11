// What WKWebView can do beyond Tauri's own API, the macOS counterpart of
// webview_win.rs: write the page as a PDF (createPDFWithConfiguration,
// macOS 11) and take a picture of it (takeSnapshotWithConfiguration). Both
// are Objective-C calls with a completion block, hence the unsafe code.
#![allow(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use block2::RcBlock;
use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSImage};
use objc2_foundation::{NSData, NSDictionary, NSError};
use objc2_web_kit::WKWebView;
use tauri::WebviewWindow;

fn error_text(error: *mut NSError, what: &str) -> String {
    // SAFETY: WebKit passes either nil or a valid NSError.
    match unsafe { error.as_ref() } {
        Some(error) => error.localizedDescription().to_string(),
        None => what.to_string(),
    }
}

fn write(path: &Path, data: &NSData) -> Result<(), String> {
    std::fs::write(path, data.to_vec()).map_err(|e| e.to_string())
}

/// Writes the page in `webview` as a PDF to `path`. The answer comes
/// through `done` once the file is there.
pub fn print_to_pdf(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target: PathBuf = path.to_path_buf();
    let unreachable = done.clone();
    let result = webview.with_webview(move |platform| {
        let handler = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
            // SAFETY: WebKit passes either nil or valid PDF data.
            let outcome = match unsafe { data.as_ref() } {
                Some(data) => write(&target, data),
                None => Err(error_text(error, "createPDF")),
            };
            let _ = done.send(outcome);
        });
        // SAFETY: with_webview runs on the main thread and inner() is the
        // WKWebView of this window, alive for the duration of the call.
        unsafe {
            let view = &*platform.inner().cast::<WKWebView>();
            // No configuration: the whole page, as wide as the window.
            view.createPDFWithConfiguration_completionHandler(None, &handler);
        }
    });
    if let Err(e) = result {
        let _ = unreachable.send(Err(e.to_string()));
    }
}

/// The PNG form of a picture: through its TIFF data and a bitmap.
fn png(image: &NSImage) -> Option<objc2::rc::Retained<NSData>> {
    let tiff = image.TIFFRepresentation()?;
    let bitmap = NSBitmapImageRep::imageRepWithData(&tiff)?;
    // SAFETY: an empty dictionary is a valid set of properties.
    unsafe {
        bitmap.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
}

/// Saves a PNG picture of what `webview` shows to `path`.
pub fn capture_png(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target: PathBuf = path.to_path_buf();
    let unreachable = done.clone();
    let result = webview.with_webview(move |platform| {
        let handler = RcBlock::new(move |image: *mut NSImage, error: *mut NSError| {
            // SAFETY: WebKit passes either nil or a valid image.
            let outcome = match unsafe { image.as_ref() } {
                Some(image) => match png(image) {
                    Some(data) => write(&target, &data),
                    None => Err(String::from("PNG")),
                },
                None => Err(error_text(error, "takeSnapshot")),
            };
            let _ = done.send(outcome);
        });
        // SAFETY: as in print_to_pdf.
        unsafe {
            let view = &*platform.inner().cast::<WKWebView>();
            // No configuration: the visible part of the page.
            view.takeSnapshotWithConfiguration_completionHandler(None, &handler);
        }
    });
    if let Err(e) = result {
        let _ = unreachable.send(Err(e.to_string()));
    }
}
