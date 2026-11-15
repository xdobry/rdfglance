#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    rdf_glance::platform::desktop::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    rdf_glance::platform::wasm::run();
}
