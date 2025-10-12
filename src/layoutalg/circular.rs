use std::{
    collections::{BTreeSet, HashMap, HashSet},
    f32::consts::PI,
};

use egui::{Pos2, Rect};
use rand::{
    Rng,
    rngs::ThreadRng,
    seq::{SliceRandom, index::sample},
};

use crate::{SortedVec, layout::SortedNodeLayout, nobject::IriIndex};

struct GEdge {
    from: usize,
    to: usize,
}

/**
 * It does not only order the nodes in circle but uses genetic algorithms
 * to find be order this way that the sum lenght of edges and amount of crossing are minimal
 */
pub fn circular_layout(
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
    let edges: Vec<GEdge> = if let Ok(edges) = visible_nodes.edges.read() {
        edges
            .iter()
            .filter(|e| {
                !hidden_predicates.contains(e.predicate)
                    && (node_indexes.contains(&e.from) || node_indexes.contains(&e.to))
            })
            .map(|e| GEdge { from: e.from, to: e.to })
            .collect()
    } else {
        return;
    };
    let node_positions: Vec<Pos2> = if let Ok(positions) = visible_nodes.positions.read() {
        node_indexes.iter().map(|idx| positions[*idx].pos).collect()
    } else {
        return;
    };
    let mut rect = Rect::from_pos(node_positions[0]);
    for pos in node_positions.iter() {
        rect.extend_with(*pos);
    }
    let circle_center = rect.center();
    let circle_radius = rect.center().distance(rect.min);
    let mut order: Vec<usize> = Vec::with_capacity(node_indexes.len());
    let components = find_components(&edges, &node_indexes);
    for component in components.iter() {
        if component.len()>2 {
            let comp_edges = edges.iter().filter(|e| component.contains(&e.from) || component.contains(&e.to)).map(|e| GEdge { from: e.from, to: e.to }).collect();
            let best_order = genetic_opt(&comp_edges, 50, 100, 0.5, 0.01);
            order.extend(best_order);
        } else {
            order.extend(component);
        }
    }

    // let best_order = genetic_opt(&edges, 100, 1, 0.0, 0.0);
    if let Ok(mut positions) = visible_nodes.positions.write() {
        let circle_positions = circle_positions(circle_center, circle_radius, node_indexes.len());
        for (index, position) in circle_positions.iter().enumerate() {
            positions[order[index]].pos = *position;
        }
    }
}

fn circle_positions(center: Pos2, radius: f32, n: usize) -> Vec<Pos2> {
    let mut positions = Vec::with_capacity(n);
    for i in 0..n {
        // Angle in radians
        // 0 is at the top of the circle
        let angle = 2.0 * PI * (i as f32) / (n as f32) - PI / 2.0;

        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();

        positions.push(Pos2 { x, y });
    }
    positions
}

fn random_dfs(adj_map: &HashMap<usize, Vec<usize>>, start_node: usize, rng: &mut ThreadRng) -> Vec<usize> {
    let mut visited = HashSet::new();
    let mut stack = vec![start_node];
    let mut order = Vec::new();

    while let Some(node) = stack.pop() {
        if visited.insert(node) {
            order.push(node);
            if let Some(targets) = adj_map.get(&node) {
                let mut shuffled = targets.clone();
                shuffled.shuffle(rng);
                for &n in shuffled.iter() {
                    if !visited.contains(&n) {
                        stack.push(n);
                    }
                }
            }
        }
    }
    order
}

fn gen_adj_start_node(edges: &Vec<GEdge>) -> (HashMap<usize, Vec<usize>>, usize) {
    let mut adj_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for edge in edges.iter() {
        adj_map.entry(edge.from).or_default().push(edge.to);
        adj_map.entry(edge.to).or_default().push(edge.from);
    }

    let mut min_node = None;
    let mut min_neighbors = usize::MAX;
    for (&node, neighbors) in &adj_map {
        if neighbors.len() < min_neighbors {
            min_node = Some(node);
            min_neighbors = neighbors.len();
            if min_neighbors == 1 {
                break;
            }
        }
    }
    (adj_map, min_node.expect("Graph must have at least one node"))
}

fn circular_distance_by_index(i: usize, j: usize, n: usize) -> f64 {
    let d = (i as isize - j as isize).abs() as f64;
    d.min(n as f64 - d)
}

fn circular_cost_crossing_sweepline(order: &[usize], edges: &[GEdge], n: usize) -> f64 {
    // position map
    let pos: HashMap<usize, usize> = order.iter().enumerate().map(|(i, &node)| (node, i)).collect();

    // --- edge lengths ---
    let mut total = 0.0;
    for edge in edges.iter() {
        total += circular_distance_by_index(pos[&edge.from], pos[&edge.to], n);
    }

    // --- prepare intervals ---
    #[derive(Clone, Copy)]
    struct Interval {
        start: usize,
        end: usize,
    }

    let mut intervals: Vec<Interval> = edges
        .iter()
        .map(|e| {
            let p1 = pos[&e.from];
            let p2 = pos[&e.to];
            Interval {
                start: p1.min(p2),
                end: p1.max(p2),
            }
        })
        .collect();

    intervals.sort_by_key(|iv| iv.start); // sort by start position

    let mut active_ends: Vec<Interval> = Vec::new();
    let mut crossings = 0;

    for iv in intervals {
        // all active intervals with end > iv.start are crossing with iv
        crossings += active_ends.iter().filter(| i | {
            let crossing = i.start<iv.start && iv.start<i.end && iv.end>i.end;
            /*
            if crossing {
                println!("crossing iv={}-{} i={}-{}",iv.start,iv.end,i.start,i.end)
            }
             */
            crossing
        }).count();
        active_ends.push(iv);

        active_ends.retain(|i |  i.end>=iv.start);
    }

    total + crossings as f64
}

fn crossover(parent1: &Vec<usize>, parent2: &Vec<usize>, rng: &mut ThreadRng) -> Vec<usize> {
    let size = parent1.len();
    let i = rng.random_range(0..size);
    let j = rng.random_range(0..size);
    let (a, b) = (i.min(j), i.max(j));

    let mut child = vec![usize::MAX; size];
    for i in a..b {
        child[i] = parent1[i];
    }

    let fill: Vec<usize> = parent2.iter().filter(|&x| !child.contains(x)).cloned().collect();

    let mut j = 0;
    for i in 0..size {
        if child[i] == usize::MAX {
            child[i] = fill[j];
            j += 1;
        }
    }

    child
}

fn mutate(individual: &mut Vec<usize>, mutation_rate: f64, rng: &mut ThreadRng) {
    for i in 0..individual.len() {
        if rng.random::<f64>() < mutation_rate {
            let j = rng.random_range(0..individual.len());
            individual.swap(i, j);
        }
    }
}

fn select<'a>(population: &'a [Vec<usize>], fitnesses: &[f64], rng: &mut ThreadRng) -> &'a Vec<usize> {
    let k = 3;
    let indices = sample(rng, population.len(), k);

    // Find the index with minimal fitness
    let best_idx = indices
        .iter()
        .min_by(|&i, &j| fitnesses[i].partial_cmp(&fitnesses[j]).unwrap())
        .unwrap();
    &population[best_idx]
}

fn genetic_opt(
    edges: &Vec<GEdge>,
    population_size: usize,
    generations: usize,
    crossover_rate: f64,
    mutation_rate: f64,
) -> Vec<usize> {
    let (adj_map, start_node) = gen_adj_start_node(&edges);
    let mut rng = rand::rng();
    let n = adj_map.keys().len();
    let mutation_rate = mutation_rate * 10.0 / n as f64;

    let mut population: Vec<Vec<usize>> = (0..population_size)
        .map(|_| random_dfs(&adj_map, start_node, &mut rng))
        .collect();

    let mut best_fitness = f64::INFINITY;
    let mut best_generation : Vec<usize> = (0..n).collect();
    let mut stagnation = 0;
    let max_stagnation = 15;

    for generation in 0..generations {
        let fitnesses: Vec<f64> = population
            .iter()
            .map(|ind| circular_cost_crossing_sweepline(ind, &edges, n))
            .collect();

        let mut new_population = Vec::new();
        while new_population.len() < population_size {
            let parent1 = select(&population, &fitnesses, &mut rng);

            let mut child = if rng.random::<f64>() < crossover_rate {
                let parent2 = select(&population, &fitnesses, &mut rng);
                crossover(&parent1, &parent2, &mut rng)
            } else {
                parent1.clone()
            };

            mutate(&mut child, mutation_rate, &mut rng);
            new_population.push(child);
        }

        population = new_population;

        let mut best_idx = 0;
        let mut current_best = f64::INFINITY;
        for (i, fit) in fitnesses.iter().enumerate() {
            if *fit < current_best {
                current_best = *fit;
                best_generation.clone_from_slice(&population[best_idx]);
                best_idx = i;
            }
        }
        // println!("Gen {}: best fitness = {:.4}", generation, current_best);
        if current_best < best_fitness {
            best_fitness = current_best;
            stagnation = 0; // reset counter

        } else {
            stagnation += 1;
        }
        if stagnation >= max_stagnation {
            // println!("Stopping: no improvement in {} generations", max_stagnation);
            break;
        }
    }

    best_generation
}

pub fn find_components(edges: &Vec<GEdge>, nodes: &Vec<usize>) -> Vec<Vec<usize>> {
    // --- Union-Find (Disjoint Set Union) structure ---
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]); // path compression
        }
        parent[x]
    }

    fn union(parent: &mut Vec<usize>, a: usize, b: usize) {
        let pa = find(parent, a);
        let pb = find(parent, b);
        if pa != pb {
            parent[pb] = pa;
        }
    }

    // --- Step 1: Map node IDs (which might not be contiguous) to indices ---
    let mut index_map: HashMap<usize, usize> = HashMap::new();
    for (i, &node) in nodes.iter().enumerate() {
        index_map.insert(node, i);
    }

    // --- Step 2: Initialize DSU ---
    let n = nodes.len();
    let mut parent: Vec<usize> = (0..n).collect();

    // --- Step 3: Union all connected edges ---
    for edge in edges {
        if let (Some(&i_from), Some(&i_to)) = (index_map.get(&edge.from), index_map.get(&edge.to)) {
            union(&mut parent, i_from, i_to);
        }
    }

    // --- Step 4: Find representatives and group nodes ---
    let mut components_map: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &node) in nodes.iter().enumerate() {
        let root = find(&mut parent, i);
        components_map.entry(root).or_default().push(node);
    }

    // --- Step 5: Return list of components ---
    components_map.into_values().collect()
}

#[cfg(test)]
mod tests {
    fn circular_cost_crossing(order: &Vec<usize>, edges: &Vec<GEdge>, n: usize) -> f64 {
        let pos: HashMap<usize, usize> = order.iter().enumerate().map(|(i, &node)| (node, i)).collect();

        let mut total = 0.0;

        for edge in edges.iter() {
            total += circular_distance_by_index(pos[&edge.from], pos[&edge.to], n);
        }

        // count crossings
        let mut crossings = 0;
        for i in 0..edges.len() {
            let edge1 = &edges[i];
            let (a, b) = {
                let p1 = pos[&edge1.from];
                let p2 = pos[&edge1.to];
                (p1.min(p2), p1.max(p2))
            };
            for j in (i + 1)..edges.len() {
                let edge2 = &edges[j];
                let (c, d) = {
                    let p1 = pos[&edge2.from];
                    let p2 = pos[&edge2.to];
                    (p1.min(p2), p1.max(p2))
                };
                if (a < c && c < b && b < d) || (c < a && a < d && d < b) {
                    crossings += 1;
                }
            }
        }

        total + crossings as f64
    }

    use crate::layoutalg::circular::*;
    #[test]
    fn test_circular_opt() {
        let edges = vec![
            GEdge { from: 6, to: 0 },
            GEdge { from: 7, to: 0 },
            GEdge { from: 7, to: 5 },
            GEdge { from: 4, to: 2 },
            GEdge { from: 0, to: 4 },
            GEdge { from: 6, to: 1 },
            GEdge { from: 0, to: 1 },
            GEdge { from: 6, to: 2 },
            GEdge { from: 3, to: 2 },
        ];
        let seq_order: Vec<usize> = (0..8).collect();
        let seq_cost = circular_cost_crossing_sweepline(&seq_order, &edges, 8);
        let seq_cost2 = circular_cost_crossing(&seq_order, &edges, 8);
        assert_eq!(seq_cost, seq_cost2);

        let best_order = genetic_opt(&edges, 100, 200, 0.8, 0.1);
        let opt_cost = circular_cost_crossing_sweepline(&best_order, &edges, 8);
        let opt_cost2 = circular_cost_crossing(&best_order, &edges, 8);
        assert_eq!(opt_cost, opt_cost2);
        assert!(opt_cost < seq_cost);
        assert_eq!(8, best_order.len());
    }

    #[test]
    fn test_find_components() {
        let edges = vec![
            GEdge { from: 1, to: 2 },
            GEdge { from: 2, to: 3 },
            GEdge { from: 4, to: 5 },
        ];
        let nodes = vec![1, 2, 3, 4, 5, 6];

        let components = find_components(&edges, &nodes);
        assert_eq!(3, components.len());
        println!("{:?}", components);
    }
}
