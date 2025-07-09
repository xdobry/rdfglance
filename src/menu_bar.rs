use std::path::Path;

use const_format::concatcp;
use egui::{global_theme_preference_switch, Align, Layout};
#[cfg(target_arch = "wasm32")]
use poll_promise::Promise;
#[cfg(target_arch = "wasm32")]
use rfd::AsyncFileDialog;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use crate::{RdfGlanceApp, SystemMessage, style::ICON_LANG};

impl RdfGlanceApp {
    pub fn menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Load Project").clicked() {
                        self.load_project_dialog(ui.visuals().dark_mode);
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
                                self.load_project(last_project_path, ui.visuals().dark_mode);
                            }
                        });
                    }
                    ui.separator();
                }
                if ui.button("Import RDF File").clicked() {
                    self.import_file_dialog(ui);
                    ui.close_menu();
                }
                #[cfg(not(target_arch = "wasm32"))]
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
                            self.load_ttl(&last_file_clicked, ui.visuals().dark_mode);
                            ui.ctx().request_repaint();
                        }
                    });
                }
                ui.separator();
                if ui.button("Clean Data").clicked() {
                    self.clean_data();
                    ui.close_menu();
                }
            });
            global_theme_preference_switch(ui);
            if let Ok(rdf_data) = self.rdf_data.read() {
                let selected_language = rdf_data.node_data.get_language(self.ui_state.display_language);
                if let Some(selected_language) = selected_language {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_label(concatcp!(ICON_LANG, " Data language"))
                            .selected_text(selected_language)
                            .show_ui(ui, |ui| {
                                for language_index in self.ui_state.language_sort.iter() {
                                    let language_str = rdf_data.node_data.get_language(*language_index);
                                    if let Some(language_str) = language_str {
                                        if ui
                                            .selectable_label(
                                                self.ui_state.display_language == *language_index,
                                                language_str,
                                            )
                                            .clicked()
                                        {
                                            self.ui_state.display_language = *language_index;
                                        }
                                    }
                                }
                            });
                        ui.label(format!("{:.4}",self.ui_state.cpu_usage));
                    });
                }
            }
        });
    }
    pub fn import_file_dialog(&mut self, ui: &mut egui::Ui) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Files", &["ttl", "rdf", "xml", "nt", "trig", "nq"])
            .pick_file()
        {
            let selected_file = Some(path.display().to_string());
            if let Some(selected_file) = &selected_file {
                self.load_ttl(selected_file, ui.visuals().dark_mode);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.file_upload = Some(Promise::spawn_local(async {
                let file_selected = rfd::AsyncFileDialog::new()
                    .add_filter("rdf", &["ttl", "rdf", "xml", "nt", "trig", "nq"])
                    .pick_file()
                    .await;
                if let Some(curr_file) = file_selected {
                    let buf = curr_file.read().await;
                    return Ok(crate::File {
                        path: curr_file.file_name(),
                        data: buf,
                    });
                }
                // no file selected
                Err(anyhow::anyhow!("Upload: no file Selected"))
            }));
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn handle_files(&mut self, ctx: &egui::Context, visuals: &egui::Visuals) {
        if let Some(result) = &self.file_upload {
            match &result.ready() {
                Some(Ok(crate::File { path, data })) => {
                    let language_filter = self.persistent_data.config_data.language_filter();
                    let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
                        let rdfttl = crate::rdfwrap::RDFWrap::load_file_data(
                            path,
                            data,
                            &mut rdf_data,
                            &language_filter,
                        );
                        Some(rdfttl)
                    } else {
                        None
                    };
                    if let Some(rdfttl) = rdfttl {
                        match rdfttl {
                            Err(err) => {
                                self.system_message = SystemMessage::Error(format!("File not found: {}", err));
                            }
                            Ok(triples_count) => {
                                let load_message = format!("Loaded: {} triples: {}", path, triples_count);
                                self.set_status_message(&load_message);
                                self.update_data_indexes(visuals.dark_mode);
                            }
                        }
                    }
                    self.file_upload = None;
                }
                Some(Err(e)) => {
                    self.file_upload = None;
                }
                None => {}
            }
        }
    }
    pub fn load_project_dialog(&mut self, is_dark_mode: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Glance project", &["rdfglance"])
            .pick_file()
        {
            self.load_project(path.as_path(), is_dark_mode);
        }
    }
    pub fn load_project(&mut self, path: &Path, is_dark_mode: bool) {
        let restore = Self::restore(Path::new(path));
        match restore {
            Err(e) => {
                self.system_message = SystemMessage::Error(format!("Can not load project: {}", e));
            }
            Ok(app_data) => {
                self.clean_data();
                self.rdf_data = app_data.rdf_data;
                self.ui_state = app_data.ui_state;
                self.visible_nodes = app_data.visible_nodes;
                self.update_data_indexes(is_dark_mode);
                if !app_data.visualisation_style.node_styles.is_empty() {
                    self.visualisation_style = app_data.visualisation_style;
                }
                let file_name: Box<str> = Box::from(path.display().to_string());
                if !self.persistent_data.last_projects.iter().any(|f| *f == file_name) {
                    self.persistent_data.last_projects.push(file_name);
                }
            }
        }
    }
    pub fn save_project_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = FileDialog::new()
            .add_filter("RDF Glance project", &["rdfglance"])
            .set_file_name("project.rdfglance")
            .save_file()
        {
            let store_res = self.store(Path::new(path.as_path()));
            match store_res {
                Err(e) => {
                    self.system_message = SystemMessage::Error(format!("Can not save project: {}", e));
                }
                Ok(_) => {
                    let file_name: Box<str> = Box::from(path.display().to_string());
                    if !self.persistent_data.last_projects.iter().any(|f| *f == file_name) {
                        self.persistent_data.last_projects.push(file_name);
                    }
                    self.set_status_message("Project saved");
                }
            }
        }
    }
}
