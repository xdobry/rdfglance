use crate::{
    IriIndex,
};
use rayon::prelude::*;
use rdf_core::{Literal, NodeData};
use std::collections::HashMap;
use crate::domain::type_index::TypeInstanceIndex;

pub fn resolve_references(
    node_data: &mut NodeData,
    type_instance_index: &TypeInstanceIndex,
    from_type: IriIndex,
    from_predicate: IriIndex,
    to_type: IriIndex,
    to_predicate: IriIndex,
    predicate: IriIndex,
) -> usize {
    // 'from' is the type that hold the reference (so foreign key value) and 'to' is te type that hold the primary key value (only first is searched)
    // it add the rdf reference to the node data (it is not checked if the reference already exists for speed reason)

    let mut reference_count = 0;
    // create index of the to types
    // Map<short string index, instance index>
    let Some(to_type) = type_instance_index.types.get(&to_type) else {
        return 0;
    };
    let Some(from_type) = type_instance_index.types.get(&from_type) else {
        return 0;
    };

    let to_index: HashMap<IriIndex, IriIndex> = to_type
        .instances
        .par_iter()
        .fold(
            || HashMap::new(),
            |mut local_map, inst_index| {
                if let Some((_iri, node)) = node_data.get_node_by_index(*inst_index) {
                    for (predicate_index, literal) in &node.properties {
                        if *predicate_index == to_predicate {
                            if let Literal::StringShort(str_index) = literal {
                                local_map.insert(*str_index, *inst_index);
                                break;
                            }
                        }
                    }
                }
                local_map
            },
        )
        .reduce(
            || HashMap::new(),
            |mut left, right| {
                left.extend(right);
                left
            },
        );

    let mut target_nodes: Vec<IriIndex> = Vec::new();
    for inst_index in from_type.instances.iter() {
        let node = node_data.get_node_by_index_mut(*inst_index);
        if let Some((_iri, node)) = node {
            for (predicate_index, literal) in node.properties.iter() {
                if *predicate_index == from_predicate {
                    if let Literal::StringShort(str_index) = literal {
                        if let Some(to_inst_index) = to_index.get(str_index) {
                            // add reference
                            node.references.push((predicate, *to_inst_index));
                            reference_count += 1;
                            target_nodes.push(*to_inst_index);
                        }
                    }
                }
            }
        }
        for to_inst_index in target_nodes.iter() {
            let target_node = node_data.get_node_by_index_mut(*to_inst_index);
            if let Some((_iri, target_node)) = target_node {
                // add reverse reference
                target_node.reverse_references.push((predicate, *inst_index));
            }
        }
        target_nodes.clear();
    }
    reference_count
}

