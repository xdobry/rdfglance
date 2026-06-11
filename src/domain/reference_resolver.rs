use std::collections::HashMap;

use crate::{IriIndex, domain::{Literal, NodeData, type_index::TypeInstanceIndex}};


pub fn resolve_references(node_data: &mut NodeData, type_instance_index: &TypeInstanceIndex, from_type: IriIndex, from_predicate: IriIndex, 
    to_type: IriIndex, to_predicate: IriIndex, predicate: IriIndex) -> usize {
    // 'from' is the type that hold the reference (so foreign key value) and 'to' is te type that hold the primary key value (only first is searched)
    // it add the rdf reference to the node data (it is not checked if the reference already exists for speed reason)

    let mut reference_count = 0;
    // create index of the to types
    // Map<short string index, instance index>
    let mut to_index: HashMap<IriIndex, IriIndex> = HashMap::new();
    type_instance_index.types.get(&to_type).map(|type_data| {
        for inst_index in type_data.instances.iter() {
            let node = node_data.get_node_by_index(*inst_index);
            if let Some((_iri, node)) = node {
                for (predicate_index, literal) in node.properties.iter() {
                    if *predicate_index == to_predicate {
                        if let Literal::StringShort(str_index) = literal {
                            to_index.insert(*str_index, *inst_index);
                            break;
                        }   
                    }
                }
            }
        }
    });

    // resolve references for the from types
    type_instance_index.types.get(&from_type).map(|type_data| {
        let mut target_nodes : Vec<IriIndex> = Vec::new();
        for inst_index in type_data.instances.iter() {
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
    });
    reference_count   
}

#[cfg(test)]
mod tests {
    use string_interner::Symbol;

use crate::{domain::{RdfData, prefix_manager::PrefixManager, reference_resolver, type_index::TypeInstanceIndex}, integration::rdfwrap::RDFWrap};
    use super::*;

    
    #[test]
    fn test_resolve_references_xml() -> std::io::Result<()> {
        let mut rdf_data = RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
        };
        let language_filter: Vec<String> = Vec::new();
        let load_result = RDFWrap::load_file(
                        "sample-rdf-data/ChinookData.xml".to_string(),
                        &mut rdf_data,
                        &language_filter,
                        None,
                    );
        // println!("result {}",load_result.err().unwrap());
        assert!(load_result.is_ok());
        assert!(load_result.unwrap()>0);
        let mut t = TypeInstanceIndex::new();
        t.update(&rdf_data.node_data);

        let from_type = t.types_order.iter()
            .find(|t_index| rdf_data.node_data.get_type(**t_index).map_or(false, |type_iri| type_iri.ends_with("#track"))).unwrap();
        let predicate = rdf_data.node_data.indexers.predicate_indexer.map.iter()
            .find(|(_p_index,pstr)| pstr.ends_with("albumid")).unwrap().0.to_usize() as IriIndex;
        let to_type = t.types_order.iter()
            .find(|t_index| rdf_data.node_data.get_type(**t_index).map_or(false, |type_iri| type_iri.ends_with("#album"))).unwrap();   

        let res = reference_resolver::resolve_references(&mut rdf_data.node_data, &t, *from_type, predicate, *to_type, predicate, predicate);
        assert!(res>0); 

        Ok(())
    }
}

