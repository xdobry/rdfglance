use bimap::BiMap;

use crate::{NodeAction, RdfGlanceApp};

pub struct PrefixManager {
    // key is the full iri and value is the prefix
    prefixes: BiMap<Box<str>, Box<str>>,
}

impl Default for PrefixManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PrefixManager {
    pub fn new() -> Self {
        let mut prefix_manager = PrefixManager {
            prefixes: BiMap::new(),
        };
        prefix_manager.add_defaults();
        prefix_manager
    }

    fn add_defaults(&mut self) {
        self.prefixes.insert(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".into(),
            "rdf".into(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2000/01/rdf-schema#".into(),
            "rdfs".into(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2002/07/owl#".into(),
            "owl".into(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2001/XMLSchema#".into(),
            "xsd".into(),
        );
        self.prefixes.insert(
            "http://www.w3.org/2004/02/skos/core#".into(),
            "skos".into(),
        );
        self
            .prefixes
            .insert("http://purl.org/dc/terms/".into(), "dc".into());
        self
            .prefixes
            .insert("http://xmlns.com/foaf/0.1/".into(), "foaf".into());
        self
            .prefixes
            .insert("https://schema.org/".into(), "schema".into());
        self
            .prefixes
            .insert("http://www.w3.org/ns/prov#".into(), "prov".into());
        self.prefixes.insert(
            "http://www.opengis.net/ont/geosparql#".into(),
            "geo".into(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/ontology/".into(),
            "dbo".into(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/property/".into(),
            "dbp".into(),
        );
        self.prefixes.insert(
            "http://dbpedia.org/resource/".into(),
            "dbr".into(),
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
        iri.to_string()
    }

    pub fn get_prefixed_opt(&self, iri: &str) -> Option<String> {
        let delimiter_pos = iri.rfind(&['#', '/'][..]).unwrap_or(0) + 1;
        let base_iri = &iri[..delimiter_pos];
        let prefix = self.prefixes.get_by_left(base_iri);
        if let Some(prefix) = prefix {
            return Some(format!("{}:{}", prefix, &iri[delimiter_pos..]));
        }
        None
    }

    pub fn get_full_opt(&self, iri: &str) -> Option<Box<str>> {
        let delimiter_pos = iri.find(':');
        if let Some(delimiter_pos) = delimiter_pos {
            let prefix = &iri[..delimiter_pos];
            let suffix = &iri[delimiter_pos + 1..];
            let base_iri = self.prefixes.get_by_right(prefix);
            if let Some(base_iri) = base_iri {
                return Some(format!("{}{}", base_iri, suffix).into());
            }
        }
        None
    }
    pub fn has_known_prefix(&self, iri: &str) -> bool {
        let delimiter_pos = iri.find(':');
        if let Some(delimiter_pos) = delimiter_pos {
            let prefix = &iri[..delimiter_pos];
            return self.prefixes.get_by_right(prefix).is_some();
        }
        false
    }

    pub fn add_prefix(&mut self, prefix: &str, iri: &str) {
        let iri_exists = self.prefixes.get_by_right(prefix);
        if iri_exists.is_none() {
            self.prefixes.insert(iri.into(), prefix.into());
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
        NodeAction::None
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_prefix_manager()  {
        let mut prefix_manager = super::PrefixManager::new();
        assert_eq!(prefix_manager.get_prefixed("http://www.w3.org/2000/01/rdf-schema#Class"),"rdfs:Class");
        assert_eq!(prefix_manager.get_prefixed("http://not_managed#Foo"),"http://not_managed#Foo");
        assert_eq!(prefix_manager.get_prefixed_opt("http://not_managed#Foo"),None);
        assert_eq!(prefix_manager.get_full_opt("rdfs:Class"),Some("http://www.w3.org/2000/01/rdf-schema#Class".into()));
        assert_eq!(prefix_manager.get_full_opt("unknown:Foo"),None);
        assert_eq!(prefix_manager.has_known_prefix("rdfs:Class"),true);
        assert_eq!(prefix_manager.has_known_prefix("unknown:Foo"),false);
        prefix_manager.add_prefix("atk", "http://atk.com#");
        assert_eq!(prefix_manager.get_prefixed("http://atk.com#Foo"),"atk:Foo");
        assert_eq!(prefix_manager.get_full_opt("atk:Foo"),Some("http://atk.com#Foo".into()));
        prefix_manager.clean();
        assert_eq!(prefix_manager.get_full_opt("atk:Foo"),None);
    }
}
