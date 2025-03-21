use std::collections::HashMap;

use crate::{NodeAction, VisualRdfApp};

pub struct PrefixManager {
    // key is the full iri and value is the prefix
    prefixes: HashMap<String, String>,
}

impl PrefixManager {
    pub fn new() -> Self {
        let mut prefix_manager = PrefixManager {
            prefixes: HashMap::new(),
        };
        prefix_manager.prefixes.insert(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
            "rdf".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
            "rdfs".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://www.w3.org/2002/07/owl#".to_string(),
            "owl".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://www.w3.org/2001/XMLSchema#".to_string(),
            "xsd".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://www.w3.org/2004/02/skos/core#".to_string(),
            "skos".to_string(),
        );
        prefix_manager
            .prefixes
            .insert("http://purl.org/dc/terms/".to_string(), "dc".to_string());
        prefix_manager
            .prefixes
            .insert("http://xmlns.com/foaf/0.1/".to_string(), "foaf".to_string());
        prefix_manager
            .prefixes
            .insert("https://schema.org/".to_string(), "schema".to_string());
        prefix_manager
            .prefixes
            .insert("http://www.w3.org/ns/prov#".to_string(), "prov".to_string());
        prefix_manager.prefixes.insert(
            "http://www.opengis.net/ont/geosparql#".to_string(),
            "geo".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://dbpedia.org/ontology/".to_string(),
            "dbo".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://dbpedia.org/property/".to_string(),
            "dbp".to_string(),
        );
        prefix_manager.prefixes.insert(
            "http://dbpedia.org/resource/".to_string(),
            "dbr".to_string(),
        );
        return prefix_manager;
    }

    pub fn get_prefixed(&self, iri: &str) -> String {
        let delimiter_pos = iri.rfind(&['#', '/'][..]).unwrap_or(0) + 1;
        let base_iri = &iri[..delimiter_pos];
        let prefix = self.prefixes.get(base_iri);
        if let Some(prefix) = prefix {
            return format!("{}:{}", prefix, &iri[delimiter_pos..]);
        }
        return iri.to_string();
    }

    pub fn get_prefixed_opt(&self, iri: &str) -> Option<String> {
        let delimiter_pos = iri.rfind(&['#', '/'][..]).unwrap_or(0) + 1;
        let base_iri = &iri[..delimiter_pos];
        let prefix = self.prefixes.get(base_iri);
        if let Some(prefix) = prefix {
            return Some(format!("{}:{}", prefix, &iri[delimiter_pos..]));
        }
        return None;
    }
}

impl VisualRdfApp {
    pub fn show_prefixes(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        egui::Grid::new("prefixes")
            .striped(true)
            .show(ui, |ui| {
                ui.heading("Prefix");
                ui.heading("Iri");
                ui.end_row();
            for (iri, prefix) in &self.prefix_manager.prefixes {
                    ui.label(prefix);
                    ui.label(iri);
                    ui.end_row();
                }
            });
        return NodeAction::None;
    }
}
