use std::{
    collections::{BTreeSet, HashMap, HashSet}, hash::Hash, sync::{Arc, RwLock}
};

use egui::{Pos2, Rect};

use crate::{
    IriIndex,
    domain::{RdfData, graph_styles::GVisualizationStyle, rdf_data},
    layoutalg::{
        LayoutOrientation,
        circular::{GEdge, find_components, gen_adj_start_node, random_dfs},
    },
    support::SortedVec,
    uistate::layout::SortedNodeLayout,
};

pub fn multipartite_layout(
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
    visualization_style: &GVisualizationStyle,
    rdf_data: Arc<RwLock<RdfData>>,
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
    let mut adj_map: HashMap<usize, Vec<usize>> = HashMap::with_capacity(node_indexes.len());
    if let Ok(edges) = visible_nodes.edges.read() {
        edges
            .iter()
            .filter(| e| {
                !hidden_predicates.contains(e.predicate)
                    && (node_indexes.contains(&e.from) || node_indexes.contains(&e.to))
            })
            .for_each(|e| {
                adj_map.entry(e.from).or_default().push(e.to);
                adj_map.entry(e.to).or_default().push(e.from);
            });
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
    // First build types list, so create map with type and nodes, the node need to be alligned only to one type
    // We need access to NodeData and VisualisationStyle to get type priorities
    // If there are only one type then return (maybe add message that the layout can not be build)
    // If there are 2 types just build to lines of nodes
    // If there are more then 3 types. Analyse possible connections between or types and build connection graph
    // Arrange connected types in circle and other place on side
    let mut node_types: HashMap<IriIndex, Vec<usize>> = HashMap::new();
    if let Ok(rdf_data) = rdf_data.read() {
        if let Ok(nodes) = visible_nodes.nodes.read() {
            for node_idx in node_indexes.iter() {
                if let Some((_, nnode)) = rdf_data.node_data.get_node_by_index(nodes[*node_idx].node_index) {
                    let htypes = nnode.highest_priority_types(visualization_style);
                    if let Some(first_type) = htypes.first() {
                        node_types.entry(*first_type).or_default().push(*node_idx);
                    }
                }
            }
        }
    } else {
        return;
    }
    if node_types.len() < 2 {
        return;
    }
    let mut xpos = rect.left();
    let mut types_height: HashMap<IriIndex, f32> = HashMap::with_capacity(node_types.len());
    let mut max_height: f32 = 0.0;
    let spacing: f32 = 50.0;
    // Compute each type height and max height of all types
    // needed to align types to center
    for (ntype, nodes) in node_types.iter() {
        let mut start_pos: f32 = 0.0;
        if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
            for node_idx in nodes.iter() {
                let node_size = node_shapes[*node_idx].size.y;
                start_pos += node_size + spacing;
            }
        }
        types_height.insert(*ntype, start_pos);
        max_height = max_height.max(start_pos);
    }
    let type_order = optimize_order(&mut node_types, &adj_map);
    for (type_pos, ntype) in type_order.iter().enumerate() {
        let nodes = node_types.get(ntype).unwrap();
        let mut start_pos: f32 = rect.top() + (max_height - types_height[ntype]) * 0.5;
        let mut max_width: f32 = 0.0;
        if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
            if let Ok(mut positions) = visible_nodes.positions.write() {
                for node_idx in nodes.iter() {
                    if let Some(pos) = positions.get_mut(*node_idx) {
                        let node_size = node_shapes[*node_idx].size.y;
                        let node_xpos = if type_pos == 0 {
                            xpos - node_shapes[*node_idx].size.x * 0.5
                        } else if type_pos == type_order.len() -1 {
                            xpos + node_shapes[*node_idx].size.x * 0.5
                        } else {
                            xpos
                        };
                        pos.pos = Pos2::new(node_xpos, start_pos + node_size * 0.5);
                        start_pos += node_size + spacing;
                        max_width = max_width.max(node_shapes[*node_idx].size.x);
                    }
                }
            }
        }
        xpos += max_height / 1.618 + max_width;
    }
    // Optimierung
    // Ordne die Noden so dass wenige Kreuzungen entstehen. Nehme die Order von den wenigen instanzen und ordne die anderen so dass alle die Kanten haben am anfang stehen
    // und dann die anderen folgen in der Reihenfolge wie sie verbunden sind. Oder lose wenn nicht verbraucht
    // Noch weiter: Die mit vielen Verbindungen in der Mitte, Am Rande, diejenigen die nur einmal verbunden sind.
    // Versuche die Gruppen zentral auszurichten, allign so dass die zu der Partner ausgerichtet sind (allso nach innen)
}

fn optimize_order(nodes: &mut HashMap<IriIndex, Vec<usize>>, adj_map: &HashMap<usize,Vec<usize>>) -> Vec<IriIndex> {
    assert!(nodes.len() >= 2);
    let mut order = nodes.keys().cloned().collect::<Vec<IriIndex>>();
    order.sort_unstable_by(|a,b| nodes[a].len().cmp(&nodes[b].len()));
    let first_type = order.first().unwrap();
    if let Some(nodes) = nodes.get_mut(first_type) {
        nodes.sort_unstable_by(|a,b| adj_map.get(a).map_or(0, |v| v.len()).cmp(&adj_map.get(b).map_or(0, |v| v.len())));
        mountain_order(nodes);
    }   
    for types_arr in order.windows(2) {
        let out_order = nodes.get_mut(&types_arr[1]).unwrap();
        let mut all_nodes: HashSet<usize> = HashSet::<usize>::from_iter(out_order.iter().cloned());
        let mut out_order_new: Vec<usize> = Vec::with_capacity(out_order.len());
        if let Some(in_order) = nodes.get(&types_arr[0]) {
            for (node_pos, in_node) in in_order.iter().enumerate() {               
                let mut connected_nodes: Vec<usize> = Vec::new();
                if let Some(adj_nodes) = adj_map.get(in_node) {
                    for adj_node in adj_nodes.iter() {
                        if all_nodes.contains(adj_node) {
                            connected_nodes.push(*adj_node);
                            all_nodes.remove(adj_node);
                        }
                    }
                }
                if node_pos < in_order.len() / 2 {
                    connected_nodes.sort_unstable_by(|a,b| adj_map.get(a).map_or(0, |v| v.len()).cmp(&adj_map.get(b).map_or(0, |v| v.len())));
                } else {
                    connected_nodes.sort_unstable_by(|a,b| adj_map.get(b).map_or(0, |v| v.len()).cmp(&adj_map.get(a).map_or(0, |v| v.len())).reverse());
                }
                out_order_new.extend(connected_nodes.iter().cloned());
            }
        }
        out_order_new.extend(all_nodes.iter().cloned());
        let out_order = nodes.get_mut(&types_arr[1]).unwrap();
        *out_order = out_order_new;
    }

    order
}

fn mountain_order<T: Clone>(v: &mut Vec<T>) {
    let n = v.len();
    if n < 3 {
        return;
    }
    let mut result = Vec::with_capacity(n);

    // take from start: 0, 2, 4, ...
    for i in (0..n).step_by(2) {
        result.push(v[i].clone());
    }

    // take from end: last, last-2, ...
    let start = if n % 2 == 0 { n - 1 } else { n - 2 };
    for i in (0..=start).rev().step_by(2) {
        result.push(v[i].clone());
    }

    *v = result;
}