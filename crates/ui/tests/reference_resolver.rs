#[cfg(test)]
mod tests {
    use rdf_glance::{domain::RdfData, integration::rdfwrap::RDFWrap};
    use string_interner::Symbol;

    use rdf_core::{IriIndex, NodeData, PrefixManager};
    use rdf_analytics::domain::{reference_resolver, type_index::TypeInstanceIndex};

    #[test]
    fn test_resolve_references_xml() -> std::io::Result<()> {
        let mut rdf_data = RdfData {
            node_data: NodeData::new(),
            prefix_manager: PrefixManager::new(),
        };
        let language_filter: Vec<String> = Vec::new();
        let load_result = RDFWrap::load_file(
            "../../sample-rdf-data/ChinookData.xml".to_string(),
            &mut rdf_data,
            &language_filter,
            None,
        );
        // println!("result {}",load_result.err().unwrap());
        assert!(load_result.is_ok());
        assert!(load_result.unwrap() > 0);
        let mut t = TypeInstanceIndex::new();
        t.update(&rdf_data.node_data);

        let from_type = t
            .types_order
            .iter()
            .find(|t_index| {
                rdf_data
                    .node_data
                    .get_type(**t_index)
                    .map_or(false, |type_iri| type_iri.ends_with("#track"))
            })
            .unwrap();
        let predicate = rdf_data
            .node_data
            .indexers
            .predicate_indexer
            .map
            .iter()
            .find(|(_p_index, pstr)| pstr.ends_with("albumid"))
            .unwrap()
            .0
            .to_usize() as IriIndex;
        let to_type = t
            .types_order
            .iter()
            .find(|t_index| {
                rdf_data
                    .node_data
                    .get_type(**t_index)
                    .map_or(false, |type_iri| type_iri.ends_with("#album"))
            })
            .unwrap();

        let res = reference_resolver::resolve_references(
            &mut rdf_data.node_data,
            &t,
            *from_type,
            predicate,
            *to_type,
            predicate,
            predicate,
        );
        assert!(res > 0);

        Ok(())
    }
}
