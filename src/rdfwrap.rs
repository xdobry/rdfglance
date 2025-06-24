use oxrdf::NamedNode;
use oxrdf::vocab::xsd;
use oxrdf::{NamedNodeRef, Subject, Term, Triple, vocab::rdf};
use oxrdfxml::RdfXmlParser;
use oxttl::TurtleParser;

use crate::nobject::{IriIndex, Literal, NObject, NodeData, PredicateReference};
use crate::prefix_manager::PrefixManager;
use crate::{DataLoading, RdfData};
use std::fs::{self, File};
use std::io::{self, BufReader, Cursor, Read};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use std::time::Instant;

const SHORT_STR_LITERAL_LEN: usize = 32;

pub trait RDFAdapter {
    fn load_object(&mut self, iri: &str, node_data: &mut NodeData) -> Option<NObject>;
}

pub struct RDFWrap {}

pub struct IndexCache {
    pub index: IriIndex,
    pub iri: String,
}

pub struct CountingReader<R> {
    inner: R,
    pub bytes_read: Arc<AtomicUsize>,
}

impl<R: Read> CountingReader<R> {
    pub fn new(inner: R, bytes_read: Arc<AtomicUsize>) -> Self {
        CountingReader {
            inner,
            bytes_read,
        }
    }
}

impl<R: Read> Read for CountingReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.bytes_read.fetch_add(n, Ordering::Relaxed);
        Ok(n)
    }
}

enum ParseItem {
    Triple(Result<Triple, io::Error>),
    Prefix(String, String),
}

fn collect_rdf_files(dir_name: &str, files: &mut Vec<String>) -> Result<()> {
    let entries = fs::read_dir(dir_name).with_context(|| format!("Failed to read directory {}", dir_name));
    match entries {
        Err(e) => {
            eprintln!("Error reading dir {}: {}", dir_name, e)
        }
        Ok(entries) => {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                let path_name = path.to_str().unwrap();
                if path.is_dir() {
                    collect_rdf_files(path_name, files)?;
                } else if let Some(extension) = path.extension() {
                    if ["ttl", "rdf", "xml", "nt", "nq", "trig"].contains(&extension.to_str().unwrap()) {
                        files.push(path_name.to_string());
                    }
                }
            }
        }  
    }
    Ok(())
}

impl RDFWrap {
    pub fn empty() -> Self {
        RDFWrap {}
    }

    pub fn load_from_dir(dir_name: &str, rdf_data: &mut RdfData, language_filter: &Vec<String>, data_loading: Option<&DataLoading>) -> Result<u32> {
        let mut total_triples = 0;
        let mut files = Vec::new();
        collect_rdf_files(dir_name, &mut files)?;
        if let Some(data_loading) = data_loading {
            let mut size_total = 0;
            for file in &files {
                let metadata = fs::metadata(file).with_context(|| format!("Failed to get metadata for file {}", file))?;
                size_total += metadata.len() as usize;
            }
            data_loading.total_size.store(size_total, std::sync::atomic::Ordering::Relaxed);
        }   
        for file in &files {
            match RDFWrap::load_file(&file, rdf_data, language_filter, data_loading) {
                Ok(triples) => {
                    total_triples += triples;
                }
                Err(e) => {
                    eprintln!("Error processing file {}: {}", file, e);
                }
            }
        }
        Ok(total_triples)
    }

    pub fn load_file<P: AsRef<Path>>(
        file_name: P,
        rdf_data: &mut RdfData,
        language_filter: &[String],
        data_loading: Option<&DataLoading>,
    ) -> Result<u32> {
        let file_name = file_name.as_ref();
        let file = File::open(file_name).with_context(|| format!("Can not open file {}", file_name.display()))?;
        if let Some(data_loading) = data_loading {
            if data_loading.total_size.load(std::sync::atomic::Ordering::Relaxed) == 0 {
                match file.metadata() {
                    Ok(metadata) => {
                        data_loading
                            .total_size
                            .store(metadata.len() as usize, std::sync::atomic::Ordering::Relaxed);
                    }
                    Err(_e) => {
                        // ignore
                    }
                }
            }
        }

        let reader = BufReader::new(file);
        let file_extension = file_name.extension().and_then(|s| s.to_str()).unwrap_or("");
        Self::load_file_reader(file_extension, reader, rdf_data, language_filter, data_loading)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_file_data(
        file_name: &str,
        data: &Vec<u8>,
        rdf_data: &mut RdfData,
        language_filter: &[String],
    ) -> Result<u32> {
        let file_name = Path::new(file_name);
        let reader = Cursor::new(data);
        let file_extension = file_name.extension().and_then(|s| s.to_str()).unwrap_or("");
        Self::load_file_reader(file_extension, reader, rdf_data, language_filter, None)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_file_reader<R: std::io::Read>(
        file_extension: &str,
        reader: R,
        rdf_data: &mut RdfData,
        language_filter: &[String],
        data_loading: Option<&DataLoading>,
    ) -> Result<u32> {
        let mut triples_count: u32 = 0;
        let (indexer, cache) = rdf_data.node_data.split_mut();
        #[cfg(not(target_arch = "wasm32"))]
        let start = Instant::now();
        let mut index_cache = IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        let bytes_read = Arc::new(AtomicUsize::new(if let Some(data_loading) = data_loading {
            data_loading.read_pos.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            0
        }));
        let counting_reader = reader;
        match file_extension {
            "ttl" => {
                let mut parser = TurtleParser::new().for_reader(counting_reader);
                let mut prefix_read = false;
                while let Some(triple) = parser.next() {
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            rdf_data.prefix_manager.add_prefix(prefix, iri);
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
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "rdf" | "xml" => {
                let mut parser = RdfXmlParser::new().for_reader(counting_reader);
                let mut prefix_read = false;
                while let Some(triple) = parser.next() {
                    if let Some(data_loading) = data_loading {
                        if data_loading.stop_loading.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        data_loading
                            .total_triples
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        data_loading
                            .read_pos
                            .store(bytes_read.load(Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
                    }
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            rdf_data.prefix_manager.add_prefix(prefix, iri);
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
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "nt" => {
                let parser = oxttl::NTriplesParser::new().for_reader(counting_reader);
                for triple in parser {
                    if let Some(data_loading) = data_loading {
                        if data_loading.stop_loading.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        data_loading
                            .total_triples
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        data_loading
                            .read_pos
                            .store(bytes_read.load(Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
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
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "trig" => {
                let mut parser = oxttl::TriGParser::new().for_reader(counting_reader);
                let mut prefix_read = false;
                while let Some(quad) = parser.next() {
                    if let Some(data_loading) = data_loading {
                        if data_loading.stop_loading.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        data_loading
                            .total_triples
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        data_loading
                            .read_pos
                            .store(bytes_read.load(Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
                    }
                    if !prefix_read {
                        for (prefix, iri) in parser.prefixes() {
                            rdf_data.prefix_manager.add_prefix(prefix, iri);
                        }
                        prefix_read = true;
                    }
                    match quad {
                        Ok(quad) => {
                            add_triple(
                                &mut triples_count,
                                indexer,
                                cache,
                                Triple::from(quad),
                                &mut index_cache,
                                language_filter,
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            "nq" => {
                let parser = oxttl::NQuadsParser::new().for_reader(counting_reader);
                for quad in parser {
                    if let Some(data_loading) = data_loading {
                        if data_loading.stop_loading.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        data_loading
                            .total_triples
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        data_loading
                            .read_pos
                            .store(bytes_read.load(Ordering::Relaxed), std::sync::atomic::Ordering::Relaxed);
                    }
                    match quad {
                        Ok(quad) => {
                            add_triple(
                                &mut triples_count,
                                indexer,
                                cache,
                                Triple::from(quad),
                                &mut index_cache,
                                language_filter,
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported file extension {} for rdf data import (known: ttl, rdf, xml)",
                    file_extension
                ));
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        {
            let duration = start.elapsed();
            println!("Time taken to read the file {:?}", duration);
            println!(
                "Triples read per second: {}",
                triples_count as f64 / duration.as_secs_f64()
            );
        }
        Ok(triples_count)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_file_reader<R: std::io::Read + std::marker::Send + 'static>(
        file_extension: &str,
        reader: R,
        rdf_data: &mut RdfData,
        language_filter: &[String],
        data_loading: Option<&DataLoading>,
    ) -> Result<u32> {
        // This function uses 2 stages to parse and process RDF data
        // The parsing is done in a separate thread and parse items are send to main thread via a channel.
        use std::{sync::mpsc, thread};


        let mut triples_count: u32 = 0;
        let (indexer, cache) = rdf_data.node_data.split_mut();
        let start = Instant::now();
        let mut index_cache = IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        let bytes_read= Arc::new(AtomicUsize::new(if let Some(data_loading) = data_loading {
            data_loading.read_pos.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            0
        }));
        let (tx, rx) = mpsc::sync_channel(1000);

        let bytes_read_tx = Arc::clone(&bytes_read);
        let file_extension = file_extension.to_string();
        let handle = thread::spawn(move || {
            let counting_reader = CountingReader::new(reader, bytes_read_tx);
            match file_extension.as_str() {
                "ttl" => {
                    let mut parser = TurtleParser::new().for_reader(counting_reader);
                    let mut prefix_read = false;
                    while let Some(triple) = parser.next() {
                        if !prefix_read {
                            for (prefix, iri) in parser.prefixes() {
                                tx.send(ParseItem::Prefix(prefix.to_string(), iri.to_string())).unwrap();
                            }
                            prefix_read = true;
                        }
                        match triple {
                            Ok(triple) => {
                                tx.send(ParseItem::Triple(Ok(triple))).unwrap();
                            }
                            Err(e) => {
                                tx.send(ParseItem::Triple(Err(e.into()))).unwrap();
                            }
                        }
                    }
                },
                "rdf" | "xml" => {
                    let mut parser = RdfXmlParser::new().for_reader(counting_reader);
                    let mut prefix_read = false;
                    while let Some(triple) = parser.next() {
                        if !prefix_read {
                            for (prefix, iri) in parser.prefixes() {
                                tx.send(ParseItem::Prefix(prefix.to_string(), iri.to_string())).unwrap();
                            }
                            prefix_read = true;
                        }
                        match triple {
                            Ok(triple) => {
                                tx.send(ParseItem::Triple(Ok(triple))).unwrap();
                            }
                            Err(e) => {
                                tx.send(ParseItem::Triple(Err(e.into()))).unwrap();
                            }
                        }
                    }
                }
                "nt" => {
                    let parser = oxttl::NTriplesParser::new().for_reader(counting_reader);
                    for triple in parser {
                        match triple {
                            Ok(triple) => {
                                tx.send(ParseItem::Triple(Ok(triple))).unwrap();
                            }
                            Err(e) => {
                                tx.send(ParseItem::Triple(Err(e.into()))).unwrap();
                            }
                        }
                    }
                }
                "trig" => {
                    let mut parser = oxttl::TriGParser::new().for_reader(counting_reader);
                    let mut prefix_read = false;
                    while let Some(quad) = parser.next() {
                        if !prefix_read {
                            for (prefix, iri) in parser.prefixes() {
                                tx.send(ParseItem::Prefix(prefix.to_string(), iri.to_string())).unwrap();
                            }
                            prefix_read = true;
                        }
                        match quad {
                            Ok(quad) => {
                                tx.send(ParseItem::Triple(Ok(Triple::from(quad)))).unwrap();
                            }
                            Err(e) => {
                                tx.send(ParseItem::Triple(Err(e.into()))).unwrap();
                            }
                        }
                    }
                }
                "nq" => {
                    let parser = oxttl::NQuadsParser::new().for_reader(counting_reader);
                    for quad in parser {
                        match quad {
                            Ok(quad) => {
                                tx.send(ParseItem::Triple(Ok(Triple::from(quad)))).unwrap();
                            }
                            Err(e) => {
                                tx.send(ParseItem::Triple(Err(e.into()))).unwrap();
                            }
                        }
                    }
                },
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unsupported file extension {} for rdf data import (known: ttl, rdf, xml)",
                        file_extension
                    ));
                }
            };
            Ok(())
        });

        for parse_item in rx {
            if let Some(data_loading) = data_loading {
                if data_loading.stop_loading.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                data_loading
                    .total_triples
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let readed = bytes_read.load(Ordering::Relaxed);
                data_loading
                    .read_pos
                    .store(readed, std::sync::atomic::Ordering::Relaxed);
            }
            match parse_item {
                ParseItem::Prefix(prefix, iri) => {
                    rdf_data.prefix_manager.add_prefix(&prefix, &iri);
                }
                ParseItem::Triple(triple) => {
                    match triple {
                        Ok(triple) => {
                            add_triple(
                                &mut triples_count,
                                indexer,
                                cache,
                                triple,
                                &mut index_cache,
                                language_filter,
                                &rdf_data.prefix_manager,
                            );
                        }
                        Err(e) => {
                            eprintln!("Error parsing triple: {}", e);
                        }
                    }
                }
            }
        }
        let thread_res = handle.join().unwrap();
        if let Err(e) = thread_res {
            return Err(e);
        }
        let duration = start.elapsed();
        println!("Time taken to read the file {:?}", duration);
        println!(
            "Triples read per second: {}",
            triples_count as f64 / duration.as_secs_f64()
        );
        Ok(triples_count)
    }

    pub fn load_from_triples(triples: &Vec<Triple>, iri: &str, node_data: &mut NodeData) -> Option<NObject> {
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
                            let span = node_data.indexers.literal_cache.push_str(literal.value());
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                Literal::String(span),
                            ));
                        }
                        _ => {
                            let span = node_data
                                .indexers
                                .literal_cache
                                .push_str(triple.object.to_string().as_str());
                            properties.push((
                                node_data.get_predicate_index(triple.predicate.as_str()),
                                Literal::String(span),
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
                        println!("reverse reference is not named node {}", triple.subject);
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
                ref_node.reverse_references.push((predicate_index, node_index));
                *triples_count += 1;
            }
            Term::BlankNode(blank_node) => {
                let reference_index = cache.get_node_index_or_insert(blank_node.as_str(), true);
                let predicate_literal: PredicateReference = (predicate_index, reference_index);
                let (_iri, node) = cache.get_node_by_index_mut(node_index).unwrap();
                node.references.push(predicate_literal);
                node.has_subject = true;
                let (_riri, ref_node) = cache.get_node_by_index_mut(reference_index).unwrap();
                ref_node.reverse_references.push((predicate_index, node_index));
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
                        let span = indexer.literal_cache.push_str(value);
                        node.properties
                            .push((predicate_index, Literal::LangString(language_index, span)));
                    } else if datatype == xsd::STRING {
                        let literal = if value.len() < SHORT_STR_LITERAL_LEN {
                            let index = indexer.short_literal_indexer.get_index(value);
                            Literal::StringShort(index)
                        } else {
                            let span = indexer.literal_cache.push_str(value);
                            Literal::String(span)
                        };
                        node.properties.push((predicate_index, literal));
                    } else {
                        let datatype_prefixed = prefix_manager.get_prefixed(datatype.as_str());
                        let data_type_index = indexer.get_data_type_index(&datatype_prefixed);
                        let span = indexer.literal_cache.push_str(value);
                        node.properties
                            .push((predicate_index, Literal::TypedString(data_type_index, span)));
                    }
                    *triples_count += 1;
                }
            }
        }
    }
}

impl RDFAdapter for RDFWrap {
    fn load_object(&mut self, _iri: &str, _node_data: &mut NodeData) -> Option<NObject> {
        None
    }
}
