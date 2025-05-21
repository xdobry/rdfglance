use eframe::egui::Vec2;
use egui::Pos2;
use rand::Rng;
use std::collections::HashMap;

use crate::{config::Config, graph_styles::NodeShape, nobject::{IriIndex, NodeData}, SortedVec};

const MAX_DISTANCE: f32 = 2000.0;
const DUMPING : f32 = 0.2;

pub struct NodeLayout {
    pub node_index: IriIndex,
    pub size: Vec2,
    pub node_shape: NodeShape,  
}

impl NodeLayout {
    pub fn new(node_index: IriIndex) -> Self {
        Self {
            node_index,
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
    pub nodes: Vec<NodeLayout>,
    pub edges: Vec<Edge>,
    pub positions: Vec<NodePosition>,
}

impl Default for SortedNodeLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl SortedNodeLayout {
    pub fn new() -> Self {
        Self { 
            nodes: Vec::new(),
            positions: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn add(&mut self, value: NodeLayout) -> bool {
        match self.nodes.binary_search_by(|e| e.node_index.cmp(&value.node_index)) {
            Ok(_) => false,                              // Value already exists, do nothing
            Err(pos) => {
                 // Insert at correct position
                self.nodes.insert(pos, value);
                self.positions.insert(pos, NodePosition::default());
                for i in 0..self.edges.len() {
                    if self.edges[i].from >= pos {
                        self.edges[i].from += 1;
                    }
                    if self.edges[i].to >= pos {
                        self.edges[i].to += 1;
                    }
                }
                true
             }
        }
    }

    pub fn add_by_index(&mut self, value: IriIndex) -> bool {
        self.add(NodeLayout::new(value))
    }

    pub fn contains(&self, value: IriIndex) -> bool {
        self.nodes.binary_search_by(|e| e.node_index.cmp(&value)).is_ok()
    }

    pub fn remove(&mut self, value: IriIndex) {
        if let Ok(pos) = self.nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
            self.nodes.remove(pos);
            if self.positions.len() > pos {
                self.positions.remove(pos);
            }
            self.edges.retain(|e| e.from != pos && e.to != pos);
            self.edges.iter_mut().for_each(|e| {
                if e.from > pos {
                    e.from -= 1;
                }
                if e.to > pos {
                    e.to -= 1;
                }
            });
            update_edges_groups(&mut self.edges);
        }
    }

    pub fn retain(&mut self, f: impl Fn(&NodeLayout) -> bool) {
        while let Some(pos) = self.nodes.iter().position(|e| !f(e)) {
            self.nodes.remove(pos);
            if self.positions.len() > pos {
                self.positions.remove(pos);
            }
            self.edges.retain(|e| e.from != pos && e.to != pos);
            self.edges.iter_mut().for_each(|e| {
                if e.from > pos {
                    e.from -= 1;
                }
                if e.to > pos {
                    e.to -= 1;
                }
            });
        }
        update_edges_groups(&mut self.edges);
    }

    pub fn get(&self, value: IriIndex) -> Option<&NodeLayout> {
        if let Ok(pos) = self.nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
            return Some(&self.nodes[pos]);
        }
        None
    }

    pub fn get_pos(&self, value: IriIndex) -> Option<(&NodeLayout, usize)> {
        if let Ok(pos) = self.nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
            return Some((&self.nodes[pos],pos));
        }
        None
    }

    pub fn get_mut(&mut self, value: IriIndex) -> Option<&mut NodeLayout> {
        if let Ok(pos) = self.nodes.binary_search_by(|e| e.node_index.cmp(&value)) {
            return Some(&mut self.nodes[pos]);
        }
        None
    }

    pub fn to_center(&mut self) {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut count = 0;
        for node_pos in self.positions.iter() {
            x += node_pos.pos.x;
            y += node_pos.pos.y;
            count += 1;
        }
        x /= count as f32;
        y /= count as f32;
        for node_pos in self.positions.iter_mut() {
            node_pos.pos.x -= x;
            node_pos.pos.y -= y;
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.positions.clear();
    }


}    

pub fn update_edges_groups(edges: &mut Vec<Edge>) {
    // Each group has all edges that connect same nodes (dispite the direction)
    // It is needed to set parameter for bezier curves
    let mut groups: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (edge_index,edge) in edges.iter().enumerate() {
        groups
            .entry(if edge.from>edge.to { (edge.from, edge.to)} else { (edge.to, edge.from) })
            .or_insert_with(Vec::new)
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
                let full_len = (group.len()-1) as f32 * bezier_gap;
                let mut distance = - full_len / 2.0;
                for edge in group.iter() {
                    let t_edge = &edges[*edge];
                    edges[*edge].bezier_distance = if t_edge.from > t_edge.to { distance } else { -distance };
                    distance += bezier_gap;
                }
            }
        }
    }
}

pub fn layout_graph_nodes(layout_nodes: &SortedNodeLayout, config: &Config, temperature: f32) -> (f32,Vec<NodePosition>) {
    let mut max_move = 0.0;
    if layout_nodes.nodes.is_empty() {
        return (max_move, Vec::new());
    }
    let k = ((500.0*500.0) / (layout_nodes.nodes.len() as f32)).sqrt();
    let repulsion_constant = config.m_repulsion_constant;
    let attraction_constant = config.m_attraction_factor;
    let repulsion_factor: f32 = (repulsion_constant * k).powi(2);
    let attraction = k / attraction_constant;

    let mut forces: Vec<Vec2> = layout_nodes.nodes.iter().zip(layout_nodes.positions.iter()).map(|(node_layout, node_position)| {
        let mut f = Vec2::new(0.0, 0.0);
        for (nnode_layout, nnode_position) in layout_nodes.nodes.iter().zip(layout_nodes.positions.iter()) {
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
    }).collect();

    for edge in layout_nodes.edges.iter() {
        if edge.from != edge.to {
            let node_from = &layout_nodes.nodes[edge.from];
            let node_to = &layout_nodes.nodes[edge.to];
            let position_from = &layout_nodes.positions[edge.from];
            let position_to = &layout_nodes.positions[edge.to];
            let direction = position_from.pos - position_to.pos;
            let distance = direction.length() - node_from.size.x / 2.0 - node_to.size.x / 2.0 - 4.0;
            let force = distance.powi(2) / attraction;
            let force_v = (direction / distance) * force;
            forces[edge.from] -= force_v;
            forces[edge.to] += force_v;
        }
    }

    let positions = forces.iter().zip(layout_nodes.positions.iter()).map(|(f, position)| {
        let mut v = position.vel;
        let pos = position.pos;
        v *= 0.4;
        v += *f * 0.01;
        let len = v.length();
        if len > max_move {
            max_move = len;
        }
        if len > temperature {
            v = (v / len) * temperature;
        }
        NodePosition {
            pos: pos + v,
            vel: v,
        }
    }).collect();

    (max_move, positions)
}

