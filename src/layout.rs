use eframe::egui::Vec2;
use std::collections::HashMap;

use crate::{config::Config, nobject::{IriIndex, NodeData}, SortedVec};

const MAX_DISTANCE: f32 = 200.0;
const DUMPING : f32 = 0.2;

pub fn layout_graph(objects: &mut NodeData, visible_nodes: &SortedVec, hidden_predicates: &SortedVec, config: &Config) -> f32 {
    let mut max_move = 0.0;
    if visible_nodes.data.is_empty() {
        return max_move;
    }
    let mut moves: HashMap<IriIndex, Vec2> = HashMap::with_capacity(visible_nodes.data.len());
    let repulsion_factor: f32 = config.repulsion_constant * ((500.0*500.0) / (visible_nodes.data.len() as f32));
    for obj_index in visible_nodes.data.iter() {
        let object = objects.get_node_by_index(*obj_index);
        if let Some(object) = object {
            let mut f = Vec2::new(0.0, 0.0);
            for nobj_index in visible_nodes.data.iter() {
                if *nobj_index != *obj_index {
                    let nobject = objects.get_node_by_index(*nobj_index);
                    if let Some(nobject) = nobject {
                        let direction = object.pos - nobject.pos;
                        let distance = direction.length();
                        if distance > 0.0 && distance < MAX_DISTANCE {
                            let force = repulsion_factor / (distance * distance);
                            f += (direction / distance) * force;
                        }
                        for (predicate_iri, refiri) in nobject.references.iter() {
                            if *refiri == *obj_index {
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
                if *refiri!=*obj_index && visible_nodes.contains(*refiri) {
                    if hidden_predicates.contains(*predicate_iri) {
                        continue;
                    }
                    let nobject = objects.get_node_by_index(*refiri);
                    if let Some(nobject) = nobject {
                        let direction = nobject.pos - object.pos;
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
            moves.insert(*obj_index, f);
        }
    }
    for obj_index in visible_nodes.data.iter() {
        let object = objects.get_node_by_index_mut(*obj_index);
        if let Some(object) = object {
            let f = moves.get(&obj_index).unwrap();
            let len = f.length();
            if len > max_move {
                max_move = len;
            }
            object.pos += *f;
        }
    }
    return max_move;
}

