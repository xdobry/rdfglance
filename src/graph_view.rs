use std::collections::HashMap;

use crate::{
    drawing, layout::{self, NodeLayout}, nobject::{IriIndex, Literal}, uitools::{popup_at, ColorBox}, ExpandType, NodeAction, VisualRdfApp,
};
use eframe::egui::{self, FontId, Pos2, Sense, Vec2};
use egui::Slider;

struct ReferencesState {
    pub count: u32,
    pub visible: u32,
}

impl VisualRdfApp {
    pub fn show_graph(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        let mut node_to_click: NodeAction = NodeAction::None;
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.layout_data.force_compute_layout, "Force Layout");
            if self.layout_data.compute_layout || self.layout_data.force_compute_layout {
                let max_move = layout::layout_graph(
                    &mut self.node_data,
                    &mut self.layout_data.visible_nodes,
                    &self.layout_data.hidden_predicates,
                    &self.persistent_data.config_data,
                );
                if max_move < 0.8 && !self.layout_data.force_compute_layout {
                    // println!("turn off layout");
                    self.layout_data.compute_layout = false;
                } 
                if self.layout_data.compute_layout || self.layout_data.force_compute_layout {
                    self.layout_data.compute_layout = true;
                    ctx.request_repaint();
                }
            }
            if ui.button("Expand all").clicked() {
                self.expand_all();
            }
            if ui.button("To Center").clicked() {
                self.layout_data.offset = Pos2::ZERO;
                self.layout_data.visible_nodes.to_center();
            }
            if ui.button("Show all").clicked() {
                self.show_all();
            }
            ui.checkbox(&mut self.show_properties, "Show Properties");
            ui.label("nodes force");
            ui.add(Slider::new(&mut self.persistent_data.config_data.repulsion_constant, 0.3..=8.0));
            ui.label("edges force");
            ui.add(Slider::new(&mut self.persistent_data.config_data.attraction_factor, 0.005..=0.2));
            ui.checkbox(&mut self.show_labels, "Show Labels");
            ui.checkbox(&mut self.short_iri, "Short Iri");
        });
        if self.show_properties {
            egui::SidePanel::right("right_panel")
                .default_width(200.0)
                .resizable(true)
                .min_width(100.0)
                .max_width(500.0)
                .show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        node_to_click = self.display_node_details(ui);
                    });
                });
            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    self.display_graph(ctx, ui);
                });
            });
        } else {
            self.display_graph(ctx, ui);
        }
        return node_to_click;
    }

    pub fn display_node_details(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut node_to_click = NodeAction::None;
        if let Some(iri) = &self.layout_data.selected_node {
            if self.layout_data.visible_nodes.contains(*iri) {
                let current_node = self.node_data.get_node_by_index(*iri);
                if let Some((current_node_iri,current_node)) = current_node {
                    if ui.link(current_node_iri).clicked() {
                        node_to_click = NodeAction::BrowseNode(*iri);
                    }
                    ui.horizontal(|ui| {
                        ui.strong("Types:");
                        for type_index in &current_node.types {
                            let type_label = self.node_data.type_display(
                                *type_index,
                                self.layout_data.display_language,
                                self.persistent_data.config_data.iri_display,
                                &self.prefix_manager,
                            );
                            if ui.button(type_label.as_str()).clicked() {
                                node_to_click = NodeAction::ShowType(*type_index);
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
                        for (predicate_index, prop_value) in &current_node.properties {
                            if self.persistent_data.config_data.supress_other_language_data {
                                if let Literal::LangString(lang, _) = prop_value {
                                    if *lang != self.layout_data.display_language {
                                        if *lang == 0 && self.layout_data.display_language != 0 {
                                            // it is fallback language so display if reall language could not be found
                                            let mut found = false;
                                            for (predicate_index2, prop_value2) in &current_node.properties {
                                                if predicate_index2 == predicate_index && prop_value2 != prop_value {
                                                    if let Literal::LangString(lang, _) = prop_value2 {
                                                        if *lang == self.layout_data.display_language {
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
                            let predicate_label = self.node_data.predicate_display(
                                *predicate_index,
                                self.layout_data.display_language,
                                self.persistent_data.config_data.iri_display,
                                &self.prefix_manager,
                            );
                            let lab_button = egui::Button::new(predicate_label.as_str());
                            let lab_button_response = ui.add(lab_button);
                            if lab_button_response.clicked() {
                                for node_type_index in current_node.types.iter() {
                                    self.color_cache
                                        .label_predicate
                                        .insert(*node_type_index, *predicate_index);
                                }
                            }
                            lab_button_response.on_hover_text("Set this property as label for the node type");
                            ui.label(prop_value.as_ref());
                            ui.end_row();
                        }
                        });
                    }
                    if !current_node.references.is_empty() {
                        ui.add_space(10.0);
                        ui.strong("References");
                        let mut reference_state: HashMap<IriIndex, ReferencesState> =
                            HashMap::new();
                        let mut references: Vec<IriIndex> = Vec::new();
                        for (predicate_index, ref_iri) in &current_node.references {
                            let is_visible = self.layout_data.visible_nodes.contains(*ref_iri);
                            if references.contains(predicate_index) {
                                let reference_state =
                                    reference_state.get_mut(predicate_index).unwrap();
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
                        for reference_index in references.iter() {
                            ui.horizontal(|ui| {
                                let reference_label = self.rdfwrap.iri2label(
                                    self.node_data.get_predicate(*reference_index).unwrap(),
                                );
                                if ui.button(reference_label).clicked() {
                                    self.layout_data.compute_layout = true;
                                    for (predicate_index, ref_iri) in &current_node.references {
                                        if predicate_index == reference_index {
                                            self.layout_data.visible_nodes.add(NodeLayout::new(*ref_iri));
                                        }
                                    }
                                }
                                ui.add(ColorBox::new(
                                    self.color_cache.get_predicate_color(*reference_index),
                                ));
                                if ui.button("➕").clicked() {
                                    self.layout_data.compute_layout = true;
                                    let mut nodes_to_add: Vec<IriIndex> = Vec::new();
                                    for visible_index in self.layout_data.visible_nodes.data.iter() {
                                        let visible_node =
                                            self.node_data.get_node_by_index(visible_index.node_index);
                                        if let Some((_v_node_iri,visible_node)) = visible_node {
                                            if visible_node.has_same_type(&current_node.types) {
                                                for (predicate_index, ref_iri) in
                                                    &visible_node.references
                                                {
                                                    if predicate_index == reference_index {
                                                        nodes_to_add.push(*ref_iri);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    for node_to_add in nodes_to_add.iter() {
                                        self.layout_data.visible_nodes.add(NodeLayout::new(*node_to_add));
                                    }
                                }
                                let reference_state = reference_state.get(reference_index).unwrap();
                                let state = format!(
                                    "{}/{}",
                                    reference_state.count, reference_state.visible
                                );
                                ui.label(state);
                                if self
                                    .layout_data
                                    .hidden_predicates
                                    .contains(*reference_index)
                                {
                                    if ui.button("👁").clicked() {
                                        self.layout_data.compute_layout = true;
                                        self.layout_data.hidden_predicates.remove(*reference_index);
                                    }
                                } else {
                                    if ui.button("❌").clicked() {
                                        self.layout_data.compute_layout = true;
                                        self.layout_data.hidden_predicates.add(*reference_index);
                                    }
                                }
                            });
                        }
                    }
                    if !current_node.reverse_references.is_empty() {
                        ui.add_space(10.0);
                        ui.strong("Referenced by");
                        let mut reference_state: HashMap<IriIndex, ReferencesState> =
                            HashMap::new();
                        let mut references: Vec<IriIndex> = Vec::new();
                        for (predicate_index, ref_iri) in &current_node.reverse_references {
                            let is_visible = self.layout_data.visible_nodes.contains(*ref_iri);
                            if references.contains(predicate_index) {
                                let reference_state =
                                    reference_state.get_mut(predicate_index).unwrap();
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
                        for reference_index in references.iter() {
                            ui.horizontal(|ui| {
                                let reference_label = self.rdfwrap.iri2label(
                                    self.node_data.get_predicate(*reference_index).unwrap(),
                                );
                                if ui.button(reference_label).clicked() {
                                    self.layout_data.compute_layout = true;
                                    for (predicate_index, ref_iri) in
                                        &current_node.reverse_references
                                    {
                                        if predicate_index == reference_index {
                                            self.layout_data.visible_nodes.add_by_index(*ref_iri);
                                        }
                                    }
                                }
                                ui.add(ColorBox::new(
                                    self.color_cache.get_predicate_color(*reference_index),
                                ));
                                if ui.button("➕").clicked() {
                                    self.layout_data.compute_layout = true;
                                    let mut nodes_to_add: Vec<IriIndex> = Vec::new();
                                    for node_layout in &self.layout_data.visible_nodes.data {
                                        let visible_node =
                                            self.node_data.get_node_by_index(node_layout.node_index);
                                        if let Some((_,visible_node)) = visible_node {
                                            if visible_node.has_same_type(&current_node.types) {
                                                for (predicate_index, ref_iri) in
                                                    &visible_node.reverse_references
                                                {
                                                    if predicate_index == reference_index {
                                                        nodes_to_add.push(*ref_iri);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    for node_to_add in nodes_to_add.iter() {
                                        self.layout_data.visible_nodes.add_by_index(*node_to_add);
                                    }
                                }
                                let reference_state = reference_state.get(reference_index).unwrap();
                                let state = format!(
                                    "{}/{}",
                                    reference_state.count, reference_state.visible
                                );
                                ui.label(state);
                                if self
                                    .layout_data
                                    .hidden_predicates
                                    .contains(*reference_index)
                                {
                                    if ui.button("👁").clicked() {
                                        self.layout_data.compute_layout = true;
                                        self.layout_data.hidden_predicates.remove(*reference_index);
                                    }
                                } else {
                                    if ui.button("❌").clicked() {
                                        self.layout_data.compute_layout = true;
                                        self.layout_data.hidden_predicates.add(*reference_index);
                                    }
                                }
                            });
                        }
                    }
                }
            }
        } else {
            ui.label("no node selected");
        }
        return node_to_click;
    }

    pub fn display_graph(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut node_count = 0;
        let mut edge_count = 0;
        let available_width = ui.available_width();
        let available_height = ui.available_height();
        let size = Vec2::new(available_width, available_height);
        let (rect, response) = ui.allocate_at_least(size, Sense::click_and_drag());
        let painter = ui.painter();

        let center = rect.center() + self.layout_data.offset.to_vec2();
        let font = FontId::proportional(16.0);
        let radius = 10.0;
        let popup_id = ui.make_persistent_id("node_context_menu");
        let (secondary_clicked, sec_mouse_pos) = {
            (
                response.secondary_clicked(),
                response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0)),
            )
        };
        for node_layout in self.layout_data.visible_nodes.data.iter() {
            if let Some((_,object)) = self.node_data.get_node_by_index(node_layout.node_index) {
                for (pred_index, ref_iri) in &object.references {
                    if !self.layout_data.hidden_predicates.contains(*pred_index) {
                        if self.layout_data.visible_nodes.contains(*ref_iri) {
                            if let Some(ref_object) = self.layout_data.visible_nodes.get(*ref_iri) {
                                let pos1 = center + node_layout.pos.to_vec2();
                                let pos2 = center + ref_object.pos.to_vec2();
                                drawing::draw_arrow_to_circle(
                                    painter,
                                    pos1,
                                    pos2,
                                    radius,
                                    self.color_cache.get_predicate_color(*pred_index),
                                );
                                edge_count += 1;
                            }
                        }
                    }
                }
            }
        }
        let (double_clicked, mouse_pos) = {
            (
                response.double_clicked(),
                response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0)),
            )
        };
        let (single_clicked, sc_mouse_pos) = {
            (
                response.clicked(),
                response.hover_pos().unwrap_or(Pos2::new(0.0, 0.0)),
            )
        };
        let hover_pos = response.hover_pos();
        let mut node_to_click: Option<IriIndex> = None;
        let mut node_to_hover: Option<IriIndex> = None;
        let drag_started = response.drag_started();
        if response.drag_stopped() {
            self.layout_data.node_to_drag = None;
            self.layout_data.offset_drag_start = None;
        }
        if response.dragged() && self.layout_data.offset_drag_start.is_some() {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                self.layout_data.offset = pointer_pos - self.layout_data.offset_drag_start.unwrap().to_vec2();
            }
        }
        if let Some(node_to_drag_index) = &self.layout_data.node_to_drag {
            if let Some(pointer_pos) = response.interact_pointer_pos() {
                if let Some(node_to_drag) =
                    self.layout_data.visible_nodes.get_mut(*node_to_drag_index)
                {
                    node_to_drag.pos = (pointer_pos - center).to_pos2();
                }
            }
        }
        let mut was_context_click = false;
        for node_layout in self.layout_data.visible_nodes.data.iter() {
            if let Some((object_iri,object)) = self.node_data.get_node_by_index(node_layout.node_index) {
                let pos = center + node_layout.pos.to_vec2();
                let mut was_action = false;
                if single_clicked {
                    if (pos - sc_mouse_pos).length() < radius {
                        self.layout_data.selected_node = Some(node_layout.node_index);
                        was_action = true;
                    }
                }
                if double_clicked {
                    if (pos - mouse_pos).length() < radius {
                        node_to_click = Some(node_layout.node_index);
                        was_action = true;
                    }
                }
                if secondary_clicked {
                    if (pos - sec_mouse_pos).length() < radius {
                        was_context_click = true;
                        self.layout_data.context_menu_pos = sec_mouse_pos;
                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                        self.layout_data.context_menu_node = Some(node_layout.node_index);
                        was_action = true;
                    }
                }
                if drag_started {
                    if let Some(pointer_pos) = response.interact_pointer_pos() {
                        if (pos - pointer_pos).length() < radius {
                            was_action = true;
                            self.layout_data.node_to_drag = Some(node_layout.node_index);
                        }
                    }
                }
                if !was_action
                    && hover_pos.is_some()
                    && (pos - hover_pos.unwrap()).length() < radius
                {
                    // just hover
                    node_to_hover = Some(node_layout.node_index);
                    ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                }
                if self.layout_data.selected_node.is_some()
                    && self.layout_data.selected_node.unwrap() == node_layout.node_index
                {
                    painter.circle_filled(pos, radius + 3.0, egui::Color32::YELLOW);
                }
                let type_color = self.color_cache.get_type_color(&object.types);
                painter.circle_filled(pos, radius, type_color);
                node_count += 1;
                if self.show_labels {
                    painter.text(
                        pos,
                        egui::Align2::CENTER_CENTER,
                        object.node_label(object_iri, &self.color_cache.label_predicate, self.short_iri, self.layout_data.display_language),
                        font.clone(),
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
                    );
                }
            }
        }
        if drag_started && self.layout_data.node_to_drag.is_none() &&  response.interact_pointer_pos().is_some() {
            self.layout_data.offset_drag_start = Some(response.interact_pointer_pos().unwrap() - self.layout_data.offset.to_vec2());
        }
        if !was_context_click && (secondary_clicked || single_clicked) {
            self.layout_data.context_menu_node = None;
            ui.memory_mut(|mem| mem.close_popup());
        }
        popup_at(ui, popup_id, self.layout_data.context_menu_pos, 200.0, |ui| {
            if let Some(node_index) = &self.layout_data.context_menu_node {
                let current_node = self.node_data.get_node_by_index_mut(*node_index);
                if let Some((_,current_node)) = current_node {
                    let mut close_menu = false;
                    let current_index = *node_index;
                    // TODO need to clone the types to release the mutable borrow from current_node (node_data)
                    let types = current_node.types.clone();
                    if ui.button("Hide").clicked() {
                        self.layout_data.visible_nodes.remove(current_index);
                        self.layout_data.compute_layout = true;
                        close_menu = true;
                    }
                    if ui.button("Hide this Type").clicked() {
                        self.layout_data.compute_layout = true;
                        self.layout_data.visible_nodes.remove(current_index);
                        self.layout_data.visible_nodes.data.retain(|x| {
                            let node = self.node_data.get_node_by_index(x.node_index);
                            if let Some((_,node)) = node {
                                !node.has_same_type(&types)
                            } else {
                                true
                            }
                        });
                        close_menu = true;
                    }
                    if ui.button("Hide other").clicked() {
                        self.layout_data.compute_layout = true;
                        self.layout_data.visible_nodes.data.clear();
                        self.layout_data.visible_nodes.add_by_index(current_index);
                        close_menu = true;
                    }
                    if ui.button("Hide other Types").clicked() {
                        self.layout_data.compute_layout = true;
                        self.layout_data.visible_nodes.data.retain(|x| {
                            let node = self.node_data.get_node_by_index(x.node_index);
                            if let Some((_,node)) = node {
                                node.has_same_type(&types)
                            } else {
                                false
                            }
                        });
                        close_menu = true;
                    }
                    if ui.button("Expand").clicked() {
                        self.expand_node(current_index, ExpandType::Both);
                        close_menu = true;
                    }
                    if ui.button("Expand Referenced").clicked() {
                        self.expand_node(current_index, ExpandType::References);
                        close_menu = true;
                    }
                    if ui.button("Expand Referenced By").clicked() {
                        self.expand_node(current_index, ExpandType::ReverseReferences);
                        close_menu = true;
                    }
                    if close_menu {
                        self.layout_data.context_menu_node = None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            } else {
                ui.label("no node selected");
            }
        });
        if let Some(node_to_click) = node_to_click {
            self.expand_node(node_to_click, ExpandType::Both);
        }
        if let Some(node_to_hover) = node_to_hover {
            if let Some((hover_node_iri,hover_node)) = self.node_data.get_node_by_index(node_to_hover) {
                self.status_message.clear();
                self.status_message.push_str(hover_node.node_label(hover_node_iri,&self.color_cache.label_predicate, self.short_iri, self.layout_data.display_language));
            }
        } else {
            if let Some(selected_node) = &self.layout_data.selected_node {
                self.status_message.clear();
                if let Some((selected_node_iri,selected_node)) = self.node_data.get_node_by_index(*selected_node) {
                    self.status_message.push_str(format!(
                        "Nodes: {}, Edges: {} Selected: {}",
                        node_count, edge_count, selected_node.node_label(selected_node_iri,&self.color_cache.label_predicate, self.short_iri, self.layout_data.display_language)
                    ).as_str());
                }
            } else {
                self.status_message.clear();
                self.status_message.push_str(format!("Nodes: {}, Edges: {}", node_count, edge_count).as_str());
            }
        }
    }
}
