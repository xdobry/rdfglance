

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::{HashMap, HashSet}, path::Path};
use const_format::concatcp;

use eframe::{
    egui::{self, Pos2},
    Storage,
};
use egui::{Rangef, Rect};
use egui_extras::StripBuilder;
use graph_view::NeighbourPos;
use nobject::{IriIndex, LangIndex, NodeData};
use layout::SortedNodeLayout;
use prefix_manager::PrefixManager;
use serde::{Deserialize, Serialize};
use sparql_dialog::SparqlDialog;
use string_interner::Symbol;
use style::*;
use table_view::TypeInstanceIndex;

pub mod browse_view;
pub mod config;
pub mod distinct_colors;
pub mod drawing;
pub mod graph_view;
pub mod layout;
pub mod nobject;
pub mod play_ground;
pub mod prefix_manager;
pub mod rdfwrap;
pub mod sparql;
pub mod sparql_dialog;
pub mod table_view;
pub mod uitools;
pub mod style;
pub mod persistency;
pub mod menu_bar;
pub mod string_indexer;

#[derive(Debug, PartialEq)]
pub enum DisplayType {
    Browse,
    Graph,
    Table,
    PlayGround,
    Prefixes,
    Configuration,
}

// Define the application structure
pub struct RdfGlanceApp {
    object_iri: String,
    current_iri: Option<IriIndex>,
    rdfwrap: Box<dyn rdfwrap::RDFAdapter>,
    nav_pos: usize,
    nav_history: Vec<IriIndex>,
    display_type: DisplayType,
    show_properties: bool,
    show_labels: bool,
    short_iri: bool,
    pub node_data: NodeData,
    ui_state: UIState,
    visualisation_style: GVisualisationStyle,
    sparql_dialog: Option<SparqlDialog>,
    status_message: String,
    system_message: SystemMessage,
    persistent_data: AppPersistentData,
    type_index: TypeInstanceIndex,
    prefix_manager: PrefixManager,
    play: PlayData,
    help_open: bool,
}

enum SystemMessage {
    None,
    Info(String),
    Error(String),
}

impl SystemMessage {
    fn has_message(&self) -> bool {
        !matches!(self, SystemMessage::None)
    }
    fn get_message(&self) -> &str {
        match self {
            SystemMessage::None => "",
            SystemMessage::Info(msg) => msg,
            SystemMessage::Error(msg) => msg,
        }
    }
}

pub struct PlayData {
    row_count: usize,
    row_height: f32,
    position: f32,
    drag_pos: Option<f32>
}

pub struct TypeStyle {
    pub color: egui::Color32,
    pub priority: u32,
    pub label_index: IriIndex
}

pub struct ReferenceStyle {
    pub color: egui::Color32,
}

pub struct GVisualisationStyle {
    type_styles: HashMap<IriIndex, TypeStyle>,
    reference_styles: HashMap<IriIndex, ReferenceStyle>,
}

pub struct UIState {
    selected_node: Option<IriIndex>,
    context_menu_node: Option<IriIndex>,
    context_menu_pos: Pos2,
    node_to_drag: Option<IriIndex>,
    visible_nodes: SortedNodeLayout,
    hidden_predicates: SortedVec,
    compute_layout: bool,
    force_compute_layout: bool,
    display_language: LangIndex,
    language_sort: Vec<LangIndex>,
    scene_rect: Rect,
}

pub struct SortedVec {
    pub data: Vec<IriIndex>,
}


#[derive(Serialize, Deserialize)]
struct AppPersistentData {
    last_files: Vec<Box<str>>,
    last_endpoints: Vec<Box<str>>,
    #[serde(default = "default_last_projects")]
    last_projects: Vec<Box<str>>,
    #[serde(default = "default_config_data")]
    config_data: config::Config,
}

fn default_config_data() -> config::Config {
    config::Config::default()
}

fn default_last_projects() -> Vec<Box<str>> {
    Vec::new()
}

pub enum NodeAction {
    None,
    BrowseNode(IriIndex),
    ShowType(IriIndex),
    ShowVisual(IriIndex),
}

impl UIState {
    pub fn clean(&mut self) {
        self.selected_node = None;
        self.context_menu_node = None;
        self.node_to_drag = None;
        self.visible_nodes.nodes.clear();
        self.hidden_predicates.data.clear();
        self.compute_layout = true;
        self.force_compute_layout = false;
        self.scene_rect = Rect::ZERO;
    }
}

impl GVisualisationStyle {
    pub fn preset_styles(
        &mut self,
        cache_statistics: &TypeInstanceIndex,
    ) {
        for (type_index, _type_desc) in cache_statistics.types.iter() {
            let _color = self.get_predicate_color(*type_index);
        }
    }

    fn get_type_color(&mut self, types: &Vec<IriIndex>) -> egui::Color32 {
        if types.is_empty() {
            return egui::Color32::LIGHT_GRAY;
        }
        for type_iri in types {
            if let Some(type_style) = self.type_styles.get(type_iri) {
                return type_style.color;
            }
        }
        let first_type = types.first().unwrap();
        let len = self.type_styles.len();
        let new_color = distinct_colors::next_distinct_color(len, 0.8, 0.8);
        self.type_styles.insert(*first_type, TypeStyle { 
            color: new_color,
            priority: 0,
            label_index: 0,
        });
        new_color
    }

    fn get_predicate_color(&mut self, iri: IriIndex) -> egui::Color32 {
        let len = self.reference_styles.len();
        self.reference_styles
            .entry(iri)
            .or_insert_with(|| ReferenceStyle { 
                color: distinct_colors::next_distinct_color(len, 0.5, 0.3)}).color
    }

    fn update_label(&mut self, iri: IriIndex, label_index: IriIndex) {
        if let Some(type_style) = self.type_styles.get_mut(&iri) {
            type_style.label_index = label_index;
        }
    }

    pub fn clean(&mut self) {
        self.type_styles.clear();
        self.reference_styles.clear();
    }
}

impl SortedVec {
    fn new() -> Self {
        SortedVec { data: Vec::new() }
    }

    pub fn add(&mut self, value: IriIndex) {
        match self.data.binary_search(&value) {
            Ok(_) => (),                              // Value already exists, do nothing
            Err(pos) => self.data.insert(pos, value), // Insert at correct position
        }
    }

    pub fn contains(&self, value: IriIndex) -> bool {
        self.data.binary_search(&value).is_ok()
    }

    pub fn remove(&mut self, value: IriIndex) {
        if let Ok(pos) = self.data.binary_search(&value) {
            self.data.remove(pos);
        }
    }
}

// Implement default values for MyApp
impl RdfGlanceApp {
    pub fn new(storage: Option<&dyn Storage>) -> Self {
        let presistentdata: Option<AppPersistentData> = match storage {
            Some(storage) => {
                let persistent_data_string = storage.get_string("persistent_data");
                if let Some(persistent_data_string) = persistent_data_string {
                    let mut persistent_data: AppPersistentData =
                        serde_json::from_str(&persistent_data_string)
                            .expect("Failed to parse persistent data");
                    persistent_data
                        .last_endpoints
                        .retain(|endpoint| !endpoint.is_empty());
                    Some(persistent_data)
                } else {
                    None
                }
            }
            None => None,
        };
        Self {
            object_iri: String::new(),
            current_iri: None,
            rdfwrap: Box::new(rdfwrap::RDFWrap::empty()),
            nav_pos: 0,
            nav_history: vec![],
            display_type: DisplayType::Table,
            show_properties: true,
            show_labels: true,
            short_iri: true,
            sparql_dialog: None,
            status_message: String::new(),
            node_data: NodeData::new(),
            type_index: TypeInstanceIndex::new(),
            prefix_manager: PrefixManager::new(),
            system_message: SystemMessage::None,
            persistent_data: presistentdata.unwrap_or(AppPersistentData {
                last_files: vec![],
                last_endpoints: vec![],
                last_projects: vec![],
                config_data: config::Config::default(),
            }),
            visualisation_style: GVisualisationStyle {
                type_styles: HashMap::new(),
                reference_styles: HashMap::new(),
            },
            ui_state: UIState {
                selected_node: None,
                node_to_drag: None,
                context_menu_node: None,
                context_menu_pos: Pos2::new(0.0, 0.0),
                scene_rect: Rect::ZERO,
                compute_layout: true,
                force_compute_layout: false,
                visible_nodes: SortedNodeLayout::new(),
                hidden_predicates: SortedVec::new(),
                display_language: 0,
                language_sort: Vec::new(),
            },
            play: PlayData {
                row_count: 100,
                row_height: 20.0,
                position: 0.0,
                drag_pos: None,
            },
            help_open: false,
        }
    }
}

enum ExpandType {
    References,
    ReverseReferences,
    Both,
}

impl RdfGlanceApp {
    fn show_current(&mut self) -> bool {
        let cached_object_index = self.node_data.get_node_index(&self.object_iri);
        if let Some(cached_object_index) = cached_object_index {
            let cached_object = self.node_data.get_node_by_index(cached_object_index);
            if let Some((_,cached_object)) = cached_object {
                if !cached_object.has_subject {
                    let new_object = self
                        .rdfwrap
                        .load_object(&self.object_iri, &mut self.node_data);
                    if let Some(new_object) = new_object {
                        self.current_iri = Some(cached_object_index);
                        self.node_data.put_node_replace(&self.object_iri, new_object);
                    }
                } else {
                    self.current_iri = Some(cached_object_index);
                }
            }
        } else {
            let new_object = self
                .rdfwrap
                .load_object(&self.object_iri, &mut self.node_data);
            if let Some(new_object) = new_object {
                self.current_iri = Some(self.node_data.put_node(&self.object_iri,new_object));
            } else {
                self.system_message =
                    SystemMessage::Info(format!("Object not found: {}", self.object_iri));
                return false;
            }
        }
        true
    }
    fn show_object_by_index(&mut self, index: IriIndex, add_history: bool) {
        if let Some(current_iri) = self.current_iri {
            if current_iri == index {
                return;
            }
        }
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri,_node)) = node {
            self.current_iri = Some(index);
            self.object_iri = node_iri.to_string();
            if add_history {
                self.nav_history.truncate(self.nav_pos + 1);
                self.nav_history.push(self.current_iri.unwrap());
                self.nav_pos = self.nav_history.len() - 1;
            }
        }
    }

    fn load_object(&mut self, iri: &str) -> bool {
        let iri_index = self.node_data.get_node_index(iri);
        if let Some(iri_index) = iri_index {
            self.ui_state.visible_nodes.add_by_index(iri_index);
        } else {
            let new_object = self.rdfwrap.load_object(iri, &mut self.node_data);
            if let Some(new_object) = new_object {
                self.node_data.put_node(iri,new_object);
            } else {
                return false;
            }
        }
        true
    }
    fn load_object_by_index(&mut self, index: IriIndex) -> bool {
        self.ui_state.compute_layout = true;
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri,node)) = node {
            if node.has_subject {
                return self.ui_state.visible_nodes.add_by_index(index);
            } else {
                let node_iri = node_iri.clone();
                let new_object = self.rdfwrap.load_object(&node_iri, &mut self.node_data);
                if let Some(new_object) = new_object {
                    self.node_data.put_node_replace(&node_iri,new_object);
                }
            }
        }
        false
    }
    fn show_object(&mut self) {
        if self.show_current() {
            self.nav_history.truncate(self.nav_pos + 1);
            self.nav_history.push(self.current_iri.unwrap());
            self.nav_pos = self.nav_history.len() - 1;
        }
    }
    fn expand_node(&mut self, iri_index: IriIndex, expand_type: ExpandType) {
        let refs_to_expand = {
            let nnode = self.node_data.get_node_by_index(iri_index);
            if let Some((_,nnode)) = nnode {
                let mut refs_to_expand = vec![];
                match expand_type {
                    ExpandType::References | ExpandType::Both => {
                        for (_, ref_iri) in &nnode.references {
                            refs_to_expand.push(*ref_iri);
                        }
                    }
                    _ => {}
                }
                match expand_type {
                    ExpandType::ReverseReferences | ExpandType::Both => {
                        for (_, ref_iri) in &nnode.reverse_references {
                            refs_to_expand.push(*ref_iri);
                        }
                    }
                    _ => {}
                }
                refs_to_expand
            } else {
                vec![]
            }
        };
        let mut npos = NeighbourPos::new();
        for ref_index in refs_to_expand {
            if self.load_object_by_index(ref_index) {
                npos.insert(iri_index, ref_index);
            }
        }
        npos.position(&mut self.ui_state.visible_nodes);
    }
    fn expand_all(&mut self) {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref : Vec<(IriIndex,IriIndex)> = Vec::new();
        for visible_index in &self.ui_state.visible_nodes.nodes {
            if let Some((_,nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (_, ref_iri) in nnode.references.iter() {
                    if refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index,*ref_iri));
                    }
                }
                for (_, ref_iri) in nnode.reverse_references.iter() {
                    if refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index,*ref_iri));
                    }
                }
            }
        }
        let mut npos = NeighbourPos::new();
        for (parent_index,ref_index) in parent_ref {
            if !self.ui_state.visible_nodes.contains(ref_index) && self.load_object_by_index(ref_index) {
                npos.insert(parent_index, ref_index);
            }
        }
        npos.position(&mut self.ui_state.visible_nodes);
    }
    pub fn load_ttl(&mut self, file_name: &str) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = rdfwrap::RDFWrap::load_file(file_name, &mut self.node_data, &language_filter, &mut self.prefix_manager);
        match rdfttl {
            Err(err) => {
                self.system_message = SystemMessage::Error(format!("File not found: {}", err));
            }
            Ok(triples_count) => {
                let load_message = format!(
                    "Loaded: {} triples: {}",
                    file_name, triples_count
                );
                self.set_status_message(&load_message);
                if !self
                    .persistent_data
                    .last_files
                    .iter().any(|f | *f == file_name.into())
                {
                    self.persistent_data.last_files.push(file_name.into());
                }
                self.update_data_indexes();
            }
        }
    }
    fn load_ttl_dir(&mut self, dir_name: &str) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = rdfwrap::RDFWrap::load_from_dir(dir_name, &mut self.node_data, &language_filter, &mut self.prefix_manager);
        match rdfttl {
            Err(err) => {
                self.system_message = SystemMessage::Error(format!("Directory not found: {}", err));
            }
            Ok(triples_count) => {
                self.system_message = SystemMessage::Info(format!(
                    "Loaded: {} triples: {}",
                    dir_name, triples_count
                ));
                self.update_data_indexes();
                self.visualisation_style.preset_styles(&self.type_index);
            }
        }
    }

    fn set_status_message(&mut self, message: &str) {
        self.status_message.clear();
        self.status_message.push_str(message);
    }
    pub fn update_data_indexes(&mut self) {
        self.ui_state.language_sort.clear();
        for (index, _lang) in self.node_data.indexers.language_indexer.map.iter() {
            self.ui_state.language_sort.push(index.to_usize() as LangIndex);
        }
        self.ui_state.language_sort.sort_by(|a,b| {
            self.node_data.get_language(*a).cmp(&self.node_data.get_language(*b))
        });
        if self.persistent_data.config_data.resolve_rdf_lists {
            self.node_data.resolve_rdf_lists(&self.prefix_manager);
        }
        self.type_index.update(&self.node_data);

        self.visualisation_style.preset_styles(&self.type_index);
        self.node_data.indexers.predicate_indexer.map.shrink_to_fit();
        self.node_data.indexers.type_indexer.map.shrink_to_fit();
        self.node_data.indexers.language_indexer.map.shrink_to_fit();
        self.node_data.indexers.datatype_indexer.map.shrink_to_fit();
    }
    fn empty_data_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("No data loaded. Load RDF file first.");
        let button_text = egui::RichText::new(concatcp!(ICON_OPEN_FOLDER," Open RDF File")).size(16.0);
        let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
        let b_resp = ui.add(nav_but);
        if b_resp.clicked() {
            self.import_file_dialog(ui);
        }
        StripBuilder::new(ui)
            .size(egui_extras::Size::Relative { fraction: 0.5, range: Rangef::EVERYTHING })
            .size(egui_extras::Size::Relative { fraction: 0.5, range: Rangef::EVERYTHING })
            .horizontal(|mut strip| {
                strip.cell(|ui| {
                    if !self.persistent_data.last_files.is_empty() {
                        let mut last_file_clicked: Option<String> = None;
                        let mut last_file_forget: Option<String> = None;
                        ui.spacing();
                        ui.heading("Last imported files:");
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            egui::Grid::new("lfiles").striped(true).show(ui, |ui| {
                                for last_file in &self.persistent_data.last_files {
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
                            self.load_ttl(&last_file_clicked);
                        }
                        if let Some(last_file_forget) = last_file_forget {
                            self.persistent_data.last_files.retain(|file| *file != last_file_forget.as_str().into());
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
                            self.load_project(&last_project_path);
                        }
                        if let Some(last_file_forget) = last_file_forget {
                            self.persistent_data.last_projects.retain(|file| *file != last_file_forget.as_str().into());
                        }
                    }
                });
            });
    }
    fn is_empty(&self) -> bool {
        self.node_data.len() == 0
    }

    fn clean_data(&mut self) {
        self.node_data.clean();
        self.ui_state.clean();
        self.type_index.clean();
        self.visualisation_style.clean();
        self.display_type = DisplayType::Table; 
        self.nav_history.clear();
        self.nav_pos = 0;
        self.current_iri = None;
        self.object_iri.clear();
        self.prefix_manager.clean();
    }
}

impl eframe::App for RdfGlanceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        egui::CentralPanel::default().show(ctx, |ui| {
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
            self.menu_bar(ui);
            ui.separator();
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.display_type, DisplayType::Table, concatcp!(ICON_TABLE," Tables"));
                ui.add_enabled_ui(!self.is_empty(), |ui| {
                    ui.selectable_value(&mut self.display_type, DisplayType::Graph, concatcp!(ICON_GRAPH," Visual Graph"));
                    ui.selectable_value(&mut self.display_type, DisplayType::Browse, concatcp!(ICON_BROWSE," Browse"));
                });
                ui.selectable_value(&mut self.display_type, DisplayType::Prefixes, concatcp!(ICON_PREFIX," Prefixes"));
                ui.selectable_value(&mut self.display_type, DisplayType::Configuration, concatcp!(ICON_CONFIG," Settings"));
                /*
                ui.selectable_value(
                    &mut self.display_type,
                    DisplayType::PlayGround,
                    "Play Ground",
                );
                 */
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
                                } else {
                                    self.type_index.display(
                                        ctx,
                                        ui,
                                        &mut self.node_data,
                                        &mut self.ui_state,
                                        &self.prefix_manager,
                                        &self.visualisation_style,
                                        self.persistent_data.config_data.iri_display,
                                    )
                                }
                            },
                            DisplayType::PlayGround => self.show_play(ctx, ui),
                            DisplayType::Configuration => self.show_config(ctx, ui),
                            DisplayType::Prefixes => self.show_prefixes(ctx, ui)
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
                NodeAction::BrowseNode(node_index) => {
                    self.display_type = DisplayType::Browse;
                    self.show_object_by_index(node_index, true);
                }
                NodeAction::ShowVisual(node_index) => {
                    self.display_type = DisplayType::Graph;
                    self.ui_state.visible_nodes.add_by_index(node_index);
                    self.ui_state.selected_node = Some(node_index);
                }
                NodeAction::None => {}
            }
            if let Some(dialog) = &mut self.sparql_dialog {
                let (close_dialog, result) = dialog.show(ctx, &self.persistent_data.last_endpoints);
                if close_dialog {
                    if let Some(endpoint) = result {
                        self.rdfwrap = Box::new(sparql::SparqlAdapter::new(&endpoint));
                        if !endpoint.is_empty() && !self.persistent_data.last_endpoints.iter().any(|e| *e == endpoint.as_str().into())
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
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Ok(persistent_data_string) = serde_json::to_string(&self.persistent_data) {
            _storage.set_string("persistent_data", persistent_data_string);
            // println!("save called");
        }
    }
}


