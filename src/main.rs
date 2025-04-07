#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::ViewportBuilder;
use rdf_glance::RdfGlanceApp;
use rdf_glance::uitools::load_icon;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_icon(load_icon()),
        ..eframe::NativeOptions::default()
    };
    eframe::run_native(
        "rdf-glance",
        options,
        Box::new(|cc| Ok(Box::new(RdfGlanceApp::new(cc.storage)))),
    )
}

