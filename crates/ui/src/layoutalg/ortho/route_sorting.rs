use std::collections::VecDeque;

use crate::dbgorth;

// Module for sorting routes based on topology
// Internally directed graph is used to represent ordering
// After adding all ordering relations, a topological sort can be performed
pub struct TopologyRouting {
    pub routes_order_graph: Vec<Vec<usize>>,
    pub detected_cycles: usize,
}

impl TopologyRouting {
    pub fn new(routes_count: usize) -> Self {
        TopologyRouting {
            routes_order_graph: vec![Vec::new(); routes_count],
            detected_cycles: 0,
        }
    }

    // Add a directed edge from route_greater to route_less
    // return false if adding this edge would create a cycle
    pub fn add_route_ord(&mut self, route_greater: usize, route_less: usize) -> bool {
        assert!(route_greater < self.routes_order_graph.len());
        assert!(route_less < self.routes_order_graph.len());
        dbgorth!("Adding route order: {} over {}", route_greater, route_less);
        // Check for potential cycle
        if self.has_path_bfs(route_less,route_greater) {
            self.detected_cycles += 1;
            return false; // Cycle detected
        }
        self.routes_order_graph[route_greater].push(route_less);
        true
    }

    fn has_path_bfs(&self, start: usize, end: usize) -> bool {
        if start == end {
            return true;
        }

        let mut visited = vec![false; self.routes_order_graph.len()];
        let mut queue = VecDeque::from([start]);

        while let Some(node) = queue.pop_front() {
            if visited[node] {
                continue;
            }
            visited[node] = true;

            for &neighbor in &self.routes_order_graph[node] {
                if neighbor == end {
                    return true;
                }
                if !visited[neighbor] {
                    queue.push_back(neighbor);
                }
            }
        }

        false
    }

    // Perform topological sort, return None if cycle detected
    // the order is greater routes are first
    pub fn topological_sort(&self) -> Vec<usize> {
        let mut in_degree: Vec<usize> = vec![0; self.routes_order_graph.len()];
        for edges in &self.routes_order_graph {
            for &to in edges {
                in_degree[to] += 1;
            }
        }

        let mut zero_in_degree: Vec<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|&(_idx, &deg)| deg == 0)
            .map(|(idx, _deg)| idx)
            .collect();

        let mut sorted: Vec<usize> = Vec::new();

        while let Some(node) = zero_in_degree.pop() {
            sorted.push(node);
            for &neighbor in &self.routes_order_graph[node] {
                in_degree[neighbor] -= 1;
                if in_degree[neighbor] == 0 {
                    zero_in_degree.push(neighbor);
                }
            }
        }

        if sorted.len() == self.routes_order_graph.len() {
            sorted
        } else {
            panic!("Cycle detected in topology routing graph");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_no_cycle() {
        let mut topo = TopologyRouting::new(4);
        assert!(topo.add_route_ord(0, 1));
        assert!(topo.add_route_ord(0, 2));
        assert!(topo.add_route_ord(1, 3));
        assert!(topo.add_route_ord(2, 3));

        let sorted = topo.topological_sort();
        // Possible valid orders: [0,1,2,3] or [0,2,1,3]
        assert_eq!(sorted[0], 0);
        assert_eq!(sorted[3], 3);
    }

    #[test]
    fn test_add_cycle() {
        let mut topo = TopologyRouting::new(3);
        assert!(topo.add_route_ord(0, 1));
        assert!(topo.add_route_ord(1, 2));
        assert!(!topo.add_route_ord(2, 0)); // This should create a cycle
        assert_eq!(topo.detected_cycles, 1);
        let sorted = topo.topological_sort();

        assert_eq!(sorted[0], 0);
        assert_eq!(sorted[1], 1);
        assert_eq!(sorted[2], 2);
    }
}
