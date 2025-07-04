use crate::{config::Config, graph_styles::NodeShape, nobject::IriIndex};
use atomic_float::AtomicF32;
use eframe::egui::Vec2;
use egui::Pos2;
use rand::Rng;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

const MAX_DISTANCE: f32 = 2000.0;

pub struct NodeLayout {
    pub node_index: IriIndex,
}

impl NodeLayout {
    pub fn new(node_index: IriIndex) -> Self {
        Self {
            node_index,
        }
    }
}

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
}

impl Default for NodePosition {
    fn default() -> Self {
        Self {
            pos: Pos2::new(
                rand::rng().random_range(-100.0..100.0),
                rand::rng().random_range(-100.0..100.0),
            ),
            vel: Vec2::new(0.0, 0.0),
        }
    }
}

pub struct SortedNodeLayout {
    pub nodes: Arc<RwLock<Vec<NodeLayout>>>,
    pub edges: Arc<RwLock<Vec<Edge>>>,
    pub positions: Arc<RwLock<Vec<NodePosition>>>,
    pub node_shapes: Arc<RwLock<Vec<NodeShapeData>>>,
    pub layout_temparature: f32,
    pub force_compute_layout: bool,
    pub compute_layout: bool,
    pub layout_handle: Option<JoinHandle<()>>,
    pub background_layout_finished: Arc<AtomicBool>,
    pub stop_background_layout: Arc<AtomicBool>,
    pub update_node_shapes: bool,
}

impl Default for SortedNodeLayout {
    fn default() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
            positions: Arc::new(RwLock::new(Vec::new())),
            edges: Arc::new(RwLock::new(Vec::new())),
            node_shapes: Arc::new(RwLock::new(Vec::new())),
            compute_layout: true,
            force_compute_layout: false,
            layout_temparature: 0.5,
            layout_handle: None,
            background_layout_finished: Arc::new(AtomicBool::new(false)),
            stop_background_layout: Arc::new(AtomicBool::new(false)),
            update_node_shapes: true,
        }
    }
}

impl SortedNodeLayout {
    pub fn new() -> Self {
        Self::default()
    }

    fn add(&mut self, value: NodeLayout) -> bool {
        self.stop_layout();
        if let Ok(mut nodes) = self.nodes.write() {
            if let Ok(mut node_shapes) = self.node_shapes.write() {
                if let Ok(mut positions) = self.positions.write() {
                    if let Ok(mut edges) = self.edges.write() {
                        return match nodes.binary_search_by(|e| e.node_index.cmp(&value.node_index)) {
                            Ok(_) => false, // Value already exists, do nothing
                            Err(pos) => {
                                // Insert at correct position
                                nodes.insert(pos, value);
                                positions.insert(pos, NodePosition::default());
                                node_shapes.insert(pos, NodeShapeData::default());
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
        false
    }

    pub fn add_by_index(&mut self, value: IriIndex) -> bool {
        self.add(NodeLayout::new(value))
    }

    pub fn contains(&self, value: IriIndex) -> bool {
        self.get_pos(value).is_some()
    }

    pub fn mut_nodes<R>(
        &mut self,
        mutator: impl Fn(&mut Vec<NodeLayout>, &mut Vec<NodePosition>, &mut Vec<Edge>, &mut Vec<NodeShapeData>) -> R,
    ) -> Option<R> {
        self.stop_layout();
        if let Ok(mut nodes) = self.nodes.write() {
            if let Ok(mut positions) = self.positions.write() {
                if let Ok(mut edges) = self.edges.write() {
                    if let Ok(mut node_shapes) = self.node_shapes.write() {
                        return Some(mutator(&mut nodes, &mut positions, &mut edges, &mut node_shapes));
                    }
                }
            }
        }
        None
    }

    pub fn remove(&mut self, value: IriIndex) {
        self.mut_nodes(|nodes, positions, edges, node_shapes| {
            if let Ok(pos) = nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
                nodes.remove(pos);
                if positions.len() > pos {
                    positions.remove(pos);
                }
                if node_shapes.len() > pos {
                    node_shapes.remove(pos);
                }
                edges.retain(|e| e.from != pos && e.to != pos);
                edges.iter_mut().for_each(|e| {
                    if e.from > pos {
                        e.from -= 1;
                    }
                    if e.to > pos {
                        e.to -= 1;
                    }
                });
                update_edges_groups(edges);
            }
        });
    }

    pub fn remove_all(&mut self, iris_to_remove: &[IriIndex]) {
        self.mut_nodes(|nodes, positions, edges, node_shapes| {
            for value in iris_to_remove.iter() {
                // Can be optimized if values are sorted
                if let Ok(pos) = nodes.binary_search_by(|e| e.node_index.cmp(value)) {
                    nodes.remove(pos);
                    if positions.len() > pos {
                        positions.remove(pos);
                    }
                    if node_shapes.len() > pos {
                        node_shapes.remove(pos);
                    }
                    edges.retain(|e| e.from != pos && e.to != pos);
                    edges.iter_mut().for_each(|e| {
                        if e.from > pos {
                            e.from -= 1;
                        }
                        if e.to > pos {
                            e.to -= 1;
                        }
                    });
                    update_edges_groups(edges);
                }
            }
        });
    }

    pub fn retain(&mut self, f: impl Fn(&NodeLayout) -> bool) {
        self.mut_nodes(|nodes, positions, edges, node_shapes| {
            // Can be optimized to not check nodes multiple time always from begin
            while let Some(pos) = nodes.iter().position(|e| !f(e)) {
                nodes.remove(pos);
                if positions.len() > pos {
                    positions.remove(pos);
                }
                if node_shapes.len() > pos {
                    node_shapes.remove(pos);
                }
                edges.retain(|e| e.from != pos && e.to != pos);
                edges.iter_mut().for_each(|e| {
                    if e.from > pos {
                        e.from -= 1;
                    }
                    if e.to > pos {
                        e.to -= 1;
                    }
                });
            }
            update_edges_groups(edges);
        });
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
        self.mut_nodes(|nodes, positions, edges, node_shapes| {
            positions.clear();
            nodes.clear();
            edges.clear();
            node_shapes.clear();
        });
    }

    pub fn show_handle_layout_ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, config: &Config) {
        if ui.checkbox(&mut self.force_compute_layout, "Force Layout").changed() && self.force_compute_layout {
            self.layout_temparature = 100.0;
        }
        if self.layout_handle.is_some() {
            if self.background_layout_finished.load(Ordering::Acquire) {
                // println!("Layout thread finished");
                if let Some(layout_handle) = self.layout_handle.take() {
                    layout_handle.join().unwrap();
                }
            }
            ctx.request_repaint();
        } else if self.compute_layout || self.force_compute_layout {
            let config = LayoutConfig {
                repulsion_constant: config.m_repulsion_constant,
                attraction_factor: config.m_attraction_factor,
            };
            let (max_move, new_positions) = layout_graph_nodes(
                &self.nodes.read().unwrap(),
                &self.node_shapes.read().unwrap(),
                &self.positions.read().unwrap(),
                &self.edges.read().unwrap(),
                &config,
                self.layout_temparature,
            );
            if let Ok(mut positions) = self.positions.write() {
                *positions = new_positions;
            }
            if !self.force_compute_layout {
                self.layout_temparature *= 0.98;
            }
            if (max_move < 0.8 || self.layout_temparature < 0.5) && !self.force_compute_layout {
                self.compute_layout = false;
            }
            if self.compute_layout || self.force_compute_layout {
                self.compute_layout = true;
                ctx.request_repaint();
            }
        }
        if self.layout_handle.is_none() {
            if ui.button("Start Layout").clicked() {
                self.start_layout(config);
            }
        } else if ui.button("Stop Layout").clicked() {
            self.stop_layout();
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn start_layout(&mut self, _config: &Config) {
        self.compute_layout = true;
        self.layout_temparature = 100.0;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_layout(&mut self, config: &Config) {
        self.start_background_layout(config, 100.0);
    }

    pub fn stop_layout(&mut self) {
        self.stop_background_layout.store(true, Ordering::Relaxed);
    }

    pub fn start_background_layout(&mut self, config: &Config, temperature: f32) {
        if self.layout_handle.is_some() {
            return;
        }
        // println!("Starting background layout thread");
        let nodes_clone = Arc::clone(&self.nodes);
        let edges_clone = Arc::clone(&self.edges);
        let positions_clone = Arc::clone(&self.positions);
        let node_shapes_clone = Arc::clone(&self.node_shapes);
        let force_compute_layout = self.force_compute_layout;
        let layout_config = LayoutConfig {
            repulsion_constant: config.m_repulsion_constant,
            attraction_factor: config.m_attraction_factor,
        };
        self.background_layout_finished.store(false, Ordering::Relaxed);
        self.stop_background_layout.store(false, Ordering::Relaxed);
        let is_done = Arc::clone(&self.background_layout_finished);
        let stop_layout = Arc::clone(&self.stop_background_layout);

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
                    // println!("Layout stoppend");
                    break;
                }
                let (max_move, new_positions) = {
                    let positions = positions_clone.read().unwrap();
                    let nodes = nodes_clone.read().unwrap();
                    let node_shapes = node_shapes_clone.read().unwrap();
                    let edges = edges_clone.read().unwrap();
                    layout_graph_nodes(&nodes, &node_shapes, &positions, &edges, &layout_config, temperature)
                };
                if stop_layout.load(Ordering::Relaxed) {
                    // println!("Layout stoppend");
                    break;
                }
                {
                    if let Ok(mut positions) = positions_clone.write() {
                        *positions = new_positions;
                    }
                }
                if !force_compute_layout {
                    temperature *= 0.98;
                }
                if (max_move < 0.8 || temperature < 0.5) && !force_compute_layout {
                    // println!("Layout finished with max move: {} temparature: {} lo", max_move, temperature);
                    break;
                }
            }
            is_done.store(true, Ordering::Relaxed);
        });
        self.layout_handle = Some(handle);
        // println!("Background layout thread started");
    }
}

pub fn update_edges_groups(edges: &mut [Edge]) {
    // Each group has all edges that connect same nodes (dispite the direction)
    // It is needed to set parameter for bezier curves
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
        }
    }
}

pub struct LayoutConfig {
    pub repulsion_constant: f32,
    pub attraction_factor: f32,
}

pub fn layout_graph_nodes(
    nodes: &[NodeLayout],
    node_shapes: &[NodeShapeData],
    positions: &[NodePosition],
    edges: &[Edge],
    config: &LayoutConfig,
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

    let mut forces: Vec<Vec2> = nodes
        .par_iter()
        .zip(positions.par_iter())
        .map(|(node_layout, node_position)| {
            let mut f = Vec2::new(0.0, 0.0);
            for (nnode_layout, nnode_position) in nodes.iter().zip(positions.iter()) {
                if nnode_layout.node_index != node_layout.node_index {
                    let direction = node_position.pos - nnode_position.pos;
                    let distance = direction.length();
                    if distance > 0.0 && distance < MAX_DISTANCE {
                        let force = repulsion_factor / distance;
                        f += (direction / distance) * force;
                    }
                }
            }
            f
        })
        .collect();

    for edge in edges.iter() {
        if edge.from != edge.to {
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
            let mut v = position.vel;
            let pos = position.pos;
            v *= 0.4;
            v += *f * 0.01;
            let len = v.length();
            max_move.fetch_max(len, Ordering::Relaxed);
            if len > temperature {
                v = (v / len) * temperature;
            }
            NodePosition { pos: pos + v, vel: v }
        })
        .collect();

    (max_move.load(Ordering::Relaxed), positions)
}
