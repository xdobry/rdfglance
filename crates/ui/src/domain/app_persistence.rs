use serde::{Deserialize, Serialize};
use crate::domain::config::Config;

#[derive(Serialize, Deserialize)]
pub struct AppPersistentData {
    pub last_files: Vec<Box<str>>,
    pub last_endpoints: Vec<Box<str>>,
    #[serde(default = "default_last_projects")]
    pub last_projects: Vec<Box<str>>,
    #[serde(default = "default_config_data")]
    pub config_data: Config,
}

fn default_config_data() -> Config {
    Config::default()
}

fn default_last_projects() -> Vec<Box<str>> {
    Vec::new()
}