pub mod betweenness_centrality;
pub mod degree;
pub mod closeness_centrality;
pub mod k_core;
pub mod utils;
pub mod eigenvector;
pub mod page_rank;
pub mod louvain;
pub mod spectral_clustering;
pub mod find_connections;

use crate::{config::Config, graph_algorithms::utils::normalize, layout::Edge, SortedVec};
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
    #[strum(to_string = "Clustering (Spectral)")]
    ClusteringSpectral,
}

#[derive(Debug, Clone, Copy, Display, PartialEq)]
pub enum StatisticValue {
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
    #[strum(to_string = "Clustering (Spectral)")]
    ClusteringSpectral,
    #[strum(to_string = "Fiedler Vector")]
    FiedlerVector,
}

impl GraphAlgorithm {
    pub fn is_clustering(&self) -> bool {
        matches!(self,GraphAlgorithm::ClusteringLouvain) || matches!(self,GraphAlgorithm::ClusteringSpectral)
    }
    pub fn get_statistics_values(&self) -> Vec<StatisticValue> {
        match self {
            GraphAlgorithm::DegreeCentrality => vec![StatisticValue::DegreeCentrality],
            GraphAlgorithm::BetweennessCentrality => vec![StatisticValue::BetweennessCentrality],
            GraphAlgorithm::ClosenessCentrality => vec![StatisticValue::ClosenessCentrality],
            GraphAlgorithm::KCoreCentrality => vec![StatisticValue::KCoreCentrality],
            GraphAlgorithm::EigenvectorCentrality => vec![StatisticValue::EigenvectorCentrality],
            GraphAlgorithm::PageRank => vec![StatisticValue::PageRank],
            GraphAlgorithm::ClusteringLouvain => vec![StatisticValue::ClusteringLouvain],
            GraphAlgorithm::ClusteringSpectral => vec![StatisticValue::ClusteringSpectral, StatisticValue::FiedlerVector],
        }
    }   
}

pub struct ClusterResult {
    pub cluster_size: u32,
    pub node_cluster: Vec<u32>,
    pub parameters: Option<Vec<f32>>,
}

pub fn run_algorithm(algorithm: GraphAlgorithm, nodes_len: usize, edges: &[Edge], hidden_predicates: &SortedVec) -> Vec<f32> {
    match algorithm {
        GraphAlgorithm::BetweennessCentrality => {
            let values = betweenness_centrality::compute_betweenness_centrality(nodes_len, edges, hidden_predicates).into_iter().map(|result| result.node_betweenness).collect::<Vec<f32>>();
            normalize(values)
        }
        GraphAlgorithm::DegreeCentrality => {
            let values = degree::compute_degree_centrality(nodes_len, edges, hidden_predicates);
            normalize(values)
        }
        GraphAlgorithm::ClosenessCentrality => {
            let values = closeness_centrality::compute_closeness_centrality(nodes_len, edges, hidden_predicates);
            normalize(values)
        }
        GraphAlgorithm::KCoreCentrality => {
            let values = k_core::compute_k_core(nodes_len, edges, hidden_predicates);
            normalize(values)
        },
        GraphAlgorithm::EigenvectorCentrality => {
            let values = eigenvector::compute_eigenvector_centrality(nodes_len, edges, hidden_predicates);
            normalize(values)
        },
        GraphAlgorithm::PageRank => {
            let values = page_rank::compute_page_rank(nodes_len, edges, hidden_predicates);
            normalize(values)
        },
        GraphAlgorithm::ClusteringLouvain => {
            vec![0.0; nodes_len]
        },
        GraphAlgorithm::ClusteringSpectral => {
            vec![0.0; nodes_len]
        }
    }
}

pub fn run_clustering_algorithm(algorithm: GraphAlgorithm, nodes_len: usize, edges: &[Edge], config: &Config, hidden_predicates: &SortedVec) -> ClusterResult {
    match algorithm {
        GraphAlgorithm::ClusteringLouvain => {
            louvain::Modularity::louvain(nodes_len as u32, edges, config, hidden_predicates)
        },
        GraphAlgorithm::ClusteringSpectral => {
            spectral_clustering::cluster_spectral(nodes_len as u32, edges, config, hidden_predicates)
        },
        _ => {
            panic!("Not a clustering algorithm");
        }
    }
}