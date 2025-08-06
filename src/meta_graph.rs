use std::sync::{Arc, RwLock};

use egui::{Color32, Pos2, Rect, Sense, Slider, Vec2};

use crate::{
    drawing::{self, draw_node_label}, graph_styles::{EdgeFont, EdgeStyle, NodeStyle}, graph_view::is_overlapping, layout::{update_edges_groups, Edge, LayoutConfUpdate, NodeShapeData, SortedNodeLayout}, nobject::{IriIndex, LabelContext}, table_view::TypeInstanceIndex, uitools::popup_at, NodeAction, RdfGlanceApp, SortedVec
};

const NODE_RMIN: f32 = 4.0;
const NODE_RMAX: f32 = 80.0;

impl RdfGlanceApp {
    pub fn show_meta_graph(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> NodeAction {
        let mut node_action = NodeAction::None;

        ui.horizontal(|ui| {
            if ui.button("Rebuild Meta Graph").clicked() {
                self.build_meta_graph();
            }
            self.meta_nodes
                .show_handle_layout_ui(ctx, ui, &self.persistent_data.config_data);
            if ui.checkbox(&mut self.ui_state.meta_count_to_size, "Instance Count as Size").clicked() {
                self.meta_nodes.update_node_shapes = true;
            }
            ui.label("nodes force");
            let response = ui.add(Slider::new(
                &mut self.persistent_data.config_data.m_repulsion_constant,
                0.1..=8.0,
            ));
            if response.changed() {
                if let Some(layout_handle) = &self.meta_nodes.layout_handle {
                    let _ = layout_handle.update_sender.send(LayoutConfUpdate::UpdateRepulsionConstant(
                        self.persistent_data.config_data.m_repulsion_constant));
                }
            }
            ui.label("edges force");
            let response = ui.add(Slider::new(
                &mut self.persistent_data.config_data.m_attraction_factor,
                0.02..=3.0,
            ));
            if response.changed() {
                if let Some(layout_handle) = &self.meta_nodes.layout_handle {
                    let _ = layout_handle.update_sender.send(LayoutConfUpdate::UpdateAttractionFactor(
                        self.persistent_data.config_data.m_attraction_factor));
                }
            }
        });

        egui::SidePanel::right("right_panel")
            .exact_width(500.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    let detail_node_action = self.display_type_node_details(ui);
                    if matches!(node_action, NodeAction::None) {
                        node_action = detail_node_action;
                    }
                });
            });
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let mut node_count = 0;
            let mut edge_count = 0;
            let mut was_context_click = false;
            let mut secondary_clicked = false;
            let mut single_clicked = false;
            let mut double_clicked = false;
            let mut primary_down = false;
            // let mut was_context_click = false;
            let mut node_to_click: Option<IriIndex> = None;
            let mut node_to_hover: Option<IriIndex> = None;
            let mut was_action = false;

            let global_mouse_pos = ctx.pointer_hover_pos().unwrap_or(Pos2::new(0.0, 0.0));

            let scene = egui::Scene::new().zoom_range(0.3..=4.0);
            scene.show(ui, &mut self.meta_graph_state.scene_rect, |ui| {
                let available_width = ui.available_width();
                let available_height = ui.available_height();
                let size = Vec2::new(available_width, available_height);

                let (id, rect) = ui.allocate_space(size);
                let painter = ui.painter();

                let center = rect.center();

                // The code is complicated because of event handling, especially for click and dragging
                // If node is clicked/dragged the event sould not be probagated to scene layer
                // so we need to handle events manually by input and if input are consumed
                // after it create big interact area that consume all events

                let transform = ctx.layer_transform_to_global(ui.layer_id());
                let mouse_pos = if let Some(transform) = transform {
                    let local_mouse_pos = ctx.pointer_hover_pos().unwrap_or(Pos2::new(0.0, 0.0));
                    transform.inverse() * local_mouse_pos
                } else {
                    Pos2::new(0.0, 0.0)
                };
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
                });
                if let Ok(rdf_data) = self.rdf_data.read() {
                    let label_context = LabelContext::new(
                        self.ui_state.display_language,
                        self.persistent_data.config_data.iri_display,
                        &rdf_data.prefix_manager,
                    );
                    let mut edge_style: EdgeStyle = EdgeStyle {
                        edge_font: Some(EdgeFont {
                            font_size: 14.0,
                            font_color: Color32::BLACK,
                        }),
                        ..EdgeStyle::default()
                    };
                    edge_count += self.meta_nodes.edges.read().unwrap().len();
                    if let Some(node_to_drag_index) = &self.ui_state.node_to_drag {
                        if let Some(node_pos) = self.meta_nodes.get_pos(*node_to_drag_index) {
                            if let Ok(mut positions) = self.meta_nodes.positions.write() {
                                positions[node_pos].pos =
                                    (mouse_pos - center - self.ui_state.drag_diff.to_vec2()).to_pos2();
                            }
                        }
                    }
                    if let Ok(positions) = self.meta_nodes.positions.read() {
                        if let Ok(edges) = self.meta_nodes.edges.read() {
                            if let Ok(node_shapes) = self.meta_nodes.node_shapes.read() {
                                for edge in edges.iter() {
                                    let node_label = || {
                                        let reference_label = rdf_data.node_data.predicate_display(
                                            edge.predicate,
                                            &label_context,
                                            &rdf_data.node_data.indexers,
                                        );
                                        reference_label.as_str().to_owned()
                                    };
                                    let pos1 = center + positions[edge.from].pos.to_vec2();
                                    let p_edge_style = self.visualisation_style.get_edge_syle(edge.predicate, ui.visuals().dark_mode);
                                    edge_style.color = p_edge_style.color;
                                    if edge.from != edge.to {
                                        let node_shape_from = &node_shapes[edge.from];
                                        let node_shape_to = &node_shapes[edge.to];
                                        let pos2 = center + positions[edge.to].pos.to_vec2();
                                        drawing::draw_edge(
                                            painter,
                                            pos1,
                                            node_shape_from.size,
                                            node_shape_from.node_shape,
                                            pos2,
                                            node_shape_to.size,
                                            node_shape_to.node_shape,
                                            &edge_style,
                                            node_label,
                                            false,
                                            edge.bezier_distance,
                                            ui.visuals()
                                        );
                                    } else {
                                        let node_shape_from = &node_shapes[edge.from];
                                        drawing::draw_self_edge(
                                            painter,
                                            pos1,
                                            node_shape_from.size,
                                            edge.bezier_distance,
                                            node_shape_from.node_shape,
                                            &edge_style,
                                            false,
                                            node_label,
                                            ui.visuals(),
                                        );
                                    }
                                }
                            }
                        }
                        let mut node_style: NodeStyle = NodeStyle::default();
                        if let Ok(nodes) = self.meta_nodes.nodes.read() {
                            let mut new_node_shapes: Option<Vec<NodeShapeData>> = if self.meta_nodes.update_node_shapes {
                                Some(Vec::with_capacity(nodes.len()))
                            } else {
                                None
                            };
                            for (node_layout, node_position) in nodes.iter().zip(positions.iter()) {
                                let pos = center + node_position.pos.to_vec2();
                                let type_style = self.visualisation_style.get_type_style_one(node_layout.node_index);
                                node_style.color = type_style.color;
                                if self.ui_state.meta_count_to_size
                                    && self.type_index.min_instance_type_count < self.type_index.max_instance_type_count
                                {
                                    let type_data = self.type_index.types.get(&node_layout.node_index);
                                    if let Some(type_data) = type_data {
                                        node_style.width = value_to_radius(
                                            type_data.instances.len(),
                                            self.type_index.min_instance_type_count,
                                            self.type_index.max_instance_type_count,
                                            NODE_RMIN,
                                            NODE_RMAX,
                                        );
                                    }
                                } else {
                                    node_style.width = 15.0;
                                }
                                let type_display = rdf_data.node_data.type_display(
                                    node_layout.node_index,
                                    &label_context,
                                    &rdf_data.node_data.indexers,
                                );
                                let (node_rect, node_shape) = draw_node_label(
                                    painter,
                                    type_display.as_str(),
                                    &node_style,
                                    pos,
                                    self.ui_state.selected_node == Some(node_layout.node_index),
                                    false,
                                    false,
                                    true,
                                    ui.visuals()
                                );
                                if let Some(new_node_shapes) = &mut new_node_shapes {
                                    new_node_shapes.push(NodeShapeData {
                                        node_shape,
                                        size: node_rect.size(),
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
                                        self.ui_state.context_menu_pos = global_mouse_pos;
                                        self.ui_state.context_menu_node = Some(node_layout.node_index);
                                        was_action = true;
                                    }
                                    if !was_action && is_overlapping(&node_rect, mouse_pos, node_shape) {
                                        node_to_hover = Some(node_layout.node_index);
                                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
                                    }
                                }
                                node_count += 1;
                            }
                            if let Some(new_node_shapes) = new_node_shapes {
                                if let Ok(mut node_shapes) = self.meta_nodes.node_shapes.write() {
                                    *node_shapes = new_node_shapes;
                                    self.meta_nodes.update_node_shapes = false;
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
    
                    let popup_id = ui.make_persistent_id("mnode_context_menu");
                    if was_context_click {
                        ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                    }
    
                    popup_at(ui, popup_id, self.ui_state.context_menu_pos, 200.0, |ui| {
                        if let Some(node_index) = &self.ui_state.context_menu_node {
                            let mut close_menu = false;
                            let current_index = *node_index;
                            let node_action = TypeNodeContextAction::show_menu(ui);
                            match node_action {
                                TypeNodeContextAction::Hide => {
                                    let hidden_predicates = SortedVec::new();
                                    self.meta_nodes.remove(current_index, &hidden_predicates);
                                    self.meta_nodes.start_layout(&self.persistent_data.config_data);
                                    close_menu = true;
                                }
                                TypeNodeContextAction::HideSameInstCount => {
                                    if let Some(current_node) = self.type_index.types.get(&current_index) {
                                        let to_remove = self
                                            .meta_nodes
                                            .nodes
                                            .read()
                                            .unwrap()
                                            .iter()
                                            .filter(|node| {
                                                if let Some(type_data) = self.type_index.types.get(&node.node_index) {
                                                    type_data.instances.len() <= current_node.instances.len()
                                                } else {
                                                    false
                                                }
                                            })
                                            .map(|node| node.node_index)
                                            .collect::<Vec<IriIndex>>();
                                        let hidden_predicates = SortedVec::new();
                                        for node in to_remove.iter() {
                                            self.meta_nodes.remove(*node, &hidden_predicates);
                                        }
                                    }
                                    close_menu = true;
                                }
                                TypeNodeContextAction::Expand => {
                                    if let Some(current_type_node) = self.type_index.types.get(&current_index) {
                                        node_to_click = Some(current_index);
                                    }
                                    close_menu = true;
                                },
                                TypeNodeContextAction::HideOthers => {
                                    self.meta_nodes.clear();
                                    self.meta_nodes.add_by_index(current_index);
                                    self.meta_nodes.start_layout(&self.persistent_data.config_data);
                                    close_menu = true;
                                }
                                TypeNodeContextAction::None => {
                                    // do nothing
                                }
                            }
                            if close_menu {
                                self.ui_state.context_menu_node = None;
                                ui.memory_mut(|mem| mem.close_popup());
                            }
                        } else {
                            ui.label("no node selected");
                        }
                    });

                    if let Some(node_to_click) = node_to_click {
                        if let Some(current_type_node) = self.type_index.types.get(&node_to_click) {
                            let mut was_add = false;
                            for reference_characteristics in current_type_node.references.values() {
                                for ref_type in reference_characteristics.types.iter() {
                                    if !self.meta_nodes.contains(*ref_type) {
                                        was_add = true;
                                        self.meta_nodes.add_by_index(*ref_type);
                                    }
                                }
                            }
                            for (type_index,type_node) in self.type_index.types.iter() {
                                if *type_index!=node_to_click && !self.meta_nodes.contains(*type_index) {
                                    if type_node.references.iter().any(|(_, ref_characteristics)| {
                                        ref_characteristics.types.contains(&node_to_click)
                                    }) {
                                        was_add = true;
                                        self.meta_nodes.add_by_index(*type_index);
                                    }
                                }
                            }
                            if was_add {
                                self.meta_nodes.edges = Arc::new(RwLock::new(create_types_layout_edges(
                                    &self.meta_nodes,
                                    &self.type_index,
                                )));                           
                                self.meta_nodes.start_layout(&self.persistent_data.config_data);
                            }
                        }
                    }

                    if !was_context_click && (secondary_clicked || single_clicked) {
                        self.ui_state.context_menu_node = None;
                        ui.memory_mut(|mem| mem.close_popup());
                    }
                }
            });
        });
        node_action
    }

    pub fn build_meta_graph(&mut self) {
        self.meta_nodes.clear();
        for (type_index, _type_node) in self.type_index.types.iter() {
            self.meta_nodes.add_by_index(*type_index);
        }
        self.meta_nodes.edges = Arc::new(RwLock::new(create_types_layout_edges(
            &self.meta_nodes,
            &self.type_index,
        )));
        self.meta_nodes.start_layout(&self.persistent_data.config_data);
    }

    pub fn display_type_node_details(&mut self, ui: &mut egui::Ui) -> NodeAction {
        let mut node_to_click = NodeAction::None;
        if let Some(iri_index) = &self.ui_state.selected_node {
            if self.meta_nodes.contains(*iri_index) {
                if let Some(type_data) = self.type_index.types.get(iri_index) {
                    if let Ok(rdf_data) = self.rdf_data.read() {
                        let label_context = LabelContext::new(
                            self.ui_state.display_language,
                            self.persistent_data.config_data.iri_display,
                            &rdf_data.prefix_manager,
                        );
                        let type_display =
                            rdf_data.node_data
                                .type_display(*iri_index, &label_context, &rdf_data.node_data.indexers);
                        if ui.button(type_display.as_str()).clicked() {
                            node_to_click = NodeAction::ShowType(*iri_index);
                        }
                        ui.label(format!("Instance count: {}", type_data.instances.len()));
                        ui.add_space(5.0);
                        type_data.display_data_props(ui, &label_context, &rdf_data.node_data);
                        ui.add_space(5.0);
                        type_data.display_references(ui, &label_context, &rdf_data.node_data);
                        ui.add_space(5.0);
                        type_data.display_reverse_references(ui, &label_context, &rdf_data.node_data);
                    };
                }
            }
        }
        node_to_click
    }
}

enum TypeNodeContextAction {
    None,
    Hide,
    HideSameInstCount,
    HideOthers,
    Expand,
}

impl TypeNodeContextAction {
    fn show_menu(ui: &mut egui::Ui) -> TypeNodeContextAction {
        if ui.button("Hide").clicked() {
            return TypeNodeContextAction::Hide;
        }
        if ui.button("Hide same instance count or Less").clicked() {
            return TypeNodeContextAction::HideSameInstCount;
        }
        if ui.button("Hide Others").clicked() {
            return TypeNodeContextAction::HideOthers;
        }
        if ui.button("Expand").clicked() {
            return TypeNodeContextAction::Expand;
        }
        TypeNodeContextAction::None
    }
}

/// Maps a value in [min, max] to a circle radius in [rmin, rmax],
/// such that the area of the resulting circle is proportional to the value.
fn value_to_radius(value: usize, min: usize, max: usize, rmin: f32, rmax: f32) -> f32 {
    // assert!(min < max, "min must be less than max");
    // assert!(rmin < rmax, "rmin must be less than rmax");
    // assert!(value >= min && value <= max, "value must be within [min, max]");

    // Normalize value to [0, 1]
    let norm = (value - min) as f32 / (max - min) as f32;

    // Compute area range
    let area_min = std::f32::consts::PI * rmin.powi(2);
    let area_max = std::f32::consts::PI * rmax.powi(2);

    // Map normalized value to area
    let area = area_min + norm * (area_max - area_min);

    // Convert area back to radius
    (area / std::f32::consts::PI).sqrt()
}

fn create_types_layout_edges(layout_nodes: &SortedNodeLayout, type_index: &TypeInstanceIndex) -> Vec<Edge> {
    let mut edges = Vec::new();
    for (node_pos, node_layout) in layout_nodes.nodes.read().unwrap().iter().enumerate() {
        if let Some(type_data) = type_index.types.get(&node_layout.node_index) {
            for (pred_index, ref_characteristics) in &type_data.references {
                for ref_type_iri in &ref_characteristics.types {
                    if *ref_type_iri == node_layout.node_index {
                        let edge = Edge {
                            from: node_pos,
                            to: node_pos,
                            predicate: *pred_index,
                            bezier_distance: 0.0,
                        };
                        edges.push(edge);
                    } else if let Some(ref_pos) = layout_nodes.get_pos(*ref_type_iri) {
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
        }
    }
    let hidden_predicates = SortedVec::new();
    update_edges_groups(&mut edges, &hidden_predicates);
    edges
}

#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    use crate::layout::{LayoutConfig, NodePosition, layout_graph_nodes};

    use super::*;

    #[test]
    fn test_meta_graph() -> std::io::Result<()> {
        let mut vs = RdfGlanceApp::new(None);
        vs.load_ttl("sample-rdf-data/programming_languages.ttl",true);
        vs.join_load(true);
        for (type_index, _type_node) in vs.type_index.types.iter() {
            vs.meta_nodes.add_by_index(*type_index);
        }
        assert!(vs.meta_nodes.nodes.read().unwrap().len() > 0);
        for (type_index, _type_node) in vs.type_index.types.iter() {
            vs.meta_nodes.add_by_index(*type_index);
        }

        let edges = create_types_layout_edges(&vs.meta_nodes, &vs.type_index);
        assert!(edges.len() > 0);
        vs.meta_nodes.edges = Arc::new(RwLock::new(edges));

        let mut positions: Vec<NodePosition> = Vec::with_capacity(vs.meta_nodes.nodes.read().unwrap().len());
        for _ in 0..vs.meta_nodes.nodes.read().unwrap().len() {
            positions.push(NodePosition::default());
        }
        vs.meta_nodes.positions = Arc::new(RwLock::new(positions));
        let layout_config = LayoutConfig {
            repulsion_constant: vs.persistent_data.config_data.m_repulsion_constant,
            attraction_factor: vs.persistent_data.config_data.m_attraction_factor,
        };
        let (max_move, positions) = layout_graph_nodes(
            &vs.meta_nodes.nodes.read().unwrap(),
            &vs.meta_nodes.node_shapes.read().unwrap(),
            &vs.meta_nodes.positions.read().unwrap(),
            &vs.meta_nodes.edges.read().unwrap(),
            &layout_config,
            100.0,
        );
        assert!(max_move > 0.0);
        assert!(positions.len() == vs.meta_nodes.nodes.read().unwrap().len());

        Ok(())
    }
}
