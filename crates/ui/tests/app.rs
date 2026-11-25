use rdf_glance::RdfGlanceApp;

#[test]
fn test_egui_base() {
    let mut app = RdfGlanceApp::new(None, vec![]);
    app.load_ttl("../../sample-rdf-data/programming_languages.ttl", true);
    app.join_load(false);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let output = ctx.run_ui(egui::RawInput::default(), |ui| {
        eframe::App::ui(&mut app, ui, &mut frame);
    });

    assert!(!output.shapes.is_empty(), "the app should paint egui shapes");
}
