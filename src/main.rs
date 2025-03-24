use std::collections::{HashMap, HashSet};

use eframe::{
    egui::{self, Pos2},
    Storage,
};
use egui::{Align, Layout, Vec2};
use egui_extras::StripBuilder;
use nobject::{IriIndex, LangIndex, NodeData};
use layout::{NodeLayout, SortedNodeLayout};
use oxrdf::vocab::rdfs;
use prefix_manager::PrefixManager;
use rfd::FileDialog;
use serde::{Deserialize, Serialize};
use serde_json;
use sparql_dialog::SparqlDialog;
use table_view::CacheStatistics;

mod browse_view;
mod config;
mod distinct_colors;
mod drawing;
mod graph_view;
mod layout;
mod nobject;
mod play_ground;
mod prefix_manager;
mod rdfwrap;
mod sparql;
mod sparql_dialog;
mod table_view;
mod uitools;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "rdf-glance",
        options,
        Box::new(|cc| Ok(Box::new(VisualRdfApp::new(cc.storage)))),
    )
}

#[derive(Debug, PartialEq)]
enum DisplayType {
    Browse,
    Graph,
    Table,
    PlayGround,
    Prefixes,
    Configuration,
}

// Define the application structure
struct VisualRdfApp {
    object_iri: String,
    current_iri: Option<IriIndex>,
    rdfwrap: Box<dyn rdfwrap::RDFAdapter>,
    nav_pos: usize,
    nav_history: Vec<IriIndex>,
    display_type: DisplayType,
    show_properties: bool,
    show_labels: bool,
    short_iri: bool,
    node_data: NodeData,
    layout_data: LayoutData,
    color_cache: ColorCache,
    sparql_dialog: Option<SparqlDialog>,
    status_message: String,
    system_message: SystemMessage,
    persistent_data: PersistentData,
    cache_statistics: CacheStatistics,
    prefix_manager: PrefixManager,
    play: PlayData,
}

enum SystemMessage {
    None,
    Info(String),
    Warning(String),
    Error(String),
}

impl SystemMessage {
    fn has_message(&self) -> bool {
        match self {
            SystemMessage::None => false,
            _ => true,
        }
    }
    fn get_message(&self) -> &str {
        match self {
            SystemMessage::None => "",
            SystemMessage::Info(msg) => msg,
            SystemMessage::Warning(msg) => msg,
            SystemMessage::Error(msg) => msg,
        }
    }
}

struct PlayData {
    row_count: usize,
    row_height: f32,
    position: f32,
    drag_pos: Option<f32>,
    canvas_size: Vec2, // The size of the virtual canvas
    points: Vec<(f32, f32)>,
}

struct ColorCache {
    type_colors: HashMap<Vec<IriIndex>, egui::Color32>,
    predicate_colors: HashMap<IriIndex, egui::Color32>,
    label_predicate: HashMap<IriIndex, IriIndex>,
}

struct LayoutData {
    selected_node: Option<IriIndex>,
    offset_drag_start: Option<Pos2>,
    context_menu_node: Option<IriIndex>,
    context_menu_pos: Pos2,
    node_to_drag: Option<IriIndex>,
    visible_nodes: SortedNodeLayout,
    hidden_predicates: SortedVec,
    compute_layout: bool,
    force_compute_layout: bool,
    display_language: LangIndex,
    language_sort: Vec<LangIndex>,
    zoom: f32,
    offset: Pos2,
}

struct SortedVec {
    pub data: Vec<IriIndex>,
}


#[derive(Serialize, Deserialize)]
struct PersistentData {
    last_files: Vec<String>,
    last_endpoints: Vec<String>,
    #[serde(default = "default_config_data")]
    config_data: config::Config,
}

fn default_config_data() -> config::Config {
    config::Config::default()
}

pub enum NodeAction {
    None,
    BrowseNode(IriIndex),
    ShowType(IriIndex),
    ShowVisual(IriIndex),
}

impl ColorCache {
    fn get_type_color(&mut self, types: &Vec<IriIndex>) -> egui::Color32 {
        let len = self.type_colors.len();
        let color = self.type_colors.get(types);
        if let Some(color) = color {
            return color.clone();
        } else {
            let new_color = distinct_colors::next_distinct_color(len, 0.8, 0.8);
            self.type_colors.insert(types.clone(), new_color);
            return new_color;
        }
    }

    fn get_predicate_color(&mut self, iri: IriIndex) -> egui::Color32 {
        let len = self.predicate_colors.len();
        self.predicate_colors
            .entry(iri)
            .or_insert_with(|| distinct_colors::next_distinct_color(len, 0.5, 0.3))
            .clone()
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

impl ColorCache {
    pub fn preset_label_predicates(
        &mut self,
        cache_statistics: &CacheStatistics,
        label_predicate: IriIndex,
    ) {
        for (type_index, type_desc) in cache_statistics.types.iter() {
            if !self.label_predicate.contains_key(type_index) {
                if type_desc.properties.contains_key(&label_predicate) {
                    self.label_predicate.insert(*type_index, label_predicate);
                }
            }
        }
    }
}

// Implement default values for MyApp
impl VisualRdfApp {
    fn new(storage: Option<&dyn Storage>) -> Self {
        let presistentdata: Option<PersistentData> = match storage {
            Some(storage) => {
                let persistent_data_string = storage.get_string("persistent_data");
                if let Some(persistent_data_string) = persistent_data_string {
                    let mut persistent_data: PersistentData =
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
            cache_statistics: CacheStatistics::new(),
            prefix_manager: PrefixManager::new(),
            system_message: SystemMessage::None,
            persistent_data: presistentdata.unwrap_or(PersistentData {
                last_files: vec![],
                last_endpoints: vec![],
                config_data: config::Config::default(),
            }),
            color_cache: ColorCache {
                type_colors: HashMap::new(),
                predicate_colors: HashMap::new(),
                label_predicate: HashMap::new(),
            },
            layout_data: LayoutData {
                selected_node: None,
                node_to_drag: None,
                context_menu_node: None,
                context_menu_pos: Pos2::new(0.0, 0.0),
                zoom: 1.0,
                compute_layout: true,
                force_compute_layout: false,
                offset: Pos2::new(0.0, 0.0),
                visible_nodes: SortedNodeLayout::new(),
                hidden_predicates: SortedVec::new(),
                offset_drag_start: None,
                display_language: 0,
                language_sort: Vec::new(),
            },
            play: PlayData {
                row_count: 100,
                row_height: 20.0,
                position: 0.0,
                drag_pos: None,
                canvas_size: Vec2::new(2000.0, 2000.0), // Large virtual drawing area
                points: (0..40)
                    .map(|_| {
                        (
                            rand::random::<f32>() * 1000.0,
                            rand::random::<f32>() * 1000.0,
                        )
                    })
                    .collect(),
            },
        }
    }
}

enum ExpandType {
    References,
    ReverseReferences,
    Both,
}

impl VisualRdfApp {
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
        return true;
    }
    fn show_object_by_index(&mut self, index: IriIndex, add_history: bool) {
        if let Some(current_iri) = self.current_iri {
            if current_iri == index {
                return;
            }
        }
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri,node)) = node {
            self.current_iri = Some(index);
            self.object_iri = node_iri.clone();
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
            self.layout_data.visible_nodes.add_by_index(iri_index);
        } else {
            let new_object = self.rdfwrap.load_object(iri, &mut self.node_data);
            if let Some(new_object) = new_object {
                self.node_data.put_node(iri,new_object);
            } else {
                return false;
            }
        }
        return true;
    }
    fn load_object_by_index(&mut self, index: IriIndex) -> bool {
        self.layout_data.compute_layout = true;
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri,node)) = node {
            if node.has_subject {
                self.layout_data.visible_nodes.add_by_index(index);
            } else {
                let node_iri = node_iri.clone();
                let new_object = self.rdfwrap.load_object(&node_iri, &mut self.node_data);
                if let Some(new_object) = new_object {
                    self.node_data.put_node_replace(&node_iri,new_object);
                }
            }
            return true;
        }
        return false;
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
                            refs_to_expand.push(ref_iri.clone());
                        }
                    }
                    _ => {}
                }
                match expand_type {
                    ExpandType::ReverseReferences | ExpandType::Both => {
                        for (_, ref_iri) in &nnode.reverse_references {
                            refs_to_expand.push(ref_iri.clone());
                        }
                    }
                    _ => {}
                }
                refs_to_expand
            } else {
                vec![]
            }
        };
        for ref_index in refs_to_expand {
            self.load_object_by_index(ref_index);
        }
    }
    fn expand_all(&mut self) {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        for visible_index in &self.layout_data.visible_nodes.data {
            if let Some((_,nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (_, ref_iri) in nnode.references.iter() {
                    refs_to_expand.insert(*ref_iri);
                }
                for (_, ref_iri) in nnode.reverse_references.iter() {
                    refs_to_expand.insert(*ref_iri);
                }
            }
        }
        for ref_index in refs_to_expand {
            if !self.layout_data.visible_nodes.contains(ref_index) {
                self.load_object_by_index(ref_index);
            }
        }
    }
    fn show_all(&mut self) {
        for iri_index in 0..self.node_data.len() {
            self.node_data.get_node_by_index(iri_index).map(|(_,node)| {
                if node.has_subject {
                    self.layout_data.visible_nodes.add_by_index(iri_index);
                }
            });
        }
    }
    fn load_ttl(&mut self, file_name: &str) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = rdfwrap::RDFWrap::load_file(file_name, &mut self.node_data, &language_filter, &mut self.prefix_manager);
        match rdfttl {
            Err(err) => {
                self.system_message = SystemMessage::Error(format!("File not found: {}", err));
            }
            Ok(triples_count) => {
                self.system_message = SystemMessage::Info(format!(
                    "Loaded: {} triples: {}",
                    file_name, triples_count
                ));
                if !self
                    .persistent_data
                    .last_files
                    .contains(&file_name.to_string())
                {
                    self.persistent_data.last_files.push(file_name.to_string());
                }
                self.update_data_indexes();
                let prefixed_iri = self.prefix_manager.get_prefixed(rdfs::LABEL.as_str());
                let rdfs_label_index = self.node_data.get_predicate_index(prefixed_iri.as_str());
                self.color_cache
                    .preset_label_predicates(&self.cache_statistics, rdfs_label_index);
            }
        }
    }
    fn set_status_message(&mut self, message: &str) {
        self.status_message.clear();
        self.status_message.push_str(message);
    }
    fn update_data_indexes(&mut self) {
        self.layout_data.language_sort.clear();
        for (_lang,index) in self.node_data.indexers.language_indexer.iter() {
            self.layout_data.language_sort.push(*index as LangIndex);
        }
        self.layout_data.language_sort.sort_by(|a,b| {
            self.node_data.get_language(*a).cmp(&self.node_data.get_language(*b))
        });
        self.cache_statistics.update(&self.node_data);
    }
}

impl eframe::App for VisualRdfApp {
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
            egui::menu::bar(ui, |ui| {
                ui.menu_button("RDF Glance", |ui| {
                    if ui.button("Load RDF File").clicked() {
                        if let Some(path) = FileDialog::new().pick_file() {
                            let selected_file = Some(path.display().to_string());
                            if let Some(selected_file) = &selected_file {
                                self.load_ttl(&selected_file);
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
                    if ui.button("Clean Data").clicked() {
                        self.node_data.clean();
                        self.layout_data.visible_nodes.data.clear();
                        ui.close_menu();
                    }
                    if self.persistent_data.last_files.len() > 0 {
                        ui.separator();
                        let mut last_file_clicked: Option<String> = None;
                        ui.menu_button("Last Files:", |ui| {
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
                });
                let selected_language = self.node_data.get_language(self.layout_data.display_language);
                if let Some(selected_language) = selected_language {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_label("Data language")
                        .selected_text(selected_language)
                        .show_ui(ui, |ui| {
                            for language_index in self.layout_data.language_sort.iter() {
                                let language_str = self.node_data.get_language(*language_index);
                                if let Some(language_str) = language_str {
                                    if ui.selectable_label(self.layout_data.display_language == *language_index, language_str).clicked() {
                                        self.layout_data.display_language = *language_index;
                                    }
                                }
                            }
                        });
                  });
                }
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.display_type, DisplayType::Table, "Table");
                ui.selectable_value(&mut self.display_type, DisplayType::Graph, "Visual Graph");
                ui.selectable_value(&mut self.display_type, DisplayType::Browse, "Browse");
                ui.selectable_value(&mut self.display_type, DisplayType::Prefixes, "Prefixes");
                ui.selectable_value(&mut self.display_type, DisplayType::Configuration, "Settings");
                /*
                ui.selectable_value(
                    &mut self.display_type,
                    DisplayType::PlayGround,
                    "Play Ground",
                );
                 */
            });
            let mut node_action = NodeAction::None;
            StripBuilder::new(ui)
                .size(egui_extras::Size::remainder())
                .size(egui_extras::Size::exact(16.0)) // Two resizable panels with equal initial width
                .vertical(|mut strip| {
                    strip.cell(|ui| {
                        node_action = match self.display_type {
                            DisplayType::Browse => self.show_table(ui),
                            DisplayType::Graph => self.show_graph(&ctx, ui),
                            DisplayType::Table => self.cache_statistics.display(
                                &ctx,
                                ui,
                                &mut self.node_data,
                                &mut self.layout_data,
                                &self.prefix_manager,
                                &self.color_cache,
                                &mut *self.rdfwrap,
                                self.persistent_data.config_data.iri_display,
                            ),
                            DisplayType::PlayGround => self.show_play(&ctx, ui),
                            DisplayType::Configuration => self.show_config(&ctx, ui),
                            DisplayType::Prefixes => self.show_prefixes(&ctx, ui)
                        };
                    });
                    strip.cell(|ui| {
                        ui.label(&self.status_message);
                    });
                });

            match node_action {
                NodeAction::ShowType(type_index) => {
                    self.display_type = DisplayType::Table;
                    self.cache_statistics.selected_type = Some(type_index);
                }
                NodeAction::BrowseNode(node_index) => {
                    self.display_type = DisplayType::Browse;
                    self.show_object_by_index(node_index, true);
                }
                NodeAction::ShowVisual(node_index) => {
                    self.display_type = DisplayType::Graph;
                    self.layout_data.visible_nodes.add_by_index(node_index);
                    self.layout_data.selected_node = Some(node_index);
                }
                NodeAction::None => {}
            }
            if let Some(dialog) = &mut self.sparql_dialog {
                let (close_dialog, result) = dialog.show(ctx, &self.persistent_data.last_endpoints);
                if close_dialog {
                    match result {
                        Some(endpoint) => {
                            self.rdfwrap = Box::new(sparql::SparqlAdapter::new(&endpoint));
                            if !self.persistent_data.last_endpoints.contains(&endpoint)
                                && endpoint.len() > 0
                            {
                                self.persistent_data.last_endpoints.push(endpoint);
                            }
                        }
                        None => {}
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
