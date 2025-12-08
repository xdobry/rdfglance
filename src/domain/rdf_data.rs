use fixedbitset::FixedBitSet;
use std::{
    collections::{BTreeSet, HashSet},
};

use crate::{IriIndex, 
    support::SortedVec, 
    domain::{NodeData, config::Config, prefix_manager::PrefixManager}, 
    integration::rdfwrap::RDFAdapter, 
    ui::graph_view::{NeighborPos, update_layout_edges}, 
    uistate::layout::SortedNodeLayout
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
