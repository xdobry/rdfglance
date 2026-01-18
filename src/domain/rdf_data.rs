use fixedbitset::FixedBitSet;
use std::{cmp::Reverse, collections::{BTreeSet, BinaryHeap, HashSet}};

use crate::{IriIndex, domain::{NodeData, config::Config, prefix_manager::PrefixManager}, integration::rdfwrap::RDFAdapter, support::SortedVec, ui::graph_view::{NeighborPos, update_layout_edges}, uistate::layout::{NodeLayout, SortedNodeLayout}
};

pub struct RdfData {
    pub node_data: NodeData,
    pub prefix_manager: PrefixManager,
}

pub enum ExpandType {
    References,
    ReverseReferences,
    Both,
}

pub struct NodeChangeContext<'a> {
    pub rdfwrap: &'a mut Box<dyn RDFAdapter>,
    pub visible_nodes: &'a mut SortedNodeLayout,
    pub config: &'a Config,
}

impl RdfData {
    pub fn expand_node(
        &mut self,
        iri_indexes: &BTreeSet<IriIndex>,
        expand_type: ExpandType,
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let mut refs_to_expand: Vec<(IriIndex,IriIndex)> = Vec::new();  
        for iri_index in iri_indexes.iter() {
            let nnode = self.node_data.get_node_by_index(*iri_index);
            if let Some((_, nnode)) = nnode {
                match expand_type {
                    ExpandType::References | ExpandType::Both => {
                        for (predicate, ref_iri) in &nnode.references {
                            if !hidden_predicates.contains(*predicate) {
                                refs_to_expand.push((*iri_index, *ref_iri));
                            }
                        }
                    }
                    _ => {}
                }
                match expand_type {
                    ExpandType::ReverseReferences | ExpandType::Both => {
                        for (predicate, ref_iri) in &nnode.reverse_references {
                            if !hidden_predicates.contains(*predicate) {
                                refs_to_expand.push((*iri_index, *ref_iri));
                            }
                        }
                    }
                    _ => {}
                }
            }
        };
        if refs_to_expand.is_empty() {
            return false;
        }
        let mut npos = NeighborPos::new();
        let was_added = npos.add_many(
            node_change_context.visible_nodes,
            &refs_to_expand,
            node_change_context.config,
        );
        if was_added {
            update_layout_edges(
                &npos,
                node_change_context.visible_nodes,
                &self.node_data,
                hidden_predicates,
            );
            npos.position(node_change_context.visible_nodes);
            true
        } else {
            false
        }
    }

    pub fn expand_all_by_types(
        &mut self,
        types: &[IriIndex],
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (predicate, ref_iri) in nnode.references.iter() {
                    if !hidden_predicates.contains(*predicate) {
                        if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                            if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                                parent_ref.push((visible_index.node_index, *ref_iri));
                            }
                        }
                    }
                }
                for (predicate, ref_iri) in nnode.reverse_references.iter() {
                    if !hidden_predicates.contains(*predicate) {
                        if let Some((_, nnode)) = self.node_data.get_node_by_index(*ref_iri) {
                            if nnode.match_types(types) && refs_to_expand.insert(*ref_iri) {
                                parent_ref.push((visible_index.node_index, *ref_iri));
                            }
                        }
                    }
                }
            }
        }
        if parent_ref.is_empty() {
            return false;
        }
        let mut npos = NeighborPos::new();
        let was_added = npos.add_many(
            node_change_context.visible_nodes,
            &parent_ref,
            node_change_context.config,
        );
        if was_added {
            update_layout_edges(
                &npos,
                node_change_context.visible_nodes,
                &self.node_data,
                hidden_predicates,
            );
            npos.position(node_change_context.visible_nodes);
            true
        } else {
            false
        }
    }

    pub fn init_visual_graph(
        &mut self,
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let max_node = self.node_data.iter().enumerate().max_by_key(|(_index, (_str,node)) | 
            if node.has_subject {
                node.references.len()+node.reverse_references.len()
            } else {
                0
            }
        );
        let mut nodes_to_expand: Vec<IriIndex> = Vec::new();
        if let Some((index, (_node_iri,_node))) = max_node {
            node_change_context.visible_nodes.add_by_index(index as IriIndex);
            nodes_to_expand.push(index as IriIndex);
        }
        if nodes_to_expand.is_empty() {
            false
        } else {
            let mut parent_ref: Vec<(IriIndex,IriIndex)> = Vec::new();
            // So we should have max 1 + 5 + 20 nodes at the end
            let expand_levels : Vec<usize> = vec![5,20];
            for n in expand_levels {
                let mut heap = BinaryHeap::with_capacity(n);
                for node_index in nodes_to_expand.iter() {
                    // Search n most references nodes from the expand
                    if let Some((_, nnode)) = self.node_data.get_node_by_index(*node_index) {
                        for (predicate, ref_iri) in nnode.references.iter().chain(nnode.reverse_references.iter()) {
                            if !hidden_predicates.contains(*predicate) {
                                if let Some((_,ref_node)) = self.node_data.get_node_by_index(*ref_iri) {
                                    let ref_count = if ref_node.has_subject {
                                        (ref_node.references.len() + ref_node.reverse_references.len()) as u32
                                    } else {
                                        0
                                    };
                                    let rev = Reverse(ref_count);
                                    if heap.len() < n {
                                        heap.push((rev, *node_index, *ref_iri));
                                    } else if let Some(&(Reverse(min_val), _, _)) = heap.peek() {
                                        if ref_count > min_val {
                                            heap.pop();
                                            heap.push((rev, *node_index, *ref_iri));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                nodes_to_expand.clear();
                while let Some((_, parent_iri, ref_iri)) = heap.pop() {
                    parent_ref.push((parent_iri, ref_iri));
                    nodes_to_expand.push(ref_iri)
                }
            }

            let mut npos = NeighborPos::new();
            let was_added = npos.add_many(
                node_change_context.visible_nodes,
                &parent_ref,
                node_change_context.config,
            );
            if was_added {
                update_layout_edges(
                    &npos,
                    node_change_context.visible_nodes,
                    &self.node_data,
                    hidden_predicates,
                );
                npos.position(node_change_context.visible_nodes);
                true
            } else {
                false
            }
        }
    }

    pub fn load_object_by_index(&mut self, index: IriIndex, node_change_context: &mut NodeChangeContext) -> bool {
        let node = self.node_data.get_node_by_index_mut(index);
        if let Some((node_iri, node)) = node {
            if node.has_subject {
                return node_change_context.visible_nodes.add_by_index(index);
            } else {
                let node_iri = node_iri.clone();
                let new_object = node_change_context.rdfwrap.load_object(&node_iri, &mut self.node_data);
                if let Some(new_object) = new_object {
                    self.node_data.put_node_replace(&node_iri, new_object);
                }
            }
        }
        false
    }

    pub fn expand_all(&mut self, node_change_context: &mut NodeChangeContext, hidden_predicates: &SortedVec) -> bool {
        let mut refs_to_expand: HashSet<IriIndex> = HashSet::new();
        let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
        for visible_index in node_change_context.visible_nodes.nodes.read().unwrap().iter() {
            if let Some((_, nnode)) = self.node_data.get_node_by_index(visible_index.node_index) {
                for (predicate, ref_iri) in nnode.references.iter() {
                    if !hidden_predicates.contains(*predicate) && refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
                for (predicate, ref_iri) in nnode.reverse_references.iter() {
                    if !hidden_predicates.contains(*predicate) && refs_to_expand.insert(*ref_iri) {
                        parent_ref.push((visible_index.node_index, *ref_iri));
                    }
                }
            }
        }
        if parent_ref.is_empty() {
            false
        } else {
            let mut npos = NeighborPos::new();
            let was_added = npos.add_many(
                node_change_context.visible_nodes,
                &parent_ref,
                node_change_context.config,
            );
            if was_added {
                update_layout_edges(
                    &npos,
                    node_change_context.visible_nodes,
                    &self.node_data,
                    hidden_predicates,
                );
                npos.position(node_change_context.visible_nodes);
                true
            } else {
                false
            }
        }
    }

    pub fn unexpand_all(&mut self, node_change_context: &mut NodeChangeContext, hidden_predicates: &SortedVec) -> bool {
        let node_len = node_change_context.visible_nodes.nodes.read().unwrap().len();
        if node_len == 0 {
            return false;
        }
        let mut nodes_bits = FixedBitSet::with_capacity(node_len);
        for edge in node_change_context.visible_nodes.edges.read().unwrap().iter() {
            nodes_bits.insert(edge.from);
        }
        if nodes_bits.is_full() {
            false
        } else {
            let mut nodes_indexes_to_remove: Vec<usize> = nodes_bits.zeroes().collect();
            nodes_indexes_to_remove.sort_unstable();
            node_change_context
                .visible_nodes
                .remove_pos_list(&nodes_indexes_to_remove, hidden_predicates);
            true
        }
    }

    pub fn resolve_rdf_lists(&mut self) {
        self.node_data.resolve_rdf_lists(&self.prefix_manager);
    }
}
