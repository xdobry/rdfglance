pub mod betweenness_centrality;
pub mod degree;
pub mod closeness_centrality;
pub mod k_core;
pub mod utils;
pub mod eigenvector;
pub mod page_rank;
pub mod louvain;

use crate::{config::Config, graph_algorithms::utils::normalize, layout::Edge};
use strum_macros::{EnumIter, Display};

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq)]
pub enum GraphAlgorithm {
    #[strum(to_string = "Degree Centrality")]
    DegreeCentrality,
    #[strum(to_string = "Betweenness Centrality")]
    BetweennessCentrality,
    #[strum(to_string = "Closeness Centrality")]
    ClosenessCentrality,
    #[strum(to_string = "K-Core Centrality")]
    KCoreCentrality,
    #[strum(to_string = "Eigenvector Centrality")]
    EigenvectorCentrality,
    #[strum(to_string = "Page rank")]
    PageRank,
    #[strum(to_string = "Clustering (Louvain)")]
    ClusteringLouvain,
}

impl GraphAlgorithm {
    pub fn is_clustering(&self) -> bool {
        match self {
            GraphAlgorithm::ClusteringLouvain => true,
            _ => false,
        }
    }

}

pub struct ClusterResult {
    pub cluster_size: u32,
    pub node_cluster: Vec<u32>,
}

pub fn run_algorithm(algorithm: GraphAlgorithm, nodes_len: usize, edges: &[Edge]) -> Vec<f32> {
    match algorithm {
        GraphAlgorithm::BetweennessCentrality => {
            let values = betweenness_centrality::compute_betweenness_centrality(nodes_len, edges).into_iter().map(|result| result.node_betweenness).collect::<Vec<f32>>();
            normalize(values)
        }
        GraphAlgorithm::DegreeCentrality => {
            let values = degree::compute_degree_centrality(nodes_len, edges);
            normalize(values)
        }
        GraphAlgorithm::ClosenessCentrality => {
            let values = closeness_centrality::compute_closeness_centrality(nodes_len, edges);
            normalize(values)
        }
        GraphAlgorithm::KCoreCentrality => {
            let values = k_core::compute_k_core(nodes_len, edges);
            normalize(values)
        },
        GraphAlgorithm::EigenvectorCentrality => {
            let values = eigenvector::compute_eigenvector_centrality(nodes_len, edges);
            normalize(values)
        },
        GraphAlgorithm::PageRank => {
            let values = page_rank::compute_page_rank(nodes_len, edges);
            normalize(values)
        },
        GraphAlgorithm::ClusteringLouvain => {
            vec![0.0; nodes_len]
        }
    }
}

pub fn run_clustering_algorithm(algorithm: GraphAlgorithm, nodes_len: usize, edges: &[Edge], config: &Config) -> ClusterResult {
    match algorithm {
        GraphAlgorithm::ClusteringLouvain => {
            louvain::Modularity::louvain(nodes_len as u32, edges, &config)
        }
        _ => {
            panic!("Not a clustering algorithm");
        }
    }
}