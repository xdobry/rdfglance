use serde::{Deserialize, Serialize};

use crate::{NodeAction, VisualRdfApp};

#[derive(Serialize, Deserialize)]
pub struct Config {
    // nodes force
    pub repulsion_constant: f32,
    // edges force
    pub attraction_factor: f32,
    pub language_filter: String,
    #[serde(default = "default_true")]
    pub supress_other_language_data: bool,
    #[serde(default = "default_true")]
    pub create_iri_prefixes_automatically: bool,
    #[serde(default = "default_iri_display")]
    pub iri_display: IriDisplay,
    #[serde(default = "default_true")]
    pub resolve_rdf_lists: bool,
}

#[derive(Serialize, Deserialize, PartialEq)]
#[derive(Copy, Clone)]
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
            attraction_factor: 0.05,
            language_filter: "en".to_string(),
            supress_other_language_data: true,
            create_iri_prefixes_automatically: true,
            iri_display: IriDisplay::Full,
            resolve_rdf_lists: true,
        }
    }
}

fn default_true() -> bool {
    return true;
}

fn default_iri_display() -> IriDisplay {
    return IriDisplay::Full;
}

impl Config {
    pub fn language_filter(&self) -> Vec<String> {
        self.language_filter.split(',').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect()
    }
}

impl VisualRdfApp {
    pub fn show_config(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        ui.horizontal(|ui| {
            ui.label("language filter (comma separated):");
            ui.text_edit_singleline(&mut self.persistent_data.config_data.language_filter);
        });
        ui.checkbox(&mut self.persistent_data.config_data.supress_other_language_data, "Supress data in not display language");
        ui.label("Predicate and Type display:");
        ui.radio_value(&mut self.persistent_data.config_data.iri_display, IriDisplay::Label, "Label");
        ui.radio_value(&mut self.persistent_data.config_data.iri_display, IriDisplay::LabelOrShorten, "Label or IRI Shorten");
        ui.radio_value(&mut self.persistent_data.config_data.iri_display, IriDisplay::Prefixed, "IRI Prefixed");
        ui.radio_value(&mut self.persistent_data.config_data.iri_display, IriDisplay::Shorten, "IRI Shorten");
        ui.radio_value(&mut self.persistent_data.config_data.iri_display, IriDisplay::Full, "Full IRI");
        ui.checkbox(&mut self.persistent_data.config_data.resolve_rdf_lists, "Resolve rdf lists");
        return NodeAction::None;
    }
}