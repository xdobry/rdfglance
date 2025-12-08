use egui::{Align, Layout, Slider};

use crate::{
    uistate::actions::NodeAction, 
    RdfGlanceApp, 
    domain::config::IriDisplay
};

impl RdfGlanceApp {
    pub fn show_config(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        ui.horizontal(|ui| {
            ui.label("language filter (comma separated):");
            ui.text_edit_singleline(&mut self.persistent_data.config_data.language_filter);
        });
        ui.checkbox(
            &mut self.persistent_data.config_data.suppress_other_language_data,
            "Supress data in not display language",
        );
        ui.label("Predicate and Type display:");
        ui.radio_value(
            &mut self.persistent_data.config_data.iri_display,
            IriDisplay::Label,
            "Label",
        );
        ui.radio_value(
            &mut self.persistent_data.config_data.iri_display,
            IriDisplay::LabelOrShorten,
            "Label or IRI Shorten",
        );
        ui.radio_value(
            &mut self.persistent_data.config_data.iri_display,
            IriDisplay::Prefixed,
            "IRI Prefixed",
        );
        ui.radio_value(
            &mut self.persistent_data.config_data.iri_display,
            IriDisplay::Shorten,
            "IRI Shorten",
        );
        ui.radio_value(
            &mut self.persistent_data.config_data.iri_display,
            IriDisplay::Full,
            "Full IRI",
        );
        ui.checkbox(
            &mut self.persistent_data.config_data.resolve_rdf_lists,
            "Resolve rdf lists",
        );
        //ui.text_edit_singleline(text)
        ui.horizontal(|ui| {
            ui.label("Community resolution:");
            ui.add(
                egui::DragValue::new(&mut self.persistent_data.config_data.community_resolution)
                    .speed(0.01)
                    .range(0.10..=3.0),
            );
        });
        ui.checkbox(
            &mut self.persistent_data.config_data.community_randomize,
            "community detection randomize",
        );
        ui.add(Slider::new(&mut self.persistent_data.config_data.max_visible_nodes, 1000..=200_000).text("Max nodes in visual graph"));
        ui.add(Slider::new(&mut self.persistent_data.config_data.gravity_effect_radius, 50.0..=1000.0).text("Gravity effect radius for layout"));
        NodeAction::None
    }

    pub fn show_about(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        if self.ui_state.about_window {
            egui::Window::new("About")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]) // Center the modal
                .show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.heading("RDF Glance");
                        ui.spacing();
                        ui.label("Version 0.12 - 11/2025");
                        ui.label("A lightweight RDF visualizer");
                        ui.label("GNU General Public License 3.0 Software");
                        ui.spacing();
                        ui.hyperlink_to("GitHub Site", "https://github.com/xdobry/rdfglance");
                        ui.label("Author: Artur T. <mail@xdobry.de>");
                    });
                    ui.spacing();
                    if ui.button("Cancel").clicked() {
                        self.ui_state.about_window = false;
                    }
                });
            ui.disable();
        }
    }
}
