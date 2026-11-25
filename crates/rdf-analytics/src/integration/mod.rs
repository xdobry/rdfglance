pub mod csv2rdf;
pub mod json2rdf;

use oxrdf::Triple;
use std::io;

pub enum ParseItem {
    Triple(Result<Triple, io::Error>),
    Prefix(String, String),
}
