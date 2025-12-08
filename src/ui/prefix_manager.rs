

use crate::{uistate::actions::NodeAction, RdfGlanceApp};


impl RdfGlanceApp {
    pub fn show_prefixes(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("prefixes").striped(true).show(ui, |ui| {
                ui.heading("Prefix");
                ui.heading("Iri");
                ui.end_row();
                self.read_rdf_data(|rdf_data| {
                    for (iri, prefix) in &rdf_data.prefix_manager.prefixes {
                        ui.label(prefix);
                        ui.label(iri);
                        ui.end_row();
                    }
                });
            });
        });
        NodeAction::None
    }
}

