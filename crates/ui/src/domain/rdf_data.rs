use fixedbitset::FixedBitSet;
use std::{
    cmp::Reverse,
    collections::{BTreeSet, BinaryHeap, HashSet},
};
use rayon::prelude::*;

use crate::{
    IriIndex,
    domain::{Literal, NodeData, config::Config, prefix_manager::PrefixManager},
    integration::rdfwrap::RDFAdapter,
    support::SortedVec,
    ui::graph_view::{NeighborPos, update_layout_edges},
    uistate::layout::SortedNodeLayout,
};

pub struct RdfData {
    pub node_data: NodeData,
    pub prefix_manager: PrefixManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchResult {
    pub node_index: IriIndex,
    pub found_property_index: IriIndex,
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
    pub fn search_nodes(&self, value: &str) -> Vec<SearchResult> {
        if value.is_empty() {
            return Vec::new();
        }

        let short_literal_indexer = &self.node_data.indexers.short_literal_indexer;
        let matching_short_indices: HashSet<IriIndex> = (0..short_literal_indexer.map.len())
            .filter_map(|index| {
                let index = index as IriIndex;
                short_literal_indexer
                    .index_to_str(index)
                    .filter(|literal| literal.contains(value))
                    .map(|_| index)
            })
            .collect();

        let matching_short_indices = &matching_short_indices;
        let indexers = &self.node_data.indexers;
        let mut results: Vec<SearchResult> = (0..self.node_data.len())
            .into_par_iter()
            .flat_map_iter(|node_index| {
                self.node_data
                    .get_node_by_index(node_index as IriIndex)
                    .into_iter()
                    .flat_map(move |(_, node)| {
                        node.properties.iter().filter_map(move |(property_index, literal)| {
                            let found = match literal {
                                Literal::StringShort(index) | Literal::LangShortString(_, index) => {
                                    matching_short_indices.contains(index)
                                }
                                Literal::String(span)
                                | Literal::LangString(_, span)
                                | Literal::TypedString(_, span) => {
                                    indexers.literal_cache.get_str(*span).contains(value)
                                }
                                Literal::NoValue() => false,
                            };
                            found.then_some(SearchResult {
                                node_index: node_index as IriIndex,
                                found_property_index: *property_index,
                            })
                        })
                    })
            })
            .collect();

        results.sort_unstable_by_key(|result| (result.node_index, result.found_property_index));
        results.dedup_by_key(|result| (result.node_index, result.found_property_index));
        results
    }

    pub fn expand_node(
        &mut self,
        iri_indexes: &BTreeSet<IriIndex>,
        expand_type: ExpandType,
        node_change_context: &mut NodeChangeContext,
        hidden_predicates: &SortedVec,
    ) -> bool {
        let mut refs_to_expand: Vec<(IriIndex, IriIndex)> = Vec::new();
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
        }
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
        let max_node = self.node_data.iter().enumerate().max_by_key(|(_index, (_str, node))| {
            if node.has_subject {
                node.references.len() + node.reverse_references.len()
            } else {
                0
            }
        });
        let mut nodes_to_expand: Vec<IriIndex> = Vec::new();
        if let Some((index, (_node_iri, _node))) = max_node {
            node_change_context.visible_nodes.add_by_index(index as IriIndex);
            nodes_to_expand.push(index as IriIndex);
        }
        if nodes_to_expand.is_empty() {
            false
        } else {
            let mut parent_ref: Vec<(IriIndex, IriIndex)> = Vec::new();
            // So we should have max 1 + 5 + 20 nodes at the end
            let expand_levels: Vec<usize> = vec![5, 20];
            for n in expand_levels {
                let mut heap = BinaryHeap::with_capacity(n);
                for node_index in nodes_to_expand.iter() {
                    // Search n most references nodes from the expand
                    if let Some((_, nnode)) = self.node_data.get_node_by_index(*node_index) {
                        for (predicate, ref_iri) in nnode.references.iter().chain(nnode.reverse_references.iter()) {
                            if !hidden_predicates.contains(*predicate) {
                                if let Some((_, ref_node)) = self.node_data.get_node_by_index(*ref_iri) {
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

#[cfg(test)]
mod tests {
    use super::{RdfData, SearchResult};
    use crate::domain::{Literal, NObject, NodeData, PrefixManager};

    #[test]
    fn search_nodes_matches_short_and_span_literals_once_per_property() {
        let mut rdf_data = RdfData {
            node_data: NodeData::new(),
            prefix_manager: PrefixManager::new(),
        };
        let short_index = rdf_data
            .node_data
            .indexers
            .short_literal_indexer
            .get_index("short needle value");
        let matching_span = rdf_data.node_data.indexers.literal_cache.push_str("long needle value");
        let non_matching_span = rdf_data.node_data.indexers.literal_cache.push_str("nothing here");
        let short_property = rdf_data.node_data.get_predicate_index("short");
        let lang_short_property = rdf_data.node_data.get_predicate_index("lang-short");
        let string_property = rdf_data.node_data.get_predicate_index("string");
        let lang_string_property = rdf_data.node_data.get_predicate_index("lang-string");
        let typed_string_property = rdf_data.node_data.get_predicate_index("typed-string");
        let non_matching_property = rdf_data.node_data.get_predicate_index("non-matching");
        let node_index = rdf_data.node_data.put_node(
            "urn:test:node",
            NObject {
                types: Vec::new(),
                properties: vec![
                    (short_property, Literal::StringShort(short_index)),
                    (short_property, Literal::StringShort(short_index)),
                    (lang_short_property, Literal::LangShortString(0, short_index)),
                    (string_property, Literal::String(matching_span)),
                    (lang_string_property, Literal::LangString(0, matching_span)),
                    (typed_string_property, Literal::TypedString(0, matching_span)),
                    (non_matching_property, Literal::String(non_matching_span)),
                    (non_matching_property, Literal::NoValue()),
                ],
                references: Vec::new(),
                reverse_references: Vec::new(),
                has_subject: true,
                is_blank_node: false,
            },
        );

        let results = rdf_data.search_nodes("needle");

        assert_eq!(
            results,
            vec![
                SearchResult { node_index, found_property_index: short_property },
                SearchResult { node_index, found_property_index: lang_short_property },
                SearchResult { node_index, found_property_index: string_property },
                SearchResult { node_index, found_property_index: lang_string_property },
                SearchResult { node_index, found_property_index: typed_string_property },
            ]
        );
        assert!(rdf_data.search_nodes("").is_empty());
    }
}
