pub mod persistency;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql;
pub mod rdfwrap;
pub mod svg;
pub mod visual_query;
pub mod csv2rdf;
pub mod json2rdf;
pub mod xml2rdf;

pub use self::persistency::*;