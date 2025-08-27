
use crate::layout::Edge;

pub fn compute_k_core(n: usize, edges: &[Edge]) -> Vec<f32> {
    // Build adjacency
    let mut adj = vec![Vec::<usize>::new(); n];
    for e in edges {
        adj[e.from].push(e.to);
        adj[e.to].push(e.from);
    }

    // Current degrees
    let mut deg: Vec<usize> = adj.iter().map(|v| v.len()).collect();
    let max_deg = deg.iter().copied().max().unwrap_or(0);

    // bin[d] = count of nodes with degree d
    let mut bin = vec![0usize; max_deg + 1];
    for &d in &deg { bin[d] += 1; }

    // start[d] = starting index in 'vert' for nodes of degree d
    let mut start = vec![0usize; max_deg + 1];
    {
        let mut sum = 0usize;
        for d in 0..=max_deg {
            start[d] = sum;
            sum += bin[d];
        }
    }

    // vert: nodes ordered by (current) degree; pos[v] = index of v in vert
    let mut vert = vec![0usize; n];
    let mut pos  = vec![0usize; n];
    {
        let mut next = start.clone();
        for v in 0..n {
            let d = deg[v];
            vert[next[d]] = v;
            pos[v] = next[d];
            next[d] += 1;
        }
    }

    // Core numbers
    let mut core = vec![0usize; n];

    // Main peeling loop
    for i in 0..n {
        let v = vert[i];
        core[v] = deg[v];

        // For each neighbor u with higher current degree than v,
        // move u down by one degree (constant time via bins/positions).
        for &u in &adj[v] {
            if deg[u] > deg[v] {
                let du = deg[u];
                let pu = pos[u];
                let pw = start[du];
                let w  = vert[pw];

                if u != w {
                    // swap u with the first node of bin du
                    vert[pu] = w;  pos[w] = pu;
                    vert[pw] = u;  pos[u] = pw;
                }
                start[du] += 1;   // shrink bin du
                deg[u] -= 1;      // u drops to degree du-1
            }
        }
    }

    core.iter().map(|&c| c as f32).collect()
}


#[cfg(test)]
mod tests {
    #[test]
    fn test_alg_k_core() {
        // cargo test test_alg_k_core -- --nocapture
        use super::*;
        let nodes_len = 5;
        // Graph structure:
        //       0(2)
        //      / \
        //    1(2)-2(2)
        //          |
        //        3(1)
        //          |
        //        4(1)
        let edges = vec![
            Edge {from:0,to:1, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 0, to: 2, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 1, to: 2, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 2, to: 3, predicate: 0, bezier_distance: 0.0 },
            Edge { from: 3, to: 4, predicate: 0, bezier_distance: 0.0 },
        ];
        let centrality = compute_k_core(nodes_len, &edges);
        assert_eq!(centrality.len(), nodes_len);
        let should_centrality = [2.0,2.0,2.0,1.0,1.0];
        for i in 0..nodes_len {
            println!("Node {}: K-Core = {}", i, centrality[i]);
            assert!(centrality[i] >= 0.0);
            assert_eq!(should_centrality[i],centrality[i]);
        }       
    }
}