use crate::{
    support::{
        SortedVec, quad_tree::{BHQuadtree, WeightedPoint}
    }, 
    uistate::{
        layout::{Edge, LayoutConfig, NodeLayout, NodePosition, NodeShapeData}
    }
};
use atomic_float::AtomicF32;
use eframe::egui::Vec2;
use rayon::prelude::*;
use std::sync::atomic::Ordering;

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
