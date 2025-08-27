use crate::layout::Edge;

pub fn compute_degree_centrality(nodes_len: usize, edges: &[Edge]) -> Vec<f32> {
    let mut result: Vec<f32> = vec![0.0; nodes_len];
    for e in edges {
        result[e.from] += 1.0;
        result[e.to] += 1.0;
    }
    result
}