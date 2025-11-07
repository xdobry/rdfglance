pub mod circular;
pub mod hierarchical;
pub mod spectral;
pub mod force;
pub mod overlap;
pub mod ortho;

use std::collections::BTreeSet;

use strum_macros::{EnumIter, Display};

use crate::{IriIndex, support::SortedVec, uistate::layout::SortedNodeLayout};

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq)]
pub enum LayoutAlgorithm {
    #[strum(to_string = "Cicular")]
    Circular,
    #[strum(to_string = "Hierarchical Horizontal")]
    HierarchicalHorizontal,
    #[strum(to_string = "Hierarchical Vertical")]
    HierarchicalVertical,
    #[strum(to_string = "Spectral")]
    Spectral,
    #[strum(to_string = "Node Overlap Removal")]
    NodeOverlapRemoval,
}

pub fn run_layout_algorithm(algorithm: LayoutAlgorithm, 
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec) {
    match algorithm {
        LayoutAlgorithm::Circular => {
            circular::circular_layout(visible_nodes, selected_nodes,hidden_predicates);
        },
        LayoutAlgorithm::HierarchicalHorizontal => {
            hierarchical::hierarchical_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                hierarchical::LayoutOrientation::Horizontal,
            );
        },
        LayoutAlgorithm::HierarchicalVertical => {
            hierarchical::hierarchical_layout(
                visible_nodes,
                selected_nodes,
                hidden_predicates,
                hierarchical::LayoutOrientation::Vertical,
            );
        },
        LayoutAlgorithm::Spectral => {
            spectral::spectral_layout(visible_nodes, selected_nodes, hidden_predicates);
        },
        LayoutAlgorithm::NodeOverlapRemoval => {
            overlap::nachmanson_layout(visible_nodes, selected_nodes);
        },
    }
}