use std::{
    collections::{BTreeSet, HashMap},
};

use egui::{Pos2, Rect};

use crate::{
    IriIndex,
    layoutalg::{
        LayoutOrientation,
        circular::{GEdge, find_components, gen_adj_start_node, random_dfs},
    },
    support::SortedVec,
    uistate::layout::SortedNodeLayout,
};

pub fn linear_layout(
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
    layout_orientation: LayoutOrientation,
) {
    let node_indexes: Vec<usize> = if let Ok(nodes) = visible_nodes.nodes.read() {
        if selected_nodes.len() < 3 {
            (0..nodes.len()).collect()
        } else {
            selected_nodes
                .iter()
                .filter_map(|selected_node| nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)).ok())
                .collect()
        }
    } else {
        return;
    };
    if node_indexes.len() < 2 {
        return;
    }
    let mut edge_indexes: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    let edges: Vec<GEdge> = if let Ok(edges) = visible_nodes.edges.read() {
        edges
            .iter()
            .enumerate()
            .filter(|(_e_index, e)| {
                !hidden_predicates.contains(e.predicate)
                    && (node_indexes.contains(&e.from) || node_indexes.contains(&e.to))
            })
            .map(|(e_index, e)| {
                edge_indexes.entry((e.from, e.to)).or_default().push(e_index);
                return GEdge { from: e.from, to: e.to };
            })
            .collect()
    } else {
        return;
    };
    let mut rect = Rect::NOTHING;
    if let Ok(positions) = visible_nodes.positions.read() {
        for node_idx in node_indexes.iter() {
            let pos = positions[*node_idx];
            rect.extend_with(pos.pos);
        }
    } else {
        return;
    }
    let center = rect.center();
    let spacing = 50.0;
    let mut start_pos = match layout_orientation {
        LayoutOrientation::Horizontal => rect.left(),
        LayoutOrientation::Vertical => rect.top(),
    };
    let mut order: Vec<usize> = Vec::with_capacity(node_indexes.len());
    let components = find_components(&edges, &node_indexes);
    for component in components.iter() {
        if component.len() > 2 {
            if let Ok(mut edges) = visible_nodes.edges.write() {
                let comp_edges = edges
                    .iter()
                    .filter(|e| component.contains(&e.from) || component.contains(&e.to))
                    .map(|e| GEdge { from: e.from, to: e.to })
                    .collect();
                let best_order = linear_order(&comp_edges);
                for comp_edge in comp_edges.iter() {
                    if comp_edge.from == comp_edge.to {
                        continue;
                    }
                    if let Some(e_indexes) = edge_indexes.get(&(comp_edge.from, comp_edge.to)) {
                        let index_from = best_order.iter().position(|x| *x == comp_edge.from).unwrap();
                        let index_to = best_order.iter().position(|x| *x == comp_edge.to).unwrap();
                        let mut add: f32 = 0.0;
                        for e_index in e_indexes.iter() {
                            edges[*e_index].bezier_distance =
                                spacing * ((index_to as f32 - index_from as f32).abs() - 1.0) + add;
                            add += 5.0;
                        }
                    }
                }
                order.extend(best_order);
            } else {
                return;
            }
        } else {
            order.extend(component);
        }
    }
    if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
        if let Ok(mut positions) = visible_nodes.positions.write() {
            for node_idx in order.iter() {
                if let Some(pos) = positions.get_mut(*node_idx) {
                    match layout_orientation {
                        LayoutOrientation::Horizontal => {
                            let node_size = node_shapes[*node_idx].size.x;
                            pos.pos = Pos2::new(start_pos + node_size*0.5, center.y);
                            start_pos += node_size + spacing;
                        }
                        LayoutOrientation::Vertical => {
                            let node_size = node_shapes[*node_idx].size.y;
                            pos.pos = Pos2::new(center.x, start_pos + node_size * 0.5);
                            start_pos += node_size + spacing;
                        }
                    }
                }
            }
        } else {
            return;
        }
    } else {
        return;
    }
}

fn linear_order(edges: &Vec<GEdge>) -> Vec<usize> {
    let (adj_map, start_node) = gen_adj_start_node(&edges);
    let mut rng = rand::rng();
    let order = random_dfs(&adj_map, start_node, &mut rng);
    order
}
