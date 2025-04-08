use oxrdf::vocab::xsd;
use oxrdf::NamedNode;
use oxrdf::{vocab::rdf, NamedNodeRef, Subject, Term, Triple};
use oxrdfxml::RdfXmlParser;
use oxttl::TurtleParser;

use crate::nobject::{IriIndex, Literal, NObject, NodeData, PredicateReference};
use crate::prefix_manager::PrefixManager;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::BufReader;

use anyhow::{Context, Result};
use std::time::Instant;

pub trait RDFAdapter {
    fn load_object(&mut self, iri: &str, node_data: &mut NodeData) -> Option<NObject>;
    fn iri2label<'a>(&mut self, iri: &'a str) -> &'a str;
}

pub struct RDFWrap {
    file_name: String,
}

pub struct IndexCache {
    pub index: IriIndex,
    pub iri: String,
}

impl RDFWrap {
    pub fn empty() -> Self {
        RDFWrap {
            file_name: "empty".to_string(),
        }
    }

    pub fn load_from_dir(
        dir_name: &str,
        node_data: &mut NodeData,
        language_filter: &Vec<String>,
        prefix_manager: &mut PrefixManager,
    ) -> Result<u32> {
        let mut total_triples = 0;
        let mut seen_files: HashSet<String> = HashSet::new();

        let entries = fs::read_dir(dir_name)
            .with_context(|| format!("Failed to read directory {}", dir_name));
        match entries {
            Err(e) => {
                eprintln!("Error reading dir {}: {}", dir_name, e)
            }
            Ok(entries) => {
                for entry in entries {
                    let entry = entry?;
                    let path = entry.path();
                    let path_name = path.to_str().unwrap();
                    if seen_files.insert(path_name.to_string()) {
                        if path.is_dir() {
                            total_triples += RDFWrap::load_from_dir(
                                path_name,
                                node_data,
                                language_filter,
                                prefix_manager,
                            )?;
                        } else if let Some(extension) = path.extension() {
                            if ["ttl", "rdf", "xml", "nt", "nq", "trig"]
                                .contains(&extension.to_str().unwrap())
                            {
                                match RDFWrap::load_file(
                                    path_name,
                                    node_data,
                                    language_filter,
                                    prefix_manager,
                                ) {
                                    Ok(triples) => {
                                        total_triples += triples;
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Error processing file {}: {}",
                                            path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(total_triples)
    }

    pub fn load_file(
        file_name: &str,
        node_data: &mut NodeData,
        language_filter: &[String],
        prefix_manager: &mut PrefixManager,
    ) -> Result<u32> {
        let file =
            File::open(file_name).with_context(|| format!("Can not open file {}", file_name))?;
        let reader = BufReader::new(file);
        let file_extension = file_name.split('.').last().unwrap();
        let mut triples_count = 0;
        let (indexer, cache) = node_data.split_mut();
        let start = Instant::now();
        let mut index_cache = IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        match file_extension {
            "ttl" => {
                let mut parser = TurtleParser::new().for_reader(reader);
                let mut prefix_read = false;
                while let Some(triple) = parser.next() {
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            prefix_manager.add_prefix(prefix, iri);
                        }
                        prefix_read = true;
                    }
                    match triple {
                        Ok(triple) => {
                            add_triple(
                                &mut triples_count,
                                indexer,
                                cache,
                                triple,
                                &mut index_cache,
                                language_filter,
                                prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "rdf" | "xml" => {
                let mut parser = RdfXmlParser::new().for_reader(reader);
                let mut prefix_read = false;
                while let Some(triple) = parser.next() {
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            prefix_manager.add_prefix(prefix, iri);
                        }
                        prefix_read = true;
                    }
                    let triple = triple?;
                    add_triple(
                        &mut triples_count,
                        indexer,
                        cache,
                        triple,
                        &mut index_cache,
                        language_filter,
                        prefix_manager,
                    );
                }
            }
            "nt" => {
                let parser = oxttl::NTriplesParser::new().for_reader(reader);
                for triple in parser {
                    let triple = triple?;
                    add_triple(
                        &mut triples_count,
                        indexer,
                        cache,
                        triple,
                        &mut index_cache,
                        language_filter,
                        prefix_manager,
                    );
                }
            }
            "trig" => {
                let mut parser = oxttl::TriGParser::new().for_reader(reader);
                let mut prefix_read = false;
                while let Some(quad) = parser.next() {
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            prefix_manager.add_prefix(prefix, iri);
                        }
                        prefix_read = true;
                    }
                    let quad = quad?;
                    add_triple(
                        &mut triples_count,
                        indexer,
                        cache,
                        Triple::from(quad),
                        &mut index_cache,
                        language_filter,
                        prefix_manager,
                    );
                }
            }
            "nq" => {
                let parser = oxttl::NQuadsParser::new().for_reader(reader);
                for quad in parser {
                    let quad = quad?;
                    add_triple(
                        &mut triples_count,
                        indexer,
                        cache,
                        Triple::from(quad),
                        &mut index_cache,
                        language_filter,
                        prefix_manager,
                    );
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file extension {} for rdf data import (known: ttl, rdf, xml)",
                    file_extension
                ));
            }
        };
        let duration = start.elapsed();
        println!(
            "Time taken to read the file '{}': {:?}",
            file_name, duration
        );
        println!(
            "Triples read per second: {}",
            triples_count as f64 / duration.as_secs_f64()
        );
        Ok(triples_count)
    }

    pub fn load_from_triples(
        triples: &Vec<Triple>,
        iri: &str,
        node_data: &mut NodeData,
    ) -> Option<NObject> {
        let mut properties = Vec::new();
        let mut references = Vec::new();
        let mut types = Vec::new();
        let mut reverse_references = Vec::new();
        let mut found = false;
        let subject_iri = NamedNodeRef::new(iri).unwrap();
        for triple in triples {
            if triple.subject == subject_iri.into() {
                found = true;
                if triple.predicate == rdf::TYPE {
                    match &triple.object {
                        Term::NamedNode(named_object) => {
                            let type_index = node_data.get_type_index(named_object.as_str());
                            if !types.contains(&type_index) {
                                types.push(type_index);
                            }
                        }
                        _ => {
                            // types.push(triple.object.to_string());
                            println!("type is not named node {}", triple.object);
                        }
                    }
                } else {
                    match &triple.object {
                        Term::NamedNode(named_object) => {
                            references.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                node_data.get_node_index_or_insert(named_object.as_str(), false),
                            ));
                        }
                        Term::Literal(literal) => {
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                Literal::String(literal.value().into()),
                            ));
                        }
                        _ => {
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                Literal::String(triple.object.to_string().into()),
                            ));
                        }
                    }
                }
            } else if triple.object == subject_iri.into() {
                match &triple.subject {
                    Subject::NamedNode(named_subject) => {
                        reverse_references.push((
                            node_data.get_predicate_index(triple.predicate.as_str()),
                            node_data.get_node_index_or_insert(named_subject.as_str(), false),
                        ));
                    }
                    _ => {
                        // reverse_references.push((node_data.get_predicate_index(triple.predicate.as_str()), triple.subject.to_string()));
                        println!("reverse reference is not named node {}",triple.subject);
                    }
                }
            }
        }
        if !found {
            println!("Object not found: {}", iri);
            return None;
        }
        Some(NObject {
            properties,
            references,
            reverse_references,
            types,
            has_subject: true,
            is_blank_node: false,
        })
    }
    pub fn iri2label_fallback(iri: &str) -> &str {
        let last_index_slash = iri.rfind('/');
        let last_index_hash = iri.rfind('#');
        let last_index = if last_index_slash.is_none() && last_index_hash.is_none() {
            0
        } else if last_index_slash.is_none() {
            last_index_hash.unwrap_or(0)
        } else if last_index_hash.is_none() {
            last_index_slash.unwrap_or(0)
        } else {
            std::cmp::max(last_index_slash.unwrap(), last_index_hash.unwrap())
        };
        if last_index == 0 {
            let first_colon = iri.find(':');
            if let Some(first_colon) = first_colon {
                &iri[first_colon + 1..]
            } else {
                iri
            }
        } else {
            &iri[last_index + 1..]
        }
    }
}

pub fn add_triple(
    triples_count: &mut u32,
    indexer: &mut crate::nobject::Indexers,
    cache: &mut crate::nobject::NodeCache,
    triple: Triple,
    index_cache: &mut IndexCache,
    language_filter: &[String],
    prefix_manager: &PrefixManager,
) {
    match &triple.subject {
        Subject::BlankNode(blank_node) => {
            let iri = blank_node.as_str();
            if index_cache.iri != iri {
                index_cache.index = cache.get_node_index_or_insert(iri, true);
                index_cache.iri.clear();
                index_cache.iri.push_str(iri);
            }
            let node_index = index_cache.index;
            add_predicate_object(
                triples_count,
                indexer,
                cache,
                node_index,
                triple.predicate,
                triple.object,
                language_filter,
                prefix_manager,
            );
        }
        Subject::NamedNode(named_subject) => {
            let iri = prefix_manager.get_prefixed(named_subject.as_str());
            if index_cache.iri != iri {
                index_cache.index = cache.get_node_index_or_insert(&iri, false);
                index_cache.iri.clear();
                index_cache.iri.push_str(&iri);
            }
            let node_index = index_cache.index;
            add_predicate_object(
                triples_count,
                indexer,
                cache,
                node_index,
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
    indexer: &mut crate::nobject::Indexers,
    cache: &mut crate::nobject::NodeCache,
    node_index: IriIndex,
    predicate: NamedNode,
    object: Term,
    language_filter: &[String],
    prefix_manager: &PrefixManager,
) {
    if predicate == rdf::TYPE {
        match &object {
            Term::NamedNode(named_object) => {
                let (_iri, node) = cache.get_node_by_index_mut(node_index).unwrap();
                node.has_subject = true;
                *triples_count += 1;
                let type_iri = prefix_manager.get_prefixed(named_object.as_str());
                let type_index = indexer.get_type_index(&type_iri);
                if !node.types.contains(&type_index) {
                    node.types.push(type_index);
                }
            }
            _ => {
                println!("type is not named node {}", object);
            }
        }
    } else {
        let predicate_iri = prefix_manager.get_prefixed(predicate.as_str());
        let predicate_index = indexer.get_predicate_index(&predicate_iri);
        match &object {
            Term::NamedNode(named_object) => {
                let reference_iri = prefix_manager.get_prefixed(named_object.as_str());
                let reference_index = cache.get_node_index_or_insert(&reference_iri, false);
                let predicate_literal: PredicateReference = (predicate_index, reference_index);
                let (_iri, node) = cache.get_node_by_index_mut(node_index).unwrap();
                node.references.push(predicate_literal);
                node.has_subject = true;
                let (_riri, ref_node) = cache.get_node_by_index_mut(reference_index).unwrap();
                ref_node
                    .reverse_references
                    .push((predicate_index, node_index));
                *triples_count += 1;
            }
            Term::BlankNode(blank_node) => {
                let reference_index = cache.get_node_index_or_insert(blank_node.as_str(), true);
                let predicate_literal: PredicateReference = (predicate_index, reference_index);
                let (_iri, node) = cache.get_node_by_index_mut(node_index).unwrap();
                node.references.push(predicate_literal);
                node.has_subject = true;
                let (_riri, ref_node) = cache.get_node_by_index_mut(reference_index).unwrap();
                ref_node
                    .reverse_references
                    .push((predicate_index, node_index));
                *triples_count += 1;
            }
            Term::Literal(literal) => {
                let mut skip = false;
                if !language_filter.is_empty() {
                    if let Some(language) = literal.language() {
                        if language_filter.iter().all(|filter| filter != language) {
                            skip = true;
                        }
                    }
                }
                if !skip {
                    let (_iri, node) = cache.get_node_by_index_mut(node_index).unwrap();
                    node.has_subject = true;
                    let value = literal.value();
                    let language = literal.language();
                    let datatype = literal.datatype();
                    if let Some(language) = language {
                        let language_index = indexer.get_language_index(language);
                        node.properties.push((
                            predicate_index,
                            Literal::LangString(language_index, value.into()),
                        ));
                    } else if datatype == xsd::STRING {
                        node.properties
                            .push((predicate_index, Literal::String(value.into())));
                    } else {
                        let datatype_prefixed = prefix_manager.get_prefixed(datatype.as_str());
                        let data_type_index = indexer.get_data_type_index(&datatype_prefixed);
                        node.properties.push((
                            predicate_index,
                            Literal::TypedString(data_type_index, value.into()),
                        ));
                    }
                    *triples_count += 1;
                }
            }
        }
    }
}

impl RDFAdapter for RDFWrap {
    fn iri2label<'a>(&mut self, iri: &'a str) -> &'a str {
        RDFWrap::iri2label_fallback(iri)
    }
    fn load_object(&mut self, _iri: &str, _node_data: &mut NodeData) -> Option<NObject> {
        None
    }
}
