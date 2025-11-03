pub mod persistency;
#[cfg(not(target_arch = "wasm32"))]
pub mod sparql;
pub mod rdfwrap;

pub use self::persistency::*;