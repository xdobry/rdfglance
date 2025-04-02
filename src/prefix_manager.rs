use bimap::BiMap;
use serde::de;

use crate::{NodeAction, RdfGlanceApp};

pub struct PrefixManager {
    // key is the full iri and value is the prefix
    prefixes: BiMap<String, String>,
}

impl PrefixManager {
    pub fn new() -> Self {
        let mut prefix_manager = PrefixManager {
            prefixes: BiMap::new(),
        };
        prefix_manager.add_defaults();
        return prefix_manager;
    }

    fn add_defaults(&mut self) {
        self.prefixes.insert(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
            "rdf".to_string(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
            "rdfs".to_string(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2002/07/owl#".to_string(),
            "owl".to_string(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2001/XMLSchema#".to_string(),
            "xsd".to_string(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2004/02/skos/core#".to_string(),
            "skos".to_string(),
        );
        self
            .prefixes
            .insert("http://purl.org/dc/terms/".to_string(), "dc".to_string());
        self
            .prefixes
            .insert("http://xmlns.com/foaf/0.1/".to_string(), "foaf".to_string());
        self
            .prefixes
            .insert("https://schema.org/".to_string(), "schema".to_string());
        self
            .prefixes
            .insert("http://www.w3.org/ns/prov#".to_string(), "prov".to_string());
        self.prefixes.insert(
            "http://www.opengis.net/ont/geosparql#".to_string(),
            "geo".to_string(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/ontology/".to_string(),
            "dbo".to_string(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/property/".to_string(),
            "dbp".to_string(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/resource/".to_string(),
            "dbr".to_string(),
        );
    }

    pub fn get_prefixed(&self, iri: &str) -> String {
        let delimiter_pos = iri.rfind(&['#', '/'][..]);
        if let Some(delimiter_pos) = delimiter_pos {
            let delimiter_pos = delimiter_pos + 1;
            let base_iri = &iri[..delimiter_pos];
            let prefix = self.prefixes.get_by_left(base_iri);
            if let Some(prefix) = prefix {
                return format!("{}:{}", prefix, &iri[delimiter_pos..]);
            } else {
                let new_search = &iri[..delimiter_pos-1];
                let delemiter_pos2 = new_search.rfind('/');
                if let Some(delemiter_pos2) = delemiter_pos2 {
                    let delemiter_pos2 = delemiter_pos2 + 1;
                    let base_iri2 = &iri[..delemiter_pos2];
                    let prefix2 = self.prefixes.get_by_left(base_iri2);
                    if let Some(prefix2) = prefix2 {
                        return format!("{}:{}", prefix2, &iri[delemiter_pos2..]);
                    }
                }
            }
        }
        return iri.to_string();
    }

    pub fn get_prefixed_opt(&self, iri: &str) -> Option<String> {
        let delimiter_pos = iri.rfind(&['#', '/'][..]).unwrap_or(0) + 1;
        let base_iri = &iri[..delimiter_pos];
        let prefix = self.prefixes.get_by_left(base_iri);
        if let Some(prefix) = prefix {
            return Some(format!("{}:{}", prefix, &iri[delimiter_pos..]));
        }
        return None;
    }

    pub fn get_full_opt(&self, iri: &str) -> Option<String> {
        let delimiter_pos = iri.find(':');
        if let Some(delimiter_pos) = delimiter_pos {
            let prefix = &iri[..delimiter_pos];
            let suffix = &iri[delimiter_pos + 1..];
            let base_iri = self.prefixes.get_by_right(prefix);
            if let Some(base_iri) = base_iri {
                return Some(format!("{}{}", base_iri, suffix));
            }
        }
        return None;
    }
    pub fn has_known_prefix(&self, iri: &str) -> bool {
        let delimiter_pos = iri.find(':');
        if let Some(delimiter_pos) = delimiter_pos {
            let prefix = &iri[..delimiter_pos];
            return self.prefixes.get_by_right(prefix).is_some();
        }
        return false;
    }

    pub fn add_prefix(&mut self, prefix: &str, iri: &str) {
        let iri_exists = self.prefixes.get_by_right(prefix);
        if iri_exists.is_none() {
            self.prefixes.insert(iri.to_string(), prefix.to_string());
        }
    }

    pub fn clean(&mut self) {
        self.prefixes.clear();
        self.add_defaults();
    }
}

impl RdfGlanceApp {
    pub fn show_prefixes(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("prefixes").striped(true).show(ui, |ui| {
                ui.heading("Prefix");
                ui.heading("Iri");
                ui.end_row();
                for (iri, prefix) in &self.prefix_manager.prefixes {
                    ui.label(prefix);
                    ui.label(iri);
                    ui.end_row();
                }
            });
        });
        return NodeAction::None;
    }
}
