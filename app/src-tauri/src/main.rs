// Caruspex desktop app. For now only the window: the frameless main window
// with the web interface. The commands and events of
// app/src/api/contract.ts are wired up here once the backend crates are in.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Caruspex");
}
