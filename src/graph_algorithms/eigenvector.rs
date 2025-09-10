use crate::{layout::Edge, SortedVec};

pub fn compute_eigenvector_centrality(nodes_len: usize, edges: &[Edge], hidden_predicates: &SortedVec) -> Vec<f32> {
    // Build adjacency list
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes_len];
    for e in edges {
        if !hidden_predicates.contains(e.predicate) {
            adj[e.from].push(e.to);
            adj[e.to].push(e.from); // assuming undirected graph
        }
    }

    // Initialize centrality values (uniform distribution)
    let mut centrality = vec![1.0; nodes_len];
    let mut new_centrality = vec![0.0; nodes_len];

    // Parameters for power iteration
    let max_iter = 100;
    let tol = 1e-6;

    for _ in 0..max_iter {
        // Multiply adjacency * centrality
        for i in 0..nodes_len {
            new_centrality[i] = adj[i]
                .iter()
                .map(|&nbr| centrality[nbr])
                .sum();
        }

        // Normalize (so vector doesn’t blow up)
        let norm: f32 = new_centrality.iter().map(|&x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in new_centrality.iter_mut() {
                *val /= norm;
            }
        }

        // Check convergence
        let diff: f32 = centrality
            .iter()
            .zip(new_centrality.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();

        centrality.clone_from_slice(&new_centrality);

        if diff < tol {
            break;
        }
    }

    centrality
}