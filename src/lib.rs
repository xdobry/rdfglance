#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Error;
use const_format::concatcp;
use fixedbitset::FixedBitSet;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering}, Arc, RwLock
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use eframe::{
    Storage,
    egui::{self, Pos2},
};
use egui::{Key, Rangef, Rect};
use egui_extras::StripBuilder;
use graph_styles::{EdgeStyle, NodeStyle};
use graph_view::{NeighborPos, update_layout_edges};
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

use crate::{config::Config, graph_view::NodeContextAction, nobject::NObject, statistics::StatisticsData, uitools::primary_color};

pub mod browse_view;
pub mod config;
pub mod distinct_colors;
pub mod drawing;
pub mod graph_algorithms;
pub mod layoutalg;
pub mod graph_styles;
pub mod graph_view;
pub mod layout;
pub mod menu_bar;
pub mod meta_graph;
pub mod nobject;
pub mod persistency;
pub mod prefix_manager;
pub mod quad_tree;
pub mod rdfwrap;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql_dialog;
pub mod statistics;
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
    Statistics,
}

// Define the application structure
pub struct RdfGlanceApp {
    object_iri: String,
    current_iri: Option<IriIndex>,
    ref_selection: RefSelection,
    rdfwrap: Box<dyn rdfwrap::RDFAdapter>,
    nav_pos: usize,
    nav_history: Vec<IriIndex>,
    display_type: DisplayType,
    ui_state: UIState,
    visible_nodes: SortedNodeLayout,
    meta_nodes: SortedNodeLayout,
    graph_state: GraphState,
    meta_graph_state: GraphState,
    visualization_style: GVisualizationStyle,
    statistics_data: Option<StatisticsData>,
    #[cfg(not(target_arch = "wasm32"))]
    sparql_dialog: Option<SparqlDialog>,
    status_message: String,
    system_message: SystemMessage,
    pub rdf_data: Arc<RwLock<RdfData>>,
    type_index: TypeInstanceIndex,
    pub persistent_data: AppPersistentData,
    help_open: bool,
    load_handle: Option<JoinHandle<Option<Result<LoadResult, Error>>>>,
    #[cfg(target_arch = "wasm32")]
    file_upload: Option<Promise<Result<File, anyhow::Error>>>,
    data_loading: Option<Arc<DataLoading>>,
    import_from_url: Option<ImportFromUrlData>,
}

// Used to indicate which reference is selected in the browse view
#[derive(Debug)]
pub enum RefSelection {
    None,
    Reference(usize),
    ReverseReverence(usize),
}

impl RefSelection {
    pub fn init_from_node(&mut self, node: &NObject) {
        if !node.references.is_empty() {
            *self = RefSelection::Reference(0);
        } else if !node.reverse_references.is_empty() {
            *self = RefSelection::ReverseReverence(0);
        } else {
            *self = RefSelection::None;
        }
    }
    pub fn ref_index(&self, is_reverse: bool) -> Option<usize> {
        match self {
            RefSelection::None => None,
            RefSelection::Reference(idx) => {
                if is_reverse {
                    None
                } else {
                    Some(*idx)
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if is_reverse {
                    Some(*idx)
                } else {
                    None
                }
            }
        }
    }
    pub fn move_up(&mut self) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if *idx > 0 {
                    *idx -= 1;
                }
            }
        }
    }
    pub fn move_down(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if *idx < node.references.len() - 1 {
                    *idx += 1;
                }
            }
            RefSelection::ReverseReverence(idx) => {
                if *idx < node.reverse_references.len() - 1 {
                    *idx += 1;
                }
            }
        }
    }
    pub fn move_right(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(idx) => {
                if !node.reverse_references.is_empty() {
                    if *idx > node.reverse_references.len() - 1 {
                        *idx = node.reverse_references.len() - 1;
                    }
                    *self = RefSelection::ReverseReverence(*idx);
                }
            }
            RefSelection::ReverseReverence(_) => {}
        }
    }
    pub fn move_left(&mut self, node: &NObject) {
        match self {
            RefSelection::None => {}
            RefSelection::Reference(_) => {}
            RefSelection::ReverseReverence(idx) => {
                if !node.references.is_empty() {
                    if *idx > node.references.len() - 1 {
                        *idx = node.references.len() - 1;
                    }
                    *self = RefSelection::Reference(*idx);
                }
            }
        }
    }
}

pub struct DataLoading {
    pub stop_loading: Arc<AtomicBool>,
    pub progress: Arc<AtomicUsize>,
    pub total_triples: Arc<AtomicUsize>,
    pub read_pos: Arc<AtomicUsize>,
    pub total_size: Arc<AtomicUsize>,
    pub finished: Arc<AtomicBool>,
}

pub struct ImportFromUrlData {
    pub url: String,
    pub format: ImportFormat,
    pub focus_requested: bool,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum ImportFormat {
    Turtle,
    RdfXml,
    NTriples,
}

impl ImportFormat {
    pub fn mime_type(&self) -> &str {
        match self {
            ImportFormat::Turtle => "text/turtle",
            ImportFormat::RdfXml => "application/rdf+xml",
            ImportFormat::NTriples => "application/n-triples",
        }
    }

    pub fn file_extension(&self) -> &str {
        match self {
            ImportFormat::Turtle => "ttl",
            ImportFormat::RdfXml => "rdf",
            ImportFormat::NTriples => "nt",
        }
    }
}

pub struct LoadResult {
    pub triples_count: u32,
    pub file_name: Option<String>,
}

pub struct RdfData {
    pub node_data: NodeData,
    pub prefix_manager: PrefixManager,
}

pub struct NodeChangeContext<'a> {
    pub rdfwrap: &'a mut Box<dyn rdfwrap::RDFAdapter>,
    pub visible_nodes: &'a mut SortedNodeLayout,
    pub config: &'a Config,
}

impl RdfData {
    fn expand_node(
        &mut self,
        iri_indexes: &BTreeSet<IriIndex>,
        expand_type: ExpandType,
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let mut refs_to_expand: Vec<(IriIndex,IriIndex)> = Vec::new();  
        for iri_index in iri_indexes.iter() {
            let nnode = self.node_data.get_node_by_index(*iri_index);
            if let Some((_, nnode)) = nnode {
                match expand_type {
                    ExpandType::References | ExpandType::Both => {
                        for (predicate, ref_iri) in &nnode.references {
                            if !hidden_predicates.contains(*predicate) {
                                refs_to_expand.push((*iri_index, *ref_iri));
                            }
                        }
                    }
                    _ => {}
                }
                match expand_type {
                    ExpandType::ReverseReferences | ExpandType::Both => {
                        for (predicate, ref_iri) in &nnode.reverse_references {
                            if !hidden_predicates.contains(*predicate) {
                                refs_to_expand.push((*iri_index, *ref_iri));
                            }
                        }
                    }
                    _ => {}
                }
            }
        };
        if refs_to_expand.is_empty() {
            return false;
        }
        let mut npos = NeighborPos::new();
        let was_added = npos.add_many(
            node_change_context.visible_nodes,
            &refs_to_expand,
            node_change_context.config,
        );
        if was_added {
            update_layout_edges(
                &npos,
                node_change_context.visible_nodes,
                &self.node_data,
                hidden_predicates,
            );
            npos.position(node_change_context.visible_nodes);
            true
        } else {
            false
        }
    }

    fn expand_all_by_types(
        &mut self,
        types: &[IriIndex],
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (predicate, ref_iri) in nnode.references.iter() {
                    if !hidden_predicates.contains(*predicate) {
                        if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                            if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                                parent_ref.push((visible_index.node_index, *ref_iri));
                            }
                        }
                    }
                }
                for (predicate, ref_iri) in nnode.reverse_references.iter() {
                    if !hidden_predicates.contains(*predicate) {
                        if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                            if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                                parent_ref.push((visible_index.node_index, *ref_iri));
                            }
                        }
                    }
                }
            }
        }
        if parent_ref.is_empty() {
            return false;
        }
        let mut npos = NeighborPos::new();
        let was_added = npos.add_many(
            node_change_context.visible_nodes,
            &parent_ref,
            node_change_context.config,
        );
        if was_added {
            update_layout_edges(
                &npos,
                node_change_context.visible_nodes,
                &self.node_data,
                hidden_predicates,
            );
            npos.position(node_change_context.visible_nodes);
            true
        } else {
            false
        }
    }

    fn load_object_by_index(&mut self, index: IriIndex, node_change_context: &mut NodeChangeContext) -> bool {
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri, node)) = node {
            if node.has_subject {
                return node_change_context.visible_nodes.add_by_index(index);
            } else {
                let node_iri = node_iri.clone();
                let new_object = node_change_context.rdfwrap.load_object(&node_iri, &mut self.node_data);
                if let Some(new_object) = new_object {
                    self.node_data.put_node_replace(&node_iri, new_object);
                }
            }
        }
        false
    }

    fn expand_all(&mut self, node_change_context: &mut NodeChangeContext, hidden_predicates: &SortedVec) -> bool {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (predicate, ref_iri) in nnode.references.iter() {
                    if !hidden_predicates.contains(*predicate) && refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
                for (predicate, ref_iri) in nnode.reverse_references.iter() {
                    if !hidden_predicates.contains(*predicate) && refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
            }
        }
        if parent_ref.is_empty() {
            false
        } else {
            let mut npos = NeighborPos::new();
            let was_added = npos.add_many(
                node_change_context.visible_nodes,
                &parent_ref,
                node_change_context.config,
            );
            if was_added {
                update_layout_edges(
                    &npos,
                    node_change_context.visible_nodes,
                    &self.node_data,
                    hidden_predicates,
                );
                npos.position(node_change_context.visible_nodes);
                true
            } else {
                false
            }
        }
    }

    fn unexpand_all(&mut self, node_change_context: &mut NodeChangeContext, hidden_predicates: &SortedVec) -> bool {
        let node_len = node_change_context.visible_nodes.nodes.read().unwrap().len();
        if node_len == 0 {
            return false;
        }
        let mut nodes_bits = FixedBitSet::with_capacity(node_len);
        for edge in node_change_context.visible_nodes.edges.read().unwrap().iter() {
            nodes_bits.insert(edge.from);
        }
        if nodes_bits.is_full() {
            false
        } else {
            let mut nodes_indexes_to_remove: Vec<usize> = nodes_bits.zeroes().collect();
            nodes_indexes_to_remove.sort_unstable();
            node_change_context
                .visible_nodes
                .remove_pos_list(&nodes_indexes_to_remove, hidden_predicates);
            true
        }
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

pub struct GVisualizationStyle {
    pub node_styles: HashMap<IriIndex, NodeStyle>,
    pub default_node_style: NodeStyle,
    pub edge_styles: HashMap<IriIndex, EdgeStyle>,
    pub use_size_overwrite: bool,
    pub use_color_overwrite: bool,
    pub min_size: f32,
    pub max_size: f32,
}

pub struct UIState {
    selected_node: Option<IriIndex>,
    selected_nodes: BTreeSet<IriIndex>,
    context_menu_node: Option<IriIndex>,
    context_menu_pos: Pos2,
    context_menu_opened_by_keyboard: bool,
    node_to_drag: Option<IriIndex>,
    // used for own translattion dragging, it start mouse pos and diff to left corner of scene rect
    // need to calculate new scene rect position
    translate_drag: Option<(Pos2,Pos2)>,
    selection_start_rect: Option<Pos2>,
    // Set if dragging for difference to dragged node center
    drag_diff: Pos2,
    drag_start: Pos2,
    hidden_predicates: SortedVec,
    // 1 - magnitude see most nodes, 0 - should be not used, meaning all nodes (also the possible cluster nodes)
    semantic_zoom_magnitude: u8,
    meta_count_to_size: bool,
    display_language: LangIndex,
    language_sort: Vec<LangIndex>,
    show_properties: bool,
    show_labels: bool,
    fade_unselected: bool,
    show_num_hidden_refs: bool,
    style_edit: StyleEdit,
    icon_name_filter: String,
    cpu_usage: f32,
    about_window: bool,
    last_visited_selection: LastVisitedSelection,
    menu_action: Option<NodeContextAction>,
}

pub enum LastVisitedSelection {
    None,
    File(usize),
    Project(usize),
}

pub struct GraphState {
    scene_rect: Rect,
}

#[derive(Clone)]
pub struct SortedVec {
    pub data: Vec<IriIndex>,
}

pub enum StyleEdit {
    Node(IriIndex),
    Edge(IriIndex),
    None,
}

#[derive(Serialize, Deserialize)]
pub struct AppPersistentData {
    last_files: Vec<Box<str>>,
    last_endpoints: Vec<Box<str>>,
    #[serde(default = "default_last_projects")]
    last_projects: Vec<Box<str>>,
    #[serde(default = "default_config_data")]
    pub config_data: config::Config,
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
    ShowTypeInstances(IriIndex, Vec<IriIndex>),
    ShowVisual(IriIndex),
    AddVisual(IriIndex)
}

impl UIState {
    pub fn clean(&mut self) {
        self.selected_node = None;
        self.context_menu_node = None;
        self.node_to_drag = None;
        self.hidden_predicates.data.clear();
    }
}

impl GVisualizationStyle {
    pub fn preset_styles(&mut self, type_instance_index: &TypeInstanceIndex, is_dark_mode: bool) {
        for (type_index, _type_desc) in type_instance_index.types.iter() {
            let type_style = self.node_styles.get(type_index);
            if type_style.is_none() {
                let lightness = if is_dark_mode { 0.3 } else { 0.6 };
                let new_color = distinct_colors::next_distinct_color(self.node_styles.len(), 0.8, lightness, 200);
                let order = type_instance_index.types_order.iter().position(|&i| i == *type_index);
                let priority = order.map(|o| o as u32).unwrap_or(0);
                self.node_styles.insert(
                    *type_index,
                    NodeStyle {
                        color: new_color,
                        priority,
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

    fn get_predicate_color(&mut self, iri: IriIndex, is_dark_mode: bool) -> egui::Color32 {
        let len = self.edge_styles.len();
        self.edge_styles
            .entry(iri)
            .or_insert_with(|| {
                let lightness = if is_dark_mode { 0.6 } else { 0.3 };
                EdgeStyle {
                    color: distinct_colors::next_distinct_color(len, 0.5, lightness, 170),
                    ..EdgeStyle::default()
                }
            })
            .color
    }

    fn get_edge_syle(&mut self, iri: IriIndex, is_dark_mode: bool) -> &EdgeStyle {
        let len = self.edge_styles.len();
        self.edge_styles.entry(iri).or_insert_with(|| {
            let lightness = if is_dark_mode { 0.6 } else { 0.3 };
            EdgeStyle {
                color: distinct_colors::next_distinct_color(len, 0.5, lightness, 170),
                ..EdgeStyle::default()
            }
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
            persistent_data: persistent_data.unwrap_or(AppPersistentData {
                last_files: vec![],
                last_endpoints: vec![],
                last_projects: vec![],
                config_data: config::Config::default(),
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
                min_size: 5.0,
                max_size: 50.0,
            },
            graph_state: GraphState { scene_rect: Rect::ZERO },
            meta_graph_state: GraphState { scene_rect: Rect::ZERO },
            statistics_data: None,
            ui_state: UIState {
                selected_node: None,
                selected_nodes: BTreeSet::new(),
                node_to_drag: None,
                context_menu_node: None,
                context_menu_pos: Pos2::new(0.0, 0.0),
                context_menu_opened_by_keyboard: false,
                hidden_predicates: SortedVec::new(),
                display_language: 0,
                language_sort: Vec::new(),
                show_properties: true,
                show_labels: true,
                style_edit: StyleEdit::None,
                drag_diff: Pos2::ZERO,
                drag_start: Pos2::ZERO,
                icon_name_filter: String::new(),
                fade_unselected: false,
                meta_count_to_size: true,
                cpu_usage: 0.0,
                semantic_zoom_magnitude: 1,
                about_window: false,
                show_num_hidden_refs: true,
                last_visited_selection: LastVisitedSelection::None,
                menu_action: None,
                selection_start_rect: None,
                translate_drag: None,
            },
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
}

pub enum ExpandType {
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
    fn show_object(&mut self) {
        if self.show_current() {
            self.nav_history.truncate(self.nav_pos + 1);
            self.nav_history.push(self.current_iri.unwrap());
            self.nav_pos = self.nav_history.len() - 1;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_ttl(&mut self, file_name: &str, is_dark_mode: bool) {
        let language_filter = self.persistent_data.config_data.language_filter();
        let rdfttl = if let Ok(mut rdf_data) = self.rdf_data.write() {
            Some(rdfwrap::RDFWrap::load_file(
                file_name,
                &mut rdf_data,
                &language_filter,
                None,
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
                    rdfwrap::RDFWrap::load_file(
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
                    rdfwrap::RDFWrap::load_from_url(
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
        self.file_upload = Some(Promise::spawn_local(async move {
            let client = reqwest::Client::new();
            let request = client.get(url_cpy.as_str()).header("Accept", "text/turtle");
            match request.send().await {
                Ok(resp) => {
                    if let Ok(bytes) = resp.bytes().await {
                        return Ok(crate::File {
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
                    self.update_data_indexes(is_dark_mode);
                }
            }
        }
    }
    fn load_ttl_dir(&mut self, dir_name: &str) {
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
                    rdfwrap::RDFWrap::load_from_dir(
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

    fn set_status_message(&mut self, message: &str) {
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

            self.visualization_style.preset_styles(&self.type_index, is_dark_mode);
            rdf_data.node_data.indexers.predicate_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.type_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.language_indexer.map.shrink_to_fit();
            rdf_data.node_data.indexers.datatype_indexer.map.shrink_to_fit();
        }
    }
    fn empty_data_ui(&mut self, ui: &mut egui::Ui) {
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
        if matches!(self.ui_state.last_visited_selection,LastVisitedSelection::None) && self.persistent_data.last_files.len()>0 {
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
    fn is_empty(&self) -> bool {
        self.rdf_data.read().unwrap().node_data.len() == 0
    }

    fn clean_data(&mut self) {
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
                            },
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
                            type_desc.instance_view.selected_idx = Some((type_desc.filtered_instances[0],0))
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
                        update_layout_edges(&npos, &mut self.visible_nodes, &rdf_data.node_data, &self.ui_state.hidden_predicates);
                    }
                    self.ui_state.selected_node = Some(node_index);
                    self.ui_state.selected_nodes.insert(node_index);
                    self.ui_state.selection_start_rect = None;
                }
                NodeAction::AddVisual(node_index) => {
                    self.visible_nodes.add_by_index(node_index);
                    if let Ok(rdf_data) = self.rdf_data.read() {
                        let npos = NeighborPos::one(node_index);
                        update_layout_edges(&npos, &mut self.visible_nodes, &rdf_data.node_data, &self.ui_state.hidden_predicates);
                    }
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
