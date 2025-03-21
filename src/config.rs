use serde::{Deserialize, Serialize};

use crate::{NodeAction, VisualRdfApp};

#[derive(Serialize, Deserialize)]
pub struct Config {
    // nodes force
    pub repulsion_constant: f32,
    // edges force
    pub attraction_factor: f32,
    pub language_filter: String,
    #[serde(default = "default_supress_other_language_data")]
    pub supress_other_language_data: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repulsion_constant: 1.5,
            attraction_factor: 0.05,
            language_filter: "en".to_string(),
            supress_other_language_data: true,
        }
    }
}

fn default_supress_other_language_data() -> bool {
    return true;
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
        return NodeAction::None;
    }
}