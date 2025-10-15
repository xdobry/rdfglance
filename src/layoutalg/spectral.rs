use crate::{SortedVec, layout::SortedNodeLayout, nobject::IriIndex};
use egui::Pos2;
use nalgebra::linalg::SymmetricEigen;
use nalgebra::{DMatrix, DVector, RowDVector};
use std::collections::BTreeSet;

pub fn spectral_layout(
    visible_nodes: &mut SortedNodeLayout,
    selected_nodes: &BTreeSet<IriIndex>,
    hidden_predicates: &SortedVec,
) {
    let node_indexes: Vec<usize> = if let Ok(nodes) = visible_nodes.nodes.read() {
        selected_nodes
            .iter()
            .filter_map(|selected_node| nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)).ok())
            .collect()
    } else {
        return;
    };
    let n = node_indexes.len();
    if n < 2 {
        return;
    }
    let mut adj = DMatrix::<f64>::zeros(n, n);
    if let Ok(edges) = visible_nodes.edges.read() {
        for edge in edges.iter().filter(|e| !hidden_predicates.contains(e.predicate)) {
            if let (Some(i), Some(j)) = (
                node_indexes.iter().position(|&idx| idx == edge.from),
                node_indexes.iter().position(|&idx| idx == edge.to),
            ) {
                adj[(i, j)] = 1.0;
                adj[(j, i)] = 1.0; // undirected graph
            }
        }
    } else {
        return;
    }
    let lap = laplacian_from_adjacency(&adj);
    let coords = match spectral_layout_from_laplacian(&lap, 2) {
        Ok(c) => c,
        Err(_) => return,
    };
    let coords = rescale_layout(coords, 1.0);
    let scale = 800.0;
    if let Ok(mut positions) = visible_nodes.positions.write() {
        for (i, &node_idx) in node_indexes.iter().enumerate() {
            let x = coords[(i, 0)] * scale;
            let y = coords[(i, 1)] * scale;
            positions[node_idx].pos = Pos2::new(x as f32, y as f32);
        }
    }
}

pub fn rescale_layout(mut pos: DMatrix<f64>, scale: f64) -> DMatrix<f64> {
    let (n, d) = pos.shape();

    // --- Step 1: subtract column means (center)
    let mut mean = RowDVector::zeros(d);
    for j in 0..d {
        mean[j] = pos.column(j).mean();
    }

    // subtract mean from each row
    for i in 0..n {
        for j in 0..d {
            pos[(i, j)] -= mean[j];
        }
    }

    // --- Step 2: find maximum absolute coordinate
    let lim = pos.iter().fold(0.0, |acc: f64, &x| acc.max(x.abs()));

    // --- Step 3: rescale to (-scale, scale)
    if lim > 0.0 {
        pos *= scale / lim;
    }
    pos
}

/// Compute unnormalized graph Laplacian from adjacency matrix.
/// Assumes adjacency is symmetric and has zeros on diagonal for simple graphs.
fn laplacian_from_adjacency(adj: &DMatrix<f64>) -> DMatrix<f64> {
    assert_eq!(adj.nrows(), adj.ncols(), "adjacency must be square");
    let n = adj.nrows();
    let mut lap = DMatrix::<f64>::zeros(n, n);

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

/// Compute spectral layout coordinates from Laplacian.
/// - `lap`: symmetric Laplacian (n x n)
/// - `dim`: desired output dimension (e.g., 2)
/// Returns n x dim matrix where each row is coordinates for a node.
///
/// Implementation details:
/// - We compute the eigen-decomposition of `lap`.
/// - We sort eigenpairs by eigenvalue ascending.
/// - We skip the first eigenpair corresponding to eigenvalue ≈ 0 (constant vector).
/// - We take the next `dim` eigenvectors as coordinate axes.
fn spectral_layout_from_laplacian(lap: &DMatrix<f64>, dim: usize) -> Result<DMatrix<f64>, &'static str> {
    let n = lap.nrows();
    if lap.ncols() != n {
        return Err("laplacian must be square");
    }
    if dim == 0 || dim >= n {
        return Err("dim must be >=1 and < n");
    }

    // Symmetric eigen-decomposition (since Laplacian is symmetric)
    let se = SymmetricEigen::new(lap.clone());

    // se.eigenvalues: DVector<f64> (length n)
    // se.eigenvectors: DMatrix<f64> (n x n) columns are eigenvectors
    let mut pairs: Vec<(f64, DVector<f64>)> = Vec::with_capacity(n);
    for i in 0..n {
        let eigval = se.eigenvalues[i];
        let eigvec = se.eigenvectors.column(i).into_owned();
        pairs.push((eigval, eigvec));
    }

    // Sort by eigenvalue ascending
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // Find first index to use (skip smallest approximately zero eigenvalue)
    // We'll skip the first eigenvalue (usually zero for Laplacian) and take the
    // next `dim` eigenvectors. If there are multiple zeros because graph is disconnected,
    // this still picks the first non-trivial eigenvectors after those zeros.
    let mut start = 0usize;
    // consider small tolerance to detect zero
    let tol = 1e-9_f64;
    while start < n && pairs[start].0.abs() <= tol {
        start += 1;
    }
    // If all eigenvalues are ~0 (degenerate), just set start = 1 to avoid zero vector
    if start == n {
        start = 1.min(n.saturating_sub(1));
    }
    if start + dim > n {
        // not enough eigenvectors after skipping zeros; fallback: take from index 1
        if 1 + dim <= n {
            start = 1;
        } else {
            return Err("not enough eigenvectors to produce requested dimension");
        }
    }

    // Build coordinates matrix: n rows, dim columns
    let mut coords = DMatrix::<f64>::zeros(n, dim);
    for d in 0..dim {
        let vec = &pairs[start + d].1;
        for i in 0..n {
            coords[(i, d)] = vec[i];
        }
    }
    
    Ok(coords)
}

#[cfg(test)]
mod tests {
    use nalgebra::DMatrix;
    use crate::layoutalg::spectral::*;

    #[test]
    fn test_spectral_from_adj_matrix() {
        // Example adjacency for a 6-node graph (symmetric, undirected)
        // Graph: two triangles connected by one bridge, for example
        let adj = DMatrix::from_row_slice(
            6,
            6,
            &[
                0.0, 1.0, 1.0, 0.0, 0.0, 0.0, // node 0
                1.0, 0.0, 1.0, 0.0, 0.0, 0.0, // node 1
                1.0, 1.0, 0.0, 1.0, 0.0, 0.0, // node 2 (bridge to node 3)
                0.0, 0.0, 1.0, 0.0, 1.0, 1.0, // node 3
                0.0, 0.0, 0.0, 1.0, 0.0, 1.0, // node 4
                0.0, 0.0, 0.0, 1.0, 1.0, 0.0, // node 5
            ],
        );

        let lap = laplacian_from_adjacency(&adj);
        println!("Laplacian:\n{}", lap);

        // compute 2D spectral layout (use eigenvectors 2 and 3)
        let coords = spectral_layout_from_laplacian(&lap, 2).unwrap();
        println!("Coordinates (rows are nodes):\n{}", coords);

        // Print coords per node for clarity
        for i in 0..coords.nrows() {
            println!("node {}: ({:.6}, {:.6})", i, coords[(i, 0)], coords[(i, 1)]);
        }
    }
}
