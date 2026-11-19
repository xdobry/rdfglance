#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    use eframe::egui::ViewportBuilder;
    use rdf_glance::{RdfGlanceApp, support::uitools::load_icon};
    use tracing_subscriber::EnvFilter;

    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(error) if std::env::var_os("RUST_LOG").is_some() => {
            eprintln!("Invalid RUST_LOG filter; using info: {error}");
            EnvFilter::new("info")
        }
        Err(_) => EnvFilter::new("info"),
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_icon(load_icon()),
        ..eframe::NativeOptions::default()
    };
    let args = std::env::args().skip(1).collect();

    eframe::run_native(
        "rdf-glance",
        options,
        Box::new(|cc| Ok(Box::new(RdfGlanceApp::new(cc.storage, args)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}
