use std::path::Path;

use const_format::concatcp;
use egui::{Align, Layout};
use rfd::FileDialog;

use crate::{style::ICON_LANG, RdfGlanceApp, SystemMessage};


impl RdfGlanceApp {
    pub fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load Project").clicked() {
                    self.load_project_dialog();
                    ui.close_menu();
                }
                if ui.button("Save Project").clicked() {
                    self.save_project_dialog();
                    ui.close_menu();
                }
                if !self.persistent_data.last_projects.is_empty() {
                    let mut last_project_clicked: Option<Box<str>> = None;
                    ui.menu_button("Last Visited Projects:", |ui| {
                        for last_file in &self.persistent_data.last_projects {
                            if ui.button(last_file).clicked() {
                                last_project_clicked = Some(last_file.clone());
                            }
                        }
                        if let Some(last_project_clicked) = last_project_clicked {
                            ui.close_menu();
                            let last_project_path = Path::new(&*last_project_clicked);
                            self.load_project(&last_project_path);
                        }
                    });
                }
                ui.separator();
                if ui.button("Import RDF File").clicked() {
                    self.import_file_dialog(ui);
                    ui.close_menu();
                }
                if ui.button("Import all from dir").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        let selected_dir = Some(path.display().to_string());
                        if let Some(selected_dir) = &selected_dir {
                            self.load_ttl_dir(selected_dir);
                        }
                    }
                    ui.close_menu();
                }
                /*
                if ui.button("Sparql Endpoint").clicked() {
                    self.sparql_dialog =
                        Some(SparqlDialog::new(&self.persistent_data.last_endpoints));
                    ui.close_menu();
                }
                 */
                if !self.persistent_data.last_files.is_empty() {
                    let mut last_file_clicked: Option<Box<str>> = None;
                    ui.menu_button("Last Imported Files:", |ui| {
                        for last_file in &self.persistent_data.last_files {
                            if ui.button(last_file).clicked() {
                                last_file_clicked = Some(last_file.clone());
                            }
                        }
                        if let Some(last_file_clicked) = last_file_clicked {
                            ui.close_menu();
                            self.load_ttl(&last_file_clicked);
                        }
                    });
                }
                ui.separator();
                if ui.button("Clean Data").clicked() {
                    self.clean_data();
                    ui.close_menu();
                }
            });
            let selected_language = self.node_data.get_language(self.ui_state.display_language);
            if let Some(selected_language) = selected_language {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_label(concatcp!(ICON_LANG," Data language"))
                    .selected_text(selected_language)
                    .show_ui(ui, |ui| {
                        for language_index in self.ui_state.language_sort.iter() {
                            let language_str = self.node_data.get_language(*language_index);
                            if let Some(language_str) = language_str {
                                if ui.selectable_label(self.ui_state.display_language == *language_index, language_str).clicked() {
                                    self.ui_state.display_language = *language_index;
                                }
                            }
                        }
                    });
              });
            }
        });
    }
    pub fn import_file_dialog(&mut self, _ui: &mut egui::Ui) {
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Files", &vec!["ttl","rdf","xml","nt","trig","nq"]).pick_file() {
            let selected_file = Some(path.display().to_string());
            if let Some(selected_file) = &selected_file {
                self.load_ttl(selected_file);
            }
        }
    }
    pub fn load_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Glance project", &vec!["rdfglance"]).pick_file() {
            self.load_project(path.as_path());
        }
    }
    pub fn load_project(&mut self, path: &Path) {
        let restore = Self::restore(Path::new(path));
        match restore {
            Err(e) => {
                self.system_message = SystemMessage::Error(format!("Can not load porject: {}", e));
            }
            Ok(app_data) => {
                self.clean_data();
                self.node_data = app_data.node_data;
                self.update_data_indexes();
                let file_name: Box<str> = Box::from(path.display().to_string());
                if !self
                    .persistent_data
                    .last_projects
                    .iter().any(|f | *f == file_name)
                {
                    self.persistent_data.last_projects.push(file_name);
                }
            }
        }
    }
    pub fn save_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Glance project", &vec!["rdfglance"]).set_file_name("project.rdfglance").save_file() {
            let store_res = self.store(Path::new(path.as_path()));
            match store_res {
                Err(e) => {
                    self.system_message = SystemMessage::Error(format!("Can not save project: {}", e));
                }
                Ok(_) => {
                    self.set_status_message("Project saved");
                }
            }
        }
        
    }
}