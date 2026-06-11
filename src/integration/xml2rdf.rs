use std::io::{self, BufRead};
use oxrdf::{Literal, NamedNode, Triple};
use slug::slugify;
use json_event_parser::{ReaderJsonParser, JsonEvent};
use quick_xml::{Reader, XmlVersion, escape::resolve_xml_entity, events::Event};

use crate::integration::{csv2rdf::hash_string, rdfwrap::ParseItem};

pub struct XmlRdfParser<R: BufRead> {
    reader: Reader<R>,
    obj_path: Vec<NamedNode>,
    obj_count: usize,
    file_hash: String,
    // The tag name that is open and we do not know if it will be some predicate (so only text) or node (it has subtags)
    pending_tag: Option<String>,
    was_pred_tag: bool,
    text: String,
}

impl<R: BufRead> XmlRdfParser<R> {
    pub fn for_reader(reader: R, file_base: String) -> Self {
        Self {
            reader: Reader::from_reader(reader),
            obj_path: Vec::new(),
            obj_count: 0,
            file_hash: hash_string(&file_base),
            pending_tag: None,
            was_pred_tag: false,
            text: String::new(),
        }
    }
}

impl<R: BufRead> XmlRdfParser<R> {
    pub fn parse<F>(&mut self, mut f: F) -> io::Result<()>
    where
        F: FnMut(ParseItem),
    {
        let mut buf = Vec::new();        
        loop {
            let event = self.reader.read_event_into(&mut buf);
            match event {
                Ok(event) => {
                    match event {
                        Event::Start(s) => {
                            let mut obj_iri: Option<NamedNode> = None;
                            for attr in s.attributes() {
                                if let Ok(attr) = attr {
                                    if obj_iri.is_none() {
                                        if let Some(pending_tag) = self.pending_tag.take() {
                                            let type_iri = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,pending_tag));
                                            let obj_iri_c = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,self.obj_count));
                                            self.obj_count += 1;
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), oxrdf::vocab::rdf::TYPE, type_iri))));
                                            if let Some(parent_obj) = self.obj_path.last() {
                                                let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,pending_tag));
                                                f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), pred, parent_obj.clone()))));
                                            }
                                            self.obj_path.push(obj_iri_c);
                                        }                                       
                                        let tag_name = slugify(String::from_utf8_lossy(s.local_name().as_ref()));
                                        let type_iri = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,tag_name));
                                        let obj_iri_c = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,self.obj_count));
                                        self.obj_count += 1;
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), oxrdf::vocab::rdf::TYPE, type_iri))));
                                        if let Some(parent_obj) = self.obj_path.last() {
                                            let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,tag_name));
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), pred, parent_obj.clone()))));
                                        }
                                        obj_iri = Some(obj_iri_c);
                                    }
                                    if let Some(obj_iri) = &obj_iri {
                                        let key = attr.key.as_ref();
                                        let value = attr.normalized_value(XmlVersion::Implicit1_0);
                                        if let Ok(value) = value {
                                            let pred = NamedNode::new_unchecked(format!("urn:xml:attr:{}#{}",self.file_hash,String::from_utf8_lossy(key)));
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(),pred, Literal::new_simple_literal(value)))));
                                        }
                                    }
                                }
                            }
                            if let Some(obj_iri) = obj_iri {
                                self.obj_path.push(obj_iri);
                            } else {
                                if let Some(pending_tag) = self.pending_tag.take() {
                                    let type_iri = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,pending_tag));
                                    let obj_iri_c = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,self.obj_count));
                                    self.obj_count += 1;
                                    f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), oxrdf::vocab::rdf::TYPE, type_iri))));
                                    if let Some(parent_obj) = self.obj_path.last() {
                                        let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,pending_tag));
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), pred, parent_obj.clone()))));
                                    }    
                                    self.obj_path.push(obj_iri_c);
                                } 
                                self.pending_tag = Some(slugify(String::from_utf8_lossy(s.local_name().as_ref())));
                                self.was_pred_tag = false;
                            }
                        }
                        Event::End(_s) => {
                            if !self.text.is_empty() {
                                if let Some(obj_iri) = self.obj_path.last() {
                                    if let Some(pending_tag) = self.pending_tag.take() {
                                        let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,pending_tag));
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(),pred, Literal::new_simple_literal(&self.text)))));
                                        self.was_pred_tag = true;
                                    } else {
                                        let pred = NamedNode::new_unchecked("urn:xml:text");
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(),pred, Literal::new_simple_literal(&self.text)))));
                                    }
                                }
                                self.text.clear();
                            }
                            if let Some(_pending_tag) = self.pending_tag.take() {
                                // Empty tag no attr no childs no text
                                self.was_pred_tag = true;
                            }
                            if !self.was_pred_tag {
                                if self.obj_path.len()>0 {
                                    let _s = self.obj_path.pop();
                                }
                            }
                            self.was_pred_tag = false;
                            self.pending_tag = None;                                                     
                        }
                        Event::Empty(s) => {
                            let mut obj_iri: Option<NamedNode> = None;
                            for attr in s.attributes() {
                                if let Ok(attr) = attr {
                                    if obj_iri.is_none() {
                                        if let Some(pending_tag) = self.pending_tag.take() {
                                            let type_iri = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,pending_tag));
                                            let obj_iri_c = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,self.obj_count));
                                            self.obj_count += 1;
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), oxrdf::vocab::rdf::TYPE, type_iri))));
                                            if let Some(parent_obj) = self.obj_path.last() {
                                                let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,pending_tag));
                                                f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), pred, parent_obj.clone()))));
                                            }
                                            self.obj_path.push(obj_iri_c);
                                        }
                                        let tag_name = slugify(String::from_utf8_lossy(s.local_name().as_ref()));
                                        let type_iri = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,tag_name));
                                        let obj_iri_c = NamedNode::new_unchecked(format!("urn:xml:{}#{}",self.file_hash,self.obj_count));
                                        self.obj_count += 1;
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), oxrdf::vocab::rdf::TYPE, type_iri))));      
                                        if let Some(parent_obj) = self.obj_path.last() {
                                            let pred = NamedNode::new_unchecked(format!("urn:xml:pred:{}#{}",self.file_hash,tag_name));
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri_c.clone(), pred, parent_obj.clone()))));
                                        }
                                        obj_iri = Some(obj_iri_c);
                                    }
                                    if let Some(obj_iri) = &obj_iri {
                                        let key = attr.key.as_ref();
                                        let value = attr.normalized_value(XmlVersion::Implicit1_0);
                                        if let Ok(value) = value {
                                            let pred = NamedNode::new_unchecked(format!("urn:xml:attr:{}#{}",self.file_hash,String::from_utf8_lossy(key)));
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(),pred, Literal::new_simple_literal(value)))));
                                        }
                                    }
                                }
                            }
                        }
                        Event::Text(mut s) => {
                            if !s.inplace_trim_start() {
                                let d = s.decode();
                                if let Ok(content) = d {
                                    self.text.push_str(&content);
                                }
                            }
                        }
                        Event::Eof => {
                            break;
                        }
                        Event::GeneralRef(s) => {
                            if s.is_char_ref() {
                                let c = s.resolve_char_ref();
                                if let Ok(Some(c)) = c {
                                    self.text.push(c);
                                }
                            } else {
                                if let Ok(content) = s.decode() {
                                    let r = resolve_xml_entity(&content);
                                    if let Some(c) = r {
                                        self.text.push_str(c);
                                    }
                                }
                            }
                        }
                        _ => {
                            // println!("unsupported event found");
                        }
                    }
                }
                Err(err) => {
                    return Err(io::Error::other(err));
                }
            }
            buf.clear();
        }
        Ok(())
    }
}