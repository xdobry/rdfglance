use std::collections::HashMap;

use eframe::egui::Pos2;
use indexmap::IndexMap;
use rand::Rng;

use crate::SortedVec;

pub type IriIndex = usize;
pub type ObjectType = String;
pub type PredicateLiteral = (IriIndex, ObjectType);
pub type PredicateReference = (IriIndex, IriIndex);

pub struct NObject {
    // The iri exists twice in the object and in the cache as key,
    // It could be optimized to store the node index in the nobject
    pub iri: String,
    pub types: Vec<IriIndex>,
    pub properties: Vec<PredicateLiteral>,
    pub references: Vec<PredicateReference>,
    pub reverse_references: Vec<PredicateReference>,
    pub pos: Pos2,
    pub has_subject: bool,
    pub is_bank_node: bool,
}

impl NObject {
    pub fn has_same_type(&self,types: &Vec<IriIndex>) -> bool {
        for types in types {
            if self.types.contains(types) {
                return true;
            }
        }
        return false;
    }

    pub fn node_label(&self, label_predicate: &HashMap<IriIndex,IriIndex>, short_iri: bool) -> &str {
        let label_opt = self.node_label_opt(label_predicate);
        if let Some(label) = label_opt {
            return label;
        }
        if short_iri {
            let last_hash = self.iri.rfind('#');
            if let Some(last_hash) = last_hash {
                return &self.iri[last_hash+1..];
            } else {
                let last_slash = self.iri.rfind('/');
                if let Some(last_slash) = last_slash {
                    return &self.iri[last_slash+1..];
                }
            }
        }
        return &self.iri
    }

    pub fn node_label_opt(&self, label_predicate: &HashMap<IriIndex,IriIndex>) -> Option<&str> {
        for type_index in self.types.iter() {
            if let Some(label_predicate) = label_predicate.get(type_index) {
                for (predicate_index, value) in &self.properties {
                    if label_predicate == predicate_index {
                        return Some(value);
                    }
                }
            }
        }
        return None;
    }

    pub fn get_property(&self, predicate_index: IriIndex) -> Option<&ObjectType> {
        for (predicate, value) in &self.properties {
            if predicate == &predicate_index {
                return Some(value);
            }
        }
        None
    }

    pub fn apply_filter(&self, filter: &str) -> bool {
        if self.iri.contains(filter) {
            return true;
        }
        for (_predicate, value) in &self.properties {
            if value.contains(filter) {
                return true;
            }
        }
        return false;
    }
}

pub struct NodeData {
    pub node_cache: NodeCache,
    indexers: Indexers,
}

pub struct NodeCache {
    cache: IndexMap<String, NObject>,
}

pub struct Indexers {
    predicate_indexer: StringIndexer,
    type_indexer: StringIndexer,
}

impl Indexers {
    pub fn get_type_index(&mut self, type_name: &str) -> IriIndex {
        self.type_indexer.to_index(type_name)
    }
    pub fn get_predicate_index(&mut self, predicate_name: &str) -> IriIndex {
        self.predicate_indexer.to_index(predicate_name)
    }
}

impl NodeCache {
    pub fn get_node_by_index(&self, index: IriIndex) -> Option<&NObject> {
        self.cache.get_index(index).map(|(_, node)| node)
    }
    pub fn get_node_by_index_mut(&mut self, index: IriIndex) -> Option<&mut NObject> {
        self.cache.get_index_mut(index).map(|(_, node)| node)
    }
    pub fn get_node(&self, iri: &str) -> Option<&NObject> {
        self.cache.get(iri)
    }
    pub fn get_node_mut(&mut self, iri: &str) -> Option<&mut NObject> {
        self.cache.get_mut(iri)
    }
    pub fn get_node_index(&self, iri: &str) -> Option<IriIndex> {
        self.cache.get_full(iri).map(|(index, _, _)| index)
    }
    pub fn get_node_index_or_insert(&mut self, iri: &str) -> IriIndex {
        if let Some(index) = self.get_node_index(iri) {
            index
        } else {
            self.put_node(NObject {
                iri: iri.to_string(),
                types: Vec::new(),
                properties: Vec::new(),
                references: Vec::new(),
                reverse_references: Vec::new(),
                pos: Pos2::new(
                    rand::rng().random_range(0.0..100.0),
                    rand::rng().random_range(0.0..100.0),
                ),
                has_subject: false,
                is_bank_node: false,
            })
        }
    }
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn iter(&self) -> indexmap::map::Iter<String, NObject> {
        self.cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<String, NObject> {
        self.cache.iter_mut()
    }
    pub fn put_node(&mut self, node: NObject) -> usize {
        let new_index = self.cache.len();
        let option = self.cache.insert(node.iri.clone(), node);
        if !option.is_none() {
            panic!("Node already exists");
        }
        return new_index;
    }
    pub fn put_node_replace(&mut self, node: NObject) {
        let option = self.cache.insert(node.iri.clone(), node);
        if option.is_none() {
            panic!("Node can not be replaced");
        }
    }
    pub fn to_center(&mut self, visible_nodes: &SortedVec) {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut count = 0;
        for visible_index in visible_nodes.data.iter() {
            if let Some(node) = self.get_node_by_index(*visible_index) {
                x += node.pos.x;
                y += node.pos.y;
                count += 1;
            }    
        }
        x /= count as f32;
        y /= count as f32;
        for node in self.cache.iter_mut() {
            node.1.pos.x -= x;
            node.1.pos.y -= y;
        }
        /*
        for visible_index in visible_nodes.iter() {
            if let Some(node) = self.get_node_by_index(*visible_index) {
                node.pos.x -= x;
                node.pos.y -= y;
                }
        }
         */
    }
}

impl NodeData {
    pub fn new() -> Self {
        Self {
            node_cache: NodeCache {
                cache: IndexMap::new(),
            },
            indexers : Indexers {
                predicate_indexer: StringIndexer::new(),
                type_indexer: StringIndexer::new(),
            }
        }
    }
    pub fn get_node_by_index(&self, index: IriIndex) -> Option<&NObject> {
        self.node_cache.get_node_by_index(index)
    }
    pub fn get_node_by_index_mut(&mut self, index: IriIndex) -> Option<&mut NObject> {
        self.node_cache.get_node_by_index_mut(index)
    }
    pub fn get_node(&self, iri: &str) -> Option<&NObject> {
        self.node_cache.get_node(iri)
    }
    pub fn get_node_mut(&mut self, iri: &str) -> Option<&mut NObject> {
        self.node_cache.get_node_mut(iri)
    }
    pub fn get_node_index(&self, iri: &str) -> Option<IriIndex> {
        self.node_cache.cache.get_full(iri).map(|(index, _, _)| index)
    }
    pub fn get_node_index_or_insert(&mut self, iri: &str) -> IriIndex {
        self.node_cache.get_node_index_or_insert(iri)
    }
    pub fn len(&self) -> usize {
        self.node_cache.len()
    }
    pub fn iter(&self) -> indexmap::map::Iter<String, NObject> {
        self.node_cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<String, NObject> {
        self.node_cache.iter_mut()
    }
    pub fn put_node(&mut self, node: NObject) -> usize {
        self.node_cache.put_node(node)
    }
    pub fn put_node_replace(&mut self, node: NObject) {
        self.node_cache.put_node_replace(node)
    }
    pub fn split_mut(&mut self) -> (&mut Indexers, &mut NodeCache) {
        (&mut self.indexers, &mut self.node_cache)
    }
    pub fn get_type(&self, type_index: IriIndex) -> Option<&str> {
        self.indexers.type_indexer.from_index(type_index)
    }
    pub fn get_predicate(&self, predicate_index: IriIndex) -> Option<&str> {
        self.indexers.predicate_indexer.from_index(predicate_index)
    }
    pub fn get_type_index(&mut self, type_name: &str) -> IriIndex {
        self.indexers.type_indexer.to_index(type_name)
    }
    pub fn get_predicate_index(&mut self, predicate_name: &str) -> IriIndex {
        self.indexers.predicate_indexer.to_index(predicate_name)
    }
    pub fn unique_predicates(&self) -> usize {
        self.indexers.predicate_indexer.map.len()
    }
    pub fn unique_types(&self) -> usize {
        self.indexers.type_indexer.map.len()
    }
    pub fn clean(&mut self) {
        self.node_cache.cache.clear();
    }
}

struct StringIndexer {
    map: IndexMap<String, usize>,
}

impl StringIndexer {
    fn new() -> Self {
        Self { map: IndexMap::new() }
    }

    /// Converts a string to an index, assigning a new index if it's unknown
    fn to_index(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.map.get(s) {
            idx
        } else {
            let idx = self.map.len();
            self.map.insert(s.to_string(), idx);
            idx
        }
    }

    /// Retrieves a string from an index
    fn from_index(&self, index: usize) -> Option<&str> {
        self.map.get_index(index).map(|(key, _)| key.as_str())
    }
}
