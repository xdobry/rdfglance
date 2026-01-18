use const_format::concatcp;
#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::ui::style::*;
use anyhow::Error;
use eframe::Storage;
use egui::{Key, Rangef, Rect};
use egui_extras::StripBuilder;
use oxrdf::vocab::rdf;
use string_interner::Symbol;

#[cfg(target_arch = "wasm32")]
use crate::uistate::File;

#[cfg(target_arch = "wasm32")]
const SAMPLE_DATA: &[u8] = include_bytes!("../../sample-rdf-data/programming_languages.ttl");

#[cfg(not(target_arch = "wasm32"))]
use crate::ui::sparql_dialog::SparqlDialog;
use crate::{
    DisplayType, IriIndex, SystemMessage,
    domain::{
        LangIndex, NodeChangeContext, NodeData, RdfData,
        app_persistence::AppPersistentData,
        config::Config,
        graph_styles::{GVisualizationStyle, NodeStyle},
        prefix_manager::PrefixManager,
        statistics::StatisticsData,
    },
    integration::rdfwrap::{RDFAdapter, RDFWrap},
    support::uitools::primary_color,
    ui::{
        graph_view::{NeighborPos, update_layout_edges},
        style::{ICON_DELETE, ICON_OPEN_FOLDER},
        table_view::TypeInstanceIndex,
    },
    uistate::{
        DataLoading, GraphState, ImportFormat, ImportFromUrlData, LastVisitedSelection, LoadResult, UIState,
        actions::NodeAction, layout::SortedNodeLayout, ref_selection::RefSelection,
    },
};

pub struct RdfGlanceApp {
    pub object_iri: String,
    pub current_iri: Option<IriIndex>,
    pub ref_selection: RefSelection,
    pub rdfwrap: Box<dyn RDFAdapter>,
    pub nav_pos: usize,
    pub nav_history: Vec<IriIndex>,
    pub display_type: DisplayType,
    pub ui_state: UIState,
    pub visible_nodes: SortedNodeLayout,
    pub meta_nodes: SortedNodeLayout,
    pub graph_state: GraphState,
    pub meta_graph_state: GraphState,
    pub visualization_style: GVisualizationStyle,
    pub statistics_data: Option<StatisticsData>,
    #[cfg(not(target_arch = "wasm32"))]
    pub sparql_dialog: Option<SparqlDialog>,
    pub status_message: String,
    pub system_message: SystemMessage,
    pub rdf_data: Arc<RwLock<RdfData>>,
    pub type_index: TypeInstanceIndex,
    pub persistent_data: AppPersistentData,
    pub help_open: bool,
    pub load_handle: Option<JoinHandle<Option<Result<LoadResult, Error>>>>,
    #[cfg(target_arch = "wasm32")]
    pub file_upload: Option<poll_promise::Promise<Result<File, anyhow::Error>>>,
    pub data_loading: Option<Arc<DataLoading>>,
    pub import_from_url: Option<ImportFromUrlData>,
}

// Implement default values for MyApp
impl RdfGlanceApp {
    pub fn new(storage: Option<&dyn Storage>, args: Vec<String>) -> Self {
        let persistent_data: Option<AppPersistentData> = match storage {
            Some(storage) => {
                let persistent_data_string = storage.get_string("persistent_data");
                if let Some(persistent_data_string) = persistent_data_string {
                    let mut persistent_data: AppPersistentData =
                        serde_json::from_str(&persistent_data_string).expect("Failed to parse persistent data");
                    persistent_data.last_endpoints.retain(|endpoint| !endpoint.is_empty());
                    Some(persistent_data)
                } else {
                    None
                }
            }
            None => None,
        };
        let mut app = Self {
            object_iri: String::new(),
            current_iri: None,
            ref_selection: RefSelection::None,
            rdfwrap: Box::new(RDFWrap::empty()),
            nav_pos: 0,
            nav_history: vec![],
            display_type: DisplayType::Table,
            #[cfg(not(target_arch = "wasm32"))]
            sparql_dialog: None,
            status_message: String::new(),
            type_index: TypeInstanceIndex::new(),
            system_message: SystemMessage::None,
            visible_nodes: SortedNodeLayout::new(),
            meta_nodes: SortedNodeLayout::new(),
            persistent_data: persistent_data.unwrap_or(AppPersistentData {
                last_files: vec![],
                last_endpoints: vec![],
                last_projects: vec![],
                config_data: Config::default(),
            }),
            rdf_data: Arc::new(RwLock::new(RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
            })),
            visualization_style: GVisualizationStyle {
                node_styles: HashMap::new(),
                edge_styles: HashMap::new(),
                default_node_style: NodeStyle::default(),
                use_size_overwrite: false,
                use_color_overwrite: false,
                default_label_in_node: false,
                min_size: 5.0,
                max_size: 50.0,
            },
            graph_state: GraphState { scene_rect: Rect::ZERO },
            meta_graph_state: GraphState { scene_rect: Rect::ZERO },
            statistics_data: None,
            ui_state: UIState::default(),
            help_open: false,
            load_handle: None,
            data_loading: None,
            #[cfg(target_arch = "wasm32")]
            file_upload: None,
            import_from_url: None,
        };
        #[cfg(not(target_arch = "wasm32"))]
        if !args.is_empty() {
            let first_arg = args[0].as_str();
            // TODO does not know the dark mode yet.
            app.load_ttl(first_arg, false);
        }
        #[cfg(target_arch = "wasm32")]
        if args.len() > 0 {
            let first_arg = args[0].as_str();
            app.load_ttl_from_url(first_arg, ImportFormat::Turtle, true);
        }
        app
    }

    fn show_current(&mut self) -> bool {
        if let Ok(mut rdf_data) = self.rdf_data.write() {
            let cached_object_index = rdf_data.node_data.get_node_index(&self.object_iri);
            if let Some(cached_object_index) = cached_object_index {
                let cached_object = rdf_data.node_data.get_node_by_index(cached_object_index);
                if let Some((_, cached_object)) = cached_object {
                    if !cached_object.has_subject {
                        let new_object = self.rdfwrap.load_object(&self.object_iri, &mut rdf_data.node_data);
                        if let Some(new_object) = new_object {
                            self.current_iri = Some(cached_object_index);
                            rdf_data.node_data.put_node_replace(&self.object_iri, new_object);
                        }
                    } else {
                        self.current_iri = Some(cached_object_index);
                    }
                }
            } else {
                let new_object = self.rdfwrap.load_object(&self.object_iri, &mut rdf_data.node_data);
                if let Some(new_object) = new_object {
                    self.current_iri = Some(rdf_data.node_data.put_node(&self.object_iri, new_object));
                } else {
                    self.system_message = SystemMessage::Info(format!("Object not found: {}", self.object_iri));
                    return false;
                }
            }
        }
        true
    }

    pub fn show_object_by_index(&mut self, index: IriIndex, add_history: bool) {
        if let Some(current_iri) = self.current_iri {
            if current_iri == index {
                return;
            }
        }
        if let Ok(rdf_data) = self.rdf_data.read() {
            let node = rdf_data.node_data.get_node_by_index(index);
            if let Some((node_iri, current_node)) = node {
                self.current_iri = Some(index);
                self.object_iri = node_iri.to_string();
                if add_history {
                    self.nav_history.truncate(self.nav_pos + 1);
                    self.nav_history.push(self.current_iri.unwrap());
                    self.nav_pos = self.nav_history.len() - 1;
                }
                self.ref_selection.init_from_node(current_node);
            }
        }
    }

    fn load_object(&mut self, iri: &str) -> bool {
        if let Ok(mut rdf_data) = self.rdf_data.write() {
            let iri_index = rdf_data.node_data.get_node_index(iri);
            if let Some(iri_index) = iri_index {
                self.visible_nodes.add_by_index(iri_index);
            } else {
                let new_object = self.rdfwrap.load_object(iri, &mut rdf_data.node_data);
                if let Some(new_object) = new_object {
                    rdf_data.node_data.put_node(iri, new_object);
                } else {
                    return false;
                }
            }
        }
        true
    }

    pub fn show_object(&mut self) {
        if self.show_current() {
            self.nav_history.truncate(self.nav_pos + 1);
            self.nav_history.push(self.current_iri.unwrap());
            self.nav_pos = self.nav_history.len() - 1;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_ttl(&mut self, file_name: &str, is_dark_mode: bool) {
        use crate::integration::rdfwrap::RDFWrap;
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(RDFWrap::load_file(file_name, &mut rdf_data, &language_filter, None))
        } else {
            None
        };

        if let Some(rdfttl) = rdfttl {
            match rdfttl {
                Err(err) => {
                    self.system_message = SystemMessage::Error(format!("File not found: {}", err));
                }
                Ok(triples_count) => {
                    let load_message = format!("Loaded: {} triples: {}", file_name, triples_count);
                    self.set_status_message(&load_message);
                    if !self.persistent_data.last_files.iter().any(|f| *f == file_name.into()) {
                        self.persistent_data.last_files.push(file_name.into());
                    }
                    self.update_data_indexes(is_dark_mode);
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_ttl(&mut self, file_name: &str, _is_dark_mode: bool) {
        use std::{sync::atomic::AtomicUsize, thread};

        use crate::uistate::DataLoading;

        if self.load_handle.is_some() || self.data_loading.is_some() {
            self.system_message = SystemMessage::Info("Loading in progress".to_string());
            return;
        }
        let rdf_data_clone = Arc::clone(&self.rdf_data);
        let language_filter = self.persistent_data.config_data.language_filter();
        let file_name_cpy = file_name.to_string();
        let data_loading = Arc::new(DataLoading {
            stop_loading: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            total_triples: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            total_size: Arc::new(AtomicUsize::new(0)),
            finished: Arc::new(AtomicBool::new(false)),
        });
        let data_loading_clone = Arc::clone(&data_loading);
        self.data_loading = Some(data_loading);
        let handle = thread::spawn(move || {
            let my_data_loading = data_loading_clone.as_ref();
            let erg = if let Ok(mut rdf_data) = rdf_data_clone.write() {
                Some(
                    RDFWrap::load_file(
                        file_name_cpy.as_str(),
                        &mut rdf_data,
                        &language_filter,
                        Some(my_data_loading),
                    )
                    .map(|triples_count| LoadResult {
                        triples_count,
                        file_name: Some(file_name_cpy),
                    }),
                )
            } else {
                None
            };
            my_data_loading.finished.store(true, Ordering::Relaxed);
            erg
        });
        self.load_handle = Some(handle);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_ttl_from_url(&mut self, url: &str, format: ImportFormat, _is_dark_mode: bool) {
        use std::{sync::atomic::AtomicUsize, thread};

        use crate::uistate::DataLoading;

        if self.load_handle.is_some() {
            self.system_message = SystemMessage::Info("Loading in progress".to_string());
            return;
        }
        let rdf_data_clone = Arc::clone(&self.rdf_data);
        let language_filter = self.persistent_data.config_data.language_filter();
        let url_cpy = url.to_string();
        let data_loading = Arc::new(DataLoading {
            stop_loading: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            total_triples: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            total_size: Arc::new(AtomicUsize::new(0)),
            finished: Arc::new(AtomicBool::new(false)),
        });
        let data_loading_clone = Arc::clone(&data_loading);
        self.data_loading = Some(data_loading);
        let handle = thread::spawn(move || {
            let my_data_loading = data_loading_clone.as_ref();
            let erg = if let Ok(mut rdf_data) = rdf_data_clone.write() {
                Some(
                    RDFWrap::load_from_url(
                        url_cpy.as_ref(),
                        &mut rdf_data,
                        &language_filter,
                        format,
                        Some(my_data_loading),
                    )
                    .map(|triples_count| LoadResult {
                        triples_count,
                        file_name: None,
                    }),
                )
            } else {
                None
            };
            my_data_loading.finished.store(true, Ordering::Relaxed);
            erg
        });
        self.load_handle = Some(handle);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_ttl_from_url(&mut self, url: &str, _format: ImportFormat, _is_dark_mode: bool) {
        let url_cpy = url.to_string();
        use poll_promise::Promise;
        self.file_upload = Some(Promise::spawn_local(async move {
            let client = reqwest::Client::new();
            let request = client.get(url_cpy.as_str()).header("Accept", "text/turtle");
            match request.send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        return Ok(crate::uistate::File {
                            path: "url.ttl".to_string(),
                            data: bytes.to_vec(),
                        });
                    }
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Error downloading from URL {}", err));
                }
            }
            Err(anyhow::anyhow!("Upload: no file Selected"))
        }));
    }

    pub fn join_load(&mut self, is_dark_mode: bool) {
        if let Some(handle) = self.load_handle.take() {
            match handle.join() {
                Ok(Some(Ok(load_result))) => {
                    self.set_status_message(&format!("Loaded {} triples", load_result.triples_count));
                    self.update_data_indexes(is_dark_mode);
                    if let Some(file_name) = load_result.file_name {
                        let file_name_cpy = file_name.into_boxed_str();
                        if let Some(position) = self.persistent_data.last_files.iter().position(|f| *f == file_name_cpy)
                        {
                            self.persistent_data.last_files.remove(position);
                            self.persistent_data.last_files.insert(0, file_name_cpy);
                        } else {
                            self.persistent_data.last_files.insert(0, file_name_cpy);
                        }
                    }
                }
                Ok(Some(Err(err))) => {
                    self.system_message = SystemMessage::Error(format!("Error loading data: {}", err));
                }
                Ok(None) => {
                    self.system_message = SystemMessage::Error("Error loading data".to_string());
                }
                Err(_) => {
                    self.system_message = SystemMessage::Error("Thread panicked".to_string());
                }
            }
            self.data_loading = None;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_ttl_data(&mut self, file_name: &str, data: &Vec<u8>, is_dark_mode: bool) {
        use crate::integration::rdfwrap::RDFWrap;
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(RDFWrap::load_file_data(
                file_name,
                data,
                &mut rdf_data,
                &language_filter,
            ))
        } else {
            None
        };
        if let Some(rdfttl) = rdfttl {
            match rdfttl {
                Err(err) => {
                    self.system_message = SystemMessage::Error(format!("File not found: {}", err));
                }
                Ok(triples_count) => {
                    let load_message = format!("Loaded: {} triples: {}", file_name, triples_count);
                    self.set_status_message(&load_message);
                    self.update_data_indexes(is_dark_mode);
                }
            }
        }
    }

    pub fn load_ttl_dir(&mut self, dir_name: &str) {
        if self.load_handle.is_some() {
            self.system_message = SystemMessage::Info("Loading in progress".to_string());
            return;
        }
        let rdf_data_clone = Arc::clone(&self.rdf_data);
        let language_filter = self.persistent_data.config_data.language_filter();
        let dir_name_cpy = dir_name.to_string();
        let data_loading = Arc::new(DataLoading {
            stop_loading: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            total_triples: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            total_size: Arc::new(AtomicUsize::new(0)),
            finished: Arc::new(AtomicBool::new(false)),
        });
        let data_loading_clone = Arc::clone(&data_loading);
        self.data_loading = Some(data_loading);
        let handle = thread::spawn(move || {
            let my_data_loading = data_loading_clone.as_ref();
            let erg = if let Ok(mut rdf_data) = rdf_data_clone.write() {
                Some(
                    RDFWrap::load_from_dir(
                        dir_name_cpy.as_str(),
                        &mut rdf_data,
                        &language_filter,
                        Some(my_data_loading),
                    )
                    .map(|triples_count| LoadResult {
                        triples_count,
                        file_name: Some(dir_name_cpy),
                    }),
                )
            } else {
                None
            };
            my_data_loading.finished.store(true, Ordering::Relaxed);
            erg
        });
        self.load_handle = Some(handle);
    }

    pub fn set_status_message(&mut self, message: &str) {
        self.status_message.clear();
        self.status_message.push_str(message);
    }
    pub fn update_data_indexes(&mut self, is_dark_mode: bool) {
        if let Ok(mut rdf_data) = self.rdf_data.write() {
            self.ui_state.language_sort.clear();
            for (index, _lang) in rdf_data.node_data.indexers.language_indexer.map.iter() {
                self.ui_state.language_sort.push(index.to_usize() as LangIndex);
            }
            self.ui_state.language_sort.sort_by(|a, b| {
                rdf_data
                    .node_data
                    .get_language(*a)
                    .cmp(&rdf_data.node_data.get_language(*b))
            });
            if self.persistent_data.config_data.resolve_rdf_lists {
                rdf_data.resolve_rdf_lists();
            }
            for (_iri, node) in rdf_data.node_data.iter_mut() {
                node.references.sort_by(|a, b| a.0.cmp(&b.0));
                node.reverse_references.sort_by(|a, b| a.0.cmp(&b.0));
            }
            self.type_index.update(&rdf_data.node_data);

            self.visualization_style.preset_styles(
                &self.type_index,
                &rdf_data.node_data.indexers.predicate_indexer,
                is_dark_mode,
            );
            rdf_data.node_data.indexers.predicate_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.type_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.language_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.datatype_indexer.map.shrink_to_fit();
        }
    }
    pub fn empty_data_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("No data loaded. Load RDF file first.");
        let button_text = egui::RichText::new(concatcp!(ICON_OPEN_FOLDER, "Open RDF File (Ctrl-O)")).size(16.0);
        let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
        let b_resp = ui.add(nav_but);
        if b_resp.clicked() {
            self.import_file_dialog(ui);
        }
        #[cfg(target_arch = "wasm32")]
        {
            ui.add_space(20.0);
            ui.strong("0 React, 0 HTML, Full Power!");
            ui.strong("Try Desktop version for full functionality! Especially multithread more performant non-blocking processing.");
            let button_text = egui::RichText::new(concatcp!(ICON_OPEN_FOLDER, "Open Sample Data")).size(16.0);
            let nav_but = egui::Button::new(button_text).fill(primary_color(ui.visuals()));
            let b_resp = ui.add(nav_but);
            if b_resp.clicked() {
                self.load_ttl_data(
                    "programming_languages.ttl",
                    SAMPLE_DATA.to_vec().as_ref(),
                    ui.visuals().dark_mode,
                );
            }
        }
        let mut enter_pressed = false;
        let mut delete_pressed = false;
        if let LastVisitedSelection::File(i) = self.ui_state.last_visited_selection {
            if i >= self.persistent_data.last_files.len() {
                self.ui_state.last_visited_selection =
                    LastVisitedSelection::File(self.persistent_data.last_files.len() - 1);
            }
        }
        if matches!(self.ui_state.last_visited_selection, LastVisitedSelection::None)
            && self.persistent_data.last_files.len() > 0
        {
            self.ui_state.last_visited_selection = LastVisitedSelection::File(0);
        }

        ui.input(|i| {
            if i.key_pressed(egui::Key::Enter) {
                enter_pressed = true;
            } else if i.key_pressed(egui::Key::Delete) {
                delete_pressed = true;
            } else if i.key_pressed(egui::Key::ArrowUp) {
                if let LastVisitedSelection::File(i) = self.ui_state.last_visited_selection {
                    if i > 0 {
                        self.ui_state.last_visited_selection = LastVisitedSelection::File(i - 1);
                    }
                }
            } else if i.key_pressed(egui::Key::ArrowDown) {
                if let LastVisitedSelection::File(i) = self.ui_state.last_visited_selection {
                    if i + 1 < self.persistent_data.last_files.len() {
                        self.ui_state.last_visited_selection = LastVisitedSelection::File(i + 1);
                    }
                }
            }
        });
        StripBuilder::new(ui)
            .size(egui_extras::Size::Relative {
                fraction: 0.5,
                range: Rangef::EVERYTHING,
            })
            .size(egui_extras::Size::Relative {
                fraction: 0.5,
                range: Rangef::EVERYTHING,
            })
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    if !self.persistent_data.last_files.is_empty() {
                        let mut last_file_clicked: Option<String> = None;
                        let mut last_file_forget: Option<String> = None;
                        ui.spacing();
                        ui.heading("Last imported files:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("lfiles").striped(true).show(ui, |ui| {
                                for (index,last_file) in self.persistent_data.last_files.iter().enumerate() {
                                    if matches!(self.ui_state.last_visited_selection, LastVisitedSelection::File(i) if i == index) {
                                        let painter = ui.painter();
                                        painter.rect_filled(ui.available_rect_before_wrap(), 0.0, ui.visuals().selection.bg_fill);
                                        if enter_pressed {
                                            last_file_clicked = Some(last_file.to_string());
                                        } else if delete_pressed {
                                            last_file_forget = Some(last_file.to_string());
                                        }
                                    }
                                    if ui.button(last_file).clicked() {
                                        last_file_clicked = Some(last_file.to_string());
                                    }
                                    if ui.button(ICON_DELETE).clicked() {
                                        last_file_forget = Some(last_file.to_string());
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                        if let Some(last_file_clicked) = last_file_clicked {
                            let path = Path::new(last_file_clicked.as_str());
                            if path.exists() {
                                if path.is_dir() {
                                    self.load_ttl_dir(&last_file_clicked);
                                } else {
                                    self.load_ttl(&last_file_clicked, ui.visuals().dark_mode);
                                }
                            }
                        }
                        if let Some(last_file_forget) = last_file_forget {
                            self.persistent_data
                                .last_files
                                .retain(|file| *file != last_file_forget.as_str().into());
                        }
                    }
                });
                strip.cell(|ui| {
                    if !self.persistent_data.last_files.is_empty() {
                        let mut last_file_clicked: Option<String> = None;
                        let mut last_file_forget: Option<String> = None;
                        ui.spacing();
                        ui.heading("Last visited projects:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("lprojects").striped(true).show(ui, |ui| {
                                for last_file in &self.persistent_data.last_projects {
                                    if ui.button(last_file).clicked() {
                                        last_file_clicked = Some(last_file.to_string());
                                    }
                                    if ui.button(ICON_DELETE).clicked() {
                                        last_file_forget = Some(last_file.to_string());
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                        if let Some(last_file_clicked) = last_file_clicked {
                            let last_project_path = Path::new(&*last_file_clicked);
                            self.load_project(last_project_path, ui.visuals().dark_mode);
                        }
                        if let Some(last_file_forget) = last_file_forget {
                            self.persistent_data
                                .last_projects
                                .retain(|file| *file != last_file_forget.as_str().into());
                        }
                    }
                });
            });
    }
    pub fn is_empty(&self) -> bool {
        self.rdf_data.read().unwrap().node_data.len() == 0
    }

    pub fn clean_data(&mut self) {
        self.ui_state.clean();
        self.type_index.clean();
        self.visualization_style.clean();
        self.display_type = DisplayType::Table;
        self.nav_history.clear();
        self.nav_pos = 0;
        self.current_iri = None;
        self.object_iri.clear();
        self.mut_rdf_data(|rdf_data| {
            rdf_data.node_data.clean();
            rdf_data.prefix_manager.clean();
        });
        self.visible_nodes.clear();
        self.meta_nodes.clear();
    }

    pub fn mut_rdf_data<R>(&mut self, mut mutator: impl FnMut(&mut RdfData) -> R) -> Option<R> {
        if let Ok(mut rdf_data) = self.rdf_data.write() {
            return Some(mutator(&mut rdf_data));
        }
        None
    }

    pub fn read_rdf_data<R>(&mut self, mut mutator: impl FnMut(&RdfData) -> R) -> Option<R> {
        if let Ok(rdf_data) = self.rdf_data.read() {
            return Some(mutator(&rdf_data));
        }
        None
    }

    pub fn node_change_context(&mut self) -> NodeChangeContext<'_> {
        NodeChangeContext {
            rdfwrap: &mut self.rdfwrap,
            visible_nodes: &mut self.visible_nodes,
            config: &self.persistent_data.config_data,
        }
    }

    pub fn export_svg_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = FileDialog::new()
            .add_filter("SVG", &["svg"])
            .set_file_name("graph.svg")
            .save_file()
        {
            if let Ok(rdf_data) = self.rdf_data.read() {
                use crate::domain::LabelContext;
                use std::fs::File;
                let label_context = LabelContext::new(
                    self.ui_state.display_language,
                    self.persistent_data.config_data.iri_display,
                    &rdf_data.prefix_manager,
                );
                let file = File::create(path);
                if let Ok(mut file) = file {
                    let store_res = self.export_svg(&mut file, &rdf_data.node_data, &label_context);
                    match store_res {
                        Err(e) => {
                            self.system_message = SystemMessage::Error(format!("Can not export svg: {}", e));
                        }
                        Ok(_) => {}
                    }
                } else {
                    self.system_message = SystemMessage::Error("Can not save svg".to_string());
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        if let Ok(rdf_data) = self.rdf_data.read() {
            use crate::domain::graph_model::LabelContext;
            let label_context = LabelContext::new(
                self.ui_state.display_language,
                self.persistent_data.config_data.iri_display,
                &rdf_data.prefix_manager,
            );
            let mut buf = Vec::new();
            let store_res = self.export_svg(&mut buf, &rdf_data.node_data, &label_context);
            match store_res {
                Err(e) => {
                    self.system_message = SystemMessage::Error(format!("Can not export svg: {}", e));
                }
                Ok(_) => {
                    use crate::support::uitools::web_download;
                    let _ = web_download("graph.svg", &buf);
                }
            }
        }
    }
}

impl eframe::App for RdfGlanceApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(cpu_usage) = frame.info().cpu_usage {
            self.ui_state.cpu_usage = self.ui_state.cpu_usage * 0.95 + cpu_usage * 0.05;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(data_loading) = &self.data_loading {
                if !data_loading.finished.load(Ordering::Relaxed) {
                    ui.label("RDF data is currently being loaded. Please wait...");
                    let total_size = data_loading.total_size.load(Ordering::Relaxed);
                    if total_size > 0 {
                        let progress = data_loading.read_pos.load(Ordering::Relaxed) as f32 / total_size as f32;
                        let progress_bar = egui::ProgressBar::new(progress).desired_width(300.0).show_percentage();
                        ui.add(progress_bar);
                    }
                    ui.label(format!(
                        "Read triples: {}",
                        data_loading.total_triples.load(Ordering::Relaxed)
                    ));
                    if !data_loading.stop_loading.load(Ordering::Relaxed) && ui.button("Stop Loading").clicked() {
                        data_loading.stop_loading.store(true, Ordering::Relaxed);
                    }
                    ctx.request_repaint_after(Duration::from_millis(100));
                    return;
                } else {
                    self.join_load(ui.visuals().dark_mode);
                }
            }
            #[cfg(target_arch = "wasm32")]
            if self.file_upload.is_some() {
                ui.label("RDF data is currently being loaded. Please wait...");
                ctx.request_repaint_after(Duration::from_millis(100));
                self.handle_files(ctx, ui.visuals());
                return;
            }

            if self.system_message.has_message() {
                egui::Window::new("System Message")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]) // Center the modal
                    .show(ctx, |ui| {
                        ui.label(self.system_message.get_message());
                        if ui.button("OK").clicked() {
                            self.system_message = SystemMessage::None;
                        }
                    });
                ui.disable();
            }
            let mut cancel_clicked = false;
            let mut ok_clicked = false;
            if let Some(import_from_url_data) = &mut self.import_from_url {
                egui::Window::new("Import from URL")
                    .collapsible(false)
                    .resizable(true)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]) // Center the modal
                    .show(ctx, |ui| {
                        ui.label("Import RDF data from URL:");
                        ui.horizontal(|ui| {
                            ui.label("URL:");
                            let import_url = ui.text_edit_singleline(&mut import_from_url_data.url);
                            if !import_from_url_data.focus_requested {
                                import_url.request_focus();
                                import_from_url_data.focus_requested = true;
                            }
                            if import_url.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                ok_clicked = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            let import_but =
                                ui.add_enabled(!import_from_url_data.url.is_empty(), egui::Button::new("Import"));
                            if import_but.clicked() {
                                ok_clicked = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel_clicked = true;
                            }
                        });
                    });
                ui.disable();
            }
            if cancel_clicked {
                self.import_from_url = None;
            }
            if ok_clicked {
                if let Some(import_from_url_data) = &self.import_from_url {
                    if import_from_url_data.url.is_empty() {
                        self.system_message = SystemMessage::Error("URL cannot be empty".to_string());
                    } else {
                        let url = import_from_url_data.url.clone();
                        self.load_ttl_from_url(&url, import_from_url_data.format, ui.visuals().dark_mode);
                    }
                }
                self.import_from_url = None;
            }
            self.show_about(ctx, ui);

            self.menu_bar(ui);
            // The menu bar action could start loading data, so we check if data is being loaded
            // if loading data the display coud block on waiting access to rdf_data
            if self.data_loading.is_some() {
                return;
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.display_type,
                    DisplayType::Table,
                    concatcp!(ICON_TABLE, " Tables"),
                );
                ui.add_enabled_ui(!self.is_empty(), |ui| {
                    ui.selectable_value(
                        &mut self.display_type,
                        DisplayType::Graph,
                        concatcp!(ICON_GRAPH, " Visual Graph"),
                    );
                    ui.selectable_value(
                        &mut self.display_type,
                        DisplayType::Browse,
                        concatcp!(ICON_BROWSE, " Browse"),
                    );
                    ui.selectable_value(
                        &mut self.display_type,
                        DisplayType::MetaGraph,
                        concatcp!(ICON_METADATA, " Meta Graph"),
                    )
                    .clicked();
                    ui.selectable_value(
                        &mut self.display_type,
                        DisplayType::Statistics,
                        concatcp!(ICON_STATISTICS, " Statistics"),
                    );
                });
                ui.selectable_value(
                    &mut self.display_type,
                    DisplayType::Prefixes,
                    concatcp!(ICON_PREFIX, " Prefixes"),
                );
                ui.selectable_value(
                    &mut self.display_type,
                    DisplayType::Configuration,
                    concatcp!(ICON_CONFIG, " Settings"),
                );
                #[cfg(target_arch = "wasm32")]
                ui.small("Num+Alt to Switch");
                #[cfg(not(target_arch = "wasm32"))]
                ui.small("Num+Ctrl to Switch");
                ui.input(|i| {
                    #[cfg(target_arch = "wasm32")]
                    let is_mod = i.modifiers.alt;
                    #[cfg(not(target_arch = "wasm32"))]
                    let is_mod = i.modifiers.ctrl;
                    if is_mod && i.key_pressed(Key::Num1) {
                        self.display_type = DisplayType::Table;
                    } else if is_mod && i.key_pressed(Key::Num7) {
                        self.display_type = DisplayType::Configuration;
                    } else if is_mod && i.key_pressed(Key::Num6) {
                        self.display_type = DisplayType::Prefixes;
                    } else if !self.is_empty() {
                        if is_mod && i.key_pressed(Key::Num2) {
                            self.ui_state.selection_start_rect = None;
                            self.display_type = DisplayType::Graph;
                        } else if is_mod && i.key_pressed(Key::Num3) {
                            self.display_type = DisplayType::Browse;
                        } else if is_mod && i.key_pressed(Key::Num4) {
                            self.display_type = DisplayType::MetaGraph;
                        } else if is_mod && i.key_pressed(Key::Num5) {
                            self.display_type = DisplayType::Statistics;
                        }
                    }
                })
            });
            ui.separator();
            let mut node_action = NodeAction::None;
            StripBuilder::new(ui)
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(16.0)) // Two resizable panels with equal initial width
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        node_action = match self.display_type {
                            DisplayType::Browse => self.show_table(ui),
                            DisplayType::Graph => self.show_graph(ctx, ui),
                            DisplayType::Table => {
                                if self.is_empty() {
                                    self.empty_data_ui(ui);
                                    NodeAction::None
                                } else if let Ok(mut rdf_data) = self.rdf_data.write() {
                                    self.type_index.display(
                                        ctx,
                                        ui,
                                        &mut rdf_data,
                                        &mut self.ui_state,
                                        &self.visualization_style,
                                        self.persistent_data.config_data.iri_display,
                                    )
                                } else {
                                    NodeAction::None
                                }
                            }
                            DisplayType::Configuration => self.show_config(ctx, ui),
                            DisplayType::Prefixes => self.show_prefixes(ctx, ui),
                            DisplayType::MetaGraph => {
                                let is_empty = self.meta_nodes.nodes.read().unwrap().is_empty();
                                if is_empty {
                                    self.build_meta_graph();
                                }
                                self.show_meta_graph(ctx, ui)
                            }
                            DisplayType::Statistics => self.show_statistics(ctx, ui),
                        };
                    });
                    strip.cell(|ui| {
                        ui.label(&self.status_message);
                    });
                });

            match node_action {
                NodeAction::ShowType(type_index) => {
                    self.display_type = DisplayType::Table;
                    self.type_index.selected_type = Some(type_index);
                }
                NodeAction::ShowTypeInstances(type_index, instances) => {
                    self.display_type = DisplayType::Table;
                    self.type_index.selected_type = Some(type_index);
                    if let Some(type_desc) = self.type_index.types.get_mut(&type_index) {
                        type_desc.filtered_instances = instances;
                        type_desc.instance_view.pos = 0.0;
                        if type_desc.filtered_instances.len() > 0 {
                            type_desc.instance_view.selected_idx = Some((type_desc.filtered_instances[0], 0))
                        } else {
                            type_desc.instance_view.selected_idx = None;
                        }
                    }
                }
                NodeAction::BrowseNode(node_index) => {
                    self.display_type = DisplayType::Browse;
                    self.show_object_by_index(node_index, true);
                }
                NodeAction::ShowVisual(node_index) => {
                    self.display_type = DisplayType::Graph;
                    self.visible_nodes.add_by_index(node_index);
                    if let Ok(rdf_data) = self.rdf_data.read() {
                        let npos = NeighborPos::one(node_index);
                        update_layout_edges(
                            &npos,
                            &mut self.visible_nodes,
                            &rdf_data.node_data,
                            &self.ui_state.hidden_predicates,
                        );
                    }
                    self.visible_nodes.update_node_shapes = true;
                    self.ui_state.selected_node = Some(node_index);
                    self.ui_state.selected_nodes.insert(node_index);
                    self.ui_state.selection_start_rect = None;
                }
                NodeAction::AddVisual(node_index) => {
                    self.visible_nodes.add_by_index(node_index);
                    if let Ok(rdf_data) = self.rdf_data.read() {
                        let npos = NeighborPos::one(node_index);
                        update_layout_edges(
                            &npos,
                            &mut self.visible_nodes,
                            &rdf_data.node_data,
                            &self.ui_state.hidden_predicates,
                        );
                    }
                    self.visible_nodes.update_node_shapes = true;
                    self.ui_state.selected_node = Some(node_index);
                    self.ui_state.selected_nodes.insert(node_index);
                }
                NodeAction::None => {}
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(dialog) = &mut self.sparql_dialog {
                let (close_dialog, result) = dialog.show(ctx, &self.persistent_data.last_endpoints);
                if close_dialog {
                    if let Some(endpoint) = result {
                        use crate::integration::sparql::SparqlAdapter;

                        self.rdfwrap = Box::new(SparqlAdapter::new(&endpoint));
                        if !endpoint.is_empty()
                            && !self
                                .persistent_data
                                .last_endpoints
                                .iter()
                                .any(|e| *e == endpoint.as_str().into())
                        {
                            self.persistent_data.last_endpoints.push(endpoint.into());
                        }
                    }
                    self.sparql_dialog = None;
                }
            }
            /*
            if !self.status_message.is_empty() {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::RED, &self.status_message);
                });
            }
             */
        });

        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(file_path) = &file.path {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let path = Path::new(file_path);
                        if path.exists() {
                            if path.is_dir() {
                                self.load_ttl_dir(file_path.to_str().unwrap());
                            } else {
                                let file_path = path.to_str();
                                if let Some(file_path) = file_path {
                                    self.load_ttl(file_path, false);
                                } else {
                                    println!("File dropped path is not valid UTF-8: {:?}", path);
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Ok(persistent_data_string) = serde_json::to_string(&self.persistent_data) {
            _storage.set_string("persistent_data", persistent_data_string);
            // println!("save called");
        }
    }
}
