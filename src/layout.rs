use eframe::egui::Vec2;
use egui::Pos2;
use rand::Rng;
use std::collections::HashMap;

use crate::{config::Config, nobject::{IriIndex, NodeData}, SortedVec};

const MAX_DISTANCE: f32 = 200.0;
const DUMPING : f32 = 0.2;

pub struct NodeLayout {
    pub node_index: IriIndex,
    pub pos: Pos2,
    pub vel: Vec2,   
}

impl NodeLayout {
    pub fn new(node_index: IriIndex) -> Self {
        Self {
            node_index,
            pos: Pos2::new(
                rand::rng().random_range(-100.0..100.0),
                rand::rng().random_range(-100.0..100.0),
            ),
            vel: Vec2::new(0.0, 0.0),
        }
    }
}

pub struct SortedNodeLayout {
    pub data: Vec<NodeLayout>,
}

impl SortedNodeLayout {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn add(&mut self, value: NodeLayout) -> bool {
        match self.data.binary_search_by(|e| e.node_index.cmp(&value.node_index)) {
            Ok(_) => false,                              // Value already exists, do nothing
            Err(pos) => {
                 // Insert at correct position
                self.data.insert(pos, value);
                true
             }
        }
    }

    pub fn add_by_index(&mut self, value: IriIndex) -> bool {
        self.add(NodeLayout::new(value))
    }

    pub fn contains(&self, value: IriIndex) -> bool {
        self.data.binary_search_by(|e| e.node_index.cmp(&value)).is_ok()
    }

    pub fn remove(&mut self, value: IriIndex) {
        if let Ok(pos) = self.data.binary_search_by(|e| e.node_index.cmp(&value)) {
            self.data.remove(pos);
        }
    }

    pub fn get(&self, value: IriIndex) -> Option<&NodeLayout> {
        if let Ok(pos) = self.data.binary_search_by(|e| e.node_index.cmp(&value)) {
            return Some(&self.data[pos]);
        }
        return None;
    }

    pub fn get_mut(&mut self, value: IriIndex) -> Option<&mut NodeLayout> {
        if let Ok(pos) = self.data.binary_search_by(|e| e.node_index.cmp(&value)) {
            return Some(&mut self.data[pos]);
        }
        return None;
    }

    pub fn to_center(&mut self) {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut count = 0;
        for node_layout in self.data.iter() {
            x += node_layout.pos.x;
            y += node_layout.pos.y;
            count += 1;
        }
        x /= count as f32;
        y /= count as f32;
        for node in self.data.iter_mut() {
            node.pos.x -= x;
            node.pos.y -= y;
        }
    }


}    

pub fn layout_graph(objects: &mut NodeData, visible_nodes: &mut SortedNodeLayout, hidden_predicates: &SortedVec, config: &Config) -> f32 {
    let mut max_move = 0.0;
    if visible_nodes.data.is_empty() {
        return max_move;
    }
    let mut moves: HashMap<IriIndex, Vec2> = HashMap::with_capacity(visible_nodes.data.len());
    let repulsion_factor: f32 = config.repulsion_constant * ((500.0*500.0) / (visible_nodes.data.len() as f32));
    for node_layout in visible_nodes.data.iter() {
        let object = objects.get_node_by_index(node_layout.node_index);
        if let Some((_,object)) = object {
            let mut f = Vec2::new(0.0, 0.0);
            for nnode_layout in visible_nodes.data.iter() {
                if nnode_layout.node_index != node_layout.node_index {
                    let nobject = objects.get_node_by_index(nnode_layout.node_index);
                    if let Some((_,nobject)) = nobject {
                        let direction = node_layout.pos - nnode_layout.pos;
                        let distance = direction.length();
                        if distance > 0.0 && distance < MAX_DISTANCE {
                            let force = repulsion_factor / (distance * distance);
                            f += (direction / distance) * force;
                        }
                        for (predicate_iri, refiri) in nobject.references.iter() {
                            if *refiri == node_layout.node_index {
                                if hidden_predicates.contains(*predicate_iri) {
                                    continue;
                                }
                                let force = config.attraction_factor * distance;
                                f -= (direction / distance) * force;
                            }
                        }
                    }
                }
            }
            for (predicate_iri, refiri) in object.references.iter() {
                if *refiri!=node_layout.node_index && visible_nodes.contains(*refiri) {
                    if hidden_predicates.contains(*predicate_iri) {
                        continue;
                    }
                    let nobject = visible_nodes.get(*refiri);
                    if let Some(nobject) = nobject {
                        let direction = nobject.pos - node_layout.pos;
                        let distance = direction.length();
                        if distance > 0.0 {
                            let force = config.attraction_factor * distance;
                            f += (direction / distance) * force;
                        }
                    }
                }
            }
            f *= DUMPING;
            f = f.clamp(Vec2::new(-200.0,-200.0), Vec2::new(200.0,200.0));
            // println!("{}: {:?}", iri, f);
            moves.insert(node_layout.node_index, f);
        }
    }
    for node_layout in visible_nodes.data.iter_mut() {
        let f = moves.get(&node_layout.node_index).unwrap();
        node_layout.vel *= 0.8;
        node_layout.vel += *f;
        let len = node_layout.vel.length();
        if len > max_move {
            max_move = len;
        }
        node_layout.pos += node_layout.vel;
    }
    return max_move;
}

