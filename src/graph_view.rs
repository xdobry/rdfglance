use std::collections::{HashMap, HashSet};

use crate::{
    ExpandType, GVisualizationStyle, NodeAction, NodeChangeContext, RdfGlanceApp, SortedVec, StyleEdit, UIState,
    config::Config,
    distinct_colors::next_distinct_color,
    drawing::{self, draw_node_label},
    graph_styles::{NodeShape, NodeSize, NodeStyle},
    layout::{
        Edge, IndividualNodeStyleData, LayoutConfUpdate, NodeCommand, NodeShapeData, SortedNodeLayout,
        update_edges_groups,
    },
    nobject::{Indexers, IriIndex, LabelContext, Literal, NObject, NodeData},
    style::{
        ICON_CENTER, ICON_CLEAN_ALL, ICON_EXPAND, ICON_GRAPH, ICON_HELP, ICON_HIGHLIGHT, ICON_LABEL, ICON_PROPERTIES,
        ICON_REDO, ICON_UNDO, ICON_UNEXPAND, ICON_WRENCH,
    },
    uitools::popup_at,
};
use const_format::concatcp;
use eframe::egui::{self, Pos2, Sense, Vec2};
use egui::{Key, Painter, Rect, Slider};
use rand::Rng;

const INITIAL_DISTANCE: f32 = 100.0;

struct ReferencesState {
    pub count: u32,
    pub visible: u32,
}

enum NodeContextAction {
    None,
    Hide,
    HideThisType,
    HideOther,
    HideOtherTypes,
    HideUnrelated,
    HideUnconnected,
    HideOrphans,
    HideRedundantEdges,
    HideZoomInvisible,
    Expand,
    ExpandReferenced,
    ExpandReferencedBy,
    ExpandThisType,
    HideThisTypePreserveEdges,
    ShowAllInstanceInTable,
}

impl NodeContextAction {
    fn show_menu(ui: &mut egui::Ui, opened_by_keyboard: bool, has_zoom: bool) -> NodeContextAction {
        let hide_button = ui.button("Hide (H)");
        if opened_by_keyboard {
            hide_button.request_focus();
        }
        if hide_button.clicked() {
            return NodeContextAction::Hide;
        }
        if ui.button("Hide this type (T)").clicked() {
            return NodeContextAction::HideThisType;
        }
        if ui.button("Hide this type with Edge Preservation").clicked() {
            return NodeContextAction::HideThisTypePreserveEdges;
        }
        if ui.button("Hide other (O)").clicked() {
            return NodeContextAction::HideOther;
        }
        if ui.button("Hide other types").clicked() {
            return NodeContextAction::HideOtherTypes;
        }
        if ui.button("Hide unrelated").clicked() {
            return NodeContextAction::HideUnrelated;
        }
        if ui.button("Hide unconnected").clicked() {
            return NodeContextAction::HideUnconnected;
        }
        if ui.button("Hide Orphan Nodes").clicked() {
            return NodeContextAction::HideOrphans;
        }
        if ui.button("Hide Redundant Edges").clicked() {
            return NodeContextAction::HideRedundantEdges;
        }
        let req = ui.add_enabled(has_zoom, egui::Button::new("Hide Semantic Zoom Invisible"));
        if req.clicked() {
            return NodeContextAction::HideZoomInvisible;
        }
        if ui.button("Expand (E)").clicked() {
            return NodeContextAction::Expand;
        }
        if ui.button("Expand Referenced").clicked() {
            return NodeContextAction::ExpandReferenced;
        }
        if ui.button("Expand Referenced By").clicked() {
            return NodeContextAction::ExpandReferencedBy;
        }
        if ui.button("Expand this type").clicked() {
            return NodeContextAction::ExpandThisType;
        }
        if ui.button("List all Instances of this type").clicked() {
            return NodeContextAction::ShowAllInstanceInTable;
        }
        NodeContextAction::None
    }
}

enum NodeSelectionMove {
    None,
    Right,
    Left,
    Up,
    Down,
}

struct NextNodeSelection {
    current_selected: Option<(IriIndex, Pos2)>,
    node_selection_move: NodeSelectionMove,
    next_selected: Option<(IriIndex, Pos2)>,
}

impl NextNodeSelection {
    fn new(current_index: IriIndex, current_pos: Pos2, node_selection_move: NodeSelectionMove) -> Self {
        Self {
            current_selected: Some((current_index, current_pos)),
            node_selection_move,
            next_selected: None,
        }
    }
    fn empty() -> Self {
        Self {
            current_selected: None,
            node_selection_move: NodeSelectionMove::None,
            next_selected: None,
        }
    }
    fn consider_node(&mut self, node_index: IriIndex, node_pos: Pos2) {
        if let Some((current_index, current_pos)) = self.current_selected {
            if current_index != node_index {
                match self.node_selection_move {
                    NodeSelectionMove::None => {}
                    NodeSelectionMove::Right => {
                        if node_pos.x > current_pos.x
                            && (node_pos.x - current_pos.x) >= (node_pos.y - current_pos.y).abs()
                        {
                            if let Some((_next_selected, pos)) = self.next_selected {
                                if node_pos.x < pos.x {
                                    self.next_selected = Some((node_index, node_pos));
                                }
                            } else {
                                self.next_selected = Some((node_index, node_pos));
                            }
                        }
                    }
                    NodeSelectionMove::Left => {
                        if node_pos.x < current_pos.x
                            && (current_pos.x - node_pos.x) >= (node_pos.y - current_pos.y).abs()
                        {
                            if let Some((_next_selected, pos)) = self.next_selected {
                                if node_pos.x > pos.x {
                                    self.next_selected = Some((node_index, node_pos));
                                }
                            } else {
                                self.next_selected = Some((node_index, node_pos));
                            }
                        }
                    }
                    NodeSelectionMove::Up => {
                        if node_pos.y < current_pos.y
                            && (current_pos.y - node_pos.y) >= (node_pos.x - current_pos.x).abs()
                        {
                            if let Some((_next_selected, pos)) = self.next_selected {
                                if node_pos.y > pos.y {
                                    self.next_selected = Some((node_index, node_pos));
                                }
                            } else {
                                self.next_selected = Some((node_index, node_pos));
                            }
                        }
                    }
                    NodeSelectionMove::Down => {
                        if node_pos.y > current_pos.y
                            && (node_pos.y - current_pos.y) >= (node_pos.x - current_pos.x).abs()
                        {
                            if let Some((_next_selected, pos)) = self.next_selected {
                                if node_pos.y < pos.y {
                                    self.next_selected = Some((node_index, node_pos));
                                }
                            } else {
                                self.next_selected = Some((node_index, node_pos));
                            }
                        }
                    }
                }
            }
        }
    }
}

impl RdfGlanceApp {
    pub fn show_graph(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        let mut node_to_click: NodeAction = NodeAction::None;
        if self.visible_nodes.nodes.read().unwrap().is_empty() {
            ui.heading(concatcp!(
                "No nodes to display. Go to tables or browser and add a node to graph using button with ",
                ICON_GRAPH
            ));
            return NodeAction::None;
        }
        ui.horizontal(|ui| {
            let undo_button = ui.add_enabled(!self.visible_nodes.undo_stack.is_empty(), egui::Button::new(ICON_UNDO));
            if undo_button.on_hover_text("Undo").clicked() 
                || ui.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
                self.visible_nodes.undo(&self.persistent_data.config_data, &self.ui_state.hidden_predicates );
            }
            let redo_button = ui.add_enabled(!self.visible_nodes.redo_stack.is_empty(), egui::Button::new(ICON_REDO));
            if redo_button.on_hover_text("Redo").clicked() 
                || ui.input(|i| i.modifiers.command && i.key_pressed(Key::Y)) {
                self.visible_nodes.redo(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
            }
            if ui.button(ICON_EXPAND).on_hover_text("Expand Nodes (+)").clicked() 
                || ui.input(|i| i.key_pressed(Key::Plus)) {
                if let Ok(mut rdf_data) = self.rdf_data.write() {
                    let mut node_change_context =  NodeChangeContext {
                        rdfwrap: &mut self.rdfwrap,
                        visible_nodes: &mut self.visible_nodes,
                        config: &self.persistent_data.config_data,
                    };
                    if rdf_data.expand_all(&mut node_change_context, &self.ui_state.hidden_predicates) {
                        self.visible_nodes.start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                    }
                }
            }
            if ui.button(ICON_UNEXPAND).on_hover_text("Unexpand Nodes, Remove Leaves (-)").clicked() 
                || ui.input(|i| i.key_pressed(Key::Minus)) {
                if let Ok(mut rdf_data) = self.rdf_data.write() {
                    let mut node_change_context =  NodeChangeContext {
                        rdfwrap: &mut self.rdfwrap,
                        visible_nodes: &mut self.visible_nodes,
                        config: &self.persistent_data.config_data,
                    };
                    if rdf_data.unexpand_all(&mut node_change_context, &self.ui_state.hidden_predicates) {
                        self.visible_nodes.start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                    }
                }
            }
            if ui.button(ICON_CENTER).on_hover_text("Center Graph and reset zoom (Home)").clicked()
                || ui.input(|i| i.key_pressed(Key::Home))
             {
                self.graph_state.scene_rect = Rect::ZERO;
                self.visible_nodes.to_center();
            }
            if self.visible_nodes.has_semantic_zoom {
                ui.label("Semantic zoom");
                ui.add(Slider::new(&mut self.ui_state.semantic_zoom_magnitude, 1..=10));
            }
            self.visible_nodes.show_handle_layout_ui(ctx, ui, &self.persistent_data.config_data, &self.ui_state.hidden_predicates);
            ui.label("nodes force");
            let response = ui.add(Slider::new(&mut self.persistent_data.config_data.m_repulsion_constant, 0.1..=8.0));
            if response.changed() {
                if let Some(layout_handle) = &self.visible_nodes.layout_handle {
                    let _ = layout_handle.update_sender.send(LayoutConfUpdate::UpdateRepulsionConstant(
                        self.persistent_data.config_data.m_repulsion_constant));
                }
            }
            ui.label("edges force");
            let response = ui.add(Slider::new(&mut self.persistent_data.config_data.m_attraction_factor, 0.02..=3.0));
            if response.changed() {
                if let Some(layout_handle) = &self.visible_nodes.layout_handle {
                    let _ = layout_handle.update_sender.send(LayoutConfUpdate::UpdateAttractionFactor(
                        self.persistent_data.config_data.m_attraction_factor));
                }
            }
            if ui.selectable_label(self.ui_state.show_labels, ICON_LABEL).on_hover_text("Show Node Labels").clicked() {
                self.ui_state.show_labels = !self.ui_state.show_labels;
            }
            if ui.selectable_label(self.ui_state.fade_unselected, ICON_HIGHLIGHT).on_hover_text("Highlight selected nodes and related").clicked() {
                self.ui_state.fade_unselected = !self.ui_state.fade_unselected;
            }
            if ui.button(ICON_CLEAN_ALL).on_hover_text("Remove all nodes and edges from graph").clicked() {
                self.visible_nodes.clean_all();
            }
            let help_but = ui.button(ICON_HELP);
            if help_but.clicked() {
                self.help_open = true;
            }
            if self.help_open {
                egui::Window::new("Quick Help")
                    .collapsible(false)
                    .resizable(false)
                    .default_size([300.0, 100.0])
                    .default_pos(help_but.rect.left_bottom())
                    .open(&mut self.help_open) // Small window
                    .show(ctx, |ui| {
                        ui.label("Use right mouse click on node to open context Menu\n\nZoom use Ctrl + mouse wheel\n\nExpand Relations - double click on node");
                    });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.selectable_label(self.ui_state.show_properties, ICON_PROPERTIES).on_hover_text("Show/Hide  Node Properties Panel").clicked() {
                    self.ui_state.show_properties = !self.ui_state.show_properties;
                }
            });
        });
        match self.ui_state.style_edit {
            StyleEdit::Node(type_style_edit) => {
                self.display_node_style(ui, type_style_edit);
            }
            StyleEdit::Edge(edge_style_edit) => {
                self.display_edge_style(ui, edge_style_edit);
            }
            StyleEdit::None => {
                if self.ui_state.show_properties {
                    egui::SidePanel::right("right_panel")
                        .exact_width(500.0)
                        .show_inside(ui, |ui| {
                            egui::ScrollArea::both().show(ui, |ui| {
                                node_to_click = self.display_node_details(ui);
                            });
                        });
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        self.display_graph(ctx, ui, &mut node_to_click);
                    });
                } else {
                    self.display_graph(ctx, ui, &mut node_to_click);
                }
            }
        }
        node_to_click
    }

    pub fn display_node_details(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut node_to_click = NodeAction::None;
        if let Some(iri_index) = &self.ui_state.selected_node {
            if self.visible_nodes.contains(*iri_index) {
                if let Ok(rdf_data) = self.rdf_data.read() {
                    let current_node = rdf_data.node_data.get_node_by_index(*iri_index);
                    if let Some((current_node_iri, current_node)) = current_node {
                        if ui.link(current_node_iri).clicked() {
                            node_to_click = NodeAction::BrowseNode(*iri_index);
                        }
                        ui.horizontal(|ui| {
                            ui.strong("Types:");
                            let label_context = LabelContext::new(
                                self.ui_state.display_language,
                                self.persistent_data.config_data.iri_display,
                                &rdf_data.prefix_manager,
                            );
                            for type_index in &current_node.types {
                                let type_label = rdf_data.node_data.type_display(
                                    *type_index,
                                    &label_context,
                                    &rdf_data.node_data.indexers,
                                );
                                if ui.button(type_label.as_str()).clicked() {
                                    node_to_click = NodeAction::ShowType(*type_index);
                                }
                                if ui.button(ICON_WRENCH).clicked() {
                                    self.ui_state.style_edit = StyleEdit::Node(*type_index);
                                }
                            }
                        });
                        ui.add_space(10.0);
                        if !current_node.properties.is_empty() {
                            let available_width = (ui.available_width() - 100.0).max(400.0);
                            ui.strong("Data Properties:");
                            egui::Grid::new("properties")
                                .striped(true)
                                .max_col_width(available_width)
                                .show(ui, |ui| {
                                    let label_context = LabelContext::new(
                                        self.ui_state.display_language,
                                        self.persistent_data.config_data.iri_display,
                                        &rdf_data.prefix_manager,
                                    );
                                    for (predicate_index, prop_value) in &current_node.properties {
                                        if self.persistent_data.config_data.suppress_other_language_data {
                                            if let Literal::LangString(lang, _) = prop_value {
                                                if *lang != self.ui_state.display_language {
                                                    if *lang == 0 && self.ui_state.display_language != 0 {
                                                        // it is fallback language so display if real language could not be found
                                                        let mut found = false;
                                                        for (predicate_index2, prop_value2) in &current_node.properties
                                                        {
                                                            if predicate_index2 == predicate_index
                                                                && prop_value2 != prop_value
                                                            {
                                                                if let Literal::LangString(lang, _) = prop_value2 {
                                                                    if *lang == self.ui_state.display_language {
                                                                        found = true;
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        if found {
                                                            continue;
                                                        }
                                                    } else {
                                                        continue;
                                                    }
                                                }
                                            }
                                        }
                                        let predicate_label = rdf_data.node_data.predicate_display(
                                            *predicate_index,
                                            &label_context,
                                            &rdf_data.node_data.indexers,
                                        );
                                        let lab_button = egui::Button::new(predicate_label.as_str());
                                        let lab_button_response = ui.add(lab_button);
                                        if lab_button_response.clicked() {
                                            for node_type_index in current_node.types.iter() {
                                                self.visualization_style
                                                    .update_label(*node_type_index, *predicate_index);
                                            }
                                        }
                                        lab_button_response
                                            .on_hover_text("Set this property as label for the node type");
                                        ui.label(prop_value.as_str_ref(&rdf_data.node_data.indexers));
                                        ui.end_row();
                                    }
                                });
                        }
                        if !current_node.references.is_empty() {
                            ui.add_space(10.0);
                            ui.strong("References");
                            let mut reference_state: HashMap<IriIndex, ReferencesState> = HashMap::new();
                            let mut references: Vec<IriIndex> = Vec::new();
                            for (predicate_index, ref_iri) in &current_node.references {
                                let is_visible = self.visible_nodes.contains(*ref_iri);
                                if references.contains(predicate_index) {
                                    let reference_state = reference_state.get_mut(predicate_index).unwrap();
                                    reference_state.count += 1;
                                    if is_visible {
                                        reference_state.visible += 1;
                                    }
                                } else {
                                    references.push(*predicate_index);
                                    reference_state.insert(
                                        *predicate_index,
                                        ReferencesState {
                                            count: 1,
                                            visible: if is_visible { 1 } else { 0 },
                                        },
                                    );
                                }
                            }
                            let label_context = LabelContext::new(
                                self.ui_state.display_language,
                                self.persistent_data.config_data.iri_display,
                                &rdf_data.prefix_manager,
                            );
                            for reference_index in references.iter() {
                                ui.horizontal(|ui| {
                                    let reference_label = rdf_data.node_data.predicate_display(
                                        *reference_index,
                                        &label_context,
                                        &rdf_data.node_data.indexers,
                                    );
                                    if ui.button(reference_label.as_str()).clicked() {
                                        let mut npos = NeighborPos::new();
                                        let mut nodes_to_add: Vec<(IriIndex, IriIndex)> = Vec::new();
                                        for (predicate_index, ref_iri) in &current_node.references {
                                            if predicate_index == reference_index {
                                                nodes_to_add.push((*iri_index, *ref_iri));
                                            }
                                        }
                                        npos.add_many(
                                            &mut self.visible_nodes,
                                            &nodes_to_add,
                                            &self.persistent_data.config_data,
                                        );
                                        if !npos.is_empty() {
                                            update_layout_edges(
                                                &npos,
                                                &mut self.visible_nodes,
                                                &rdf_data.node_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                            npos.position(&mut self.visible_nodes);
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    }
                                    let edge_style_button = egui::Button::new(ICON_WRENCH).fill(
                                        self.visualization_style
                                            .get_predicate_color(*reference_index, ui.visuals().dark_mode),
                                    );
                                    if ui.add(edge_style_button).clicked() {
                                        self.ui_state.style_edit = StyleEdit::Edge(*reference_index);
                                    }
                                    let ext_button = ui.button("➕");
                                    // ext_button.show_tooltip_text("Extend this relation for all visible nodes");
                                    if ext_button.clicked() {
                                        let mut nodes_to_add: Vec<(IriIndex, IriIndex)> = Vec::new();
                                        for visible_index in self.visible_nodes.nodes.read().unwrap().iter() {
                                            let visible_node =
                                                rdf_data.node_data.get_node_by_index(visible_index.node_index);
                                            if let Some((_v_node_iri, visible_node)) = visible_node {
                                                for (predicate_index, ref_iri) in &visible_node.references {
                                                    if predicate_index == reference_index {
                                                        nodes_to_add.push((visible_index.node_index, *ref_iri));
                                                    }
                                                }
                                            }
                                        }
                                        let mut npos = NeighborPos::new();
                                        npos.add_many(
                                            &mut self.visible_nodes,
                                            &nodes_to_add,
                                            &self.persistent_data.config_data,
                                        );
                                        if !npos.is_empty() {
                                            update_layout_edges(
                                                &npos,
                                                &mut self.visible_nodes,
                                                &rdf_data.node_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                            npos.position(&mut self.visible_nodes);
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    }
                                    let reference_state = reference_state.get(reference_index).unwrap();
                                    let state = format!("{}/{}", reference_state.count, reference_state.visible);
                                    ui.label(state);
                                    if self.ui_state.hidden_predicates.contains(*reference_index) {
                                        let show_but = ui.button("👁");
                                        // show_but.show_tooltip_text("Show all relations of this type");
                                        if show_but.clicked() {
                                            self.ui_state.hidden_predicates.remove(*reference_index);
                                            if let Ok(mut edges) = self.visible_nodes.edges.write() {
                                                update_edges_groups(&mut edges, &self.ui_state.hidden_predicates);
                                            }
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    } else {
                                        let hide_but = ui.button("❌");
                                        // hide_but.show_tooltip_text("Hide all relations of this type");
                                        if hide_but.clicked() {
                                            self.ui_state.hidden_predicates.add(*reference_index);
                                            if let Ok(mut edges) = self.visible_nodes.edges.write() {
                                                update_edges_groups(&mut edges, &self.ui_state.hidden_predicates);
                                            }
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    }
                                });
                            }
                        }
                        if !current_node.reverse_references.is_empty() {
                            ui.add_space(10.0);
                            ui.strong("Referenced by");
                            let mut reference_state: HashMap<IriIndex, ReferencesState> = HashMap::new();
                            let mut references: Vec<IriIndex> = Vec::new();
                            for (predicate_index, ref_iri) in &current_node.reverse_references {
                                let is_visible = self.visible_nodes.contains(*ref_iri);
                                if references.contains(predicate_index) {
                                    let reference_state = reference_state.get_mut(predicate_index).unwrap();
                                    reference_state.count += 1;
                                    if is_visible {
                                        reference_state.visible += 1;
                                    }
                                } else {
                                    references.push(*predicate_index);
                                    reference_state.insert(
                                        *predicate_index,
                                        ReferencesState {
                                            count: 1,
                                            visible: if is_visible { 1 } else { 0 },
                                        },
                                    );
                                }
                            }
                            let label_context = LabelContext::new(
                                self.ui_state.display_language,
                                self.persistent_data.config_data.iri_display,
                                &rdf_data.prefix_manager,
                            );
                            for reference_index in references.iter() {
                                ui.horizontal(|ui| {
                                    let reference_label = rdf_data.node_data.predicate_display(
                                        *reference_index,
                                        &label_context,
                                        &rdf_data.node_data.indexers,
                                    );
                                    if ui.button(reference_label.as_str()).clicked() {
                                        let mut npos = NeighborPos::new();
                                        let mut nodes_to_add: Vec<(IriIndex, IriIndex)> = Vec::new();
                                        for (predicate_index, ref_iri) in &current_node.reverse_references {
                                            if predicate_index == reference_index {
                                                nodes_to_add.push((*iri_index, *ref_iri));
                                            }
                                        }
                                        npos.add_many(
                                            &mut self.visible_nodes,
                                            &nodes_to_add,
                                            &self.persistent_data.config_data,
                                        );
                                        if !npos.is_empty() {
                                            update_layout_edges(
                                                &npos,
                                                &mut self.visible_nodes,
                                                &rdf_data.node_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                            npos.position(&mut self.visible_nodes);
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    }
                                    let edge_style_button = egui::Button::new(ICON_WRENCH).fill(
                                        self.visualization_style
                                            .get_predicate_color(*reference_index, ui.visuals().dark_mode),
                                    );
                                    if ui.add(edge_style_button).clicked() {
                                        self.ui_state.style_edit = StyleEdit::Edge(*reference_index);
                                    }
                                    if ui.button("➕").clicked() {
                                        let mut nodes_to_add: Vec<(IriIndex, IriIndex)> = Vec::new();
                                        for node_layout in self.visible_nodes.nodes.read().unwrap().iter() {
                                            let visible_node =
                                                rdf_data.node_data.get_node_by_index(node_layout.node_index);
                                            if let Some((_, visible_node)) = visible_node {
                                                for (predicate_index, ref_iri) in &visible_node.reverse_references {
                                                    if predicate_index == reference_index {
                                                        nodes_to_add.push((node_layout.node_index, *ref_iri));
                                                    }
                                                }
                                            }
                                        }
                                        let mut npos = NeighborPos::new();
                                        npos.add_many(
                                            &mut self.visible_nodes,
                                            &nodes_to_add,
                                            &self.persistent_data.config_data,
                                        );
                                        if !npos.is_empty() {
                                            update_layout_edges(
                                                &npos,
                                                &mut self.visible_nodes,
                                                &rdf_data.node_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                            npos.position(&mut self.visible_nodes);
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    }
                                    let reference_state = reference_state.get(reference_index).unwrap();
                                    let state = format!("{}/{}", reference_state.count, reference_state.visible);
                                    ui.label(state);
                                    if self.ui_state.hidden_predicates.contains(*reference_index) {
                                        if ui.button("👁").clicked() {
                                            self.ui_state.hidden_predicates.remove(*reference_index);
                                            if let Ok(mut edges) = self.visible_nodes.edges.write() {
                                                update_edges_groups(&mut edges, &self.ui_state.hidden_predicates);
                                            }
                                            self.visible_nodes.start_layout(
                                                &self.persistent_data.config_data,
                                                &self.ui_state.hidden_predicates,
                                            );
                                        }
                                    } else if ui.button("❌").clicked() {
                                        self.ui_state.hidden_predicates.add(*reference_index);
                                        if let Ok(mut edges) = self.visible_nodes.edges.write() {
                                            update_edges_groups(&mut edges, &self.ui_state.hidden_predicates);
                                        }
                                        self.visible_nodes.start_layout(
                                            &self.persistent_data.config_data,
                                            &self.ui_state.hidden_predicates,
                                        );
                                    }
                                });
                            }
                        }
                    }
                }
            }
        } else {
            ui.label("no node selected");
        }
        node_to_click
    }

    pub fn display_graph(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, node_browse: &mut NodeAction) {
        let mut node_count = 0;
        let mut edge_count = 0;
        let mut secondary_clicked = false;
        let mut single_clicked = false;
        let mut double_clicked = false;
        let mut primary_down = false;
        let mut was_context_click = false;
        let mut node_to_click: Option<IriIndex> = None;
        let mut node_to_hover: Option<IriIndex> = None;
        let mut was_action = false;

        let global_mouse_pos = ctx.pointer_hover_pos().unwrap_or(Pos2::new(0.0, 0.0));

        let scene = egui::Scene::new().zoom_range(0.1..=4.0);
        let popup_id = ui.make_persistent_id("node_context_menu");
        let is_context_menu_open = ui.memory(|mem| mem.is_popup_open(popup_id));

        if let Ok(rdf_data) = self.rdf_data.read() {
            scene.show(ui, &mut self.graph_state.scene_rect, |ui| {
                let available_width = ui.available_width();
                let available_height = ui.available_height();
                let size = Vec2::new(available_width, available_height);

                let (id, rect) = ui.allocate_space(size);
                let painter = ui.painter();

                let center = rect.center();

                // The code is complicated because of event handling, especially for click and dragging
                // If node is clicked/dragged the event should not be propagated to scene layer
                // so we need to handle events manually by input and if input are consumed
                // after it create big interact area that consume all events

                let transform = ctx.layer_transform_to_global(ui.layer_id());
                let mouse_pos = if let Some(transform) = transform {
                    let local_mouse_pos = ctx.pointer_hover_pos().unwrap_or(Pos2::new(0.0, 0.0));
                    transform.inverse() * local_mouse_pos
                } else {
                    Pos2::new(0.0, 0.0)
                };
                let mut node_selection_move = NodeSelectionMove::None;
                ctx.input(|input| {
                    single_clicked = input.pointer.button_clicked(egui::PointerButton::Primary);
                    secondary_clicked = input.pointer.button_clicked(egui::PointerButton::Secondary);
                    double_clicked = input.pointer.button_double_clicked(egui::PointerButton::Primary);
                    if input.pointer.button_pressed(egui::PointerButton::Primary) {
                        primary_down = true;
                    }
                    if input.pointer.button_released(egui::PointerButton::Primary) {
                        self.ui_state.node_to_drag = None;
                    }
                    if !is_context_menu_open {
                        if self.ui_state.context_menu_node.is_some() {
                            self.ui_state.context_menu_opened_by_keyboard = false;
                            self.ui_state.context_menu_node = None;
                        }
                        if input.key_pressed(egui::Key::ArrowUp) {
                            node_selection_move = NodeSelectionMove::Up;
                        } else if input.key_pressed(egui::Key::ArrowDown) {
                            node_selection_move = NodeSelectionMove::Down;
                        } else if input.key_pressed(egui::Key::ArrowLeft) {
                            node_selection_move = NodeSelectionMove::Left;
                        } else if input.key_pressed(egui::Key::ArrowRight) {
                            node_selection_move = NodeSelectionMove::Right;
                        }
                        // There is currently not defined key for opening context menu so use Shift + F10
                        if input.modifiers.shift && input.key_pressed(egui::Key::F10) {
                            if let Some(selected_index) = self.ui_state.selected_node {
                                if let Ok(pos) = self
                                    .visible_nodes
                                    .nodes
                                    .read()
                                    .unwrap()
                                    .binary_search_by(|e| e.node_index.cmp(&selected_index))
                                {
                                    if let Ok(positions) = self.visible_nodes.positions.read() {
                                        if let Some(transform) = transform {
                                            was_context_click = true;
                                            was_action = true;
                                            let pos = transform * (center + positions[pos].pos.to_vec2());
                                            self.ui_state.context_menu_pos = pos;
                                            self.ui_state.context_menu_opened_by_keyboard = true;
                                            self.ui_state.context_menu_node = Some(selected_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
                let label_context = LabelContext::new(
                    self.ui_state.display_language,
                    self.persistent_data.config_data.iri_display,
                    &rdf_data.prefix_manager,
                );
                edge_count += self.visible_nodes.edges.read().unwrap().len() as u32;
                let mut selected_related_nodes_pos = Vec::new();
                if let Ok(nodes) = self.visible_nodes.nodes.read() {
                    if let Ok(positions) = self.visible_nodes.positions.read() {
                        if let Ok(individual_node_styles) = self.visible_nodes.individual_node_styles.read() {
                            if let Ok(edges) = self.visible_nodes.edges.read() {
                                if self.ui_state.fade_unselected {
                                    if let Some(selected_node) = &self.ui_state.selected_node {
                                        let selected_pos = nodes.binary_search_by(|e| e.node_index.cmp(selected_node));
                                        if let Ok(selected_pos) = selected_pos {
                                            selected_related_nodes_pos.push(selected_pos);
                                            for edge in edges.iter() {
                                                if edge.from == selected_pos {
                                                    selected_related_nodes_pos.push(edge.to);
                                                } else if edge.to == selected_pos {
                                                    selected_related_nodes_pos.push(edge.from);
                                                }
                                            }
                                        }
                                        selected_related_nodes_pos.sort_unstable();
                                        selected_related_nodes_pos.dedup();
                                    }
                                }
                                if let Ok(node_shapes) = self.visible_nodes.node_shapes.read() {
                                    for edge in edges.iter() {
                                        if self.ui_state.hidden_predicates.contains(edge.predicate) {
                                            continue;
                                        }
                                        if self.visible_nodes.has_semantic_zoom {
                                            if !individual_node_styles[edge.from]
                                                .semantic_zoom_interval
                                                .is_visible(self.ui_state.semantic_zoom_magnitude)
                                                || !individual_node_styles[edge.to]
                                                    .semantic_zoom_interval
                                                    .is_visible(self.ui_state.semantic_zoom_magnitude)
                                            {
                                                continue;
                                            }
                                        }

                                        let node_label = || {
                                            let reference_label = rdf_data.node_data.predicate_display(
                                                edge.predicate,
                                                &label_context,
                                                &rdf_data.node_data.indexers,
                                            );
                                            reference_label.as_str().to_owned()
                                        };
                                        let pos1 = center + positions[edge.from].pos.to_vec2();
                                        if edge.from != edge.to {
                                            let node_shape_from = &node_shapes[edge.from];
                                            let node_shape_to = &node_shapes[edge.to];
                                            let pos2 = center + positions[edge.to].pos.to_vec2();
                                            let faded = !selected_related_nodes_pos.is_empty()
                                                && !(selected_related_nodes_pos.binary_search(&edge.from).is_ok()
                                                    && selected_related_nodes_pos.binary_search(&edge.to).is_ok());
                                            drawing::draw_edge(
                                                painter,
                                                pos1,
                                                node_shape_from.size,
                                                node_shape_from.node_shape,
                                                pos2,
                                                node_shape_to.size,
                                                node_shape_to.node_shape,
                                                self.visualization_style
                                                    .get_edge_syle(edge.predicate, ui.visuals().dark_mode),
                                                node_label,
                                                faded,
                                                edge.bezier_distance,
                                                ui.visuals(),
                                            );
                                        } else {
                                            let faded = !selected_related_nodes_pos.is_empty()
                                                && selected_related_nodes_pos.binary_search(&edge.from).is_err();
                                            let node_shape_from = &node_shapes[edge.from];
                                            drawing::draw_self_edge(
                                                painter,
                                                pos1,
                                                node_shape_from.size,
                                                edge.bezier_distance,
                                                node_shape_from.node_shape,
                                                self.visualization_style
                                                    .get_edge_syle(edge.predicate, ui.visuals().dark_mode),
                                                faded,
                                                node_label,
                                                ui.visuals(),
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(node_to_drag_index) = &self.ui_state.node_to_drag {
                    if let Some(node_pos) = self.visible_nodes.get_pos(*node_to_drag_index) {
                        if let Ok(mut positions) = self.visible_nodes.positions.write() {
                            positions[node_pos].pos =
                                (mouse_pos - center - self.ui_state.drag_diff.to_vec2()).to_pos2();
                        }
                    }
                }
                if let Ok(positions) = self.visible_nodes.positions.read() {
                    if let Ok(nodes) = self.visible_nodes.nodes.read() {
                        if let Ok(individual_node_style) = self.visible_nodes.individual_node_styles.read() {
                            let mut next_node_selection = if !matches!(node_selection_move, NodeSelectionMove::None) {
                                if let Some(selected_index) = self.ui_state.selected_node {
                                    if let Ok(pos) = nodes.binary_search_by(|e| e.node_index.cmp(&selected_index)) {
                                        NextNodeSelection::new(selected_index, positions[pos].pos, node_selection_move)
                                    } else {
                                        // The selected node is not visible anymore just pick the first
                                        if let Some(first_node) = nodes.first() {
                                            NextNodeSelection::new(
                                                first_node.node_index,
                                                positions[0].pos,
                                                node_selection_move,
                                            )
                                        } else {
                                            NextNodeSelection::empty()
                                        }
                                    }
                                } else {
                                    if let Some(first_node) = nodes.first() {
                                        NextNodeSelection::new(
                                            first_node.node_index,
                                            positions[0].pos,
                                            node_selection_move,
                                        )
                                    } else {
                                        NextNodeSelection::empty()
                                    }
                                }
                            } else {
                                NextNodeSelection::empty()
                            };
                            let mut new_node_shapes: Option<Vec<NodeShapeData>> =
                                if self.visible_nodes.update_node_shapes {
                                    Some(Vec::with_capacity(nodes.len()))
                                } else {
                                    None
                                };
                            for ((node_pos, node_layout), node_position) in
                                nodes.iter().enumerate().zip(positions.iter())
                            {
                                next_node_selection.consider_node(node_layout.node_index, node_position.pos);
                                if let Some((object_iri, object)) =
                                    rdf_data.node_data.get_node_by_index(node_layout.node_index)
                                {
                                    if self.visible_nodes.has_semantic_zoom && !self.visible_nodes.update_node_shapes {
                                        if !individual_node_style[node_pos]
                                            .semantic_zoom_interval
                                            .is_visible(self.ui_state.semantic_zoom_magnitude)
                                        {
                                            continue;
                                        }
                                    }
                                    let pos = center + node_position.pos.to_vec2();
                                    let faded = !selected_related_nodes_pos.is_empty()
                                        && selected_related_nodes_pos.binary_search(&node_pos).is_err();
                                    let (node_rect, node_shape) = draw_node(
                                        &self.visualization_style,
                                        individual_node_style.get(node_pos),
                                        &rdf_data.node_data.indexers,
                                        &self.ui_state,
                                        &self.persistent_data.config_data,
                                        painter,
                                        object,
                                        object_iri,
                                        pos,
                                        self.ui_state.selected_node == Some(node_layout.node_index),
                                        false,
                                        faded,
                                        ui.visuals(),
                                    );
                                    // TODO Error can not refresh nodes_shapes if semantic zoom is enabled.
                                    // because it will produce too few of them
                                    // quick fix always compute new shapes if semantic zoom is 0
                                    if let Some(new_node_shapes) = &mut new_node_shapes {
                                        new_node_shapes.push(NodeShapeData {
                                            node_shape,
                                            size: node_rect.size(),
                                            ..Default::default()
                                        });
                                    }
                                    if self.ui_state.context_menu_node.is_none() || was_action {
                                        if single_clicked && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                            self.ui_state.selected_node = Some(node_layout.node_index);
                                            was_action = true;
                                        }
                                        if primary_down && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                            self.ui_state.node_to_drag = Some(node_layout.node_index);
                                            self.ui_state.drag_diff = (mouse_pos - node_rect.center()).to_pos2();
                                            was_action = true;
                                        }
                                        if double_clicked && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                            node_to_click = Some(node_layout.node_index);
                                            was_action = true;
                                        }
                                        if secondary_clicked && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                            was_context_click = true;
                                            self.ui_state.context_menu_opened_by_keyboard = false;
                                            self.ui_state.context_menu_pos = global_mouse_pos;
                                            self.ui_state.context_menu_node = Some(node_layout.node_index);
                                            was_action = true;
                                        }
                                    }
                                    if !was_action && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                        node_to_hover = Some(node_layout.node_index);
                                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                                    }
                                    node_count += 1;
                                }
                            }
                            if let Some(new_node_shapes) = new_node_shapes {
                                if let Ok(mut node_shapes) = self.visible_nodes.node_shapes.write() {
                                    *node_shapes = new_node_shapes;
                                    self.visible_nodes.update_node_shapes = false;
                                }
                            }
                            if let Some((next_selected, _pos)) = next_node_selection.next_selected {
                                self.ui_state.selected_node = Some(next_selected);
                            }
                        }
                    }
                }
                /* TODO Unselect node but only in clicked in the graph area
                if primary_down && !was_action {
                    self.ui_state.selected_node = None;
                }
                */
                if let Some(node_to_hover) = node_to_hover {
                    let node_layout = self.visible_nodes.get_pos(node_to_hover);
                    if let Some(node_pos) = node_layout {
                        if let Some((object_iri, object)) = rdf_data.node_data.get_node_by_index(node_to_hover) {
                            if let Ok(positions) = self.visible_nodes.positions.read() {
                                let pos = center + positions[node_pos].pos.to_vec2();
                                draw_node(
                                    &self.visualization_style,
                                    None,
                                    &rdf_data.node_data.indexers,
                                    &self.ui_state,
                                    &self.persistent_data.config_data,
                                    painter,
                                    object,
                                    object_iri,
                                    pos,
                                    self.ui_state.selected_node == Some(node_to_hover),
                                    true,
                                    false,
                                    ui.visuals(),
                                );
                            }
                        }
                    }
                }

                let consume_events = was_action || self.ui_state.node_to_drag.is_some() || node_to_hover.is_some();
                if consume_events {
                    // ui.max_rect does not give enough
                    // so create a very big rect that capture all ares in scene
                    let max_rect: Rect =
                        Rect::from_min_max(Pos2::new(-5_000.0, -5_000.0), Pos2::new(10_000.0, 10_000.0));
                    let _response = ui.interact(max_rect, id, Sense::click_and_drag());
                }
            });
        }
        if was_context_click {
            ui.memory_mut(|mem| mem.toggle_popup(popup_id));
        }
        let mut node_action: NodeContextAction = NodeContextAction::None;
        popup_at(ui, popup_id, self.ui_state.context_menu_pos, 200.0, |ui| {
            if let Some(_node_index) = &self.ui_state.context_menu_node {
                let has_zoom = self.visible_nodes.has_semantic_zoom && self.ui_state.semantic_zoom_magnitude > 1;
                node_action = NodeContextAction::show_menu(ui, self.ui_state.context_menu_opened_by_keyboard, has_zoom);
                self.ui_state.context_menu_opened_by_keyboard = false;
                if !matches!(node_action, NodeContextAction::None) {
                    ui.memory_mut(|mem| mem.close_popup());
                }
            } else {
                ui.label("no node selected");
            }
        });

        if let Some(current_index) = self.ui_state.selected_node {
            if matches!(node_action, NodeContextAction::None) {
                ui.input(|i| {
                    if i.key_pressed(Key::E) {
                        node_action = NodeContextAction::Expand;
                        self.ui_state.context_menu_node = Some(current_index);
                    } else if i.key_pressed(Key::H) {
                        node_action = NodeContextAction::Hide;
                        self.ui_state.context_menu_node = Some(current_index);
                    } else if i.key_pressed(Key::O) {
                        node_action = NodeContextAction::HideOther;
                        self.ui_state.context_menu_node = Some(current_index);
                    } else if i.key_pressed(Key::T) {
                        node_action = NodeContextAction::HideThisType;
                        self.ui_state.context_menu_node = Some(current_index);
                    } else if i.key_pressed(Key::Enter) {
                        *node_browse = NodeAction::BrowseNode(current_index);
                        self.ui_state.context_menu_node = Some(current_index);
                    }
                });
            }
        }
        if let Some(current_index) = self.ui_state.context_menu_node {
            if !matches!(node_action, NodeContextAction::None) {
                if let Ok(mut rdf_data) = self.rdf_data.write() {
                    let current_node = rdf_data.node_data.get_node_by_index_mut(current_index);
                    let mut check_selection = false;
                    if let Some((_, current_node)) = current_node {
                        match node_action {
                            NodeContextAction::Hide => {
                                self.visible_nodes
                                    .remove(current_index, &self.ui_state.hidden_predicates);
                                self.visible_nodes
                                    .start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                                check_selection = true;
                            }
                            NodeContextAction::HideThisType => {
                                let types = current_node.highest_priority_types(&self.visualization_style);
                                if !types.is_empty() {
                                    self.visible_nodes.retain(&self.ui_state.hidden_predicates, false, |x| {
                                        let node = rdf_data.node_data.get_node_by_index(x.node_index);
                                        if let Some((_, node)) = node {
                                            !node.has_same_type(&types)
                                        } else {
                                            true
                                        }
                                    });
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                                check_selection = true;
                            }
                            NodeContextAction::HideOther => {
                                self.visible_nodes.retain(&self.ui_state.hidden_predicates, false, |x| {
                                    x.node_index == current_index
                                });
                                self.visible_nodes
                                    .start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                                check_selection = true;
                            }
                            NodeContextAction::HideOtherTypes => {
                                let types = current_node.highest_priority_types(&self.visualization_style);
                                if !types.is_empty() {
                                    self.visible_nodes.retain(&self.ui_state.hidden_predicates, false, |x| {
                                        let node = rdf_data.node_data.get_node_by_index(x.node_index);
                                        if let Some((_, node)) = node {
                                            node.has_same_type(&types)
                                        } else {
                                            false
                                        }
                                    });
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                                check_selection = true;
                            }
                            NodeContextAction::HideUnrelated => {
                                let was_change =
                                    self.visible_nodes.retain(&self.ui_state.hidden_predicates, false, |x| {
                                        if x.node_index == current_index {
                                            return true;
                                        }
                                        current_node
                                            .references
                                            .iter()
                                            .any(|(_predicate_index, ref_iri)| *ref_iri == x.node_index)
                                            || current_node
                                                .reverse_references
                                                .iter()
                                                .any(|(_predicate_index, ref_iri)| *ref_iri == x.node_index)
                                    });
                                if was_change {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                                check_selection = true;
                            }
                            NodeContextAction::HideUnconnected => {
                                if self
                                    .visible_nodes
                                    .hide_unconnected(current_index, &self.ui_state.hidden_predicates)
                                {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                                check_selection = true;
                            }
                            NodeContextAction::HideOrphans => {
                                self.visible_nodes.hide_orphans(&self.ui_state.hidden_predicates);
                                check_selection = true;
                            }
                            NodeContextAction::HideRedundantEdges => {
                                self.visible_nodes
                                    .remove_redundant_edges(&self.ui_state.hidden_predicates);
                            }
                            NodeContextAction::HideZoomInvisible => {
                                if self.visible_nodes.has_semantic_zoom && self.ui_state.semantic_zoom_magnitude > 1 {
                                    let to_remove: Vec<usize> = if let Ok(individual_node_styles) = self.visible_nodes.individual_node_styles.read() {
                                        let to_remove: Vec<usize> = individual_node_styles.iter()
                                            .enumerate().filter_map(|(index,node_style)| 
                                            if !node_style.semantic_zoom_interval.is_visible(self.ui_state.semantic_zoom_magnitude) {
                                                Some(index)
                                            } else {
                                                None
                                            }
                                        ).collect();
                                        to_remove
                                    } else {
                                        Vec::new()
                                    };
                                    self.visible_nodes.remove_pos_list(&to_remove, &self.ui_state.hidden_predicates);
                                }
                            }
                            NodeContextAction::Expand => {
                                let mut node_change_context = NodeChangeContext {
                                    rdfwrap: &mut self.rdfwrap,
                                    visible_nodes: &mut self.visible_nodes,
                                    config: &self.persistent_data.config_data,
                                };
                                if rdf_data.expand_node(
                                    current_index,
                                    ExpandType::Both,
                                    &mut node_change_context,
                                    &self.ui_state.hidden_predicates,
                                ) {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                            }
                            NodeContextAction::ExpandReferenced => {
                                let mut node_change_context = NodeChangeContext {
                                    rdfwrap: &mut self.rdfwrap,
                                    visible_nodes: &mut self.visible_nodes,
                                    config: &self.persistent_data.config_data,
                                };
                                if rdf_data.expand_node(
                                    current_index,
                                    ExpandType::References,
                                    &mut node_change_context,
                                    &self.ui_state.hidden_predicates,
                                ) {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                                {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                            }
                            NodeContextAction::ExpandReferencedBy => {
                                let mut node_change_context = NodeChangeContext {
                                    rdfwrap: &mut self.rdfwrap,
                                    visible_nodes: &mut self.visible_nodes,
                                    config: &self.persistent_data.config_data,
                                };
                                if rdf_data.expand_node(
                                    current_index,
                                    ExpandType::ReverseReferences,
                                    &mut node_change_context,
                                    &self.ui_state.hidden_predicates,
                                ) {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                            }
                            NodeContextAction::ExpandThisType => {
                                let types = current_node.highest_priority_types(&self.visualization_style);
                                let mut node_change_context = NodeChangeContext {
                                    rdfwrap: &mut self.rdfwrap,
                                    visible_nodes: &mut self.visible_nodes,
                                    config: &self.persistent_data.config_data,
                                };
                                if rdf_data.expand_all_by_types(
                                    &types,
                                    &mut node_change_context,
                                    &self.ui_state.hidden_predicates,
                                ) {
                                    self.visible_nodes.start_layout(
                                        &self.persistent_data.config_data,
                                        &self.ui_state.hidden_predicates,
                                    );
                                }
                            }
                            NodeContextAction::HideThisTypePreserveEdges => {
                                let types = current_node.types.clone();
                                add_preserved_edges(
                                    &types,
                                    &mut self.visible_nodes,
                                    &rdf_data.node_data,
                                    &self.ui_state.hidden_predicates,
                                );
                                self.visible_nodes.retain(&self.ui_state.hidden_predicates, false, |x| {
                                    let node = rdf_data.node_data.get_node_by_index(x.node_index);
                                    if let Some((_, node)) = node {
                                        !node.has_same_type(&types)
                                    } else {
                                        true
                                    }
                                });
                                self.visible_nodes
                                    .start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                            }
                            NodeContextAction::ShowAllInstanceInTable => {
                                let types = current_node.highest_priority_types(&self.visualization_style);
                                if !types.is_empty() {
                                    if let Ok(nodes) = self.visible_nodes.nodes.read() {
                                        let h_type = types[0];
                                        let mut all_instances: Vec<IriIndex> = Vec::new();
                                        for node_layout in nodes.iter() {
                                            let node = rdf_data.node_data.get_node_by_index(node_layout.node_index);
                                            if let Some((_, node)) = node {
                                                if node.types.contains(&h_type) {
                                                    all_instances.push(node_layout.node_index);
                                                }
                                            }
                                        }
                                        *node_browse = NodeAction::ShowTypeInstances(h_type, all_instances);
                                    }
                                }
                            }
                            NodeContextAction::None => {
                                // do nothing
                            }
                        }
                        if check_selection {
                            if let Some(selected_node) = self.ui_state.selected_node {
                                if self.visible_nodes.get_pos(selected_node).is_none() {
                                    self.ui_state.selected_node = None;
                                }
                            }
                        }
                    }
                }
                self.ui_state.context_menu_node = None;
            }
        }
        if !was_context_click && (secondary_clicked || single_clicked) {
            if is_context_menu_open {
                self.ui_state.context_menu_node = None;
                ui.memory_mut(|mem| {
                    mem.close_popup();
                });
            };
        }

        if let Some(node_to_click) = node_to_click {
            if let Ok(mut rdf_data) = self.rdf_data.write() {
                let mut node_change_context = NodeChangeContext {
                    rdfwrap: &mut self.rdfwrap,
                    visible_nodes: &mut self.visible_nodes,
                    config: &self.persistent_data.config_data,
                };
                if rdf_data.expand_node(
                    node_to_click,
                    ExpandType::Both,
                    &mut node_change_context,
                    &self.ui_state.hidden_predicates,
                ) {
                    self.visible_nodes
                        .start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                }
            }
        }

        if let Ok(rdf_data) = self.rdf_data.read() {
            if let Some(node_to_hover) = node_to_hover {
                if let Some((hover_node_iri, hover_node)) = rdf_data.node_data.get_node_by_index(node_to_hover) {
                    self.status_message.clear();
                    self.status_message.push_str(hover_node.node_label(
                        hover_node_iri,
                        &self.visualization_style,
                        self.persistent_data.config_data.short_iri,
                        self.ui_state.display_language,
                        &rdf_data.node_data.indexers,
                    ));
                }
            } else if let Some(selected_node) = &self.ui_state.selected_node {
                self.status_message.clear();
                if let Some((selected_node_iri, selected_node)) = rdf_data.node_data.get_node_by_index(*selected_node) {
                    self.status_message.push_str(
                        format!(
                            "Nodes: {}, Edges: {} Selected: {}",
                            node_count,
                            edge_count,
                            selected_node.node_label(
                                selected_node_iri,
                                &self.visualization_style,
                                self.persistent_data.config_data.short_iri,
                                self.ui_state.display_language,
                                &rdf_data.node_data.indexers
                            )
                        )
                        .as_str(),
                    );
                }
            } else {
                self.status_message.clear();
                self.status_message
                    .push_str(format!("Nodes: {}, Edges: {}", node_count, edge_count).as_str());
            }
        }
    }
}

pub fn is_overlapping(node_rect: &Rect, pos: Pos2, node_shape: NodeShape) -> bool {
    if node_rect.contains(pos) {
        if node_shape == NodeShape::Circle || node_shape == NodeShape::None {
            let center = node_rect.center();
            let radius = node_rect.width() / 2.0;
            if (pos.x - center.x).powi(2) + (pos.y - center.y).powi(2) < radius.powi(2) {
                return true;
            }
        } else if node_shape == NodeShape::Elipse {
            let center = node_rect.center();
            let radius_x = node_rect.width() / 2.0;
            let radius_y = node_rect.height() / 2.0;
            if ((pos.x - center.x) / radius_x).powi(2) + ((pos.y - center.y) / radius_y).powi(2) < 1.0 {
                return true;
            }
        } else if node_shape == NodeShape::Rect {
            return true;
        }
        return false;
    }
    false
}

pub fn draw_node(
    visualization_style: &GVisualizationStyle,
    individual_node_style: Option<&IndividualNodeStyleData>,
    indexers: &Indexers,
    ui_state: &UIState,
    config: &Config,
    painter: &Painter,
    node_object: &NObject,
    object_iri: &str,
    pos: Pos2,
    selected: bool,
    highlighted: bool,
    faded: bool,
    visuals: &egui::Visuals,
) -> (Rect, NodeShape) {
    let node_type_style = visualization_style.get_type_style(&node_object.types);
    let type_style = if (visualization_style.use_size_overwrite || visualization_style.use_color_overwrite)
        && individual_node_style.is_some()
    {
        let individual_node_style = individual_node_style.unwrap();
        let overwrite_size = visualization_style.use_size_overwrite && !individual_node_style.size_overwrite.is_nan();
        &NodeStyle {
            color: if visualization_style.use_color_overwrite && individual_node_style.color_overwrite > 0 {
                let lightness = if visuals.dark_mode { 0.3 } else { 0.6 };
                next_distinct_color(individual_node_style.color_overwrite as usize - 1, 0.8, lightness, 200)
            } else {
                node_type_style.color
            },
            priority: 100,
            label_index: node_type_style.label_index,
            node_shape: if overwrite_size {
                NodeShape::Circle
            } else {
                node_type_style.node_shape
            },
            node_size: if overwrite_size {
                NodeSize::Fixed
            } else {
                node_type_style.node_size
            },
            width: if overwrite_size {
                individual_node_style.size_overwrite
            } else {
                node_type_style.width
            },
            height: node_type_style.height,
            border_width: node_type_style.border_width,
            border_color: node_type_style.border_color,
            corner_radius: node_type_style.corner_radius,
            max_lines: node_type_style.max_lines,
            label_position: node_type_style.label_position,
            label_max_width: node_type_style.label_max_width,
            font_size: node_type_style.font_size,
            label_color: node_type_style.label_color,
            icon_style: None,
        }
    } else {
        node_type_style
    };
    let node_label = node_object.node_label(
        object_iri,
        visualization_style,
        config.short_iri,
        ui_state.display_language,
        indexers,
    );
    draw_node_label(
        painter,
        node_label,
        type_style,
        pos,
        selected,
        highlighted,
        faded,
        ui_state.show_labels,
        visuals,
    )
}

pub fn update_layout_edges(
    new_nodes: &NeighborPos,
    layout_nodes: &mut SortedNodeLayout,
    node_data: &NodeData,
    hidden_predicates: &SortedVec,
) {
    let mut visited_nodes: HashSet<IriIndex> = HashSet::new();
    if let Ok(mut edges) = layout_nodes.edges.write() {
        for node_index in new_nodes.iter_values() {
            if let Some(node_pos) = layout_nodes.get_pos(*node_index) {
                if let Some((_str, nobject)) = node_data.get_node_by_index(*node_index) {
                    for (pred_index, ref_iri) in nobject.references.iter() {
                        if *ref_iri == *node_index {
                            let edge = Edge {
                                from: node_pos,
                                to: node_pos,
                                predicate: *pred_index,
                                bezier_distance: 0.0,
                            };
                            edges.push(edge);
                        } else if !visited_nodes.contains(ref_iri) {
                            if let Some(ref_pos) = layout_nodes.get_pos(*ref_iri) {
                                let edge = Edge {
                                    from: node_pos,
                                    to: ref_pos,
                                    predicate: *pred_index,
                                    bezier_distance: 0.0,
                                };
                                edges.push(edge);
                            }
                        }
                    }
                    for (pred_index, ref_iri) in nobject.reverse_references.iter() {
                        if *ref_iri != *node_index && !visited_nodes.contains(ref_iri) {
                            if let Some(ref_pos) = layout_nodes.get_pos(*ref_iri) {
                                let edge = Edge {
                                    from: ref_pos,
                                    to: node_pos,
                                    predicate: *pred_index,
                                    bezier_distance: 0.0,
                                };
                                edges.push(edge);
                            }
                        }
                    }
                }
            }
            visited_nodes.insert(*node_index);
        }
        /*
        println!("Layout edges: {}", layout_nodes.edges.len());
        for edge in layout_nodes.edges.iter_mut() {
            println!("Edge: {} -> {}", edge.from, edge.to);
        }
         */
        update_edges_groups(&mut edges, hidden_predicates);
    }
}

pub fn add_preserved_edges(
    hidden_types: &[IriIndex],
    layout_nodes: &mut SortedNodeLayout,
    node_data: &NodeData,
    hidden_predicates: &SortedVec,
) {
    if let Ok(mut edges) = layout_nodes.edges.write() {
        if let Ok(nodes) = layout_nodes.nodes.read() {
            let mut positions_to_preserve: Vec<usize> = Vec::new();
            for (pos, node_layout) in nodes.iter().enumerate() {
                let node = node_data.get_node_by_index(node_layout.node_index);
                if let Some((_iri, nobject)) = node {
                    if hidden_types.iter().any(|type_index| nobject.types.contains(type_index)) {
                        positions_to_preserve.push(pos)
                    }
                }
            }
            nodes
                .iter()
                .enumerate()
                .filter(|(_pos, node_layout)| {
                    let node = node_data.get_node_by_index(node_layout.node_index);
                    if let Some((_iri, nobject)) = node {
                        hidden_types.iter().any(|type_index| nobject.types.contains(type_index))
                    } else {
                        false
                    }
                })
                .map(|(pos, _node_layout)| pos)
                .for_each(|position_to_preserve| {
                    let mut preserved_edges: Vec<Edge> = Vec::new();
                    let pos_edges: Vec<&Edge> = edges
                        .iter()
                        .filter(|edge| edge.from == position_to_preserve || edge.to == position_to_preserve)
                        .collect();
                    let mut has_to = false;
                    for edge in pos_edges.iter() {
                        // We create preserved edges only if the are 2 edges that jump over the node
                        if edge.to == position_to_preserve {
                            has_to = true;
                            for edge2 in pos_edges.iter() {
                                if edge2.from == position_to_preserve && edge2.to != edge.from {
                                    if preserved_edges.iter().any(|e| {
                                        (e.from == edge.from && e.to == edge2.to)
                                            || (e.from == edge2.to && e.to == edge.from)
                                    }) {
                                        continue;
                                    }
                                    preserved_edges.push(Edge {
                                        from: edge.from,
                                        to: edge2.to,
                                        predicate: edge.predicate,
                                        bezier_distance: 0.0,
                                    });
                                }
                            }
                        }
                    }
                    if preserved_edges.is_empty() {
                        // No edges found. But create edges if to nodes point to or from the node for hide
                        if has_to {
                            for edge in pos_edges.iter() {
                                if edge.to == position_to_preserve {
                                    for edge2 in pos_edges.iter() {
                                        if edge2.to == position_to_preserve
                                            && edge2.from != edge.from
                                            && edge.from < edge2.from
                                        {
                                            if preserved_edges
                                                .iter()
                                                .any(|e| e.from == edge.from && e.to == edge2.from)
                                            {
                                                continue;
                                            }
                                            preserved_edges.push(Edge {
                                                from: edge.from,
                                                to: edge2.from,
                                                predicate: edge.predicate,
                                                bezier_distance: 0.0,
                                            });
                                        }
                                    }
                                }
                            }
                        } else {
                            for edge in pos_edges.iter() {
                                if edge.from == position_to_preserve {
                                    for edge2 in pos_edges.iter() {
                                        if edge2.from == position_to_preserve
                                            && edge2.to != edge.to
                                            && edge.to < edge2.to
                                        {
                                            if preserved_edges.iter().any(|e| e.from == edge.to && e.to == edge2.to) {
                                                continue;
                                            }
                                            preserved_edges.push(Edge {
                                                from: edge.to,
                                                to: edge2.to,
                                                predicate: edge.predicate,
                                                bezier_distance: 0.0,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    edges.extend(preserved_edges);
                    update_edges_groups(&mut edges, hidden_predicates);
                });
        }
    }
}

pub struct NeighborPos {
    nodes: HashMap<IriIndex, Vec<IriIndex>>,
}

impl Default for NeighborPos {
    fn default() -> Self {
        Self::new()
    }
}

impl NeighborPos {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    pub fn add_many(
        &mut self,
        node_layout: &mut SortedNodeLayout,
        nodes_to_add: &[(IriIndex, IriIndex)],
        config: &Config,
    ) -> bool {
        node_layout.add_many(nodes_to_add, config, |(parent_index, node_index)| {
            self.insert(*parent_index, *node_index);
        })
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn insert(&mut self, root_node: IriIndex, node_index: IriIndex) {
        if let Some(neighbours) = self.nodes.get_mut(&root_node) {
            neighbours.push(node_index);
        } else {
            self.nodes.insert(root_node, vec![node_index]);
        }
    }

    pub fn position(&self, node_layout: &mut SortedNodeLayout) {
        if let Ok(mut positions) = node_layout.positions.write() {
            for (root_node_index, neighbours) in self.nodes.iter() {
                let root_node = node_layout.get_pos(*root_node_index);
                if let Some(root_pos) = root_node {
                    let mut angle: f32 = rand::rng().random_range(0.0..std::f32::consts::TAU);
                    let angle_diff = std::f32::consts::TAU / neighbours.len() as f32;
                    let root_pos = positions[root_pos].pos;
                    for node_iri in neighbours.iter() {
                        let x = root_pos.x + INITIAL_DISTANCE * angle.cos();
                        let y = root_pos.y + INITIAL_DISTANCE * angle.sin();
                        if let Some(node_pos) = node_layout.get_pos(*node_iri) {
                            positions[node_pos].pos = Pos2::new(x, y);
                        }
                        angle += angle_diff;
                    }
                }
            }
        }
        self.create_undo(node_layout);
    }

    pub fn create_undo(&self, node_layout: &mut SortedNodeLayout) {
        let added_nodes: Vec<IriIndex> = self.iter_values().cloned().collect();
        if !added_nodes.is_empty() {
            let command = NodeCommand::AddElements(added_nodes);
            node_layout.undo_stack.push(command);
            node_layout.redo_stack.clear();
        }
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &IriIndex> {
        self.nodes
            .values() // -> Values<IriIndex, Vec<IriIndex>>
            .flat_map(|vec| vec.iter()) // -> Iterator over &IriIndex
    }
}
