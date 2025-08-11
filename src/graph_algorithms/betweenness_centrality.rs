use std::collections::VecDeque;
use rayon::prelude::*;

use crate::layout::Edge;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BetweennessCentralityResult {
    pub node_betweenness: f32,
}

impl Default for BetweennessCentralityResult {
    fn default() -> Self {
        Self {
            node_betweenness: 0.0,
        }
    }
}

pub fn compute_betweenness_centrality(nodes_len: usize, edges: &[Edge]) -> Vec<BetweennessCentralityResult> {
    // Precompute adjacency list
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nodes_len];
    for e in edges {
        adj[e.from].push(e.to as u32);
        adj[e.to].push(e.from as u32);
    }

    let centrality = (0..nodes_len)
        .into_par_iter()
        .map(|i| {
            let mut distances = vec![-1; nodes_len];
            let mut sigma = vec![0u32; nodes_len];
            let mut stack = Vec::with_capacity(nodes_len);
            let mut queue = VecDeque::with_capacity(nodes_len);
            let mut predecessors: Vec<Vec<u32>> = vec![Vec::new(); nodes_len];
            let mut delta = vec![0.0f32; nodes_len];

            distances[i] = 0;
            sigma[i] = 1;
            queue.push_back(i as u32);

            // BFS
            while let Some(v) = queue.pop_front() {
                stack.push(v);
                for &w in &adj[v as usize] {
                    if distances[w as usize] < 0 {
                        distances[w as usize] = distances[v as usize] + 1;
                        queue.push_back(w);
                    }
                    if distances[w as usize] == distances[v as usize] + 1 {
                        sigma[w as usize] += sigma[v as usize];
                        predecessors[w as usize].push(v);
                    }
                }
            }

            // Dependency accumulation
            let mut local = vec![0.0f32; nodes_len];
            while let Some(w) = stack.pop() {
                for &v in &predecessors[w as usize] {
                    delta[v as usize] += (sigma[v as usize] as f32 / sigma[w as usize] as f32)
                        * (1.0 + delta[w as usize]);
                }
                if w != i as u32 {
                    local[w as usize] += delta[w as usize];
                }
            }
            local
        }).reduce(
            || vec![0.0; nodes_len],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(b) {
                    *x += y;
                }
                a
            },
        );

    centrality
        .into_iter()
        .map(|v| BetweennessCentralityResult { node_betweenness: v })
        .collect()
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_alg_betweennes_centrality() {
        // cargo test test_alg_betweennes_centrality -- --nocapture
        use super::*;
        let nodes_len = 5;
        // Graph structure:
        // 0 -- 1 -- 3 --  4
        //  \-- 2 --/
    
        let edges = vec![
            Edge {from:0,to:1, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 1, to: 3, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 0, to: 2, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 3, to: 4, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 2, to: 3, predicate: 0, bezier_distance: 0.0 },
        ];
        let centrality = compute_betweenness_centrality(nodes_len, &edges);
        assert_eq!(centrality.len(), nodes_len);
        let should_centrality = [1.0,2.0,2.0,7.0,0.0];
        for i in 0..nodes_len {
            println!("Node {}: Betweenness Centrality = {}", i, centrality[i].node_betweenness);
            assert!(centrality[i].node_betweenness >= 0.0);
            assert_eq!(should_centrality[i],centrality[i].node_betweenness);
        }       
    }
}