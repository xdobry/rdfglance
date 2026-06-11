use std::io::{self, BufReader, Cursor, Read};
use egui::TextBuffer;
use oxrdf::{Literal, NamedNode, Triple};
use slug::slugify;
use json_event_parser::{ReaderJsonParser, JsonEvent};
use serde_json::{Value};
use std::io::BufRead;

use crate::integration::{csv2rdf::hash_string, rdfwrap::ParseItem};

pub struct JsonRdfParser<R: Read> {
    reader: ReaderJsonParser<R>,
    prop_path: Vec<String>,
    obj_path: Vec<(NamedNode,usize)>,
    pub(crate) obj_count: usize,
    file_hash: String,
    array_depth: usize,
}

impl<R: Read> JsonRdfParser<R> {
    pub fn for_reader(reader: R, file_base: String) -> Self {
        Self {
            reader: ReaderJsonParser::new(reader),
            prop_path: Vec::new(),
            obj_path: Vec::new(),
            obj_count: 0,
            array_depth: 0,
            file_hash: hash_string(&file_base),
        }
    }

    pub fn for_reader_continue(reader: R, file_hash: String, obj_count: usize) -> Self {
        Self {
            reader: ReaderJsonParser::new(reader),
            prop_path: Vec::new(),
            obj_path: Vec::new(),
            obj_count,
            array_depth: 0,
            file_hash,
        }
    }
}

impl<R: Read> JsonRdfParser<R> {
    pub fn parse<F>(&mut self, mut f: F) -> io::Result<()>
    where
        F: FnMut(ParseItem),
    {
        loop {
            let event = self.reader.parse_next();
            match event {
                Ok(event) => {
                    match event {
                        JsonEvent::Eof => {
                            break;
                        }
                        JsonEvent::StartObject => {
                            let obj_iri = NamedNode::new_unchecked(format!("urn:json:{}#{}",self.file_hash,self.obj_count));
                            let mut type_iri = format!("urn:json:{}",self.file_hash);
                            for path in &self.prop_path {
                                type_iri.push(':');
                                type_iri.push_str(&path);
                            }
                            self.obj_count += 1;
                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(), oxrdf::vocab::rdf::TYPE, NamedNode::new_unchecked(&type_iri)))));
                            // Link to parent object
                            if let Some(parent_obj) = self.obj_path.last() {
                                if let Some(prop) = self.prop_path.last() {
                                    let prop_pred = NamedNode::new_unchecked(format!("urn:json:{}#{}",self.file_hash, prop));
                                    f(ParseItem::Triple(Ok(Triple::new(obj_iri.clone(),prop_pred, parent_obj.0.clone()))));
                                }
                            }
                            self.obj_path.push((obj_iri,self.array_depth));
                            self.array_depth = 0;
                        }
                        JsonEvent::EndObject => {
                            if let Some(last_obj) = self.obj_path.pop() {
                                self.array_depth = last_obj.1;
                            }
                            if self.array_depth==0 && self.prop_path.len()>0 {
                                let p = self.prop_path.pop();
                            }
                        }
                        JsonEvent::StartArray => {
                            self.array_depth += 1;
                        }
                        JsonEvent::EndArray => {
                            self.array_depth -= 1;
                            if self.prop_path.len()>0 {
                                let _p = self.prop_path.pop();
                            }                           
                        }
                        JsonEvent::ObjectKey(k) => {
                            self.prop_path.push(slugify(k.as_str()));
                        }
                        JsonEvent::Null => {
                            if self.array_depth==0 && self.prop_path.len()>0 {
                               let _p = self.prop_path.pop();
                            }   
                        }
                        JsonEvent::String(v) => {
                            if v.len()>0 {
                                if let Some(obj_iri) = self.obj_path.last() {
                                    if let Some(prop) = self.prop_path.last() {
                                        let prop_pred = NamedNode::new_unchecked(format!("urn:json:{}#{}",self.file_hash, prop));
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::new_simple_literal(v)))));
                                    }
                                }
                            }
                            if self.array_depth==0 && self.prop_path.len()>0 {
                               let _p = self.prop_path.pop();
                            }
                        }
                        JsonEvent::Number(v) => {
                            if let Some(obj_iri) = self.obj_path.last() {
                                if let Some(prop) = self.prop_path.last() {
                                    let prop_pred = NamedNode::new_unchecked(format!("urn:json:{}#{}",self.file_hash, prop));
                                    let v_str: &str = v.as_str();
                                    let value: Result<Value,_> = serde_json::from_str(v_str);
                                    if let Ok(Value::Number(n)) = value {
                                        if n.is_i64() {
                                            let nv: i64 = n.as_i64().unwrap();
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::from(nv)))));
                                        } else if n.is_u64() {
                                            let nv: u64 = n.as_u64().unwrap();
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::from(nv)))));
                                        } else if n.is_f64() {
                                            let nv: f64 = n.as_f64().unwrap();
                                            f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::from(nv)))));
                                        }
                                    } else {
                                        f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::new_simple_literal(v)))));
                                    }
                                }
                            }
                            if self.array_depth==0 && self.prop_path.len()>0 {
                               let _p = self.prop_path.pop();
                            }
                        }
                        JsonEvent::Boolean(v) => {
                            if let Some(obj_iri) = self.obj_path.last() {
                                if let Some(prop) = self.prop_path.last() {
                                    let prop_pred = NamedNode::new_unchecked(format!("urn:json:{}#{}",self.file_hash, prop));
                                    f(ParseItem::Triple(Ok(Triple::new(obj_iri.0.clone(),prop_pred, Literal::from(v)))));
                                }
                            }
                            if self.array_depth==0 && self.prop_path.len()>0 {
                               let _p = self.prop_path.pop();
                            }
                        }
                    }
                }
                Err(err) => {
                    return Err(io::Error::other(err));
                }
            }
        }
        Ok(())
    }
}

pub struct NdJsonRdfParser<R: Read> {
    buffer: BufReader<R>,
    file_hash: String,
}

impl<R: Read> NdJsonRdfParser<R> {
    pub fn for_reader(reader: R, file_base: String) -> Self {
        Self {
            buffer: BufReader::new(reader),
            file_hash: hash_string(&file_base),
        }
    }
}

impl<R: Read> NdJsonRdfParser<R> {
    pub fn parse<F>(&mut self, mut f: F) -> io::Result<()>
    where
        F: FnMut(ParseItem),
    {
        let mut count: usize = 0;
        loop {
            let mut line = String::new();
            let n = self.buffer.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            let cursor = Cursor::new(line);
            let mut json_parser = JsonRdfParser::for_reader_continue(cursor, self.file_hash.clone(), count);
            json_parser.parse(|parse_item| {
                f(parse_item);
            })?;
            count = json_parser.obj_count;
        }       
        Ok(())
    }
}