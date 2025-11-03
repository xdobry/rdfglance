use nalgebra::DMatrix;
// use lanczos::{Hermitian, Order};

use crate::{
    domain::config::Config, 
    graph_algorithms::ClusterResult, 
    uistate::layout::Edge, 
    layoutalg::spectral::laplacian_from_adjacency, support::SortedVec
};

pub fn cluster_spectral(nodes_len: u32, edges: &[Edge], config: &Config, hidden_predicates: &SortedVec) -> ClusterResult {
    let mut adj = DMatrix::<f64>::zeros(nodes_len as usize, nodes_len as usize);
    for edge in edges.iter().filter(|e| !hidden_predicates.contains(e.predicate)) {
        adj[(edge.from, edge.to)] = 1.0;
        adj[(edge.to, edge.from)] = 1.0; // undirected graph
    }
    let laplacian = laplacian_from_adjacency(&adj);
    // after updating to nalgebra 0.34.1, the lanczos crate does not compile 
    // let eigen = laplacian.eingsh(50, Order::Smallest);
    let eigen = laplacian.symmetric_eigen();

    let fiedler_vector = eigen.eigenvectors.column(1);
    let fiedler_vector: Vec<f32> = fiedler_vector.iter().map(|x| *x as f32).collect();

    let mean = fiedler_vector.iter().sum::<f32>() / fiedler_vector.len() as f32;
    let variance = fiedler_vector.iter().map(|&v| (v - mean).powi(2)).sum::<f32>() / fiedler_vector.len() as f32;
    let sigma = variance.sqrt();
    let threshold = config.community_resolution * sigma;

    let clusters = cluster_fiedler(&fiedler_vector, threshold, (nodes_len / 3) as usize);
    let mut node_cluster: Vec<u32> = vec![0; nodes_len as usize];
    for (i, cluster) in clusters.iter().enumerate() {
        for &node in cluster {
            node_cluster[node] = i as u32;
        }
    }
    ClusterResult {
        cluster_size: 2,
        node_cluster: node_cluster,
        parameters: Some(fiedler_vector)
    }
}

fn cluster_fiedler(
    fiedler_vector: &[f32],
    threshold: f32,
    max_iterations: usize,
) -> Vec<Vec<usize>> {
    let mut clusters: Vec<Vec<usize>> = vec![ (0..fiedler_vector.len()).collect() ];

    for _ in 0..max_iterations {
        let mut new_clusters = Vec::new();

        for cluster in clusters {
            let values: Vec<f32> = cluster.iter().map(|&i| fiedler_vector[i]).collect();
            let min_val = values.iter().copied().fold(f32::INFINITY, f32::min);
            let max_val = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);

            if max_val - min_val <= threshold || cluster.len() <= 1 {
                new_clusters.push(cluster);
            } else {
                // Split cluster into two using 1D k-means (like your previous code)
                let mut center1 = values[0];
                let mut center2 = values[1 % values.len()];
                let mut subcluster1 = Vec::new();
                let mut subcluster2 = Vec::new();

                for _ in 0..10 {
                    subcluster1.clear();
                    subcluster2.clear();
                    for &i in &cluster {
                        let v = fiedler_vector[i];
                        if (v - center1).abs() < (v - center2).abs() {
                            subcluster1.push(i);
                        } else {
                            subcluster2.push(i);
                        }
                    }
                    if !subcluster1.is_empty() {
                        center1 = subcluster1.iter().map(|&i| fiedler_vector[i]).sum::<f32>() / subcluster1.len() as f32;
                    }
                    if !subcluster2.is_empty() {
                        center2 = subcluster2.iter().map(|&i| fiedler_vector[i]).sum::<f32>() / subcluster2.len() as f32;
                    }
                }

                new_clusters.push(subcluster1);
                new_clusters.push(subcluster2);
            }
        }

        clusters = new_clusters;
    }

    clusters
}