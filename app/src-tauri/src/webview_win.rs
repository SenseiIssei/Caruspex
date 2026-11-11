// What the WebView2 control can do beyond Tauri's own API: print a page to
// a PDF file (ICoreWebView2_7::PrintToPdf) and take a picture of the page
// (CapturePreview). Both are COM calls, which is why this is the one module
// of the app with unsafe code.
#![allow(unsafe_code)]

use std::path::Path;
use std::sync::mpsc::Sender;

use tauri::WebviewWindow;
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, ICoreWebView2_7, ICoreWebView2PrintSettings,
};
use webview2_com::{CapturePreviewCompletedHandler, PrintToPdfCompletedHandler};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::System::Com::{IStream, STGM_CREATE, STGM_WRITE};
use windows::Win32::UI::Shell::SHCreateStreamOnFileEx;
use windows_core::{HSTRING, Interface};

/// Prints the page in `webview` to `path`. The answer comes through `done`
/// once WebView2 has written the file.
pub fn print_to_pdf(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target = HSTRING::from(path.as_os_str());
    let failed = done.clone();
    let unreachable = done.clone();
    let result = webview.with_webview(move |platform| {
        let run = || -> windows_core::Result<()> {
            unsafe {
                let core = platform.controller().CoreWebView2()?;
                let core7: ICoreWebView2_7 = core.cast()?;
                let handler = PrintToPdfCompletedHandler::create(Box::new(move |result, ok| {
                    let _ = done.send(match result {
                        Ok(()) if ok => Ok(()),
                        Ok(()) => Err(String::from("PrintToPdf")),
                        Err(e) => Err(e.message()),
                    });
                    Ok(())
                }));
                core7.PrintToPdf(&target, None::<&ICoreWebView2PrintSettings>, &handler)
            }
        };
        if let Err(e) = run() {
            let _ = failed.send(Err(e.message()));
        }
    });
    if let Err(e) = result {
        // with_webview could not even reach the window.
        let _ = unreachable.send(Err(e.to_string()));
    }
}

/// Saves a PNG picture of what `webview` shows to `path`.
pub fn capture_png(webview: &WebviewWindow, path: &Path, done: Sender<Result<(), String>>) {
    let target = HSTRING::from(path.as_os_str());
    let failed = done.clone();
    let _ = webview.with_webview(move |platform| {
        let run = || -> windows_core::Result<()> {
            unsafe {
                let core = platform.controller().CoreWebView2()?;
                let stream: IStream = SHCreateStreamOnFileEx(
                    &target,
                    (STGM_CREATE | STGM_WRITE).0,
                    FILE_ATTRIBUTE_NORMAL.0,
                    true,
                    None::<&IStream>,
                )?;
                let keep = stream.clone();
                let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
                    // Releasing the stream closes the file.
                    drop(keep);
                    let _ = done.send(result.map_err(|e| e.message()));
                    Ok(())
                }));
                core.CapturePreview(
                    COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
                    &stream,
                    &handler,
                )
            }
        };
        if let Err(e) = run() {
            let _ = failed.send(Err(e.message()));
        }
    });
}
