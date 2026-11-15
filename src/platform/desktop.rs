use crate::{RdfGlanceApp, support::uitools::load_icon};
use egui::ViewportBuilder;

pub fn run() -> Result<(), eframe::Error> {
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
