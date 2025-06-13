#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Error;
use const_format::concatcp;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, RwLock}, thread::{self, JoinHandle}, time::Duration,
};

use eframe::{
    Storage,
    egui::{self, Pos2},
};
use egui::{Rangef, Rect};
use egui_extras::StripBuilder;
use graph_styles::{EdgeStyle, NodeStyle};
use graph_view::{NeighbourPos, update_layout_edges};
use layout::SortedNodeLayout;
use nobject::{IriIndex, LangIndex, NodeData};
#[cfg(target_arch = "wasm32")]
use poll_promise::Promise;
use prefix_manager::PrefixManager;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use sparql_dialog::SparqlDialog;
use string_interner::Symbol;
use style::*;
use table_view::TypeInstanceIndex;

pub mod browse_view;
pub mod config;
pub mod distinct_colors;
pub mod drawing;
pub mod graph_styles;
pub mod graph_view;
pub mod layout;
pub mod menu_bar;
pub mod meta_graph;
pub mod nobject;
pub mod persistency;
pub mod prefix_manager;
pub mod rdfwrap;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql_dialog;
pub mod string_indexer;
pub mod style;
pub mod table_view;
pub mod uitools;

#[derive(Debug, PartialEq)]
pub enum DisplayType {
    Browse,
    Graph,
    Table,
    Prefixes,
    Configuration,
    MetaGraph,
}

// Define the application structure
pub struct RdfGlanceApp {
    object_iri: String,
    current_iri: Option<IriIndex>,
    rdfwrap: Box<dyn rdfwrap::RDFAdapter>,
    nav_pos: usize,
    nav_history: Vec<IriIndex>,
    display_type: DisplayType,
    ui_state: UIState,
    visible_nodes: SortedNodeLayout,
    meta_nodes: SortedNodeLayout,
    graph_state: GraphState,
    visualisation_style: GVisualisationStyle,
    #[cfg(not(target_arch = "wasm32"))]
    sparql_dialog: Option<SparqlDialog>,
    status_message: String,
    system_message: SystemMessage,
    pub rdf_data: Arc<RwLock<RdfData>>,
    type_index: TypeInstanceIndex,
    persistent_data: AppPersistentData,
    help_open: bool,
    load_handle: Option<JoinHandle<Option<Result<u32, Error>>>>,
    #[cfg(target_arch = "wasm32")]
    file_upload: Option<Promise<Result<File, anyhow::Error>>>,
    data_loading: Option<Arc<DataLoading>>,
}

pub struct DataLoading {
    pub stop_loading: Arc<AtomicBool>,
    pub progress: Arc<AtomicUsize>,
    pub total_triples: Arc<AtomicUsize>,
    pub read_pos: Arc<AtomicUsize>,
    pub total_size: Arc<AtomicUsize>,
}

pub struct RdfData {
    pub node_data: NodeData,
    pub prefix_manager: PrefixManager,
}

pub struct NodeChangeContext<'a> {
    pub rdfwrwap: &'a mut Box<dyn rdfwrap::RDFAdapter>,
    pub visible_nodes: &'a mut SortedNodeLayout,
}

impl RdfData {
    fn expand_node(
        &mut self,
        iri_index: IriIndex,
        expand_type: ExpandType,
        node_change_context: &mut NodeChangeContext,
    ) {
        let refs_to_expand = {
            let nnode = self.node_data.get_node_by_index(iri_index);
            if let Some((_, nnode)) = nnode {
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
            if self.load_object_by_index(ref_index, node_change_context) {
                npos.insert(iri_index, ref_index);
            }
        }
        update_layout_edges(&npos, node_change_context.visible_nodes, &self.node_data);
        npos.position(node_change_context.visible_nodes);
    }

    fn expand_all_by_types(&mut self, types: &[IriIndex], node_change_context: &mut NodeChangeContext) {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (_, ref_iri) in nnode.references.iter() {
                    if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                        if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                            parent_ref.push((visible_index.node_index, *ref_iri));
                        }
                    }
                }
                for (_, ref_iri) in nnode.reverse_references.iter() {
                    if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                        if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                            parent_ref.push((visible_index.node_index, *ref_iri));
                        }
                    }
                }
            }
        }
        let mut npos = NeighbourPos::new();
        for (parent_index, ref_index) in parent_ref {
            if !node_change_context.visible_nodes.contains(ref_index)
                && self.load_object_by_index(ref_index, node_change_context)
            {
                npos.insert(parent_index, ref_index);
            }
        }
        update_layout_edges(&npos,  node_change_context.visible_nodes, &self.node_data);
        npos.position(node_change_context.visible_nodes);
    }

    fn load_object_by_index(&mut self, index: IriIndex, node_change_context: &mut NodeChangeContext) -> bool {
        // self.visible_nodes.start_layout(&self.persistent_data.config_data);
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri, node)) = node {
            if node.has_subject {
                return node_change_context.visible_nodes.add_by_index(index);
            } else {
                let node_iri = node_iri.clone();
                let new_object = node_change_context.rdfwrwap.load_object(&node_iri, &mut self.node_data);
                if let Some(new_object) = new_object {
                    self.node_data.put_node_replace(&node_iri, new_object);
                }
            }
        }
        false
    }

    fn expand_all(&mut self, node_change_context: &mut NodeChangeContext) {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (_, ref_iri) in nnode.references.iter() {
                    if refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
                for (_, ref_iri) in nnode.reverse_references.iter() {
                    if refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
            }
        }
        let mut npos = NeighbourPos::new();
        for (parent_index, ref_index) in parent_ref {
            if !node_change_context.visible_nodes.contains(ref_index)
                && self.load_object_by_index(ref_index, node_change_context)
            {
                npos.insert(parent_index, ref_index);
            }
        }
        update_layout_edges(&npos, node_change_context.visible_nodes, &self.node_data);
        npos.position(node_change_context.visible_nodes);
    }

    pub fn resolve_rdf_lists(&mut self) {
        self.node_data.resolve_rdf_lists(&self.prefix_manager);
    }
}

#[cfg(target_arch = "wasm32")]
pub struct File {
    pub path: String,
    pub data: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
const SAMPLE_DATA: &[u8] = include_bytes!("../sample-rdf-data/programming_languages.ttl");

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

pub struct GVisualisationStyle {
    pub node_styles: HashMap<IriIndex, NodeStyle>,
    pub default_node_style: NodeStyle,
    pub edge_styles: HashMap<IriIndex, EdgeStyle>,
}

pub struct UIState {
    selected_node: Option<IriIndex>,
    context_menu_node: Option<IriIndex>,
    context_menu_pos: Pos2,
    node_to_drag: Option<IriIndex>,
    // Set if dragging for difference to dragged node center
    drag_diff: Pos2,
    hidden_predicates: SortedVec,
    meta_count_to_size: bool,
    display_language: LangIndex,
    language_sort: Vec<LangIndex>,
    show_properties: bool,
    show_labels: bool,
    short_iri: bool,
    fade_unselected: bool,
    style_edit: StyleEdit,
    icon_name_filter: String,
    cpu_usage: f32,
}

pub struct GraphState {
    scene_rect: Rect,
}

pub struct SortedVec {
    pub data: Vec<IriIndex>,
}

pub enum StyleEdit {
    Node(IriIndex),
    Edge(IriIndex),
    None,
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
        self.hidden_predicates.data.clear();
    }
}

impl GVisualisationStyle {
    pub fn preset_styles(&mut self, cache_statistics: &TypeInstanceIndex) {
        for (type_index, _type_desc) in cache_statistics.types.iter() {
            let type_style = self.node_styles.get(type_index);
            if type_style.is_none() {
                let new_color = distinct_colors::next_distinct_color(self.node_styles.len(), 0.8, 0.8, 200);
                self.node_styles.insert(
                    *type_index,
                    NodeStyle {
                        color: new_color,
                        ..Default::default()
                    },
                );
            }
        }
    }

    fn get_type_style(&self, types: &Vec<IriIndex>) -> &NodeStyle {
        let mut style: Option<&NodeStyle> = None;
        for type_iri in types {
            let type_style = self.node_styles.get(type_iri);
            if let Some(type_style) = type_style {
                if let Some(current_style) = style {
                    if type_style.priority > current_style.priority {
                        style = Some(type_style);
                    }
                } else {
                    style = Some(type_style);
                }
            }
        }
        style.unwrap_or(&self.default_node_style)
    }

    fn get_type_style_one(&self, type_iri: IriIndex) -> &NodeStyle {
        let mut style: Option<&NodeStyle> = None;
        let type_style = self.node_styles.get(&type_iri);
        if let Some(type_style) = type_style {
            if let Some(current_style) = style {
                if type_style.priority > current_style.priority {
                    style = Some(type_style);
                }
            } else {
                style = Some(type_style);
            }
        }
        style.unwrap_or(&self.default_node_style)
    }

    fn get_predicate_color(&mut self, iri: IriIndex) -> egui::Color32 {
        let len = self.edge_styles.len();
        self.edge_styles
            .entry(iri)
            .or_insert_with(|| EdgeStyle {
                color: distinct_colors::next_distinct_color(len, 0.5, 0.3, 170),
                ..EdgeStyle::default()
            })
            .color
    }

    fn get_edge_syle(&mut self, iri: IriIndex) -> &EdgeStyle {
        let len = self.edge_styles.len();
        self.edge_styles.entry(iri).or_insert_with(|| EdgeStyle {
            color: distinct_colors::next_distinct_color(len, 0.5, 0.3, 170),
            ..EdgeStyle::default()
        })
    }

    fn update_label(&mut self, iri: IriIndex, label_index: IriIndex) {
        if let Some(type_style) = self.node_styles.get_mut(&iri) {
            type_style.label_index = label_index;
        }
    }

    pub fn clean(&mut self) {
        self.node_styles.clear();
        self.edge_styles.clear();
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
                        serde_json::from_str(&persistent_data_string).expect("Failed to parse persistent data");
                    persistent_data.last_endpoints.retain(|endpoint| !endpoint.is_empty());
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
            #[cfg(not(target_arch = "wasm32"))]
            sparql_dialog: None,
            status_message: String::new(),
            type_index: TypeInstanceIndex::new(),
            system_message: SystemMessage::None,
            visible_nodes: SortedNodeLayout::new(),
            meta_nodes: SortedNodeLayout::new(),
            persistent_data: presistentdata.unwrap_or(AppPersistentData {
                last_files: vec![],
                last_endpoints: vec![],
                last_projects: vec![],
                config_data: config::Config::default(),
            }),
            rdf_data: Arc::new(RwLock::new(RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
            })),
            visualisation_style: GVisualisationStyle {
                node_styles: HashMap::new(),
                edge_styles: HashMap::new(),
                default_node_style: NodeStyle::default(),
            },
            graph_state: GraphState { scene_rect: Rect::ZERO },
            ui_state: UIState {
                selected_node: None,
                node_to_drag: None,
                context_menu_node: None,
                context_menu_pos: Pos2::new(0.0, 0.0),
                hidden_predicates: SortedVec::new(),
                display_language: 0,
                language_sort: Vec::new(),
                show_properties: true,
                show_labels: true,
                short_iri: true,
                style_edit: StyleEdit::None,
                drag_diff: Pos2::ZERO,
                icon_name_filter: String::new(),
                fade_unselected: false,
                meta_count_to_size: true,
                cpu_usage: 0.0,
            },
            help_open: false,
            load_handle: None,
            data_loading: None,
            #[cfg(target_arch = "wasm32")]
            file_upload: None,
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
    fn show_object_by_index(&mut self, index: IriIndex, add_history: bool) {
        if let Some(current_iri) = self.current_iri {
            if current_iri == index {
                return;
            }
        }
        if let Ok(rdf_data) = self.rdf_data.read() {
            let node = rdf_data.node_data.get_node_by_index(index);
            if let Some((node_iri, _node)) = node {
                self.current_iri = Some(index);
                self.object_iri = node_iri.to_string();
                if add_history {
                    self.nav_history.truncate(self.nav_pos + 1);
                    self.nav_history.push(self.current_iri.unwrap());
                    self.nav_pos = self.nav_history.len() - 1;
                }
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
    fn show_object(&mut self) {
        if self.show_current() {
            self.nav_history.truncate(self.nav_pos + 1);
            self.nav_history.push(self.current_iri.unwrap());
            self.nav_pos = self.nav_history.len() - 1;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_ttl(&mut self, file_name: &str) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(rdfwrap::RDFWrap::load_file(file_name, &mut rdf_data, &language_filter,None))
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
                    self.update_data_indexes();
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_ttl(&mut self, file_name: &str) {
        let rdf_data_clone = Arc::clone(&self.rdf_data);
        let language_filter = self.persistent_data.config_data.language_filter();
        let file_name_cpy = file_name.to_string();
        let data_loading = Arc::new(DataLoading {
            stop_loading: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(AtomicUsize::new(0)),
            total_triples: Arc::new(AtomicUsize::new(0)),
            read_pos: Arc::new(AtomicUsize::new(0)),
            total_size: Arc::new(AtomicUsize::new(0)),
        });
        let data_loading_clone = Arc::clone(&data_loading);
        let handle = thread::spawn(move || {
            if let Ok(mut rdf_data) = rdf_data_clone.write() {
                let my_data_loading = data_loading_clone.as_ref();
                Some(rdfwrap::RDFWrap::load_file(file_name_cpy, &mut rdf_data, &language_filter, Some(my_data_loading)))
            } else {
                None
            }
        });
        self.data_loading = Some(data_loading);
        self.load_handle = Some(handle);
    }

    pub fn join_load(&mut self) {
        if let Some(handle) = self.load_handle.take() {
            match handle.join() {
                Ok(Some(Ok(triples_count))) => {
                    self.set_status_message(&format!("Loaded {} triples", triples_count));
                    self.update_data_indexes();
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

    pub fn load_ttl_data(&mut self, file_name: &str, data: &Vec<u8>) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(rdfwrap::RDFWrap::load_file_data(
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
                    self.update_data_indexes();
                }
            }
        }
    }
    fn load_ttl_dir(&mut self, dir_name: &str) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(rdfwrap::RDFWrap::load_from_dir(
                dir_name,
                &mut rdf_data,
                &language_filter,
            ))
        } else {
            None
        };
        if let Some(rdfttl) = rdfttl {
            match rdfttl {
                Err(err) => {
                    self.system_message = SystemMessage::Error(format!("Directory not found: {}", err));
                }
                Ok(triples_count) => {
                    self.system_message =
                        SystemMessage::Info(format!("Loaded: {} triples: {}", dir_name, triples_count));
                    self.update_data_indexes();
                }
            }
        }
    }

    fn set_status_message(&mut self, message: &str) {
        self.status_message.clear();
        self.status_message.push_str(message);
    }
    pub fn update_data_indexes(&mut self) {
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
            self.type_index.update(&rdf_data.node_data);

            self.visualisation_style.preset_styles(&self.type_index);
            rdf_data.node_data.indexers.predicate_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.type_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.language_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.datatype_indexer.map.shrink_to_fit();
        }
    }
    fn empty_data_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("No data loaded. Load RDF file first.");
        let button_text = egui::RichText::new(concatcp!(ICON_OPEN_FOLDER, "Open RDF File")).size(16.0);
        let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
        let b_resp = ui.add(nav_but);
        if b_resp.clicked() {
            self.import_file_dialog(ui);
        }
        #[cfg(target_arch = "wasm32")]
        {
            ui.add_space(20.0);
            ui.strong("0 React, 0 HTML, Full Power!");
            ui.strong("Try Desktop version for full functionality! Especially multuthread more performant non-blocking processing.");
            let button_text = egui::RichText::new(concatcp!(ICON_OPEN_FOLDER, "Open Sample Data")).size(16.0);
            let nav_but = egui::Button::new(button_text).fill(egui::Color32::LIGHT_GREEN);
            let b_resp = ui.add(nav_but);
            if b_resp.clicked() {
                self.load_ttl_data("programming_languages.ttl", SAMPLE_DATA.to_vec().as_ref());
            }
        }
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
                            self.load_project(last_project_path);
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
    fn is_empty(&self) -> bool {
        self.rdf_data.read().unwrap().node_data.len() == 0
    }

    fn clean_data(&mut self) {
        self.ui_state.clean();
        self.type_index.clean();
        self.visualisation_style.clean();
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

    pub fn node_change_context(&mut self) -> NodeChangeContext {
        NodeChangeContext {
            rdfwrwap: &mut self.rdfwrap,
            visible_nodes: &mut self.visible_nodes,
        }
    }
}

impl eframe::App for RdfGlanceApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(cpu_usage) = frame.info().cpu_usage {
            self.ui_state.cpu_usage = self.ui_state.cpu_usage * 0.95 + cpu_usage * 0.05;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.rdf_data.try_read().is_err() {
                ui.label("RDF data is currently being loaded. Please wait...");
                if let Some(data_loading) = &self.data_loading {
                    let total_size = data_loading.total_size.load(Ordering::Relaxed);
                    if total_size>0 {
                        let progress = data_loading.read_pos.load(Ordering::Relaxed) as f32 / total_size as f32;
                        let progress_bar = egui::ProgressBar::new(progress)
                            .desired_width(300.0)
                            .show_percentage();
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
                }
                return;
            } else {
                self.join_load();
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
            self.menu_bar(ui);
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
                                        &self.visualisation_style,
                                        self.persistent_data.config_data.iri_display,
                                    )
                                } else {
                                    NodeAction::None
                                }
                            }
                            DisplayType::Configuration => self.show_config(ctx, ui),
                            DisplayType::Prefixes => self.show_prefixes(ctx, ui),
                            DisplayType::MetaGraph => self.show_meta_graph(ctx, ui),
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
                    self.visible_nodes.add_by_index(node_index);
                    self.ui_state.selected_node = Some(node_index);
                }
                NodeAction::None => {}
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(dialog) = &mut self.sparql_dialog {
                let (close_dialog, result) = dialog.show(ctx, &self.persistent_data.last_endpoints);
                if close_dialog {
                    if let Some(endpoint) = result {
                        self.rdfwrap = Box::new(sparql::SparqlAdapter::new(&endpoint));
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
        #[cfg(target_arch = "wasm32")]
        {
            self.handle_files(ctx);
        }
    }

    fn save(&mut self, _storage: &mut dyn Storage) {
        if let Ok(persistent_data_string) = serde_json::to_string(&self.persistent_data) {
            _storage.set_string("persistent_data", persistent_data_string);
            // println!("save called");
        }
    }
}
