use crate::{config::Config, graph_styles::NodeShape, nobject::IriIndex, quad_tree::{BHQuadtree, WeightedPoint}, SortedVec};
use atomic_float::AtomicF32;
use eframe::egui::Vec2;
use egui::Pos2;
use rand::Rng;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering}, mpsc, Arc, RwLock
    },
    thread::{self, JoinHandle}, time::Duration,
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
    pub layout_temperature: f32,
    pub keep_temperature: Arc<AtomicBool>,
    pub compute_layout: bool,
    pub layout_handle: Option<LayoutHandle>,
    pub background_layout_finished: Arc<AtomicBool>,
    pub stop_background_layout: Arc<AtomicBool>,
    pub update_node_shapes: bool,
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

impl Default for SortedNodeLayout {
    fn default() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
            positions: Arc::new(RwLock::new(Vec::new())),
            edges: Arc::new(RwLock::new(Vec::new())),
            node_shapes: Arc::new(RwLock::new(Vec::new())),
            compute_layout: true,
            keep_temperature: Arc::new(AtomicBool::new(false)),
            layout_temperature: 0.5,
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

    // use add_many operation if possible. Do not call it in the loop
    pub fn add_by_index(&mut self, value: IriIndex) -> bool {
        self.add(NodeLayout::new(value))
    }

    pub fn add_many(&mut self, values: &[(IriIndex, IriIndex)], inserted_callback: impl FnMut(&(IriIndex,IriIndex))) -> bool {
        let index_to_add = if let Ok(nodes) = self.nodes.read() {
            // First filter the list only for nodes that are not already in the layout
            // sort and dedup the nodes the parent node does not matter
            let mut index_to_add : Vec<(IriIndex,IriIndex)> = values.iter().filter(|(_parent_index, node_index)| {
                nodes.binary_search_by(|node| node.node_index.cmp(node_index)).is_err()
            }).map(|p| (p.0,p.1)).collect();
            index_to_add.sort_unstable_by(|a,b| a.1.cmp(&b.1));
            index_to_add.dedup_by(| a, b| a.1 == b.1);
            index_to_add.iter().for_each(inserted_callback);
            index_to_add
        } else {
            Vec::new()
        };
        if !index_to_add.is_empty() {
            if let Ok(mut nodes) = self.nodes.write() {
                if let Ok(mut node_shapes) = self.node_shapes.write() {
                    if let Ok(mut positions) = self.positions.write() {
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

                            let mut i = orig_len as isize - 1;
                            let mut j = b_len as isize - 1;
                            let mut k = (orig_len + b_len) as isize - 1;

                            while j >= 0 {
                                if i >= 0 && nodes[i as usize].node_index > index_to_add[j as usize].1 {
                                    nodes[k as usize] = nodes[i as usize];
                                    node_shapes[k as usize] = node_shapes[i as usize];
                                    positions[k as usize] = positions[i as usize];
                                    new_positions[i as usize] = k as usize; 
                                    i -= 1;
                                } else {
                                    nodes[k as usize] = NodeLayout { node_index: index_to_add[j as usize].1};
                                    node_shapes[k as usize] = NodeShapeData::default();
                                    positions[k as usize] = NodePosition::default();
                                    j -= 1;
                                }
                                k -= 1;
                            }

                            // now need to set new edge indexes to new ones
                            edges.iter_mut().for_each(|edge| {
                                edge.from = new_positions[edge.from];
                                edge.to = new_positions[edge.to];                                   
                            });
                        }
                    }
                }
            }
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

    // use retain operation if possible. Do not call it in the loop
    pub fn remove(&mut self, value: IriIndex, hidden_predicates: &SortedVec) {
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
                update_edges_groups(edges, hidden_predicates);
            }
        });
    }

    pub fn retain(&mut self, hidden_predicates: &SortedVec, f: impl Fn(&NodeLayout) -> bool) {
        let pos_to_remove = if let Ok(nodes) = self.nodes.read() {
            let pos_to_remove : Vec<usize> = nodes.iter().enumerate().filter(|(_node_pos, node)| {
                !f(node)
            }).map(|(node_pos, _node)| node_pos).collect();
            pos_to_remove
        } else {
            Vec::new()
        };
        if !pos_to_remove.is_empty() {
            self.mut_nodes(|nodes, positions, edges, node_shapes| {
                edges.retain(|e| {
                    !pos_to_remove.binary_search(&e.from).is_ok() && !pos_to_remove.binary_search(&e.to).is_ok()
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
                        new_positions[read] = write;
                    }
                    write += 1;
                }
                // Truncate the vector to the new length
                nodes.truncate(write);
                node_shapes.truncate(write);
                positions.truncate(write);
                edges.iter_mut().for_each(|edge| {
                    edge.from = new_positions[edge.from];
                    edge.to = new_positions[edge.to];                                   
                });
                update_edges_groups(edges, hidden_predicates);
            });
        }        
    }

    /**
     * Removes all nodes by position.
     *
     * The position list must be sorted and unique. Otherwise it will crash.
     */
    pub fn remove_pos_list(&mut self, pos_to_remove: &[usize], hidden_predicates: &SortedVec) {
        self.mut_nodes(|nodes, positions, edges, node_shapes| {
            for pos in pos_to_remove.iter().rev() {
                nodes.remove(*pos);
                if positions.len() > *pos {
                    positions.remove(*pos);
                }
                if node_shapes.len() > *pos {
                    node_shapes.remove(*pos);
                }
                edges.retain(|e| e.from != *pos && e.to != *pos);
                edges.iter_mut().for_each(|e| {
                    if e.from > *pos {
                        e.from -= 1;
                    }
                    if e.to > *pos {
                        e.to -= 1;
                    }
                });
            }
            update_edges_groups(edges, hidden_predicates);
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
        let mut keep_temperature = self.keep_temperature.load(Ordering::Relaxed);
        if ui.checkbox(&mut keep_temperature, "Keep Temparature").changed() {
            self.keep_temperature.store(keep_temperature, Ordering::Relaxed);
        }
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
        #[cfg(target_arch = "wasm32")]
        if self.compute_layout {
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
        let keep_temperature = Arc::clone(&self.keep_temperature);
        let mut layout_config = LayoutConfig {
            repulsion_constant: config.m_repulsion_constant,
            attraction_factor: config.m_attraction_factor,
        };
        self.background_layout_finished.store(false, Ordering::Relaxed);
        self.stop_background_layout.store(false, Ordering::Relaxed);
        let is_done = Arc::clone(&self.background_layout_finished);
        let stop_layout = Arc::clone(&self.stop_background_layout);
        let (tx, rx) = mpsc::channel::<LayoutConfUpdate>();

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
                    layout_graph_nodes(&nodes, &node_shapes, &positions, &edges, &layout_config, temperature)
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
        self.layout_handle = Some(LayoutHandle { join_handle: handle, update_sender: tx.clone() });
        // println!("Background layout thread started");
    }

    pub fn hide_orphans(&mut self, hidden_predicates: &SortedVec) {
        let mut used_positions: Vec<usize> = self
            .edges
            .read()
            .unwrap()
            .iter()
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
            let mut edges_pos_to_remove: Vec<usize> = groups.values().flat_map(|pos_list| {
                if pos_list.len() > 1 {
                    pos_list[1..].to_vec() // Keep the first edge, remove the rest
                } else {
                    Vec::new() // Keep single edges
                }
            }).collect();
            edges_pos_to_remove.sort_unstable();
            edges_pos_to_remove.dedup();
            for pos in edges_pos_to_remove.iter().rev() {
                edges.remove(*pos);
            }
            // println!("Removed {} redundant edges", edges.len());
            update_edges_groups(&mut edges, hidden_predicates);
        }
    }
}

pub fn update_edges_groups(edges: &mut [Edge], hidden_predicates: &SortedVec) {
    // Each group has all edges that connect same nodes (dispite the direction)
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

    let mut tree = BHQuadtree::new(0.5);
     let weight_points: Vec<WeightedPoint> = positions.par_iter().map(|pos| {
        WeightedPoint {
            pos: pos.pos.to_vec2(),
            mass: 1.0,
        }}).collect(); 
    tree.build(weight_points, 5);

    let force_fn = |target: Vec2, source: WeightedPoint| {
        // compute repulsive force
        let dir = target - source.pos;
        if dir.x == 0.0 && dir.y == 0.0 {
            return Vec2::ZERO; // Avoid division by zero
        }
        let dist2 = dir.length();
        let force_mag = (source.mass * repulsion_factor) / dist2;
        (dir/dist2) * force_mag
    };

    let mut forces: Vec<Vec2> = positions.par_iter().map(|node_position| {
        let pos = node_position.pos.to_vec2();
        tree.accumulate(pos, force_fn)
       }).collect();

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
            if len > temperature {
                v = (v / len) * temperature;
                max_move.fetch_max(temperature, Ordering::Relaxed);
            } else {
                max_move.fetch_max(len, Ordering::Relaxed);
            }
            NodePosition { pos: pos + v, vel: v }
        })
        .collect();

    (max_move.load(Ordering::Relaxed), positions)
}

#[cfg(test)]
mod tests {
    use crate::nobject::IriIndex;

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
        assert_eq!(0,sorted_nodes.get_pos(0).unwrap());
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
        let new_pairs: Vec<(IriIndex,IriIndex)> = vec![(0,5),(0,5),(5,2),(5,12),(10,2),(0,3),(0,10)];
        assert!(sorted_nodes.add_many(&new_pairs, |(_parent_index,node_index)| {
            assert_ne!(5,*node_index);
            assert_ne!(10,*node_index);
        }));
        assert!(!sorted_nodes.add_many(&new_pairs, |(_parent_index,node_index)| {
            // This should be never called
            assert!(*node_index>100);
        }));
        for (_parent_idx, node_idx) in &new_pairs {
            assert!(sorted_nodes.contains(*node_idx));
        }
        let to_remove: Vec<IriIndex> = vec![2, 5, 12];
        sorted_nodes.retain(&sorted_vec, |node| {
            !to_remove.contains(&node.node_index)
        });
        for removed_idx in &to_remove {
            assert!(!sorted_nodes.contains(*removed_idx));
        }
        assert!(sorted_nodes.contains(3));
        assert!(sorted_nodes.contains(0));
        assert!(sorted_nodes.contains(10));

    }
}