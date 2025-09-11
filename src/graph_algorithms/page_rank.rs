use crate::{layout::Edge, SortedVec};

pub fn compute_page_rank(nodes_len: usize, edges: &[Edge], hidden_predicates: &SortedVec) -> Vec<f32> {
    // Build adjacency list
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); nodes_len];
    for e in edges {
        if !hidden_predicates.contains(e.predicate) {
            adj[e.from].push(e.to); // directed: from → to
        }
    }

    // Parameters
    let damping: f32 = 0.85;
    let max_iter = 100;
    let tol = 1e-6;

    // Initialize ranks uniformly
    let mut rank = vec![1.0 / nodes_len as f32; nodes_len];
    let mut new_rank = vec![0.0; nodes_len];

    for _ in 0..max_iter {
        // Reset new rank with teleportation factor
        for r in new_rank.iter_mut() {
            *r = (1.0 - damping) / nodes_len as f32;
        }

        // Distribute rank over outgoing edges
        for i in 0..nodes_len {
            if adj[i].is_empty() {
                // Handle dangling nodes (distribute evenly)
                let share = damping * rank[i] / nodes_len as f32;
                for r in new_rank.iter_mut() {
                    *r += share;
                }
            } else {
                let share = damping * rank[i] / adj[i].len() as f32;
                for &nbr in &adj[i] {
                    new_rank[nbr] += share;
                }
            }
        }

        // Check convergence
        let diff: f32 = rank
            .iter()
            .zip(new_rank.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();

        rank.clone_from_slice(&new_rank);

        if diff < tol {
            break;
        }
    }

    rank
}