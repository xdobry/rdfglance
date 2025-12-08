use std::collections::HashMap;
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
}

impl RDFAdapter for SparqlAdapter {
    fn load_object(&mut self, iri: &str, node_data: &mut NodeData) -> Option<NObject> {
        let triples = {
            let response = {
                let mut form_data = HashMap::new();
                form_data.insert("limit", "500");
                form_data.insert("infer", "false");
                form_data.insert("offset", "0");
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
                form_data.insert("query", query.as_str());
                // println!("Query: {}", query);
                match self.client
                    .post(&self.endpoint)
                    .header(
                        "Content-Type",
                        "application/x-www-form-urlencoded;charset=UTF-8",
                    )
                    .header("Accept", "text/turtle")
                    .form(&form_data)
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
