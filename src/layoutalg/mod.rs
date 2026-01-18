pub mod circular;
pub mod hierarchical;
pub mod spectral;
pub mod force;
pub mod overlap;
pub mod ortho;
pub mod linear;
pub mod multipartite;

use std::{collections::BTreeSet, sync::{Arc, RwLock}};

use strum_macros::{EnumIter, Display};

use crate::{IriIndex, domain::{RdfData, graph_styles::GVisualizationStyle}, support::SortedVec, uistate::layout::SortedNodeLayout};

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq)]
pub enum LayoutAlgorithm {
    #[strum(to_string = "Cicular")]
    Circular,
    #[strum(to_string = "Hierarchical Horizontal")]
    HierarchicalHorizontal,
    #[strum(to_string = "Hierarchical Vertical")]
    HierarchicalVertical,
    #[strum(to_string = "Linear Horizontal")]
    LinearHorizontal,
    #[strum(to_string = "Linear Vertical")]
    LinearVertical,
    #[strum(to_string = "Multipartite")]
    Multipartite,
    #[strum(to_string = "Spectral")]
    Spectral,
    #[strum(to_string = "Node Overlap Removal")]
    NodeOverlapRemoval,
    #[strum(to_string = "Orthogonal Edge Routing")]
    Orthogonal,
}

pub fn run_layout_algorithm(algorithm: LayoutAlgorithm, 
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
    visualization_style: &GVisualizationStyle,
    rdf_data: Arc<RwLock<RdfData>>,
) {
    let mut remove_orth = true;
    match algorithm {
        LayoutAlgorithm::Circular => {
            circular::circular_layout(visible_nodes, selected_nodes,hidden_predicates);
        },
        LayoutAlgorithm::HierarchicalHorizontal => {
            hierarchical::hierarchical_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                LayoutOrientation::Horizontal,
            );
        },
        LayoutAlgorithm::HierarchicalVertical => {
            hierarchical::hierarchical_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                LayoutOrientation::Vertical,
            );
        },
        LayoutAlgorithm::LinearHorizontal => {
            linear::linear_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                LayoutOrientation::Horizontal,
            );
        },
        LayoutAlgorithm::LinearVertical => {
            linear::linear_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                LayoutOrientation::Vertical,
            );
        },
        LayoutAlgorithm::Multipartite => {
            multipartite::multipartite_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                visualization_style,
                rdf_data
            );
        },
        LayoutAlgorithm::Spectral => {
            spectral::spectral_layout(visible_nodes, selected_nodes, hidden_predicates);
        },
        LayoutAlgorithm::NodeOverlapRemoval => {
            overlap::nachmanson_layout(visible_nodes, selected_nodes);
        },
        LayoutAlgorithm::Orthogonal => {
            ortho::orthogonal_edge_routing(visible_nodes, selected_nodes, hidden_predicates);
            remove_orth = false;
        },
    }
    if remove_orth {
        visible_nodes.show_orthogonal = false;
        visible_nodes.orth_edges = None;
    }
}

pub enum LayoutOrientation {
    Horizontal,
    Vertical
}