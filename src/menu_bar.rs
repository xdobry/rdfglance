use std::path::Path;

use const_format::concatcp;
use egui::{Align, Key, Layout, Modifiers, global_theme_preference_switch};
#[cfg(target_arch = "wasm32")]
use poll_promise::Promise;
#[cfg(target_arch = "wasm32")]
use rfd::AsyncFileDialog;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
use strum::IntoEnumIterator;

use crate::{
    graph_algorithms::GraphAlgorithm, graph_view::NodeContextAction, layoutalg::{circular::circular_layout, hierarchical::{hierarchical_layout, LayoutOrientation}, spectral::spectral_layout}, statistics::StatisticsData, style::ICON_LANG, ImportFormat, ImportFromUrlData, RdfGlanceApp, SystemMessage
};

enum MenuAction {
    None,
    LoadProject,
    ImportRDF,
    SaveProject,
}

impl RdfGlanceApp {
    pub fn menu_bar(&mut self, ui: &mut egui::Ui) {
        let mut menu_action = MenuAction::None;
        ui.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(Key::O) {
                menu_action = MenuAction::ImportRDF;
            } else if i.modifiers.ctrl && i.key_pressed(Key::L) {
                menu_action = MenuAction::LoadProject;
            } else if i.modifiers.ctrl && i.key_pressed(Key::S) {
                menu_action = MenuAction::SaveProject;
            }
        });
        egui::menu::bar(ui, |ui| {
            let mut consume_keys = false;
            ui.menu_button("File", |ui| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Load Project").clicked() {
                        menu_action = MenuAction::LoadProject;
                        ui.close_menu();
                    }
                    if ui.button("Save Project\tCtrl-S").clicked() {
                        menu_action = MenuAction::SaveProject;
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
                if ui.button("Import RDF File\tCtrl-O").clicked() {
                    menu_action = MenuAction::ImportRDF;
                    ui.close_menu();
                }
                if ui.button("Import RDF File from URL").clicked() {
                    self.import_file_from_url_dialog(ui);
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
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Export Edges").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("CSV table", &["csv"])
                        .set_file_name("edges.csv")
                        .save_file()
                    {
                        if let Ok(rdf_data) = self.rdf_data.read() {
                            use crate::nobject::LabelContext;
                            let label_context = LabelContext::new(
                                self.ui_state.display_language,
                                self.persistent_data.config_data.iri_display,
                                &rdf_data.prefix_manager,
                            );
                            let store_res =
                                self.export_edges(Path::new(path.as_path()), &rdf_data.node_data, &label_context);
                            match store_res {
                                Err(e) => {
                                    self.system_message = SystemMessage::Error(format!("Can not export edges: {}", e));
                                }
                                Ok(_) => {}
                            }
                        }
                    }
                    ui.close_menu();
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button("Export Nodes").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("CSV table", &["csv"])
                        .set_file_name("nodes.csv")
                        .save_file()
                    {
                        if let Ok(rdf_data) = self.rdf_data.read() {
                            use crate::nobject::LabelContext;
                            let label_context = LabelContext::new(
                                self.ui_state.display_language,
                                self.persistent_data.config_data.iri_display,
                                &rdf_data.prefix_manager,
                            );
                            let store_res =
                                self.export_nodes(Path::new(path.as_path()), &rdf_data.node_data, &label_context, &self.visualization_style);
                            match store_res {
                                Err(e) => {
                                    self.system_message = SystemMessage::Error(format!("Can not export edges: {}", e));
                                }
                                Ok(_) => {}
                            }
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
                            let path = Path::new(last_file_clicked.as_ref());
                            if path.exists() {
                                if path.is_dir() {
                                    self.load_ttl_dir(&last_file_clicked);
                                } else {
                                    self.load_ttl(&last_file_clicked, ui.visuals().dark_mode);
                                }
                            }
                            ui.ctx().request_repaint();
                        }
                    });
                }
                ui.separator();
                if ui.button("Clean Data").clicked() {
                    self.clean_data();
                    ui.close_menu();
                }
                consume_keys = true;
            });
            if matches!(self.display_type, crate::DisplayType::Graph) {
                ui.menu_button("Selection", |ui| {
                    ui.add_enabled_ui(self.ui_state.selected_node.is_some(), |ui| {
                        if ui.button("Lock Position").clicked() {
                            self.ui_state.menu_action = Some(NodeContextAction::ChangeLockPosition(true));
                            ui.close_menu();
                        }
                        if ui.button("Unlock Position").clicked() {
                            self.ui_state.menu_action = Some(NodeContextAction::ChangeLockPosition(false));
                            ui.close_menu();
                        }
                        if ui.button("Expand Selection").clicked() {
                            self.visible_nodes.expand_selection(&mut self.ui_state);
                            ui.close_menu();
                        }
                        if ui.button("Shirk Selection").clicked() {
                            self.visible_nodes.shirk_selection(&mut self.ui_state);
                            ui.close_menu();
                        }
                        if ui.button("Invert Selection").clicked() {
                            self.visible_nodes.invert_selection(&mut self.ui_state);
                            ui.close_menu();
                        }
                        if ui.button("Deselect All").clicked() {
                            self.visible_nodes.deselect_all(&mut self.ui_state);
                            ui.close_menu();
                        }
                    });
                    if ui.button("Select All Ctrl-A").clicked() {
                        self.visible_nodes.select_all(&mut self.ui_state);
                        ui.close_menu();
                    }
                    ui.separator();
                    ui.add_enabled_ui(self.ui_state.selected_nodes.len()>2 , |ui| {
                        if ui.button("Circular Layout").clicked() {
                            circular_layout(&mut self.visible_nodes,&self.ui_state.selected_nodes,&self.ui_state.hidden_predicates);
                            ui.close_menu();
                        }
                        if ui.button("Hierarchical Layout Horizontal").clicked() {
                            hierarchical_layout(&mut self.visible_nodes,&self.ui_state.selected_nodes,&self.ui_state.hidden_predicates, LayoutOrientation::Horizontal);
                            ui.close_menu();
                        }
                        if ui.button("Hierarchical Layout Vertical").clicked() {
                            hierarchical_layout(&mut self.visible_nodes,&self.ui_state.selected_nodes,&self.ui_state.hidden_predicates,LayoutOrientation::Vertical);
                            ui.close_menu();
                        }
                         if ui.button("Spectral Layout").clicked() {
                            spectral_layout(&mut self.visible_nodes,&self.ui_state.selected_nodes,&self.ui_state.hidden_predicates);
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(self.ui_state.selected_nodes.len()>=2 , |ui| {
                        if ui.button("Find Shortest Connections").clicked() {
                            self.find_connections();
                            ui.close_menu();
                        }
                    });
                    consume_keys = true;
                });
            }
            ui.menu_button("Statistics", |ui| {
                for entry in GraphAlgorithm::iter() {
                    let label = entry.to_string();
                    if ui.button(label).clicked() {
                        if self.visible_nodes.nodes.read().unwrap().is_empty() {
                            self.system_message = SystemMessage::Info(
                                "No data to compute statistics. Add nodes to visual graph".to_string(),
                            );
                            ui.close_menu();
                            return;
                        }
                        if self.statistics_data.is_none() {
                            self.statistics_data = Some(StatisticsData::default());
                        }
                        self.visible_nodes.run_algorithm(
                            entry,
                            &self.visualization_style,
                            self.statistics_data.as_mut().unwrap(),
                            &self.persistent_data.config_data,
                            &self.ui_state.hidden_predicates,
                        );
                        // TODO ask for confirmation
                        self.visualization_style.use_size_overwrite = true;
                        self.visualization_style.use_color_overwrite = true;
                        ui.close_menu();
                    }
                }
                ui.separator();
                ui.add_enabled_ui(
                    self.statistics_data.as_ref().is_some_and(|f| !f.results.is_empty()),
                    |ui| {
                        if ui
                            .checkbox(
                                &mut self.visualization_style.use_size_overwrite,
                                "Node size from statistics",
                            )
                            .changed()
                        {
                            self.visible_nodes.update_node_shapes = true;
                        }
                        if ui
                            .checkbox(
                                &mut self.visualization_style.use_color_overwrite,
                                "Node color from statistics",
                            )
                            .changed()
                        {
                            self.visible_nodes.update_node_shapes = true;
                        }
                    },
                );
                ui.separator();
                if ui.button("Clear Statistics").clicked() {
                    if let Some(statistics_data) = &mut self.statistics_data {
                        statistics_data.results.clear();
                    }
                    self.visualization_style.use_size_overwrite = false;
                    self.visualization_style.use_color_overwrite = false;
                    self.visible_nodes.update_node_shapes = true;
                    ui.close_menu();
                }
                consume_keys = true;
            });
            ui.menu_button("Help", |ui| {
                if ui.button("About RDF Glance").clicked() {
                    self.ui_state.about_window = true;
                    ui.close_menu();
                }
                ui.hyperlink_to(
                    "Manual/Documentation",
                    "https://github.com/xdobry/rdfglance/blob/main/documentation/manual.md",
                );
                ui.hyperlink_to("Report Issue / Feedback", "https://github.com/xdobry/rdfglance/issues");
                consume_keys = true;
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
                        ui.label(format!("{:.4}", self.ui_state.cpu_usage));
                    });
                }
            }
            // consume keys so they are not processed twice, I do not know why the d
            if consume_keys {
                ui.ctx().input_mut(|i| {
                    i.consume_key(Modifiers::NONE, Key::Enter);
                    i.consume_key(Modifiers::NONE, Key::ArrowDown);
                    i.consume_key(Modifiers::NONE, Key::ArrowLeft);
                    i.consume_key(Modifiers::NONE, Key::ArrowRight);
                    i.consume_key(Modifiers::NONE, Key::ArrowUp);
                });
            }
        });
        match menu_action {
            MenuAction::ImportRDF => self.import_file_dialog(ui),
            MenuAction::LoadProject => self.load_project_dialog(ui.visuals().dark_mode),
            MenuAction::SaveProject => self.save_project_dialog(),
            MenuAction::None => {}
        }
    }
    pub fn import_file_from_url_dialog(&mut self, _ui: &mut egui::Ui) {
        self.import_from_url = Some(ImportFromUrlData {
            url: String::new(),
            format: ImportFormat::Turtle,
            focus_requested: false,
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
                        let rdfttl =
                            crate::rdfwrap::RDFWrap::load_file_data(path, data, &mut rdf_data, &language_filter);
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
                    self.system_message = SystemMessage::Error(format!("Can not load file: {}", e));
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
                if !app_data.visualization_style.node_styles.is_empty() {
                    self.visualization_style = app_data.visualization_style;
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
