#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::{wasm_bindgen::JsCast, web_sys};
    use rdf_glance::RdfGlanceApp;
    use web_sys::UrlSearchParams;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("No window").document().expect("No document");

        let query_parameter = web_sys::window()
            .expect("No window")
            .location()
            .search()
            .unwrap_or_default();

        let mut arguments: Vec<String> = Vec::new();

        UrlSearchParams::new_with_str(&query_parameter)
            .ok()
            .and_then(|params| params.get("url"))
            .map(|url| arguments.push(url));

        let canvas = document
            .get_element_by_id("the_canvas")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(RdfGlanceApp::new(cc.storage, arguments)))),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {}
