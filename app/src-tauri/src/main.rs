// Caruspex on the desktop: everything is in lib.rs, which the Android app
// loads as a library too.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unsafe_code)]

fn main() {
    caruspex_app_lib::run();
}
