use crate::{
    ui::graph_view::{update_layout_edges, NeighborPos}, IriIndex, RdfData, RdfGlanceApp, support::SortedVec};
use std::collections::{HashMap, HashSet, VecDeque};

fn bfs_distances(rdf_data: &RdfData, start: IriIndex, hidden_predicates: &SortedVec) -> HashMap<IriIndex, IriIndex> {
    let mut dist = HashMap::new();
    let mut queue = VecDeque::new();
    dist.insert(start, 0);
    queue.push_back(start);

    while let Some(iri_index) = queue.pop_front() {
        let d = dist[&iri_index];
        if let Some((_str, node)) = rdf_data.node_data.get_node_by_index(iri_index) {
            for (predicate, ref_index) in node.references.iter() {
                if !hidden_predicates.contains(*predicate) && !dist.contains_key(&ref_index) {
                    dist.insert(*ref_index, d + 1);
                    queue.push_back(*ref_index);
                }
            }
            for (predicate, ref_index) in node.reverse_references.iter() {
                if !hidden_predicates.contains(*predicate) && !dist.contains_key(&ref_index) {
                    dist.insert(*ref_index, d + 1);
                    queue.push_back(*ref_index);
                }
            }
        }
    }
    dist
}

fn nodes_on_shortest_paths(rdf_data: &RdfData, start: IriIndex, goal: IriIndex, hidden_predicates: &SortedVec) -> HashSet<IriIndex> {
    let dist_from_start = bfs_distances(rdf_data, start, hidden_predicates);
    let dist_from_goal = bfs_distances(rdf_data, goal, hidden_predicates);

    let mut result = HashSet::new();
    if let Some(&shortest_len) = dist_from_start.get(&goal) {
        for iri_index in 0..rdf_data.node_data.len() {
            let iri_index = iri_index as IriIndex;
            if let (Some(&ds), Some(&dg)) = (dist_from_start.get(&iri_index), dist_from_goal.get(&iri_index)) {
                if ds + dg == shortest_len {
                    result.insert(iri_index);
                }
            }
        }
    }
    result
}

impl RdfGlanceApp {
    pub fn find_connections(&mut self) {
        if self.ui_state.selected_nodes.len() >= 2 {
            let mut iter = self.ui_state.selected_nodes.iter();
            let start = iter.nth(0).unwrap();
            let goal = iter.nth(0).unwrap();
            if let Ok(rdf_data) = self.rdf_data.read() {
                let nodes_to_add = nodes_on_shortest_paths(&rdf_data, *start, *goal, &self.ui_state.hidden_predicates);
                let nodes_to_add: Vec<(IriIndex,IriIndex)> = nodes_to_add.iter().map(|iri_index| (*start,*iri_index)).collect();
                let mut npos = NeighborPos::new();
                npos.add_many(
                    &mut self.visible_nodes,
                    &nodes_to_add,
                    &self.persistent_data.config_data,
                );
                if !npos.is_empty() {
                    update_layout_edges(
                        &npos,
                        &mut self.visible_nodes,
                        &rdf_data.node_data,
                        &self.ui_state.hidden_predicates,
                    );
                    npos.position(&mut self.visible_nodes);
                    self.visible_nodes
                        .start_layout(&self.persistent_data.config_data, &self.ui_state.hidden_predicates);
                }
            }
        }
    }
}
