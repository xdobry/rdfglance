use std::io::BufReader;

use crate::domain::{NObject, NodeData};
use super::rdfwrap::{RDFAdapter, RDFWrap};
use oxttl::TurtleParser;
use reqwest::blocking::Client;

pub struct SparqlAdapter {
    endpoint: String,
    client: Client,
}

impl SparqlAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: Client::new(),
            // client: Client::builder().no_proxy().build().unwrap(),
        }
    }

    fn encode_form_component(value: &str) -> String {
        let mut encoded = String::with_capacity(value.len());
        for b in value.bytes() {
            match b {
                b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~' => encoded.push(b as char),
                b' ' => encoded.push('+'),
                _ => encoded.push_str(&format!("%{:02X}", b)),
            }
        }
        encoded
    }
}

impl RDFAdapter for SparqlAdapter {
    fn load_object(&mut self, iri: &str, node_data: &mut NodeData) -> Option<NObject> {
        let triples = {
            let response = {
                let query = format!(
                    r#"construct {{
   ?o ?p ?v.
   ?a ?b ?o.
}}
where {{
    BIND (<{}> as ?o)
    ?o ?p ?v.
    OPTIONAL {{?a ?b ?o.}}
}} limit 500"#,
                    iri
                );
                let form_body = format!(
                    "limit=500&infer=false&offset=0&query={}",
                    SparqlAdapter::encode_form_component(&query)
                );
                // println!("Query: {}", query);
                match self.client
                    .post(&self.endpoint)
                    .header(
                        "Content-Type",
                        "application/x-www-form-urlencoded;charset=UTF-8",
                    )
                    .header("Accept", "text/turtle")
                    .body(form_body)
                    .send()
                {
                    Ok(response) => response,
                    Err(e) => {
                        eprintln!("Error SPARQL Call: {}", e);
                        return None;
                    }
                }
            };
            if response.status() != 200 {
                eprintln!("Error SPARQL call: {}", response.status());
                return None;
            }
            let buf_reader = BufReader::new(response);
            // let buffer = String::new();
            // buf_reader.read_to_string(&mut buffer).unwrap();
            // println!("Response: {}", buffer);
            // return None;
            let parser = TurtleParser::new().for_reader(buf_reader);
            match parser.collect::<Result<Vec<_>, _>>() {
                Ok(triples) => triples,
                Err(e) => {
                    eprintln!("Error parsing Turtle: {}", e);
                    Vec::new()
                }
            }
        };
        RDFWrap::load_from_triples(&triples, iri, node_data)
    }
}
