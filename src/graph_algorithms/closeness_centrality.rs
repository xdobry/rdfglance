use rayon::prelude::*;

use crate::layout::Edge;

pub fn compute_closeness_centrality(nodes_len: usize, edges: &[Edge]) -> Vec<f32> {
    // Precompute adjacency list
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nodes_len];
    for e in edges {
        adj[e.from].push(e.to as u32);
        adj[e.to].push(e.from as u32);
    }

    (0..nodes_len)
        .into_par_iter()
        .map(|i| {
            let mut distances = vec![-1i32; nodes_len];
            let mut queue = std::collections::VecDeque::with_capacity(nodes_len);

            distances[i] = 0;
            queue.push_back(i as u32);

            // BFS for shortest paths
            while let Some(v) = queue.pop_front() {
                for &w in &adj[v as usize] {
                    if distances[w as usize] < 0 {
                        distances[w as usize] = distances[v as usize] + 1;
                        queue.push_back(w);
                    }
                }
            }

            // Compute closeness: (N-1) / sum of distances
            let mut sum_distances = 0i32;
            let mut reachable = 0;
            for (j, &d) in distances.iter().enumerate() {
                if j != i && d > 0 {
                    sum_distances += d;
                    reachable += 1;
                }
            }

            let closeness = if sum_distances > 0 {
                // normalized closeness
                (reachable as f32) / (sum_distances as f32)
            } else {
                0.0
            };

            closeness
        })
        .collect()
}
