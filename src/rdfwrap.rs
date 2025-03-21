use eframe::egui::Pos2;
use oxrdf::NamedNode;
use oxrdf::{vocab::rdf, NamedNodeRef, Subject, Term, Triple};
use oxttl::TurtleParser;
use oxrdfxml::RdfXmlParser;
use rand::Rng;

use crate::nobject::{NObject, NodeData, PredicateReference};
use std::{fs::File, io::Read};
use std::io::BufReader;

use anyhow::{Context, Result};

pub trait RDFAdapter {
    fn load_object(&mut self, iri: &str, node_data: &mut NodeData) -> Option<NObject>;
    fn iri2label<'a>(&mut self, iri: &'a str) -> &'a str;
}

pub struct RDFWrap {
    file_name: String,
}

impl RDFWrap {
    pub fn empty() -> Self {
        return RDFWrap {
            file_name: "empty".to_string(),
        };
    }

    pub fn load_file(file_name: &str, node_data: &mut NodeData) -> Result<u32> {
        // TODO Error handling for can not open ttl file
        let file =
            File::open(file_name).with_context(|| format!("Can not open file {}", file_name))?;
        let reader = BufReader::new(file);
        let file_extension = file_name.split('.').last().unwrap();
        let mut triples_count = 0;
        let (indexer, cache) = node_data.split_mut();
        match file_extension {
            "ttl" => {
                let parser = TurtleParser::new().for_reader(reader);
                for triple in parser {
                    match triple {
                        Ok(triple) => {
                            add_triple(&mut triples_count, indexer, cache, triple);
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "rdf" | "xml" => {
                let parser = RdfXmlParser::new().for_reader(reader);
                for triple in parser {
                    let triple = triple?;
                    add_triple(&mut triples_count, indexer, cache, triple);
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported file extension {} for rdf data import (known: ttl, rdf, xml)", file_extension));
            }
        };
        return Ok(triples_count);
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
                            types.push(node_data.get_type_index(named_object.as_str()));
                        }
                        _ => {
                            // types.push(triple.object.to_string());
                            println!("type is not named node {}", triple.object.to_string());
                        }
                    }
                } else {
                    match &triple.object {
                        Term::NamedNode(named_object) => {
                            references.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                node_data.get_node_index_or_insert(named_object.as_str()),
                            ));
                        }
                        Term::Literal(literal) => {
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                literal.value().to_string(),
                            ));
                        }
                        _ => {
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                triple.object.to_string(),
                            ));
                        }
                    }
                }
            } else if triple.object == subject_iri.into() {
                match &triple.subject {
                    Subject::NamedNode(named_subject) => {
                        reverse_references.push((
                            node_data.get_predicate_index(triple.predicate.as_str()),
                            node_data.get_node_index_or_insert(named_subject.as_str()),
                        ));
                    }
                    _ => {
                        // reverse_references.push((node_data.get_predicate_index(triple.predicate.as_str()), triple.subject.to_string()));
                        println!(
                            "reverse reference is not named node {}",
                            triple.subject.to_string()
                        );
                    }
                }
            }
        }
        if !found {
            println!("Object not found: {}", iri);
            return None;
        }
        return Some(NObject {
            iri: iri.to_string(),
            properties,
            references,
            reverse_references,
            types,
            has_subject: true,
            is_bank_node: false,
            pos: Pos2::new(
                rand::rng().random_range(0.0..100.0),
                rand::rng().random_range(0.0..100.0),
            ),
        });
    }
    pub fn iri2label_fallback<'a>(iri: &'a str) -> &'a str {
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
            return iri;
        } else {
            return &iri[last_index + 1..];
        }
    }
}

fn add_triple(triples_count: &mut u32, indexer: &mut crate::nobject::Indexers, cache: &mut crate::nobject::NodeCache, triple: Triple) {
    match &triple.subject {
        Subject::BlankNode(blank_node) => {
            let iri = blank_node.as_str();
            let node_index = cache.get_node_index_or_insert(iri);
            add_predicate_object(triples_count, indexer, cache, node_index, triple.predicate, triple.object);
        }
        Subject::NamedNode(named_subject) => {
            let iri = named_subject.as_str();
            let node_index = cache.get_node_index_or_insert(iri);
            add_predicate_object(triples_count, indexer, cache, node_index, triple.predicate, triple.object);
        }
        _ => {
            println!(
                "Subject is not named node {} and will be ignored",
                triple.subject.to_string()
            );
        }
    }
}

fn add_predicate_object(triples_count: &mut u32, indexer: &mut crate::nobject::Indexers, cache: &mut crate::nobject::NodeCache, node_index: usize, predicate: NamedNode, object: Term) {
    if predicate == rdf::TYPE {
        match &object {
            Term::NamedNode(named_object) => {
                let node = cache.get_node_by_index_mut(node_index).unwrap();
                node.has_subject = true;
                *triples_count += 1;
                node.types
                    .push(indexer.get_type_index(named_object.as_str()));
            }
            _ => {
                println!("type is not named node {}", object.to_string());
            }
        }
    } else {
        match &object {
            Term::NamedNode(named_object) => {
                let reference_index = cache.get_node_index_or_insert(named_object.as_str());
                let predicate_index = indexer.get_predicate_index(predicate.as_str());
                let predicate_literal: PredicateReference = (predicate_index,reference_index);
                let node = cache.get_node_by_index_mut(node_index).unwrap();
                node.references.push(predicate_literal);
                node.has_subject = true;
                let ref_node = cache.get_node_by_index_mut(reference_index).unwrap();
                ref_node.reverse_references.push((predicate_index,node_index));
                *triples_count += 1;
            }
            Term::BlankNode(blank_node) => {
                let reference_index = cache.get_node_index_or_insert(blank_node.as_str());
                let predicate_index = indexer.get_predicate_index(predicate.as_str());
                let predicate_literal: PredicateReference = (predicate_index,reference_index);
                let node = cache.get_node_by_index_mut(node_index).unwrap();
                node.references.push(predicate_literal);
                node.has_subject = true;
                let ref_node = cache.get_node_by_index_mut(reference_index).unwrap();
                ref_node.reverse_references.push((predicate_index,node_index));
                *triples_count += 1;
            }
            Term::Literal(literal) => {
                let mut skip = false;
                if let Some(language) = literal.language() {
                    if language != "en" {
                        skip = true;
                    }
                }
                if !skip {
                    let node = cache.get_node_by_index_mut(node_index).unwrap();
                    node.has_subject = true;
                    node.properties.push((
                        indexer.get_predicate_index(predicate.as_str()),
                        literal.value().to_string(),
                    ));
                    *triples_count += 1;
                }
            }
            _ => {
                print!("object is not named node {} nor literal", object.to_string());
            }
        }
    }
}

impl RDFAdapter for RDFWrap {
    fn iri2label<'a>(&mut self, iri: &'a str) -> &'a str {
        return RDFWrap::iri2label_fallback(iri);
    }
    fn load_object(&mut self, _iri: &str, _node_data: &mut NodeData) -> Option<NObject> {
        None
    }
}
