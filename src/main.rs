#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::ViewportBuilder;
use rdf_glance::RdfGlanceApp;
use rdf_glance::uitools::load_icon;


#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_icon(load_icon()),
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
fn main() {
    // Web start

    use eframe::{wasm_bindgen::JsCast, web_sys};
    use web_sys::UrlSearchParams;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let query_parameter = web_sys::window().expect("No window").location().search().unwrap_or_default();

        let mut argumetns: Vec<String> = Vec::new();

        UrlSearchParams::new_with_str(&query_parameter).ok()
            .and_then(|params| params.get("url"))
            .map(|url| {
                argumetns.push(url);
            });

        let canvas = document
            .get_element_by_id("the_canvas")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");
        
        eframe::WebRunner::new()
            .start(
                canvas, // matches id in index.html
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(RdfGlanceApp::new(cc.storage,argumetns)))),
            )
            .await
            .expect("failed to start eframe");
    });
}

