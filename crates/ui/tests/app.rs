use rdf_glance::{RdfGlanceApp, uistate::DisplayType};

#[test]
fn test_egui_base() {
    let mut app = RdfGlanceApp::new(None, vec![]);
    app.load_ttl("../../sample-rdf-data/programming_languages.ttl", true);
    app.join_load(false);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    for display_type in [
        DisplayType::Table,
        DisplayType::Graph,
        DisplayType::Browse,
        DisplayType::MetaGraph,
        DisplayType::VisualQuery,
        DisplayType::Statistics,
        DisplayType::Prefixes,
        DisplayType::Configuration,
        DisplayType::ReferenceResolver,
    ] {
        app.display_type = display_type;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            eframe::App::ui(&mut app, ui, &mut frame);
        });

        assert!(
            !output.shapes.is_empty(),
            "{:?} should paint egui shapes",
            app.display_type
        );
        output.textures_delta.clear();
    }
}
