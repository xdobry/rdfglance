use crate::{
    GVisualizationStyle, SortedVec, UIState,
    config::Config,
    graph_algorithms::{GraphAlgorithm, run_algorithm, run_clustering_algorithm},
    graph_styles::NodeShape,
    nobject::IriIndex,
    quad_tree::{BHQuadtree, WeightedPoint},
    statistics::{StatisticsData, StatisticsResult, distribute_to_zoom_layers},
    style::{ICON_KEEP_TEMPERATURE, ICON_KEY, ICON_REFRESH, ICON_STOP},
};
use atomic_float::AtomicF32;
use eframe::egui::Vec2;
use egui::Pos2;
use fixedbitset::FixedBitSet;
use rand::Rng;
use rayon::prelude::*;
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Clone, Copy)]
pub struct NodeLayout {
    pub node_index: IriIndex,
}

impl NodeLayout {
    pub fn new(node_index: IriIndex) -> Self {
        Self { node_index }
    }
}

#[derive(Clone, Copy)]
pub struct NodeShapeData {
    pub size: Vec2,
    pub node_shape: NodeShape,
}

impl Default for NodeShapeData {
    fn default() -> Self {
        Self {
            node_shape: NodeShape::Circle,
            size: Vec2::new(10.0, 10.0),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct LayerInterval {
    // 0 is open interval, so 0 has special meaning!
    // layers are build 1..10 1 is first layer with all nodes
    // semantic zoom is done 0..10 (0,1 - all nodes, 10 zoom out)
    // so the number can be interpreted as zoom magnitude (if more the magnitude then less details (nodes) you see)
    // from and to are used because there could be nodes that are visible on in one layer (for example node that represent a cluster)
    // 10..10 can be see only in magnitude 10 but not 9
    // 0..10 can be seen in all magnitudes
    // 0..5 can be see in magnitudes 0,1,2,3,4,5
    pub from: u8,
    pub to: u8,
}

impl LayerInterval {
    pub fn new(from: u8, to: u8) -> Self {
        Self { from, to }
    }

    pub fn is_visible(&self, layer: u8) -> bool {
        if (self.from == 0 && self.to == 0) || layer == 0 {
            return true; // always visible
        }
        if self.from == 0 {
            return layer <= self.to; // visible in all layers up to to
        }
        if self.to == 0 {
            return layer >= self.from; // visible in all layers from from
        }
        layer >= self.from && layer <= self.to // visible in range from to
    }

    // map 0.0..=1.0 f32 normalized value to 1..10
    // by linear mapping
    pub fn set_from_normalized(&mut self, value: f32) {
        let clamped = value.clamp(0.0, 1.0); // ensure input is in [0, 1]
        let scaled = clamped * 9.0 + 1.0; // 0→1, 1→10
        self.to = scaled.round() as u8;
        self.from = 0;
    }
    pub fn set_from_layout(&mut self, layout_layer: u8) {
        self.from = 0;
        self.to = layout_layer;
    }
}

#[derive(Clone, Copy)]
pub struct IndividualNodeStyleData {
    // Set from statistics to overwrite the size of the node
    pub size_overwrite: f32,
    // 0 means no overwrite
    pub color_overwrite: u16,
    pub semantic_zoom_interval: LayerInterval,
    pub hidden_references: u32,
}

impl Default for IndividualNodeStyleData {
    fn default() -> Self {
        Self {
            size_overwrite: f32::NAN,
            color_overwrite: 0,
            semantic_zoom_interval: LayerInterval::default(),
            hidden_references: 0,
        }
    }
}

impl IndividualNodeStyleData {
    pub fn set_size_value(&mut self, value: f32, visualization_style: &GVisualizationStyle) {
        let mapped_size: f32 =
            visualization_style.min_size + value * (visualization_style.max_size - visualization_style.min_size);
        self.size_overwrite = mapped_size;
        self.semantic_zoom_interval.set_from_normalized(value);
    }
    pub fn set_cluster(&mut self, cluster: u32) {
        self.color_overwrite = (cluster + 1) as u16;
    }
}

pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub predicate: IriIndex,
    pub bezier_distance: f32,
}

#[derive(Clone, Copy)]
pub struct NodePosition {
    pub pos: Pos2,
    pub vel: Vec2,
    pub locked: bool,
}

impl Default for NodePosition {
    fn default() -> Self {
        Self {
            pos: Pos2::new(
                rand::rng().random_range(-100.0..100.0),
                rand::rng().random_range(-100.0..100.0),
            ),
            vel: Vec2::new(0.0, 0.0),
            locked: false,
        }
    }
}

// Used to store efficiently all information needed to layout the graph
// nodes, edges, positions and node shapes
// It uses mostly indexes of nodes and provide methods to add and remove nodes
// All nodes are sorted by node index to enable fast check if node exists
// Edges stores indexes of nodes in nodes vector (not the primary node index)
// This enable fast access needed to layout the graph but make
// it hard to manipulate the structure for add and remove operation
pub struct SortedNodeLayout {
    pub nodes: Arc<RwLock<Vec<NodeLayout>>>,
    pub edges: Arc<RwLock<Vec<Edge>>>,
    pub positions: Arc<RwLock<Vec<NodePosition>>>,
    pub node_shapes: Arc<RwLock<Vec<NodeShapeData>>>,
    pub individual_node_styles: Arc<RwLock<Vec<IndividualNodeStyleData>>>,
    pub layout_temperature: f32,
    pub keep_temperature: Arc<AtomicBool>,
    pub layout_handle: Option<LayoutHandle>,
    pub background_layout_finished: Arc<AtomicBool>,
    pub stop_background_layout: Arc<AtomicBool>,
    pub update_node_shapes: bool,
    pub has_semantic_zoom: bool,
    pub compute_layout: bool,
    pub lock_layout: bool,
    pub data_epoch: u32,
    pub undo_stack: Vec<NodeCommand>,
    pub redo_stack: Vec<NodeCommand>,
}

#[derive(Debug)]
pub enum LayoutConfUpdate {
    UpdateRepulsionConstant(f32),
    UpdateAttractionFactor(f32),
}

pub struct LayoutHandle {
    pub join_handle: JoinHandle<()>,
    pub update_sender: mpsc::Sender<LayoutConfUpdate>,
}

/**
 * It protocols action that has been done on visual graph
 * It is not common command pattern because it can only undo the work
 */
pub enum NodeCommand {
    AddElements(Vec<IriIndex>),
    RemoveElements(Vec<NodeMemo>, Vec<EdgeMemo>),
}

pub struct NodeMemo {
    pub index: IriIndex,
    pub position: Pos2,
    pub hidden_references: u32,
}

pub struct EdgeMemo {
    pub from: usize,
    pub to: usize,
    pub predicate: IriIndex,
}

impl Default for SortedNodeLayout {
    fn default() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
            positions: Arc::new(RwLock::new(Vec::new())),
            edges: Arc::new(RwLock::new(Vec::new())),
            node_shapes: Arc::new(RwLock::new(Vec::new())),
            individual_node_styles: Arc::new(RwLock::new(Vec::new())),
            compute_layout: true,
            keep_temperature: Arc::new(AtomicBool::new(false)),
            layout_temperature: 0.5,
            layout_handle: None,
            background_layout_finished: Arc::new(AtomicBool::new(false)),
            stop_background_layout: Arc::new(AtomicBool::new(false)),
            update_node_shapes: true,
            has_semantic_zoom: false,
            data_epoch: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            lock_layout: false,
        }
    }
}

impl SortedNodeLayout {
    pub fn new() -> Self {
        Self::default()
    }

    // This is low level add operation that do not change undo stack
    pub fn add(&mut self, value: NodeLayout) -> bool {
        self.stop_layout();
        self.data_epoch += 1;
        if let Ok(mut nodes) = self.nodes.write() {
            if let Ok(mut node_shapes) = self.node_shapes.write() {
                if let Ok(mut positions) = self.positions.write() {
                    if let Ok(mut individual_node_styles) = self.individual_node_styles.write() {
                        if let Ok(mut edges) = self.edges.write() {
                            return match nodes.binary_search_by(|e| e.node_index.cmp(&value.node_index)) {
                                Ok(_) => false, // Value already exists, do nothing
                                Err(pos) => {
                                    // Insert at correct position
                                    nodes.insert(pos, value);
                                    positions.insert(pos, NodePosition::default());
                                    node_shapes.insert(pos, NodeShapeData::default());
                                    individual_node_styles.insert(pos, IndividualNodeStyleData::default());
                                    self.update_node_shapes = true;
                                    for i in 0..edges.len() {
                                        if edges[i].from >= pos {
                                            edges[i].from += 1;
                                        }
                                        if edges[i].to >= pos {
                                            edges[i].to += 1;
                                        }
                                    }
                                    true
                                }
                            };
                        }
                    }
                }
            }
        }
        false
    }

    // use add_many operation if possible. Do not call it in the loop
    pub fn add_by_index(&mut self, value: IriIndex) -> bool {
        let res = self.add(NodeLayout::new(value));
        if res {
            self.undo_stack.push(NodeCommand::AddElements(vec![value]));
            self.redo_stack.clear();
        }
        res
    }

    pub fn add_many(
        &mut self,
        values: &[(IriIndex, IriIndex)],
        config: &Config,
        inserted_callback: impl FnMut(&(IriIndex, IriIndex)),
    ) -> bool {
        self.stop_layout();
        let index_to_add = if let Ok(nodes) = self.nodes.read() {
            if nodes.len() >= config.max_visible_nodes {
                return false;
            }
            // First filter the list only for nodes that are not already in the layout
            // sort and dedup the nodes the parent node does not matter
            let mut index_to_add: Vec<(IriIndex, IriIndex)> = values
                .iter()
                .filter(|(_parent_index, node_index)| {
                    nodes.binary_search_by(|node| node.node_index.cmp(node_index)).is_err()
                })
                .map(|p| (p.0, p.1))
                .collect();
            index_to_add.sort_unstable_by(|a, b| a.1.cmp(&b.1));
            index_to_add.dedup_by(|a, b| a.1 == b.1);
            if index_to_add.len() + nodes.len() > config.max_visible_nodes {
                println!("Truncating nodes to add to visual graph for reaching the configured display limit");
                index_to_add.truncate(config.max_visible_nodes - nodes.len());
            }
            index_to_add.iter().for_each(inserted_callback);
            index_to_add
        } else {
            Vec::new()
        };
        if !index_to_add.is_empty() {
            if let Ok(mut nodes) = self.nodes.write() {
                if let Ok(mut node_shapes) = self.node_shapes.write() {
                    if let Ok(mut positions) = self.positions.write() {
                        if let Ok(mut individual_node_styles) = self.individual_node_styles.write() {
                            if let Ok(mut edges) = self.edges.write() {
                                // stores the new indexes for old nodes indexes, needed to update edges from and to fields
                                let mut new_positions: Vec<usize> = Vec::with_capacity(nodes.len());
                                for i in 0..nodes.len() {
                                    new_positions.push(i);
                                }
                                // we use in place merge on target vector. The vector is resized and the new elements
                                // are inserted from the end by iterating the new nodes and old nodes from the end.
                                let orig_len = nodes.len();
                                let b_len = index_to_add.len();

                                nodes.resize(orig_len + b_len, NodeLayout { node_index: 0 });
                                node_shapes.resize(orig_len + b_len, NodeShapeData::default());
                                positions.resize(orig_len + b_len, NodePosition::default());
                                individual_node_styles.resize(orig_len + b_len, IndividualNodeStyleData::default());

                                let mut i = orig_len as isize - 1;
                                let mut j = b_len as isize - 1;
                                let mut k = (orig_len + b_len) as isize - 1;

                                while j >= 0 {
                                    if i >= 0 && nodes[i as usize].node_index > index_to_add[j as usize].1 {
                                        nodes[k as usize] = nodes[i as usize];
                                        node_shapes[k as usize] = node_shapes[i as usize];
                                        positions[k as usize] = positions[i as usize];
                                        new_positions[i as usize] = k as usize;
                                        individual_node_styles[k as usize] = individual_node_styles[i as usize];
                                        i -= 1;
                                    } else {
                                        nodes[k as usize] = NodeLayout {
                                            node_index: index_to_add[j as usize].1,
                                        };
                                        node_shapes[k as usize] = NodeShapeData::default();
                                        positions[k as usize] = NodePosition::default();
                                        individual_node_styles[k as usize] = IndividualNodeStyleData::default();
                                        j -= 1;
                                    }
                                    k -= 1;
                                }

                                // now need to set new edge indexes to new ones
                                edges.par_iter_mut().for_each(|edge| {
                                    edge.from = new_positions[edge.from];
                                    edge.to = new_positions[edge.to];
                                });
                            }
                        }
                    }
                }
            }
            self.data_epoch += 1;
            true
        } else {
            false
        }
    }

    pub fn contains(&self, value: IriIndex) -> bool {
        self.get_pos(value).is_some()
    }

    pub fn mut_nodes<R>(
        &mut self,
        mutator: impl Fn(
            &mut Vec<NodeLayout>,
            &mut Vec<NodePosition>,
            &mut Vec<Edge>,
            &mut Vec<NodeShapeData>,
            &mut Vec<IndividualNodeStyleData>,
        ) -> R,
    ) -> Option<R> {
        self.stop_layout();
        if let Ok(mut nodes) = self.nodes.write() {
            if let Ok(mut positions) = self.positions.write() {
                if let Ok(mut edges) = self.edges.write() {
                    if let Ok(mut node_shapes) = self.node_shapes.write() {
                        if let Ok(mut individual_node_styles) = self.individual_node_styles.write() {
                            return Some(mutator(
                                &mut nodes,
                                &mut positions,
                                &mut edges,
                                &mut node_shapes,
                                &mut individual_node_styles,
                            ));
                        }
                    }
                }
            }
        }
        None
    }

    pub fn remove(&mut self, value: IriIndex, hidden_predicates: &SortedVec) {
        self.retain(hidden_predicates, false, |n| n.node_index != value);
    }

    pub fn clean_all(&mut self) {
        self.stop_layout();
        self.data_epoch += 1;
        self.mut_nodes(|nodes, positions, edges, node_shapes, individual_node_styles| {
            positions.clear();
            nodes.clear();
            edges.clear();
            node_shapes.clear();
            individual_node_styles.clear();
        });
        self.redo_stack.clear();
        self.undo_stack.clear();
    }

    pub fn retain(&mut self, hidden_predicates: &SortedVec, is_undo: bool, f: impl Fn(&NodeLayout) -> bool) -> bool {
        let pos_to_remove = if let Ok(nodes) = self.nodes.read() {
            let pos_to_remove: Vec<usize> = nodes
                .iter()
                .enumerate()
                .filter(|(_node_pos, node)| !f(node))
                .map(|(node_pos, _node)| node_pos)
                .collect();
            pos_to_remove
        } else {
            Vec::new()
        };
        if !pos_to_remove.is_empty() {
            self.data_epoch += 1;
            let command = self.mut_nodes(|nodes, positions, edges, node_shapes, individual_node_styles| {
                let node_memos = pos_to_remove
                    .par_iter()
                    .map(|pos| NodeMemo {
                        index: nodes[*pos].node_index,
                        position: positions[*pos].pos,
                        hidden_references: individual_node_styles[*pos].hidden_references,
                    })
                    .collect::<Vec<NodeMemo>>();
                let edge_memos: Vec<EdgeMemo> = edges
                    .iter()
                    .filter(|edge| {
                        let from_match = pos_to_remove.binary_search(&edge.from).is_ok();
                        let to_match = pos_to_remove.binary_search(&edge.to).is_ok();
                        if from_match && !to_match {
                            if let Some(individual_node_style) = individual_node_styles.get_mut(edge.from) {
                                individual_node_style.hidden_references += 1;
                            }
                        } else if to_match && !from_match {
                            if let Some(individual_node_style) = individual_node_styles.get_mut(edge.from) {
                                individual_node_style.hidden_references += 1;
                            }
                        }
                        from_match || to_match
                    })
                    .map(|edge| EdgeMemo {
                        from: edge.from,
                        to: edge.to,
                        predicate: edge.predicate,
                    })
                    .collect();

                edges.retain(|e| {
                    pos_to_remove.binary_search(&e.from).is_err() && pos_to_remove.binary_search(&e.to).is_err()
                });
                let mut new_positions: Vec<usize> = Vec::with_capacity(nodes.len());
                for i in 0..nodes.len() {
                    new_positions.push(i);
                }

                let mut remove_iter = pos_to_remove.iter().peekable();
                let mut write = 0;

                for read in 0..nodes.len() {
                    // Skip if current index should be removed
                    if Some(&&read) == remove_iter.peek() {
                        remove_iter.next();
                        continue;
                    }
                    // Otherwise, move the element to the write position
                    if write != read {
                        // All types are copy otherwise would need std::mem::replace
                        nodes[write] = nodes[read];
                        node_shapes[write] = node_shapes[read];
                        positions[write] = positions[read];
                        individual_node_styles[write] = individual_node_styles[read];
                        new_positions[read] = write;
                    }
                    write += 1;
                }
                // Truncate the vector to the new length
                nodes.truncate(write);
                node_shapes.truncate(write);
                positions.truncate(write);
                individual_node_styles.truncate(write);
                edges.iter_mut().for_each(|edge| {
                    edge.from = new_positions[edge.from];
                    edge.to = new_positions[edge.to];
                });
                update_edges_groups(edges, hidden_predicates);
                NodeCommand::RemoveElements(node_memos, edge_memos)
            });
            if let Some(command) = command {
                if is_undo {
                    self.redo_stack.push(command);
                } else {
                    self.undo_stack.push(command);
                    self.redo_stack.clear();
                }
            }
            false
        } else {
            true
        }
    }

    /**
     * Removes all nodes by position.
     *
     * The position list must be sorted and unique. Otherwise it will crash.
     */
    pub fn remove_pos_list(&mut self, pos_to_remove: &[usize], hidden_predicates: &SortedVec) {
        self.data_epoch += 1;
        let command = self.mut_nodes(|nodes, positions, edges, node_shapes, individual_node_styles| {
            let node_memos = pos_to_remove
                .par_iter()
                .map(|pos| NodeMemo {
                    index: nodes[*pos].node_index,
                    position: positions[*pos].pos,
                    hidden_references: individual_node_styles[*pos].hidden_references,
                })
                .collect::<Vec<NodeMemo>>();
            let edge_memos: Vec<EdgeMemo> = edges
                .iter()
                .filter(|edge| {
                    let from_match = pos_to_remove.binary_search(&edge.from).is_ok();
                    let to_match = pos_to_remove.binary_search(&edge.to).is_ok();
                    if from_match && !to_match {
                        if let Some(individual_node_style) = individual_node_styles.get_mut(edge.from) {
                            individual_node_style.hidden_references += 1;
                        }
                    } else if to_match && !from_match {
                        if let Some(individual_node_style) = individual_node_styles.get_mut(edge.from) {
                            individual_node_style.hidden_references += 1;
                        }
                    }
                    from_match || to_match
                })
                .map(|edge| EdgeMemo {
                    from: edge.from,
                    to: edge.to,
                    predicate: edge.predicate,
                })
                .collect();

            for pos in pos_to_remove.iter().rev() {
                nodes.remove(*pos);
                if positions.len() > *pos {
                    positions.remove(*pos);
                }
                if node_shapes.len() > *pos {
                    node_shapes.remove(*pos);
                }
                if individual_node_styles.len() > *pos {
                    individual_node_styles.remove(*pos);
                }
                edges.retain(|e| e.from != *pos && e.to != *pos);
                edges.par_iter_mut().for_each(|e| {
                    if e.from > *pos {
                        e.from -= 1;
                    }
                    if e.to > *pos {
                        e.to -= 1;
                    }
                });
            }
            update_edges_groups(edges, hidden_predicates);
            NodeCommand::RemoveElements(node_memos, edge_memos)
        });
        if let Some(command) = command {
            self.undo_stack.push(command);
            self.redo_stack.clear();
        }
    }

    pub fn get_pos(&self, value: IriIndex) -> Option<usize> {
        if let Ok(nodes) = self.nodes.read() {
            if let Ok(pos) = nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
                return Some(pos);
            }
        }
        None
    }

    pub fn to_center(&mut self) {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut count = 0;
        if let Ok(positions) = self.positions.read() {
            for node_pos in positions.iter() {
                x += node_pos.pos.x;
                y += node_pos.pos.y;
                count += 1;
            }
        }
        x /= count as f32;
        y /= count as f32;
        if let Ok(mut positions) = self.positions.write() {
            for node_pos in positions.iter_mut() {
                node_pos.pos.x -= x;
                node_pos.pos.y -= y;
            }
        }
    }

    pub fn clear(&mut self) {
        self.mut_nodes(|nodes, positions, edges, node_shapes, individual_node_styles| {
            positions.clear();
            nodes.clear();
            edges.clear();
            node_shapes.clear();
            individual_node_styles.clear();
        });
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn show_handle_layout_ui(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        config: &Config,
        hidden_predicates: &SortedVec,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.layout_handle.is_some() {
            if self.background_layout_finished.load(Ordering::Acquire) {
                // println!("Layout thread finished");
                if let Some(layout_handle) = self.layout_handle.take() {
                    layout_handle.join_handle.join().unwrap();
                }
            }
            ctx.request_repaint();
        }
        let mut keep_temperature = self.keep_temperature.load(Ordering::Relaxed);
        #[cfg(target_arch = "wasm32")]
        if self.compute_layout {
            let config = LayoutConfig {
                repulsion_constant: config.m_repulsion_constant,
                attraction_factor: config.m_attraction_factor,
                gravity_effect_radius: config.gravity_effect_radius,
            };
            let (max_move, new_positions) = layout_graph_nodes(
                &self.nodes.read().unwrap(),
                &self.node_shapes.read().unwrap(),
                &self.positions.read().unwrap(),
                &self.edges.read().unwrap(),
                &config,
                hidden_predicates,
                self.layout_temperature,
            );
            if let Ok(mut positions) = self.positions.write() {
                *positions = new_positions;
            }
            if !keep_temperature {
                self.layout_temperature *= 0.98;
            }
            if (max_move < 0.8 || self.layout_temperature < 0.5) && !keep_temperature {
                self.compute_layout = false;
            }
            if self.compute_layout || keep_temperature {
                self.compute_layout = true;
                ctx.request_repaint();
            }
        }
        if self.layout_handle.is_none() {
            let hover_text = {
                #[cfg(target_arch = "wasm32")]
                {
                    "Start Layout (Alt+R)"
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    "Start Layout (F5)"
                }
            };

            let refresh_clicked = ui.button(ICON_REFRESH).on_hover_text(hover_text).clicked();

            let refresh_key = ui.input(|i| {
                #[cfg(target_arch = "wasm32")]
                {
                    // Alt + R for web
                    i.modifiers.alt && i.key_pressed(egui::Key::R)
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // F5 for desktop
                    i.key_pressed(egui::Key::F5)
                }
            });
            if refresh_clicked || refresh_key {
                self.start_layout_force(config, hidden_predicates);
            }
        } else if ui.button(ICON_STOP).on_hover_text("Stop Layout").clicked()
            || ui.input(|i| i.key_pressed(egui::Key::X))
        {
            self.stop_layout();
        }
        if ui
            .selectable_label(keep_temperature, ICON_KEEP_TEMPERATURE)
            .on_hover_text("Continue Layout/Keep Layout Temperature")
            .clicked()
        {
            keep_temperature = !keep_temperature;
            self.keep_temperature.store(keep_temperature, Ordering::Relaxed);
        }
        if ui
            .selectable_label(self.lock_layout, ICON_KEY)
            .on_hover_text("Lock Layout (no layout changes)")
            .clicked()
        {
            self.lock_layout = !self.lock_layout;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_layout_force(&mut self, _config: &Config, _hidden_predicates: &SortedVec) {
        self.compute_layout = true;
        self.layout_temperature = 100.0;
    }

    pub fn start_layout(&mut self, config: &Config, hidden_predicates: &SortedVec) {
        if !self.lock_layout {
            self.start_layout_force(config, hidden_predicates);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_layout_force(&mut self, config: &Config, hidden_predicates: &SortedVec) {
        self.start_background_layout(config, hidden_predicates, 100.0);
    }

    pub fn stop_layout(&mut self) {
        self.stop_background_layout.store(true, Ordering::Relaxed);
    }

    pub fn start_background_layout(&mut self, config: &Config, hidden_predicates: &SortedVec, temperature: f32) {
        if self.layout_handle.is_some() {
            return;
        }
        // println!("Starting background layout thread");
        let nodes_clone = Arc::clone(&self.nodes);
        let edges_clone = Arc::clone(&self.edges);
        let positions_clone = Arc::clone(&self.positions);
        let node_shapes_clone = Arc::clone(&self.node_shapes);
        let keep_temperature = Arc::clone(&self.keep_temperature);
        let mut layout_config = LayoutConfig {
            repulsion_constant: config.m_repulsion_constant,
            attraction_factor: config.m_attraction_factor,
            gravity_effect_radius: config.gravity_effect_radius,
        };
        self.background_layout_finished.store(false, Ordering::Relaxed);
        self.stop_background_layout.store(false, Ordering::Relaxed);
        let is_done = Arc::clone(&self.background_layout_finished);
        let stop_layout = Arc::clone(&self.stop_background_layout);
        let (tx, rx) = mpsc::channel::<LayoutConfUpdate>();
        let hidden_predicates: SortedVec = hidden_predicates.clone();

        let handle = thread::spawn(move || {
            let mut temperature = temperature;
            // let mut count = 0;
            loop {
                /*
                if count % 20 == 0 {
                    println!("   looping {}", count);
                }
                count += 1;
                 */
                if stop_layout.load(Ordering::Relaxed) {
                    // println!("Layout stoppen");
                    break;
                }
                if let Ok(update) = rx.try_recv() {
                    match update {
                        LayoutConfUpdate::UpdateRepulsionConstant(value) => {
                            layout_config.repulsion_constant = value;
                        }
                        LayoutConfUpdate::UpdateAttractionFactor(value) => {
                            layout_config.attraction_factor = value;
                        }
                    }
                }
                let (max_move, new_positions) = {
                    let positions = positions_clone.read().unwrap();
                    let nodes = nodes_clone.read().unwrap();
                    let node_shapes = node_shapes_clone.read().unwrap();
                    let edges = edges_clone.read().unwrap();
                    layout_graph_nodes(
                        &nodes,
                        &node_shapes,
                        &positions,
                        &edges,
                        &layout_config,
                        &hidden_predicates,
                        temperature,
                    )
                };
                if stop_layout.load(Ordering::Relaxed) {
                    // println!("Layout stoppend");
                    break;
                }
                {
                    // TODO this could wait for ui to make the layout loop
                    // better could be to use something like double buffering and store it in update are and the ui thread
                    // copy the positions in changed in its own structure
                    // the decision is who should wait ui or layout thread, now the layout thread waits for ui
                    if let Ok(mut positions) = positions_clone.write() {
                        *positions = new_positions;
                    }
                }
                let keep_temperature = keep_temperature.load(Ordering::Relaxed);
                if !keep_temperature {
                    temperature *= 0.98;
                }
                if keep_temperature && max_move < 10.0 {
                    // Without sleep the cpu will run at 100% usage even if minimal change are made
                    thread::sleep(Duration::from_millis(100));
                }
                if (max_move < 0.8 && temperature < 0.5) && !keep_temperature {
                    // println!("Layout finished with max move: {} temparature: {} lo", max_move, temperature);
                    break;
                }
            }
            is_done.store(true, Ordering::Relaxed);
        });
        self.layout_handle = Some(LayoutHandle {
            join_handle: handle,
            update_sender: tx.clone(),
        });
        // println!("Background layout thread started");
    }

    pub fn hide_orphans(&mut self, hidden_predicates: &SortedVec) {
        let mut used_positions: Vec<usize> = self
            .edges
            .read()
            .unwrap()
            .iter()
            .filter(|edge| !hidden_predicates.contains(edge.predicate))
            .flat_map(|edge| vec![edge.from, edge.to])
            .collect();
        used_positions.sort_unstable();
        used_positions.dedup();
        let mut used_positions_cursor = 0;
        let mut pos_to_remove: Vec<usize> = Vec::new();
        for pos in 0..self.nodes.read().unwrap().len() {
            if used_positions_cursor >= used_positions.len() {
                pos_to_remove.push(pos);
            } else if pos == used_positions[used_positions_cursor] {
                used_positions_cursor += 1;
            } else {
                pos_to_remove.push(pos);
            }
        }
        self.remove_pos_list(&pos_to_remove, hidden_predicates);
    }

    pub fn hide_unconnected(&mut self, current_index: IriIndex, hidden_predicates: &SortedVec) -> bool {
        let current_index = match self.get_pos(current_index) {
            Some(pos) => pos,
            None => return false,
        };
        let len = self.nodes.read().unwrap().len();
        let mut adj = vec![Vec::new(); len];
        for edge in self.edges.read().unwrap().iter() {
            if !hidden_predicates.contains(edge.predicate) {
                adj[edge.from].push(edge.to);
                adj[edge.to].push(edge.from);
            }
        }

        // BFS traversal
        let mut visited = FixedBitSet::with_capacity(len);
        let mut queue = VecDeque::new();

        visited.insert(current_index);
        queue.push_back(current_index);

        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if v < len && !visited.contains(v) {
                    visited.insert(v);
                    queue.push_back(v);
                }
            }
        }
        let mut pos_to_remove: Vec<usize> = visited.zeroes().collect();
        if pos_to_remove.is_empty() {
            false
        } else {
            pos_to_remove.sort_unstable();
            self.remove_pos_list(&pos_to_remove, hidden_predicates);
            true
        }
    }

    pub fn remove_redundant_edges(&mut self, hidden_predicates: &SortedVec) {
        if let Ok(mut edges) = self.edges.write() {
            let mut groups: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
            for (edge_index, edge) in edges.iter().enumerate() {
                groups
                    .entry(if edge.from > edge.to {
                        (edge.from, edge.to)
                    } else {
                        (edge.to, edge.from)
                    })
                    .or_default()
                    .push(edge_index);
            }
            let mut edges_pos_to_remove: Vec<usize> = groups
                .values()
                .flat_map(|pos_list| {
                    if pos_list.len() > 1 {
                        pos_list[1..].to_vec() // Keep the first edge, remove the rest
                    } else {
                        Vec::new() // Keep single edges
                    }
                })
                .collect();
            edges_pos_to_remove.sort_unstable();
            edges_pos_to_remove.dedup();
            for pos in edges_pos_to_remove.iter().rev() {
                edges.remove(*pos);
            }
            // println!("Removed {} redundant edges", edges.len());
            update_edges_groups(&mut edges, hidden_predicates);
        }
    }

    pub fn run_algorithm(
        &mut self,
        graph_algorithm: GraphAlgorithm,
        visualization_style: &GVisualizationStyle,
        statistics_data: &mut StatisticsData,
        config: &Config,
        hidden_predicates: &SortedVec,
    ) {
        if let Ok(nodes) = self.nodes.read() {
            if !nodes.is_empty() {
                if let Ok(edges) = self.edges.read() {
                    // println!("run algorithm: {:?}", graph_algorithm);
                    let nodes_len = nodes.len();
                    if self.data_epoch != statistics_data.data_epoch {
                        if let Ok(mut individual_node_style) = self.individual_node_styles.write() {
                            if individual_node_style.len() != nodes.len() {
                                individual_node_style.resize(nodes.len(), IndividualNodeStyleData::default());
                            }
                            statistics_data.nodes.resize(nodes_len, (0, 0));
                            for (index, node) in nodes.iter().enumerate() {
                                statistics_data.nodes[index] = (node.node_index, index as u32);
                            }
                            if statistics_data.selected_idx.is_none() && !statistics_data.nodes.is_empty() {
                                statistics_data.selected_idx = Some((statistics_data.nodes[0].0, 0));
                            }
                            statistics_data.results.clear();
                            if graph_algorithm.is_clustering() {
                                let cluster = run_clustering_algorithm(
                                    graph_algorithm,
                                    nodes_len,
                                    &edges,
                                    config,
                                    hidden_predicates,
                                );
                                let values = cluster.node_cluster.iter().map(|e| *e as f32).collect::<Vec<f32>>();
                                for (index, value) in cluster.node_cluster.iter().enumerate() {
                                    individual_node_style[index].set_cluster(*value);
                                }
                                statistics_data
                                    .results
                                    .push(StatisticsResult::new_for_alg(values, graph_algorithm));
                                if let Some(parameters) = cluster.parameters {
                                    statistics_data.results.push(StatisticsResult::new_for_values(
                                        parameters,
                                        graph_algorithm.get_statistics_values()[1],
                                    ));
                                }
                            } else {
                                let values: Vec<f32> =
                                    run_algorithm(graph_algorithm, nodes_len, &edges, hidden_predicates);
                                let values_layers: Vec<u8> = distribute_to_zoom_layers(&values);
                                for (index, (layer, value)) in values_layers.iter().zip(&values).enumerate() {
                                    individual_node_style[index].set_size_value(*value, visualization_style);
                                    individual_node_style[index]
                                        .semantic_zoom_interval
                                        .set_from_layout(*layer);
                                }
                                statistics_data
                                    .results
                                    .push(StatisticsResult::new_for_alg(values, graph_algorithm));
                            }
                            self.update_node_shapes = true;
                            self.has_semantic_zoom = true;
                            statistics_data.data_epoch = self.data_epoch;
                        }
                    } else {
                        let statistic_value = graph_algorithm.get_statistics_values()[0];
                        let result = statistics_data
                            .results
                            .iter()
                            .find(|res| res.statistics_value() == statistic_value);
                        if let Some(result) = result {
                            // no action needed the data is already in result but we need to set the individual node styles
                            if let Ok(mut individual_node_style) = self.individual_node_styles.write() {
                                if graph_algorithm.is_clustering() {
                                    for (index, value) in result.get_data_vec().iter().enumerate() {
                                        let node_index = statistics_data.nodes[index].1 as usize;
                                        individual_node_style[node_index].set_cluster(*value as u32);
                                    }
                                } else {
                                    let values_layers: Vec<u8> = distribute_to_zoom_layers(result.get_data_vec());
                                    for (index, (value, layer)) in
                                        result.get_data_vec().iter().zip(&values_layers).enumerate()
                                    {
                                        let node_index = statistics_data.nodes[index].1 as usize;
                                        individual_node_style[node_index].set_size_value(*value, visualization_style);
                                        individual_node_style[index]
                                            .semantic_zoom_interval
                                            .set_from_layout(*layer);
                                    }
                                }
                            }
                        } else {
                            if graph_algorithm.is_clustering() {
                                let cluster = run_clustering_algorithm(
                                    graph_algorithm,
                                    nodes_len,
                                    &edges,
                                    config,
                                    hidden_predicates,
                                );
                                let values = statistics_data
                                    .nodes
                                    .iter()
                                    .map(|(_iri, pos)| cluster.node_cluster[*pos as usize] as f32)
                                    .collect::<Vec<f32>>();
                                if let Ok(mut individual_node_style) = self.individual_node_styles.write() {
                                    for (index, value) in cluster.node_cluster.iter().enumerate() {
                                        individual_node_style[index].set_cluster(*value);
                                    }
                                }
                                statistics_data
                                    .results
                                    .push(StatisticsResult::new_for_alg(values, graph_algorithm));
                                if let Some(parameters) = cluster.parameters {
                                    let values = statistics_data
                                        .nodes
                                        .iter()
                                        .map(|(_iri, pos)| parameters[*pos as usize])
                                        .collect::<Vec<f32>>();
                                    statistics_data.results.push(StatisticsResult::new_for_values(
                                        values,
                                        graph_algorithm.get_statistics_values()[1],
                                    ));
                                }
                            } else {
                                let values = run_algorithm(graph_algorithm, nodes_len, &edges, hidden_predicates);
                                // the values could be already resorted so use position index to get them in right order
                                let sorted_values = statistics_data
                                    .nodes
                                    .iter()
                                    .map(|(_iri, pos)| values[*pos as usize])
                                    .collect::<Vec<f32>>();
                                let values_layers: Vec<u8> = distribute_to_zoom_layers(&values);
                                if let Ok(mut individual_node_style) = self.individual_node_styles.write() {
                                    for (index, (value, layer)) in values.iter().zip(&values_layers).enumerate() {
                                        individual_node_style[index].set_size_value(*value, visualization_style);
                                        individual_node_style[index]
                                            .semantic_zoom_interval
                                            .set_from_layout(*layer);
                                    }
                                }
                                statistics_data
                                    .results
                                    .push(StatisticsResult::new_for_alg(sorted_values, graph_algorithm));
                            }
                        }
                        self.update_node_shapes = true;
                        self.has_semantic_zoom = true;
                    }
                }
            }
        }
    }

    pub fn undo(&mut self, config: &Config, hidden_predicates: &SortedVec) {
        if let Some(command) = self.undo_stack.pop() {
            command.undo(self, &hidden_predicates, config, true);
        } else {
            println!("Nothing to undo");
        }
    }

    pub fn redo(&mut self, config: &Config, hidden_predicates: &SortedVec) {
        if let Some(command) = self.redo_stack.pop() {
            command.undo(self, &hidden_predicates, config, false);
        }
    }

    pub fn select_all(&self, ui_state: &mut UIState) {
        if let Ok(nodes) = self.nodes.read() {
            for node in nodes.iter() {
                ui_state.selected_nodes.insert(node.node_index);
            }
            if ui_state.selected_node.is_none() && !nodes.is_empty() {
                ui_state.selected_node = Some(nodes[0].node_index);
            }
        }
    }

    pub fn deselect_all(&self, ui_state: &mut UIState) {
        ui_state.selected_nodes.clear();
        ui_state.selected_node = None;
    }

    pub fn invert_selection(&self, ui_state: &mut UIState) {
        if let Ok(nodes) = self.nodes.read() {
            let mut new_selection: BTreeSet<IriIndex> = BTreeSet::new();
            for node in nodes.iter() {
                if !ui_state.selected_nodes.contains(&node.node_index) {
                    new_selection.insert(node.node_index);
                }
            }
            ui_state.selected_nodes = new_selection;
            if ui_state.selected_node.is_none() && !ui_state.selected_nodes.is_empty() {
                ui_state.selected_node = Some(*ui_state.selected_nodes.iter().next().unwrap());
            } else if let Some(selected_index) = ui_state.selected_node {
                if !ui_state.selected_nodes.contains(&selected_index) && !ui_state.selected_nodes.is_empty() {
                    ui_state.selected_node = Some(*ui_state.selected_nodes.iter().next().unwrap());
                }
            }
        }
    }

    pub fn expand_selection(&self, ui_state: &mut UIState) {
        if let Ok(edges) = self.edges.read() {
            if let Ok(nodes) = self.nodes.read() {
                let mut new_selected: Vec<IriIndex> = Vec::new();
                for selected_node in ui_state.selected_nodes.iter() {
                    if let Some(pos) = nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)).ok() {
                        for edge in edges.iter() {
                            if edge.from == pos {
                                let node_index = nodes[edge.to].node_index;
                                new_selected.push(node_index);
                            } else if edge.to == pos {
                                let node_index = nodes[edge.from].node_index;
                                new_selected.push(node_index);
                            }
                        }
                    }
                }
                for node in new_selected {
                    ui_state.selected_nodes.insert(node);
                }
            }
        }
    }

    pub fn shirk_selection(&self, ui_state: &mut UIState) {
        if let Ok(edges) = self.edges.read() {
            if let Ok(nodes) = self.nodes.read() {
                let mut to_remove: Vec<IriIndex> = Vec::new();
                for selected_node in ui_state.selected_nodes.iter() {
                    if let Some(pos) = nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)).ok() {
                        let mut connected_num = 0;
                        for edge in edges.iter() {
                            if edge.from != edge.to
                                && (edge.from == pos && ui_state.selected_nodes.contains(&nodes[edge.to].node_index))
                                || (edge.to == pos && ui_state.selected_nodes.contains(&nodes[edge.from].node_index))
                            {
                                connected_num += 1;
                                if connected_num > 1 {
                                    break;
                                }
                            }
                        }
                        if connected_num <= 1 {
                            to_remove.push(*selected_node);
                        }
                    }
                }
                for node in to_remove {
                    ui_state.selected_nodes.remove(&node);
                }
                if let Some(selected_index) = ui_state.selected_node {
                    if !ui_state.selected_nodes.contains(&selected_index) && !ui_state.selected_nodes.is_empty() {
                        ui_state.selected_node = Some(*ui_state.selected_nodes.iter().next().unwrap());
                    }
                }
            }
        }
    }
}

pub fn update_edges_groups(edges: &mut [Edge], hidden_predicates: &SortedVec) {
    // Each group has all edges that connect same nodes (despite the direction)
    // It is needed to set parameter for bezier curves
    let mut groups: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (edge_index, edge) in edges.iter().enumerate() {
        if !hidden_predicates.contains(edge.predicate) {
            groups
                .entry(if edge.from > edge.to {
                    (edge.from, edge.to)
                } else {
                    (edge.to, edge.from)
                })
                .or_default()
                .push(edge_index);
        }
    }
    let bezier_gap: f32 = 30.0;
    for group in groups.values() {
        if group.len() > 1 {
            let first_edge = &edges[group[0]];
            if first_edge.from == first_edge.to {
                // For self references edges we need distribute full angle
                let diff = std::f32::consts::PI * 2.0 / group.len() as f32;
                let mut start = 0.0;
                for edge in group.iter() {
                    edges[*edge].bezier_distance = start;
                    start += diff;
                }
            } else {
                let full_len = (group.len() - 1) as f32 * bezier_gap;
                let mut distance = -full_len / 2.0;
                for edge in group.iter() {
                    let t_edge = &edges[*edge];
                    edges[*edge].bezier_distance = if t_edge.from > t_edge.to { distance } else { -distance };
                    distance += bezier_gap;
                }
            }
        } else {
            for edge in group.iter() {
                edges[*edge].bezier_distance = 0.0;
            }
        }
    }
}

fn smooth_invert(x: f32) -> f32 {
    if x <= 0.0 {
        return 1.0;
    }
    if x >= 1.0 {
        return 0.0;
    }
    let x2 = x * x;
    let x3 = x2 * x;
    let x4 = x3 * x;
    let x5 = x4 * x;
    let s = 6.0 * x5 - 15.0 * x4 + 10.0 * x3;
    1.0 - s
}

pub struct LayoutConfig {
    pub repulsion_constant: f32,
    pub attraction_factor: f32,
    pub gravity_effect_radius: f32,
}

pub fn layout_graph_nodes(
    nodes: &[NodeLayout],
    node_shapes: &[NodeShapeData],
    positions: &[NodePosition],
    edges: &[Edge],
    config: &LayoutConfig,
    hidden_predicates: &SortedVec,
    temperature: f32,
) -> (f32, Vec<NodePosition>) {
    if nodes.is_empty() {
        return (0.0, Vec::new());
    }
    // bei mehr nodes is kleiner
    // Was auch stimmt, weil es k der optimalen entfernung zwischen den nodes ist
    let k = ((500.0 * 500.0) / (nodes.len() as f32)).sqrt();
    // abstossen
    let repulsion_constant = config.repulsion_constant;
    // anziehen
    let attraction_constant = config.attraction_factor;
    let repulsion_factor: f32 = (repulsion_constant * k).powi(2);
    // 55000.0 entspricht 20 nodes. Die anziehung soll unabhängig von der Anzahl der nodes sein
    // let attraction = k / attraction_constant;
    let attraction = 111.0 / attraction_constant;

    let mut tree = BHQuadtree::new(0.5);
    let weight_points: Vec<WeightedPoint> = positions
        .par_iter()
        .map(|pos| WeightedPoint {
            pos: pos.pos.to_vec2(),
            mass: 1.0,
        })
        .collect();
    tree.build(weight_points, 5);

    let gravity_effect_radius = config.gravity_effect_radius;
    let max_smoth_effect_radius = gravity_effect_radius * 1.2;

    let force_fn = |target: Vec2, source: WeightedPoint| {
        // compute repulsive force
        let dir = target - source.pos;
        if dir.x == 0.0 && dir.y == 0.0 {
            return Vec2::ZERO; // Avoid division by zero
        }
        let dist2 = dir.length();
        let mut scale: f32 = 1.0;
        if dist2 > gravity_effect_radius {
            if dist2 > max_smoth_effect_radius {
                return Vec2::ZERO;
            } else {
                // use smotherstep function to turn off the gravity force in 20% area, so do not create spring effects
                scale = smooth_invert((dist2 - gravity_effect_radius) / (gravity_effect_radius * 0.2));
            }
        }
        let force_mag = (source.mass * repulsion_factor) / dist2;
        scale * (dir / dist2) * force_mag
    };

    let mut forces: Vec<Vec2> = positions
        .par_iter()
        .map(|node_position| {
            let pos = node_position.pos.to_vec2();
            tree.accumulate(pos, force_fn)
        })
        .collect();

    for edge in edges.iter() {
        if edge.from != edge.to && !hidden_predicates.contains(edge.predicate) {
            let node_from = &node_shapes[edge.from];
            let node_to = &node_shapes[edge.to];
            let position_from = &positions[edge.from];
            let position_to = &positions[edge.to];
            let direction = position_from.pos - position_to.pos;
            let distance = direction.length() - node_from.size.x / 2.0 - node_to.size.x / 2.0 - 4.0;
            let force = distance.powi(2) / attraction;
            let force_v = (direction / distance) * force;
            forces[edge.from] -= force_v;
            forces[edge.to] += force_v;
        }
    }

    let max_move = AtomicF32::new(0.0);

    let positions = forces
        .par_iter()
        .zip(positions.par_iter())
        .map(|(f, position)| {
            if position.locked {
                return *position;
            } else {
                let mut v = position.vel;
                let pos = position.pos;
                v *= 0.4;
                v += *f * 0.01;
                let len = v.length();
                if len > temperature {
                    v = (v / len) * temperature;
                    max_move.fetch_max(temperature, Ordering::Relaxed);
                } else {
                    max_move.fetch_max(len, Ordering::Relaxed);
                }
                NodePosition {
                    pos: pos + v,
                    vel: v,
                    locked: position.locked,
                }
            }
        })
        .collect();

    (max_move.load(Ordering::Relaxed), positions)
}

impl NodeCommand {
    pub fn undo(
        &self,
        sorted_nodes: &mut SortedNodeLayout,
        hidden_predicates: &SortedVec,
        config: &Config,
        from_undo: bool,
    ) {
        match self {
            NodeCommand::AddElements(added_nodes) => {
                sorted_nodes.retain(hidden_predicates, from_undo, |node| {
                    !added_nodes.contains(&node.node_index)
                });
            }
            NodeCommand::RemoveElements(removed_nodes, removed_edges) => {
                sorted_nodes.mut_nodes(|nodes, positions, edges, node_shapes, individual_node_styles| {
                    // The implementation is similar to add_many
                    // we assume that removed_nodes are unique and sorted by node index
                    // which must be if they are produced by retain function
                    let mut new_positions: Vec<usize> = Vec::with_capacity(nodes.len());
                    for i in 0..nodes.len() {
                        new_positions.push(i);
                    }
                    // we use in place merge on target vector. The vector is resized and the new elements
                    // are inserted from the end by iterating the new nodes and old nodes from the end.
                    let orig_len = nodes.len();
                    let b_len = removed_nodes.len();

                    nodes.resize(orig_len + b_len, NodeLayout { node_index: 0 });
                    node_shapes.resize(orig_len + b_len, NodeShapeData::default());
                    positions.resize(orig_len + b_len, NodePosition::default());
                    individual_node_styles.resize(orig_len + b_len, IndividualNodeStyleData::default());

                    let mut i = orig_len as isize - 1;
                    let mut j = b_len as isize - 1;
                    let mut k = (orig_len + b_len) as isize - 1;

                    let mut removed_nodes_pos : Vec<usize> = Vec::with_capacity(removed_nodes.len());

                    while j >= 0 {
                        if i >= 0 && nodes[i as usize].node_index > removed_nodes[j as usize].index {
                            let old_node = nodes[i as usize];
                            nodes[k as usize] = old_node;
                            node_shapes[k as usize] = node_shapes[i as usize];
                            positions[k as usize] = positions[i as usize];
                            new_positions[i as usize] = k as usize;
                            individual_node_styles[k as usize] = individual_node_styles[i as usize];
                            i -= 1;
                        } else {
                            removed_nodes_pos.push(k as usize);
                            nodes[k as usize] = NodeLayout {
                                node_index: removed_nodes[j as usize].index,
                            };
                            node_shapes[k as usize] = NodeShapeData::default();
                            positions[k as usize] = NodePosition {
                                pos: removed_nodes[j as usize].position,
                                vel: Vec2::ZERO,
                                locked: false,
                            };
                            let ins = IndividualNodeStyleData {
                                hidden_references: removed_nodes[j as usize].hidden_references,
                                ..Default::default()
                            };
                            individual_node_styles[k as usize] = ins;
                            j -= 1;
                        }
                        k -= 1;
                    }

                    // now need to set new edge indexes to new ones
                    edges.par_iter_mut().for_each(|edge| {
                        edge.from = new_positions[edge.from];
                        edge.to = new_positions[edge.to];
                    });
                    // Add also the removed edges with right indexes
                    for edge in removed_edges.iter() {
                        // We need adapt hidden references but only for nodes that are not newly added (because they have already correct count)
                        if removed_nodes_pos.binary_search(&edge.from).is_err() {
                            if let Some(individual_node_style) = individual_node_styles.get_mut(edge.from) {
                                if individual_node_style.hidden_references > 0 {
                                    individual_node_style.hidden_references -= 1;
                                }
                            }
                        }
                        if removed_nodes_pos.binary_search(&edge.to).is_err() {                     
                            if let Some(individual_node_style) = individual_node_styles.get_mut(edge.to) {
                                if individual_node_style.hidden_references > 0 {
                                    individual_node_style.hidden_references -= 1;
                                }
                            }
                        }
                        edges.push(Edge {
                            from: edge.from,
                            to: edge.to,
                            predicate: edge.predicate,
                            bezier_distance: 0.0,
                        });
                    }
                });
                let command = NodeCommand::AddElements(removed_nodes.iter().map(|n| n.index).collect());
                if from_undo {
                    sorted_nodes.redo_stack.push(command);
                } else {
                    sorted_nodes.undo_stack.push(command);
                }
            }
        }
        sorted_nodes.start_layout(config, hidden_predicates);
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::Config, nobject::IriIndex};

    #[test]
    fn test_graph_nodes() {
        let mut sorted_nodes = super::SortedNodeLayout::default();
        let nodes = vec![
            super::NodeLayout::new(0),
            super::NodeLayout::new(10),
            super::NodeLayout::new(5),
        ];
        assert!(sorted_nodes.add(nodes[0]));
        assert!(!sorted_nodes.add(nodes[0]));
        assert!(sorted_nodes.contains(0));
        assert!(!sorted_nodes.contains(1));
        assert_eq!(0, sorted_nodes.get_pos(0).unwrap());
        assert!(sorted_nodes.get_pos(1).is_none());

        let sorted_vec = super::SortedVec::new();
        sorted_nodes.remove(0, &sorted_vec);
        assert!(!sorted_nodes.contains(0));

        for node in nodes.iter() {
            assert!(sorted_nodes.add(*node));
        }
        for node in nodes.iter() {
            assert!(!sorted_nodes.add(*node));
            assert!(sorted_nodes.contains(node.node_index));
        }
        // new indexes 2, 3, 12
        let config = Config::default();
        let new_pairs: Vec<(IriIndex, IriIndex)> = vec![(0, 5), (0, 5), (5, 2), (5, 12), (10, 2), (0, 3), (0, 10)];
        assert!(
            sorted_nodes.add_many(&new_pairs, &config, |(_parent_index, node_index)| {
                assert_ne!(5, *node_index);
                assert_ne!(10, *node_index);
            })
        );
        assert!(
            !sorted_nodes.add_many(&new_pairs, &config, |(_parent_index, node_index)| {
                // This should be never called
                assert!(*node_index > 100);
            })
        );
        for (_parent_idx, node_idx) in &new_pairs {
            assert!(sorted_nodes.contains(*node_idx));
        }
        let to_remove: Vec<IriIndex> = vec![2, 5, 12];
        sorted_nodes.retain(&sorted_vec, false, |node| !to_remove.contains(&node.node_index));
        for removed_idx in &to_remove {
            assert!(!sorted_nodes.contains(*removed_idx));
        }
        assert!(sorted_nodes.contains(3));
        assert!(sorted_nodes.contains(0));
        assert!(sorted_nodes.contains(10));
    }
}
