pub mod betweenness_centrality;
pub mod utils;

use crate::{graph_algorithms::utils::normalize, layout::Edge};
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, Display};

#[derive(Debug, EnumIter, Display, PartialEq)]
pub enum GraphAlgorithm {
    #[strum(to_string = "Betweenness Centrality")]
    BetweennessCentrality,
}

pub fn run_algorithm(algorithm: GraphAlgorithm, nodes_len: usize, edges: &[Edge]) -> Vec<f32> {
    match algorithm {
        GraphAlgorithm::BetweennessCentrality => {
            let values = betweenness_centrality::compute_betweenness_centrality(nodes_len, edges).into_iter().map(|result| result.node_betweenness).collect::<Vec<f32>>();
            normalize(values)
        }
    }
}