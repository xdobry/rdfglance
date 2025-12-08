use std::{
    collections::BTreeSet, 
    sync::{Arc, atomic::{AtomicBool, AtomicUsize}}
};

use egui::{Pos2, Rect};

use crate::{
    IriIndex, 
    domain::LangIndex, 
    support::SortedVec, 
    uistate::actions::NodeContextAction
};

pub struct UIState {
    pub selected_node: Option<IriIndex>,
    pub selected_nodes: BTreeSet<IriIndex>,
    pub context_menu_node: Option<IriIndex>,
    pub context_menu_pos: Pos2,
    pub context_menu_opened_by_keyboard: bool,
    pub node_to_drag: Option<IriIndex>,
    // used for own translation dragging, it start mouse pos and diff to left corner of scene rect
    // need to calculate new scene rect position
    pub translate_drag: Option<(Pos2, Pos2)>,
    pub selection_start_rect: Option<Pos2>,
    // Set if dragging for difference to dragged node center
    pub drag_diff: Pos2,
    pub drag_start: Pos2,
    pub hidden_predicates: SortedVec,
    // 1 - magnitude see most nodes, 0 - should be not used, meaning all nodes (also the possible cluster nodes)
    pub semantic_zoom_magnitude: u8,
    pub meta_count_to_size: bool,
    pub display_language: LangIndex,
    pub language_sort: Vec<LangIndex>,
    pub show_properties: bool,
    pub show_labels: bool,
    pub fade_unselected: bool,
    pub show_num_hidden_refs: bool,
    pub style_edit: StyleEdit,
    pub icon_name_filter: String,
    pub cpu_usage: f32,
    pub about_window: bool,
    pub last_visited_selection: LastVisitedSelection,
    pub menu_action: Option<NodeContextAction>,
}

impl Default for UIState {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl UIState {
    pub fn clean(&mut self) {
        self.selected_node = None;
        self.context_menu_node = None;
        self.node_to_drag = None;
        self.hidden_predicates.data.clear();
    }
}

pub enum LastVisitedSelection {
    None,
    File(usize),
    Project(usize),
}



pub struct GraphState {
    pub scene_rect: Rect,
}


pub enum StyleEdit {
    Node(IriIndex),
    Edge(IriIndex),
    None,
}

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


// Used to indicate which reference is selected in the browse view

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




#[cfg(target_arch = "wasm32")]
pub struct File {
    pub path: String,
    pub data: Vec<u8>,
}


pub enum SystemMessage {
    None,
    Info(String),
    Error(String),
}

impl SystemMessage {
    pub fn has_message(&self) -> bool {
        !matches!(self, SystemMessage::None)
    }
    pub fn get_message(&self) -> &str {
        match self {
            SystemMessage::None => "",
            SystemMessage::Info(msg) => msg,
            SystemMessage::Error(msg) => msg,
        }
    }
}

