use std::io::{self, Read};
use csv::{Reader, StringRecord};
use oxrdf::{NamedNode, Triple};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use slug::slugify;
use base62::encode;
use xxhash_rust::xxh3::xxh3_64;

pub struct CSVRDFParser<R: Read> {
    reader: Reader<R>,
    headers: Option<Vec<NamedNode>>,
    current_record: Option<StringRecord>,
    current_column: usize,
    row_subj: Option<NamedNode>,
    row_number: usize,
    type_node: NamedNode,
    csv_hash: String,
}

impl<R: Read> CSVRDFParser<R> {
    pub fn for_reader(reader: R, file_base: String) -> Self {
        Self {
            reader: Reader::from_reader(reader),
            headers: None,
            current_record: None,
            // 0 is meaning that the type of column should be emitted
            current_column: 0,
            row_number: 0,
            type_node: NamedNode::new_unchecked(format!("urn:csv:{}",slugify(&file_base))),
            row_subj: None,
            csv_hash: hash_string(&file_base),
        }
    }
}

impl<R: Read> Iterator for CSVRDFParser<R> {
    type Item = Result<Triple, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        // Load headers once
        if self.headers.is_none() {
            match self.reader.headers() {
                Ok(headers) => {
                    let mut columns: Vec<NamedNode> = Vec::with_capacity(headers.len());
                    for col in headers {
                        let encoded = utf8_percent_encode(col, NON_ALPHANUMERIC);
                        columns.push( NamedNode::new_unchecked(format!("urn:col:{}",encoded)));
                    }
                    self.headers = Some(columns);
                }
                Err(err) => {
                    return Some(Err(io::Error::other(err)));
                }
            }
        }

        loop {
            // Need a new record?
            let need_record = self.current_record.is_none()
                || self.current_column >= self.current_record.as_ref()?.len()+1;

            if need_record {
                let mut record = StringRecord::new();

                match self.reader.read_record(&mut record) {
                    Ok(false) => return None, // EOF
                    Ok(true) => {
                        self.current_record = Some(record);
                        self.current_column = 0;
                        self.row_number += 1;
                        self.row_subj = Some( NamedNode::new_unchecked(format!("urn:row:{}:{}", self.csv_hash, self.row_number)))
                    }
                    Err(err) => {
                        return Some(Err(io::Error::other(err)));
                    }
                }
            }

            let headers = self.headers.as_ref().unwrap();
            let record = self.current_record.as_ref().unwrap();

            let col = self.current_column;
            self.current_column += 1;

            let triple = if col == 0 {
                let subj = self.row_subj.as_ref().unwrap();
                Triple::new(subj.clone(), oxrdf::vocab::rdf::TYPE, self.type_node.clone())
            } else {
                let value = &record[col-1];
                if value.len() == 0 {
                    // Do not emit empty string (no value)
                    continue;
                }
                let pred = &headers[col-1];
                // Build RDF triple here
                let subj = self.row_subj.as_ref().unwrap();
                let objv = oxrdf::Literal::new_simple_literal(value);
                Triple::new(subj.clone(), pred.clone(), objv)
            };
            return Some(Ok(triple));
        }
    }
}

pub fn hash_string(name: &str) -> String {
    let h = xxh3_64(name.as_bytes());
    encode(h)
}
