pub mod graph_model;
pub mod ingest;
pub mod prefix_manager;
pub mod sorted_vec;
pub mod string_indexer;
pub mod value_types;

pub use graph_model::*;
pub use ingest::{IndexCache, add_triple};
pub use prefix_manager::PrefixManager;
pub use sorted_vec::SortedVec;
pub use string_indexer::{IndexSpan, StringCache, StringIndexer};
pub use value_types::ValueTypes;

pub type IriIndex = u32;
pub type LangIndex = u16;
pub type DataTypeIndex = u16;

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Copy, Clone)]
pub enum IriDisplay {
    Full,
    Prefixed,
    Label,
    LabelOrShorten,
    Shorten,
}

pub trait NodeStyleCatalog {
    fn label_index(&self, type_index: IriIndex) -> Option<IriIndex>;
    fn priority(&self, type_index: IriIndex) -> Option<u32>;
}
