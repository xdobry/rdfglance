use crate::{config::Config, graph_algorithms::ClusterResult, layout::Edge};

use rand::Rng;
use std::{collections::{hash_map::Entry, HashMap, HashSet}, hash::Hash};

/**
 * Rewritten von scratch Louvain algorithm for community detection.
 * 
 * The are some rust implementation of the algorithm. But this that I tried
 * ware slower even than java gephi implementation because the modularity was calculated
 * for whole graph and not only considering the q_delta (change of modularity when moving node).
 * Some ideas are taken from gephi implementation but q_delta calculation is adapted to really match
 * the modularity changes.
 * 
 * See python/louvain.py for reference implementation with detailed tests.
 */

type CommunityId = u32;
type NodeId = u32;
pub struct Modularity {
    m: f32, // total weight of edges in the graph
    resolution: f32,
    randomize: bool,
    origin_nodes_community: Vec<CommunityId>,
    nodes: Vec<CNode>,
    communities: Vec<Community>,
    edges: Vec<Vec<WEdge>>
}

impl Modularity {
    pub fn louvain(nodes_len: u32, edges: &[Edge], config: &Config) -> ClusterResult {
        let mut modularity = Self::construct(nodes_len, edges);
        modularity.resolution = config.community_resolution as f32;
        modularity.randomize = config.community_randomize;
        modularity.init_caches();
        modularity.run_louvain()
    }

    fn run_louvain(&mut self) -> ClusterResult {
        let mut some_change = true;
        let mut rng = rand::rng();
        while some_change {
            some_change = false;
            let mut local_change = true;
            while local_change {
                local_change = false;
                let mut step = 0;
                let mut node_index = if self.randomize {rng.random_range(0..self.communities.len()) as u32} else {0 as u32};
                while step < self.communities.len() {
                    let best_community = self.find_best_community(node_index);
                    let node : &CNode = &self.nodes[node_index as usize];
                    if let Some(best_community) = best_community {
                        if best_community != node.community_id {
                            self.move_node_to(node_index, best_community);
                            local_change = true;
                        }
                    }
                    step += 1;
                    node_index = (node_index + 1) % (self.communities.len() as u32);
                }
                some_change = local_change || some_change;
            }
            if some_change {
                self.merge_nodes()
            }
        }
        ClusterResult {
            cluster_size: self.communities.len() as u32,
            node_cluster: self.origin_nodes_community.clone(),
        }
    }

    fn construct(node_len: u32, edges: &[Edge]) -> Self {
        let m = edges.len() as f32 * 2.0;
        let resolution = 1.0;
        let origin_nodes_community = (0..node_len).collect();
        let nodes = (0..node_len).map(|i| CNode::init(i)).collect();
        let communities = (0..node_len).map(|i| Community::init(i)).collect();
        let mut wedges: Vec<Vec<WEdge>> = vec![Vec::new(); node_len as usize];
        for edge in edges {
            let from = edge.from as u32;
            let to = edge.to as u32;
            let weight = 1.0;
            wedges[from as usize].push(WEdge { from, to, weight });
            wedges[to as usize].push(WEdge { from: to, to: from, weight });
        }
        Self {
            m,
            resolution,
            origin_nodes_community,
            communities,
            nodes,
            edges: wedges,
            randomize: true,
        }
    }

    fn init_caches(&mut self) {
        let node_component: Vec<CommunityId> = self.nodes.iter().map(|n| n.community_id).collect();
        for (node_index, node) in self.nodes.iter_mut().enumerate() {
            node.init_cache(&self.edges[node_index], &node_component);
        }

        for community in self.communities.iter_mut() {
            community.total_degree = Modularity::community_total_degree_compute(community, &self.nodes);
        }
    }

    fn community_total_degree_compute(community: &Community, nodes: &Vec<CNode>) -> f32 {
        let mut sum = 0.0;
        for node_index in community.nodes.iter() {
            let node = &nodes[*node_index as usize];
            sum += node.degree;
        }
        sum
    }

    fn find_best_community(&self, node_id: NodeId) -> Option<CommunityId> {
        let mut best: f32 = 0.0;
        let mut best_community = None;
        let node = &self.nodes[node_id as usize];
        for (community_index, shared_degree) in node.communities.iter() {
            if *shared_degree>0.0 {
                let q_value = self.q(node_id, *community_index, *shared_degree);
                if q_value > best {
                    best = q_value;
                    best_community = Some(*community_index);
                }
            }
        }
        return best_community
    }

    fn q(&self, node_id: NodeId, community_id: CommunityId, shared_degree: f32) -> f32 {
        // the formula is 
        // deleta_q = resolution * d_ij/m - (d_i*d_j)/(2*m*m)
        // deleta_q = (resolution*d_ij - (d_i*d_j)/(2*m))/m
        // d_ij = number of edges from node to community
        // d_i = degree of node
        // d_j = total degree of community
        let node = &self.nodes[node_id as usize];
        let current_community = node.community_id;
        let community = &self.communities[community_id as usize];
        if current_community == community_id {
            if community.nodes.len() == 1 {
                0.0
            } else {
                let d_i = node.degree;
                // we simulate the case that the node is removed from current community
                // so the community total degree is reduced by d_i
                let d_j = community.total_degree - d_i;
                let d_ij = shared_degree * 2.0;
                (self.resolution*d_ij-(d_i*d_j)/(self.m * 0.5))/(self.m)
            }
        } else {
            let d_i = node.degree;
            let d_j = community.total_degree;
            let d_ij = shared_degree * 2.0;
            (self.resolution*d_ij-(d_i*d_j)/(self.m * 0.5))/(self.m)
        }
    }

    fn move_node_to(&mut self, node_id: NodeId, community_id: CommunityId) {
        let old_community_id = self.nodes[node_id as usize].community_id;
        let node_degree = self.nodes[node_id as usize].degree;
        let old_community = &mut self.communities[old_community_id as usize];
        old_community.remove_node(node_id, node_degree);
        let new_community = &mut self.communities[community_id as usize];
        new_community.add_node(node_id, node_degree);

        for wedge in self.edges[node_id as usize].iter() {
            let neighbor = wedge.to as usize;
            let neighbor_node = &mut self.nodes[neighbor];
            if let Entry::Occupied(mut entry) = neighbor_node.communities.entry(old_community_id) {
                let new_val = *entry.get() - wedge.weight;
                if new_val == 0.0 {
                    entry.remove();
                } else {
                    *entry.get_mut() = new_val;
                }
            }
            *neighbor_node.communities.entry(community_id).or_insert(0.0 as f32) += wedge.weight;
        }

        self.nodes[node_id as usize].community_id = community_id;
    }

    fn current_partition(&self) -> Vec<CommunityId> {
        self.origin_nodes_community.iter().map(|n| self.nodes[*n as usize].community_id).collect()
    }

    fn merge_nodes(&mut self) {
        // We need new length of nodes, which is number of not empty communities
        // after it the list of edges between communities
        //  we need to map between old community id and new community id
        let mut new_community_count = 0;
        for c in self.communities.iter_mut() {
            if c.nodes.len() > 0 {
                c.next_id = new_community_count;
                new_community_count += 1
            }
        }
        let mut new_communities: Vec<Community> = Vec::with_capacity(new_community_count as usize);
        let mut new_edges: Vec<Vec<WEdge>> = vec![Vec::new(); new_community_count as usize];
        let mut new_nodes: Vec<CNode> = Vec::with_capacity(new_community_count as usize);
        let mut m = 0.0;
        for community in self.communities.iter() {
            if community.nodes.len() == 0 {
                continue;
            }
            let new_community_id = community.next_id;
            new_communities.push(Community::init(new_community_id));
            let mut new_node = CNode::init(new_community_id);
            let mut edges_for_community: HashMap<CommunityId, f32> = HashMap::new();
            let mut self_reference = 0.0;
            for node_id in community.nodes.iter() {
                for wedge in self.edges[*node_id as usize].iter() {
                    let neighbor_community = self.nodes[wedge.to as usize].community_id;
                    let neighbor_community_new = self.communities[neighbor_community as usize].next_id;
                    *edges_for_community.entry(neighbor_community_new).or_insert(0.0) += wedge.weight;
                }
                self_reference += self.nodes[*node_id as usize].self_reference_weight;
            }
            for (neighbor_community, weight) in edges_for_community.iter() {
                m += weight;
                if *neighbor_community == new_community_id {
                    self_reference += weight;
                } else {
                    new_edges[new_community_id as usize].push(WEdge{ from: new_community_id, to: *neighbor_community, weight: *weight});
                }
            }
            new_node.self_reference_weight = self_reference;
            new_nodes.push(new_node);
        }


        for i in 0..self.origin_nodes_community.len() {
            let new_community_old_id = self.nodes[self.origin_nodes_community[i] as usize].community_id;
            let new_community_id = self.communities[new_community_old_id as usize].next_id;
            self.origin_nodes_community[i] = new_community_id;
        }

        self.communities = new_communities;
        self.nodes = new_nodes;
        self.edges = new_edges;
        self.m = m;
        self.init_caches();
    }
}

pub fn compute_modularity(nodes_len: usize, edges: &[Edge], node_community: Vec<CommunityId>) -> f32 {
    let m: f32 = edges.len() as f32;
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); nodes_len];
    for e in edges {
        adj[e.from].push(e.to as u32);
        adj[e.to].push(e.from as u32);
    }

    // community -> list of nodes
    let mut communities: HashMap<CommunityId, Vec<NodeId>> = HashMap::new();

    for (node_id, community_id) in node_community.iter().enumerate() {
        communities.entry(*community_id).or_insert(Vec::new()).push(node_id as u32);
    }

    let mut k = vec![0.0f32; nodes_len];
    for (node,edges) in adj.iter().enumerate() {
        let node_degree = edges.len() as f32;
        k[node] = node_degree;
    }

    let mut Q = 0.0;
    for nodes in communities.values() {
        // sum of weights of internal edges
        let mut in_weight = 0.0;
        let mut tot_degree = 0.0;
        for u in nodes {
            tot_degree += k[*u as usize];
            let edges = adj.get(*u as usize);
            if let Some(edges) = edges {
                for v in edges {
                    if node_community[*v as usize] == node_community[*u as usize] {
                        in_weight += 1.0
                    }
                }
            }
        }
        in_weight /= 2.0;
        Q += in_weight / m - (tot_degree / (2.0*m)).powi(2);
    }
    Q   
}

struct Community {
    next_id: CommunityId,
    nodes: Vec<NodeId>,
    total_degree: f32,
}

impl Community {
    fn init(node_id: NodeId) -> Self {
        Self { nodes: vec![node_id], next_id: 0, total_degree: 0.0}
    }

    fn add_node(&mut self, node_id: NodeId, degree: f32) {
        self.nodes.push(node_id);
        self.total_degree += degree;
    }

    fn remove_node(&mut self, node_id: NodeId, degree: f32) {
        if let Some(pos) = self.nodes.iter().position(|&x| x == node_id) {
            self.nodes.swap_remove(pos);
            self.total_degree -= degree;
        } else {
            panic!("Node {} not found in community {:?}", node_id, self.nodes);
        }
    }
}

struct CNode {
    community_id: CommunityId,
    degree: f32,
    self_reference_weight: f32,
    communities: HashMap<CommunityId, f32>,
}

impl CNode {
    fn init(node_id: NodeId) -> Self {
        Self { 
            community_id: node_id, 
            degree: 0.0, 
            self_reference_weight: 0.0,
            communities: HashMap::new(),
        }
    }

    fn init_cache(&mut self, edges: &Vec<WEdge>, node_component: &Vec<CommunityId>) {
        let mut sum_weights = self.self_reference_weight;
        for edge in edges.iter() {
            sum_weights += edge.weight;
            let neighbor = edge.to as usize;
            let neighbor_community = node_component[neighbor];
            *self.communities.entry(neighbor_community).or_insert(0.0) += edge.weight;
        }
        self.degree = sum_weights;
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
struct WEdge {
    from: NodeId,
    to: NodeId,
    weight: f32,
}

#[cfg(test)]
mod tests {
    use egui::ahash::HashSet;

    use crate::{config::Config, graph_algorithms::louvain::{compute_modularity, Modularity}, layout::Edge};

    fn convert_edges(edges: &Vec<(u32, u32)>) -> (u32, Vec<Edge>) {
        let mut nodes_len = 0;
        let edges: Vec<Edge> = edges
            .iter()
            .map(|(from, to)| {
                if from > &nodes_len {
                    nodes_len = *from;
                }
                if to > &nodes_len {
                    nodes_len = *to;
                }
                Edge {
                    from: *from as usize,
                    to: *to as usize,
                    predicate: 0,
                    bezier_distance: 0.0,
                }
            })
            .collect();
        (nodes_len+1, edges)
    }

    #[test]
    fn test_modularity_basics() {
        // cargo test test_modularity_basics -- --nocapture
        let edges: Vec<(u32, u32)> = vec![
            (0,1),(0,2),
            (2,3),
            (3,4),(3,5),
            (4,5)
        ];
        let (nodes_len, edges) = convert_edges(&edges);
        let mut modularity = Modularity::construct(nodes_len, &edges);
        modularity.init_caches();
        assert_eq!(modularity.nodes.len(), nodes_len as usize);
        assert_eq!(modularity.communities.len(), nodes_len as usize);

        assert_eq!(modularity.nodes[0].degree, 2.0);
        assert_eq!(modularity.nodes[1].degree, 1.0);
        assert_eq!(modularity.nodes[3].degree, 3.0);
        assert_eq!(modularity.communities[0].total_degree, 2.0);
        assert_eq!(modularity.communities[1].total_degree, 1.0);
        assert_eq!(modularity.communities[3].total_degree, 3.0);
        let my_node_communities = &modularity.nodes[0].communities;
        assert_eq!(my_node_communities[&2], 1.0);
        assert_eq!(my_node_communities[&1], 1.0);
        assert_eq!(my_node_communities.len(), 2);

        assert!(modularity.q(0,1,my_node_communities[&2])>= 0.0);
        let expected: HashSet<_> = [0, 3].into_iter().collect();
        let keys = modularity.nodes[2].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);
        let expected: HashSet<_> = [1, 2].into_iter().collect();
        let keys = modularity.nodes[0].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);
        let expected: HashSet<_> = [0].into_iter().collect();
        let keys = modularity.nodes[1].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);

        modularity.move_node_to(1,0);
        assert_eq!(modularity.nodes[1].community_id, 0);
        assert_eq!(modularity.communities[0].nodes.len(),2);
        assert_eq!(modularity.communities[1].nodes.len(),0);
        assert_eq!(modularity.nodes[1].degree, 1.0);
        assert_eq!(modularity.communities[0].total_degree, 3.0);
        assert_eq!(modularity.communities[1].total_degree, 0.0);

        let expected: HashSet<_> = [0,3].into_iter().collect();
        let keys = modularity.nodes[2].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);

        let expected: HashSet<_> = [0,2].into_iter().collect();
        let keys = modularity.nodes[0].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);

        let expected: HashSet<_> = [0].into_iter().collect();
        let keys = modularity.nodes[1].communities.keys().cloned().collect::<HashSet<_>>();
        assert_eq!(keys, expected);

        assert_eq!(modularity.nodes.len(), 6);

        let old_m = modularity.m;
        assert_eq!(old_m, 12.0);
        modularity.merge_nodes();
        let new_m = modularity.m;
        assert_eq!(old_m, new_m);

        assert_eq!(modularity.nodes.len(), 5);
        assert_eq!(modularity.communities.len(), 5);

        assert_eq!(modularity.origin_nodes_community, vec![0,0,1,2,3,4]);
        assert_eq!(modularity.edges[0].len(), 1);
        assert_eq!(modularity.edges[1].len(), 2);
        assert_eq!(modularity.nodes[0].self_reference_weight, 2.0);

        assert_eq!(modularity.communities[0].nodes, vec![0]);
        assert_eq!(modularity.communities[1].nodes, vec![1]);

        let current_partition = modularity.current_partition();
        let modularity_value = compute_modularity(nodes_len as usize, &edges, current_partition);
        println!("Modularity: {}", modularity_value);

        // The value was computer in python version see louvain.py
        assert!((-0.04166666666666667-modularity_value).abs()<0.00001);

        let my_node_communities = &modularity.nodes[1].communities;
        let q_delta = modularity.q(1,0,my_node_communities[&0]);
        assert!((0.08333333333333333-q_delta).abs()<0.00001);

        modularity.move_node_to(1,0);

        let current_partition = modularity.current_partition();
        let new_modularity_value = compute_modularity(nodes_len as usize, &edges, current_partition);
        println!("Modularity after move: {}", new_modularity_value);
        assert!((new_modularity_value-modularity_value-q_delta).abs()<0.00001);

        /*

        assert structure.origin_nodes_community == [0,0,1,2,3,4]
        assert len(structure.edges[0]) == 1
        assert len(structure.edges[1]) == 2
        assert structure.node_selfreference[0] == 2.0
        assert structure.communities[0].nodes == [0]
        assert structure.communities[1].nodes == [1]
       
        #print(f"node degree {structure.node_degree(0)}")
        #assert structure.node_degree(0) == 3.0
        modularity = structure.compute_modularity()
        my_node_communities = structure.node_communities(1)      
        q_delta = structure.q(1,0,my_node_communities[0],1.0)
        structure.moveNodeTo(1,0)
        modularity_new = structure.compute_modularity()
        modularity_new_orig = structure.compute_orig_modularity()
        assert math.isclose(modularity_new, modularity_new_orig)
        print(f"node_selfreference {structure.node_selfreference}")
        print(f"modularity {modularity} new_modularity {modularity_new} q_delta {q_delta} diff {modularity_new - modularity}")
        assert math.isclose(modularity_new,modularity + q_delta)
        */
    }

    #[test]
    fn test_louvain_clustering() {
        // cargo test test_louvain_clustering -- --nocapture
        let edges: Vec<(u32, u32)> = vec![
            (0, 2),
            (0, 3),
            (0, 5),
            (1, 2),
            (1, 4),
            (1, 7),
            (2, 4),
            (2, 5),
            (2, 6),
            (3, 7),
            (4, 10),
            (5, 7),
            (5, 11),
            (6, 7),
            (6, 11),
            (8, 9),
            (8, 10),
            (8, 11),
            (8, 14),
            (8, 15),
            (9, 12),
            (9, 14),
            (10, 11),
            (10, 12),
            (10, 13),
            (10, 14),
            (11, 13),
        ];

        let (nodes_len, edges) = convert_edges(&edges);

        let mut config = Config::default();
        config.community_randomize = false;
        let result = Modularity::louvain(nodes_len, &edges, &config);
        println!("Communities: {:?}", result.cluster_size);
        assert_eq!(3, result.cluster_size);
    }
}
