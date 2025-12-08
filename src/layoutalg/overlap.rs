// The code was AI translated and adapted from the Scala implementation found at:
// https://github.com/WueGD/wueortho
// layout\src\main\scala\wueortho\overlaps\Nachmanson.scala

use egui::{Rect, Vec2};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::{BTreeSet, HashSet};

use crate::IriIndex;
use crate::uistate::layout::SortedNodeLayout;
use delaunator::{Point, triangulate};

// --- Weighted graph types used by algorithm ---
#[derive(Clone, Debug)]
struct WeightedEdge {
    u: usize,
    v: usize,
    weight: f32,
}

#[derive(Clone, Debug)]
struct WeightedDiLink {
    to: usize,
    weight: f32,
}

#[derive(Clone, Debug)]
struct Vertex {
    neighbors: Vec<WeightedDiLink>,
}

#[derive(Clone, Debug)]
struct WeightedDiGraph {
    vertices: Vec<Vertex>,
}

impl WeightedDiGraph {
    fn new(n: usize) -> Self {
        WeightedDiGraph {
            vertices: vec![Vertex { neighbors: Vec::new() }; n],
        }
    }

    fn add_edge(&mut self, u: usize, v: usize, w: f32) {
        self.vertices[u].neighbors.push(WeightedDiLink { to: v, weight: w });
    }
}

// --- Small helper: Kruskal MST (returns edges of MST) ---
fn minimum_spanning_tree(num_nodes: usize, edges: &[WeightedEdge]) -> Vec<WeightedEdge> {
    // simple Kruskal
    let mut edges_sorted = edges.to_vec();
    edges_sorted.sort_by(|a, b| a.weight.partial_cmp(&b.weight).unwrap_or(std::cmp::Ordering::Equal));
    let mut parent: Vec<usize> = (0..num_nodes).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    let mut res = Vec::new();
    for e in edges_sorted {
        let ru = find(&mut parent, e.u);
        let rv = find(&mut parent, e.v);
        if ru != rv {
            parent[ru] = rv;
            res.push(e);
            if res.len() == num_nodes.saturating_sub(1) {
                break;
            }
        }
    }
    res
}

// --- Overlaps detection (sweep-line) ---
fn overlapping_pairs(rects: &[Rect]) -> Vec<(usize, usize)> {
    // Similar to Scala Overlaps.overlappingPairs.
    // Build queue of start/end events on y (using center.y ± span.y).
    #[derive(Clone, Copy)]
    enum Event {
        Start(f32, usize),
        End(f32, usize),
    }
    impl Event {
        fn y(&self) -> f32 {
            match *self {
                Event::Start(y, _) => y,
                Event::End(y, _) => y,
            }
        }
        fn idx(&self) -> usize {
            match *self {
                Event::Start(_, i) => i,
                Event::End(_, i) => i,
            }
        }
        fn is_start(&self) -> bool {
            matches!(*self, Event::Start(_, _))
        }
    }
    let mut queue: Vec<Event> = Vec::new();
    for (i, r) in rects.iter().enumerate() {
        queue.push(Event::Start(r.center().y - r.height() * 0.5, i));
        queue.push(Event::End(r.center().y + r.height() * 0.5, i));
    }
    queue.sort_by(|a, b| a.y().partial_cmp(&b.y()).unwrap_or(std::cmp::Ordering::Equal));
    let mut scanline: HashSet<usize> = HashSet::new();
    let mut results = Vec::new();
    for ev in queue {
        if ev.is_start() {
            let idx = ev.idx();
            for &j in &scanline {
                if rects[j].intersects(rects[idx]) {
                    results.push((j, idx));
                }
            }
            scanline.insert(idx);
        } else {
            scanline.remove(&ev.idx());
        }
    }
    results
}

fn triangulation_centers(centers: &[Point]) -> Vec<(usize, usize)> {
    // https://github.com/mourner/delaunator-rs
    let triangulation = triangulate(centers);
    let mut edges = Vec::new();

    for (i, &h) in triangulation.halfedges.iter().enumerate() {
        if h == usize::MAX || i < h {
            // i < h ensures each undirected edge is included only once
            let from = triangulation.triangles[i];
            let to = triangulation.triangles[(i + 1) % 3 + (i / 3) * 3]; // next vertex in triangle
            let e = if from < to { (from, to) } else { (to, from) };
            edges.push(e);
        }
    }

    edges
}

const EPS: f32 = 1e-8;
const MAX_STEPS: usize = 1024;

fn translation_factor(a: &Rect, b: &Rect) -> f32 {
    let dx = (a.center().x - b.center().x).abs();
    let dy = (a.center().y - b.center().y).abs();
    let wx = a.width() * 0.5 + b.width() * 0.5;
    let wy = a.height() * 0.5 + b.height() * 0.5;
    // avoid division by zero - though if dx or dy zero this may be unstable as noted in scala comment
    let rx = if dx.abs() < EPS { f32::INFINITY } else { wx / dx };
    let ry = if dy.abs() < EPS { f32::INFINITY } else { wy / dy };
    rx.min(ry)
}

fn rect_dist(one: &Rect, other: &Rect) -> f32 {
    let d = (one.center() - other.center()).abs();
    let r = d - (one.size() + other.size()) * 0.5;
    r.length()
}

/// overlapCost(a, b)
fn overlap_cost(a: &Rect, b: &Rect) -> f32 {
    if a.intersects(*b) {
        let s = (a.center() - b.center()).length();
        let t = translation_factor(a, b);
        s - t * s
    } else {
        rect_dist(a, b)
    }
}

/// mk_path fallback - connect consecutive rects
fn mk_path(n: usize) -> Vec<WeightedEdge> {
    let mut edges = Vec::new();
    for i in 0..n.saturating_sub(1) {
        edges.push(WeightedEdge {
            u: i,
            v: i + 1,
            weight: 0.0,
        });
    }
    edges
}

/// Grow: take a tree (MST) and translate subtrees according to negative-weight links
fn grow(tree: &WeightedDiGraph, rects: &[Rect], random: &mut StdRng) -> Vec<Rect> {
    // noise function ~ gaussian small noise
    let rng = random;
    let mut noise = || -> Vec2 {
        // approximate gaussian with rand: use normal distribution if desired; simple small uniform noise works too
        let g1: f32 = rng.random_range(0.0..2.0) - 1.0;
        let g2: f32 = rng.random_range(0.0..2.0) - 1.0;
        Vec2::new(g1 % EPS, g2 % EPS)
    };

    let n = rects.len();
    let mut out: Vec<Option<Rect>> = vec![None; n];
    // Do DFS from root 0, propagate displacement; avoid parent revisits
    fn dfs(
        u: usize,
        parent: Option<usize>,
        disp: Vec2,
        tree: &WeightedDiGraph,
        rects: &[Rect],
        out: &mut [Option<Rect>],
        noise: &mut dyn FnMut() -> Vec2,
    ) {
        let r = rects[u];
        let moved = Rect::from_center_size(r.center() + disp, r.size());
        out[u] = Some(moved);

        for link in &tree.vertices[u].neighbors {
            let v = link.to;
            if Some(v) == parent {
                continue;
            }
            if link.weight <= -EPS {
                let nrect = rects[v];
                let dvec = nrect.center() - rects[u].center();
                let tf = translation_factor(&rects[u], &nrect) - 1.0;
                let n = noise();
                let d = (dvec * tf) + n;
                dfs(v, Some(u), disp + d, tree, rects, out, noise);
            } else {
                dfs(v, Some(u), disp, tree, rects, out, noise);
            }
        }
    }

    if n == 0 {
        return Vec::new();
    }
    dfs(0, None, Vec2::new(0.0, 0.0), tree, rects, &mut out, &mut noise);
    out.into_iter().enumerate().map(|(index,o)| 
        if let Some(r) = o {
            r
        } else {
            // this happen if rects exact overlap so take original position and move slightly
            let r = rects[index];
            Rect::from_center_size(r.center() + Vec2::new(0.01, 0.01), r.size())
        }
    ).collect()
}

/// Step: one iteration of the Nachmanson algorithm. Returns Some(new_rects) or None if no change needed.
fn step(rects: &[Rect], random: &mut StdRng) -> Option<Vec<Rect>> {
    // triangulation on centers (placeholder)
    let centers: Vec<Point> = rects
        .iter()
        .map(|r| {
            let center = r.center();
            Point {
                x: center.x as f64,
                y: center.y as f64,
            }
        })
        .collect();
    let tri_pairs = triangulation_centers(&centers);
    let mut edges: Vec<WeightedEdge> = if !tri_pairs.is_empty() {
        tri_pairs
            .into_iter()
            .map(|(u, v)| {
                let w = overlap_cost(&rects[u], &rects[v]);
                WeightedEdge { u, v, weight: w }
            })
            .collect()
    } else {
        // fallback mk_path
        mk_path(rects.len())
    };

    // compute weights for edges created by mk_path if they currently have weight 0
    for e in &mut edges {
        if e.weight == 0.0 {
            e.weight = overlap_cost(&rects[e.u], &rects[e.v]);
        }
    }

    // If all edges have weight >= -EPS -> maybe need to augment with overlapping pairs
    let any_negative = edges.iter().any(|e| e.weight < -EPS);
    let augmented_opt: Option<Vec<WeightedEdge>> = if !any_negative {
        // find overlapping pairs and compute their weights; include only negative weights
        let overlaps = overlapping_pairs(rects);
        let mut augments = Vec::new();
        for (u, v) in overlaps {
            let w = overlap_cost(&rects[u], &rects[v]);
            if w < -EPS {
                augments.push(WeightedEdge { u, v, weight: w });
            }
        }
        if augments.is_empty() {
            None
        } else {
            let mut all = edges.clone();
            all.extend(augments);
            Some(all)
        }
    } else {
        Some(edges)
    };

    augmented_opt.map(|edges| {
        // Build adjacency (undirected) and run MST
        let num_nodes = rects.len();
        // Build undirected edge list for MST (duplicate both directions when building adjacency for MST if needed)
        // For Kruskal we need edges with u <-> v; our WeightedEdge fits.
        let mst_edges = minimum_spanning_tree(num_nodes, &edges);

        // Build a WeightedDiGraph representing the tree, undirected but neighbors list will include both directions.
        let mut tree = WeightedDiGraph::new(num_nodes);
        for e in &mst_edges {
            tree.add_edge(e.u, e.v, e.weight);
            tree.add_edge(e.v, e.u, e.weight);
        }

        // grow
        grow(&tree, rects, random)
    })
}

/// align: iterate step until no further negative edges / augmentation required
fn align(rects: &[Rect], seed: u64) -> Vec<Rect> {
    let mut rects_cur = rects.to_vec();
    let mut rng = StdRng::seed_from_u64(seed);
    for _stepnum in 0..MAX_STEPS {
        if let Some(next) = step(&rects_cur, &mut rng) {
            rects_cur = next;
        } else {
            return rects_cur;
        }
    }
    // if exceeded max, return current rects
    rects_cur
}

// ---------- Helper skeleton: check_node_overlap ----------
/// Small helper that checks whether any of the selected nodes overlap.
/// Replace or extend with your node-size lookup (label bbox, radius, etc.)
fn check_node_overlap(rects: &[Rect]) -> bool {
    for i in 0..rects.len() {
        for j in (i + 1)..rects.len() {
            if rects[i].intersects(rects[j]) {
                return true;
            }
        }
    }
    false
}

// ---------- Main function with same signature as your circular_layout ----------
pub fn nachmanson_layout(visible_nodes: &mut SortedNodeLayout, selected_nodes: &BTreeSet<IriIndex>) {
    // 1) collect node_indexes (same pattern as your circular layout)
    let node_indexes: Vec<usize> = if let Ok(nodes) = visible_nodes.nodes.read() {
        if selected_nodes.is_empty() {
            (0..nodes.len()).collect()
        } else {
            selected_nodes
                .iter()
                .filter_map(|selected_node| {
                    // find index in nodes by comparing node_index field
                    // Adapt this binary_search to your actual NodeEntry layout
                    nodes.binary_search_by(|e| e.node_index.cmp(&selected_node)).ok()
                })
                .collect()
        }
    } else {
        return;
    };

    let gap = Vec2::new(12.0, 12.0); // optional gap between nodes after alignment

    // 3) node rects, positions +size
    let rects: Vec<Rect> = if let Ok(node_shapes) = visible_nodes.node_shapes.read() {
        if let Ok(positions) = visible_nodes.positions.read() {
            node_indexes
                .iter()
                .map(|&idx| {
                    let pos = positions[idx].pos;
                    let size = node_shapes[idx].size + gap;
                    Rect::from_center_size(pos, size)
                })
                .collect()
        } else {
            return;
        }
    } else {
        return;
    };

    // Quick no-op if nothing overlaps (optional)
    if !check_node_overlap(&rects) {
        // nothing to do
        return;
    }

    // Run alignment (seed can be deterministic or random)
    let seed = 0xfeed_f00d_u64; // deterministic seed: change as needed
    let aligned = align(&rects, seed);

    // Write back aligned centers into visible_nodes.positions
    // NOTE: You need to map aligned rects back to the corresponding positions.
    // We assumed node_indexes[i] corresponds to aligned[i]
    if let Ok(mut positions) = visible_nodes.positions.write() {
        for (i, &node_idx) in node_indexes.iter().enumerate() {
            positions[node_idx].pos = aligned[i].center();
        }
    } else {
        // could not acquire write lock; bail out
        return;
    }
}

#[cfg(test)]
mod tests {
    use crate::uistate::layout::{NodeLayout, NodePosition, NodeShapeData};

    use super::*;
    use egui::{Pos2, Rect};

    #[test]
    fn test_overlap_cost() {
        let r1 = Rect::from_center_size(Pos2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let r2 = Rect::from_center_size(Pos2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
        let cost = overlap_cost(&r1, &r2);
        assert!(cost < 0.0, "Expected negative overlap cost for overlapping rectangles");

        let r3 = Rect::from_center_size(Pos2::new(5.0, 5.0), Vec2::new(2.0, 2.0));
        let cost_non_overlap = overlap_cost(&r1, &r3);
        assert!(
            cost_non_overlap > 0.0,
            "Expected positive overlap cost for non-overlapping rectangles"
        );
    }

    #[test]
    fn test_check_node_overlap() {
        let r1 = Rect::from_center_size(Pos2::new(0.0, 0.0), Vec2::new(2.0, 2.0));
        let r2 = Rect::from_center_size(Pos2::new(1.0, 1.0), Vec2::new(2.0, 2.0));
        let r3 = Rect::from_center_size(Pos2::new(5.0, 5.0), Vec2::new(2.0, 2.0));
        let rects = vec![r1, r2, r3];
        assert!(check_node_overlap(&rects), "Expected overlap between rectangles");

        let r4 = Rect::from_center_size(Pos2::new(10.0, 10.0), Vec2::new(2.0, 2.0));
        let rects_no_overlap = vec![r1, r3, r4];
        assert!(
            !check_node_overlap(&rects_no_overlap),
            "Expected no overlap between rectangles"
        );
    }

    #[test]
    fn test_triangulation_centers() {
        let centers: Vec<Point> = vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 1.0, y: 1.0 },
            Point { x: 1.0, y: 0.0 },
            Point { x: 0.0, y: 1.0 },
        ];
        let tri_pairs = triangulation_centers(&centers);
        assert_eq!(5, tri_pairs.len(), "Expected 5 edges in triangulation of 2 points");
        println!("Triangulation edges: {:?}", tri_pairs);

        let tri = triangulate(&centers);
        println!("Triangles: {:?}", tri.triangles);
        println!("Halfedges: {:?}", tri.halfedges);
        //         3---- 1
        //         |   / |
        //         |  /  |
        //         | /   |
        //         0-----2
        //  Triangles: [0, 1, 2, 0, 3, 1]
        //  Halfedges: [5, MAX, MAX, MAX, 0]
    }

    #[test]
    fn test_nachmanson_layout_no_overlap() {
        let mut nl = SortedNodeLayout::default();
        nl.mut_nodes(|nodes, positions, _edges, node_shapes, _individual_node_styles| {
            nodes.push(NodeLayout { node_index: 0 });
            nodes.push(NodeLayout { node_index: 1 });
            nodes.push(NodeLayout { node_index: 2 });
            positions.push(NodePosition {
                pos: Pos2::new(0.0, 0.5),
                ..Default::default()
            });
            positions.push(NodePosition {
                pos: Pos2::new(1.0, 0.0),
                ..Default::default()
            });
            positions.push(NodePosition {
                pos: Pos2::new(-1.0, 0.0),
                ..Default::default()
            });
            node_shapes.push(NodeShapeData {
                size: Vec2::new(2.0, 2.0),
                ..Default::default()
            });
            node_shapes.push(NodeShapeData {
                size: Vec2::new(2.0, 2.0),
                ..Default::default()
            });
            node_shapes.push(NodeShapeData {
                size: Vec2::new(2.0, 2.0),
                ..Default::default()
            });
        });
        let selected: BTreeSet<IriIndex> = vec![0, 1, 2].into_iter().collect();
        nachmanson_layout(&mut nl, &selected);
        let positions = nl.positions.read().unwrap();
        for (idx, pos) in positions.iter().enumerate() {
            println!("{} : {:?}", idx, pos.pos);
        }
    }

    #[test]
    fn test_nachmanson_layout_no_overlap_same() {
        let mut nl = SortedNodeLayout::default();
        let len = nl.mut_nodes(|nodes, positions, _edges, node_shapes, _individual_node_styles| {
            let rects: Vec<[f32;4]> = vec![
                [0.0, 0.0, 2.0, 2.0],
                [1.0, 1.0, 2.0, 2.0],
                [-1.0, -1.0, 2.0, 2.0],
                [1.0, 1.0, 2.0, 2.0],
                [1.1, 1.3, 2.0, 2.0],
            ];
            for (pos,r) in rects.iter().enumerate() {
                nodes.push(NodeLayout { node_index: pos as IriIndex });
                positions.push(NodePosition {
                    pos: Pos2::new(r[0], r[1]),
                    ..Default::default()
                });
                node_shapes.push(NodeShapeData {
                    size: Vec2::new(r[2], r[3]),
                    ..Default::default()
                });
            }
            nodes.len()
        });
        let len32: IriIndex = len.unwrap() as IriIndex;
        let selected: BTreeSet<IriIndex> = (0..len32).collect();
        nachmanson_layout(&mut nl, &selected);
        let positions = nl.positions.read().unwrap();
        for (idx, pos) in positions.iter().enumerate() {
            println!("{} : {:?}", idx, pos.pos);
        }
    }
}
