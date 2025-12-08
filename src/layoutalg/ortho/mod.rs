use std::collections::BTreeSet;

use egui::Rect;

use crate::{IriIndex, 
    layoutalg::ortho::{
        routing::{create_routing_graph, map_routes_to_segments, route_edges}, 
        routing_slots::{calculate_edge_routes, create_channel_connectors},
        sizelayout::resize_channels,
    }, 
    support::SortedVec, 
    uistate::layout::{Edge, OrthEdge, OrthEdges, SortedNodeLayout}};

pub mod routing;
pub mod sizelayout;
pub mod routing_slots;
pub mod channels;
pub mod route_sorting;

#[cfg(feature = "debug-orth")]
#[macro_export]
macro_rules! dbgorth {
    ($($arg:tt)*) => { println!($($arg)*); }
}

#[cfg(not(feature = "debug-orth"))]
#[macro_export]
macro_rules! dbgorth {
    ($($arg:tt)*) => {};
}

pub fn orthogonal_edge_routing(
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
) {
    if let Ok(nodes) = visible_nodes.nodes.read() {
        if let Ok(edges) = visible_nodes.edges.read() {
            if let Ok(mut positions) = visible_nodes.positions.write() {
                if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
                    /*
                    let boxes: Vec<Rect> = nodes.iter().zip(positions.iter().zip(node_shapes.iter()))
                        .filter(|(node,_c)| selected_nodes.contains(&node.node_index))
                        .map(|(_node,(pos, shape))| 
                            Rect::from_center_size(pos.pos, shape.size)                    
                        ).collect();
                     */
                    // We just take all nodes not only the selected ones for now
                    let mut boxes: Vec<Rect> = positions.iter().zip(node_shapes.iter())
                        .map(|(pos, shape)| 
                            Rect::from_center_size(pos.pos, shape.size)                    
                        ).collect();
                    let g_edges: Vec<Edge> = edges
                                .iter()
                                .filter(|e| {
                                    !hidden_predicates.contains(e.predicate) && e.from != e.to
                                })
                                .map(|e| Edge {
                                    from: e.from,
                                    to: e.to,
                                    predicate: e.predicate,
                                    bezier_distance: 0.0,
                                })
                                .collect();
                    
                    let mut routing_graph = create_routing_graph(&boxes);                
                    let mut channel_connectors = create_channel_connectors(&routing_graph, &boxes);
                    let routes = route_edges(&routing_graph, &g_edges, &boxes);
                    let graph_edge_routes = calculate_edge_routes(&routing_graph, &mut channel_connectors, &g_edges, &routes, &boxes);
                                       
                    let min_channel_sizes_vertical: Vec<f32> = graph_edge_routes.channel_slots.iter().take(routing_graph.vchannels.len()).map(|c| 20.0+(*c as f32)*8.0).collect();
                    let min_channel_sizes_horizontal: Vec<f32> = graph_edge_routes.channel_slots.iter().skip(routing_graph.vchannels.len()).map(|c| 20.0+(*c as f32)*8.0).collect();
                    resize_channels(&mut routing_graph, &mut boxes, &min_channel_sizes_vertical, &min_channel_sizes_horizontal);

                    for (pos, rect) in positions.iter_mut().zip(boxes.iter()) {
                        pos.pos = rect.center();
                    }

                    let route_segments = map_routes_to_segments(&routing_graph, &boxes, &routes, &graph_edge_routes);
                    let orth_edges = OrthEdges {
                        edges: route_segments.into_iter().enumerate().map(|(i, segs)| {
                            OrthEdge {
                                from_node: g_edges[i].from,
                                to_node: g_edges[i].to,
                                predicate: g_edges[i].predicate,
                                control_points: segs,
                            }
                        }).collect()
                    };
                    visible_nodes.orth_edges = Some(orth_edges);
                    visible_nodes.show_orthogonal = true;
                }
            }
        }
    }    
}

