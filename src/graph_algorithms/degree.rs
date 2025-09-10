use crate::{layout::Edge, SortedVec};

pub fn compute_degree_centrality(nodes_len: usize, edges: &[Edge], hidden_predicates: &SortedVec) -> Vec<f32> {
    let mut result: Vec<f32> = vec![0.0; nodes_len];
    for e in edges {
        if !hidden_predicates.contains(e.predicate) {
            result[e.from] += 1.0;
            result[e.to] += 1.0;
        }
    }
    result
}