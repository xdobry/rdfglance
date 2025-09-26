use egui::{Align, Layout, Slider};
use serde::{Deserialize, Serialize};

use crate::{NodeAction, RdfGlanceApp};

#[derive(Serialize, Deserialize)]
pub struct Config {
    // nodes force
    pub repulsion_constant: f32,
    // edges force
    pub attraction_factor: f32,
    #[serde(default = "default_1")]
    pub m_repulsion_constant: f32,
    #[serde(default = "default_1")]
    pub m_attraction_factor: f32,
    pub language_filter: String,
    #[serde(default = "default_true")]
    pub suppress_other_language_data: bool,
    #[serde(default = "default_true")]
    pub create_iri_prefixes_automatically: bool,
    #[serde(default = "default_iri_display")]
    pub iri_display: IriDisplay,
    #[serde(default = "default_true")]
    pub resolve_rdf_lists: bool,
    #[serde(default = "default_1")]
    pub community_resolution: f32,
    #[serde(default = "default_true")]
    pub community_randomize: bool,
    #[serde(default = "default_true")]
    pub short_iri: bool,
    #[serde(default = "default_40_000")]
    pub max_visible_nodes: usize,
    #[serde(default = "default_250")]
    pub gravity_effect_radius: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Copy, Clone)]
pub enum IriDisplay {
    Full,
    Prefixed,
    Label,
    LabelOrShorten,
    Shorten,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repulsion_constant: 1.5,
            attraction_factor: 0.0015,
            language_filter: "en".to_string(),
            suppress_other_language_data: true,
            create_iri_prefixes_automatically: true,
            iri_display: IriDisplay::LabelOrShorten,
            resolve_rdf_lists: true,
            m_repulsion_constant: 0.5,
            m_attraction_factor: 0.5,
            community_resolution: 1.0,
            community_randomize: true,
            short_iri: true,
            max_visible_nodes: 40_000,
            gravity_effect_radius: 250.0,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_iri_display() -> IriDisplay {
    IriDisplay::Full
}

fn default_1() -> f32 {
    1.0
}

fn default_250() -> f32 {
    250.0
}

fn default_40_000() -> usize {
    40_000
}

impl Config {
    pub fn language_filter(&self) -> Vec<String> {
        self.language_filter
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}

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
                        ui.label("Version 0.10 - 09/2025");
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
