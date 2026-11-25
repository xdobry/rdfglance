use oxrdf::{
    NamedNode, NamedOrBlankNode, Term, Triple,
    vocab::{rdf, xsd},
};

use crate::{Indexers, IriIndex, Literal, NodeCache, PrefixManager};

const SHORT_STR_LITERAL_LEN: usize = 32;

pub struct IndexCache {
    pub index: IriIndex,
    pub iri: String,
}

pub fn add_triple(
    triples_count: &mut u32,
    indexer: &mut Indexers,
    cache: &mut NodeCache,
    triple: Triple,
    index_cache: &mut IndexCache,
    language_filter: &[String],
    prefix_manager: &PrefixManager,
) {
    match &triple.subject {
        NamedOrBlankNode::BlankNode(blank_node) => {
            let iri = blank_node.as_str();
            if index_cache.iri != iri {
                index_cache.index = cache.get_node_index_or_insert(iri, true);
                index_cache.iri.clear();
                index_cache.iri.push_str(iri);
            }
            add_predicate_object(
                triples_count,
                indexer,
                cache,
                index_cache.index,
                triple.predicate,
                triple.object,
                language_filter,
                prefix_manager,
            );
        }
        NamedOrBlankNode::NamedNode(named_subject) => {
            let iri = prefix_manager.get_prefixed(named_subject.as_str());
            if index_cache.iri != iri {
                index_cache.index = cache.get_node_index_or_insert(&iri, false);
                index_cache.iri.clear();
                index_cache.iri.push_str(&iri);
            }
            add_predicate_object(
                triples_count,
                indexer,
                cache,
                index_cache.index,
                triple.predicate,
                triple.object,
                language_filter,
                prefix_manager,
            );
        }
    }
}

fn add_predicate_object(
    triples_count: &mut u32,
    indexer: &mut Indexers,
    cache: &mut NodeCache,
    node_index: IriIndex,
    predicate: NamedNode,
    object: Term,
    language_filter: &[String],
    prefix_manager: &PrefixManager,
) {
    if predicate == rdf::TYPE {
        if let Term::NamedNode(named_object) = &object {
            let (_, node) = cache.get_node_by_index_mut(node_index).unwrap();
            node.has_subject = true;
            *triples_count += 1;
            let type_iri = prefix_manager.get_prefixed(named_object.as_str());
            let type_index = indexer.get_type_index(&type_iri);
            if !node.types.contains(&type_index) {
                node.types.push(type_index);
            }
        } else {
            tracing::info!("type is not named node {}", object);
        }
        return;
    }

    let predicate_iri = prefix_manager.get_prefixed(predicate.as_str());
    let predicate_index = indexer.get_predicate_index(&predicate_iri);
    match object {
        Term::NamedNode(named_object) => {
            let reference_iri = prefix_manager.get_prefixed(named_object.as_str());
            let reference_index = cache.get_node_index_or_insert(&reference_iri, false);
            let (_, node) = cache.get_node_by_index_mut(node_index).unwrap();
            node.references.push((predicate_index, reference_index));
            node.has_subject = true;
            let (_, ref_node) = cache.get_node_by_index_mut(reference_index).unwrap();
            ref_node.reverse_references.push((predicate_index, node_index));
            *triples_count += 1;
        }
        Term::BlankNode(blank_node) => {
            let reference_index = cache.get_node_index_or_insert(blank_node.as_str(), true);
            let (_, node) = cache.get_node_by_index_mut(node_index).unwrap();
            node.references.push((predicate_index, reference_index));
            node.has_subject = true;
            let (_, ref_node) = cache.get_node_by_index_mut(reference_index).unwrap();
            ref_node.reverse_references.push((predicate_index, node_index));
            *triples_count += 1;
        }
        Term::Literal(literal) => {
            if !language_filter.is_empty()
                && literal
                    .language()
                    .is_some_and(|language| language_filter.iter().all(|filter| filter != language))
            {
                return;
            }

            let (_, node) = cache.get_node_by_index_mut(node_index).unwrap();
            node.has_subject = true;
            let value = literal.value();
            let language = literal.language();
            let datatype = literal.datatype();
            let value = if let Some(language) = language {
                let language_index = indexer.get_language_index(language);
                if value.len() < SHORT_STR_LITERAL_LEN {
                    let index = indexer.short_literal_indexer.get_index(value);
                    Literal::LangShortString(language_index, index)
                } else {
                    let span = indexer.literal_cache.push_str(value);
                    Literal::LangString(language_index, span)
                }
            } else if datatype == xsd::STRING {
                if value.len() < SHORT_STR_LITERAL_LEN {
                    let index = indexer.short_literal_indexer.get_index(value);
                    Literal::StringShort(index)
                } else {
                    let span = indexer.literal_cache.push_str(value);
                    Literal::String(span)
                }
            } else {
                let datatype_prefixed = prefix_manager.get_prefixed(datatype.as_str());
                let data_type_index = indexer.get_data_type_index(&datatype_prefixed);
                let span = indexer.literal_cache.push_str(value);
                Literal::TypedString(data_type_index, span)
            };
            node.properties.push((predicate_index, value));
            *triples_count += 1;
        }
    }
}
