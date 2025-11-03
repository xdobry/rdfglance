use serde::{Deserialize, Serialize};

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