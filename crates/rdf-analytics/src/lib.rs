pub mod graph_algorithms;
pub mod domain;
pub mod integration;

use rdf_core::IriIndex;

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub predicate: IriIndex,
}

#[derive(Clone, Copy, Debug)]
pub struct ClusteringConfig {
    pub resolution: f32,
    pub randomize: bool,
}

pub fn laplacian_from_adjacency(adj: &nalgebra::DMatrix<f64>) -> nalgebra::DMatrix<f64> {
    assert_eq!(adj.nrows(), adj.ncols(), "adjacency must be square");
    let n = adj.nrows();
    let mut lap = nalgebra::DMatrix::<f64>::zeros(n, n);

    for i in 0..n {
        let degree: f64 = adj.row(i).sum();
        lap[(i, i)] = degree;
        for j in 0..n {
            if i != j {
                lap[(i, j)] = -adj[(i, j)];
            }
        }
    }
    lap
}
