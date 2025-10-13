use std::collections::BTreeSet;

use egui::{Pos2, Rect};

use crate::{SortedVec, layout::SortedNodeLayout, nobject::IriIndex};
use rust_sugiyama::{configure::Config, from_vertices_and_edges};

pub fn hierarchical_layout(
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
) {
    let node_indexes: Vec<(u32,(f64,f64))> = if let Ok(nodes) = visible_nodes.nodes.read() {
        if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
            selected_nodes
                .iter()
                .filter_map(|selected_node| {
                    match nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)) {
                        Ok(idx) => Some((idx as u32,(node_shapes[idx].size.x as f64,node_shapes[idx].size.y as f64))),
                        Err(_) => None,
                    }
                })
                .collect()
        } else {
            return;
        }
    } else {
        return;
    };
    let edges: Vec<(u32,u32)> = if let Ok(edges) = visible_nodes.edges.read() {
        edges
            .iter()
            .filter(|e| {
                !hidden_predicates.contains(e.predicate)
                    && (node_indexes.iter().find(|(idx,_)| *idx as usize == e.from || *idx as usize == e.to).is_some())
            })
            .map(|e| (e.from as u32,e.to as u32))
            .collect()
    } else {
        return;
    };
    let layouts = from_vertices_and_edges(
        &node_indexes,
        &edges,
        &Config {
            vertex_spacing: 30.0,
            ..Default::default()
        },
    );
    for (layout, _width, _height) in layouts {
        if let Ok(mut positions) = visible_nodes.positions.write() {
            for (node_index, (x, y)) in layout {
                positions[node_index].pos = Pos2::new(x as f32, -y as f32);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rust_sugiyama::{configure::Config, from_edges};

    #[test]
    fn test_sugiyama_lib() {
        let edges = [
            (0, 1),
            //
            (1, 2),
            (1, 3),
            (1, 4),
            (1, 5),
            (1, 6),
            //
            (3, 7),
            (3, 8),
            //
            (4, 7),
            (4, 8),
            //
            (5, 7),
            (5, 8),
            //
            (6, 7),
            (6, 8),
            //
            (7, 9),
            //
            (8, 9),
        ];

        let layouts = from_edges(
            &edges,
            &Config {
                vertex_spacing: 20.0,
                ..Default::default()
            },
        );

        for (layout, width, height) in layouts {
            println!("Coordinates: {:?}", layout);
            println!("width: {width}, height: {height}");
        }
    }
}
