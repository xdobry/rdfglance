use std::collections::HashMap;

use indexmap::IndexMap;
use oxrdf::vocab::rdf;

use crate::{config::IriDisplay, prefix_manager::PrefixManager};

pub type IriIndex = u32;
pub type LangIndex = u16;
pub type DataTypeIndex = u16;

#[derive(Clone,PartialEq, Eq)]
pub enum Literal {
    String(Box<str>),
    LangString(LangIndex, Box<str>),
    TypedString(DataTypeIndex, Box<str>),
}


impl AsRef<str> for Literal {
    fn as_ref(&self) -> &str {
        match self {
            Literal::String(str) => str,
            Literal::LangString(_index,str) => str,
            Literal::TypedString(_type, str) => str,
        }
    }
}

impl PartialOrd for Literal {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Literal {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}



pub type ObjectType = Literal;
pub type PredicateLiteral = (IriIndex, ObjectType);
pub type PredicateReference = (IriIndex, IriIndex);


pub struct NObject {
    pub types: Vec<IriIndex>,
    pub properties: Vec<PredicateLiteral>,
    pub references: Vec<PredicateReference>,
    pub reverse_references: Vec<PredicateReference>,
    pub has_subject: bool,
    pub is_blank_node: bool,
}

pub struct NodeData {
    pub node_cache: NodeCache,
    pub indexers: Indexers,
}

pub struct NodeCache {
    pub cache: IndexMap<Box<str>, NObject>,
}

pub struct Indexers {
    pub predicate_indexer: StringIndexer,
    pub type_indexer: StringIndexer,
    pub language_indexer: StringIndexer,
    pub datatype_indexer: StringIndexer,
}

pub enum LabelDisplayValue<'a> {
    FullStr(Box<str>),
    FullRef(&'a str),
    ShortAndIri(&'a str, &'a str)
}

impl LabelDisplayValue<'_> {
    pub fn as_str(&self) -> &str {
        match self {
            LabelDisplayValue::FullStr(str) => str,
            LabelDisplayValue::FullRef(str) => str,
            LabelDisplayValue::ShortAndIri(str, _) => str,
        }
    }
}

pub fn short_iri(iri: &str) -> &str {
    let last_hash = iri.rfind('#');
    if let Some(last_hash) = last_hash {
        return &iri[last_hash+1..];
    } else {
        let last_slash = iri.rfind('/');
        if let Some(last_slash) = last_slash {
            return &iri[last_slash+1..];
        }
    }
    let first_colon = iri.find(':');
    if let Some(first_colon) = first_colon {
        return &iri[first_colon+1..];
    }
    iri
}


impl NObject {
    pub fn has_same_type(&self,types: &Vec<IriIndex>) -> bool {
        for types in types {
            if self.types.contains(types) {
                return true;
            }
        }
        false
    }

    pub fn node_label<'a>(&'a self, iri: &'a str, label_predicate: &HashMap<IriIndex,IriIndex>, should_short_iri: bool, language_index: LangIndex) -> &'a str {
        let label_opt = self.node_label_opt(label_predicate, language_index);
        if let Some(label) = label_opt {
            return label;
        }
        if should_short_iri {
            return short_iri(iri);
        }
        iri
    }

    pub fn node_label_opt(&self, label_predicate: &HashMap<IriIndex,IriIndex>, language_index: LangIndex) -> Option<&str> {
        for type_index in self.types.iter() {
            if let Some(label_predicate) = label_predicate.get(type_index) {
                let prop = self.get_property(*label_predicate, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_ref());
                }
            }
        }
        None
    }

    pub fn get_property(&self, predicate_index: IriIndex, language_index: LangIndex) -> Option<&ObjectType> {
        let mut no_lang: Option<&ObjectType> = None;
        let mut fallback_lang: Option<&ObjectType> = None;
        for (predicate, value) in &self.properties {
            if predicate == &predicate_index {
                match value {
                    ObjectType::LangString(lang, _) => {
                        if  *lang==language_index {
                            return Some(value);
                        }
                        if *lang == 0 {
                            fallback_lang = Some(value);
                        }
                    }
                    ObjectType::String(_) | ObjectType::TypedString(_, _) => {
                        no_lang = Some(value);
                    }
                }
            }
        }
        if fallback_lang.is_some() {
            return fallback_lang;
        }
        if no_lang.is_some() {
            return no_lang;
        }
        None
    }

    pub fn apply_filter(&self, filter: &str, iri: &str) -> bool {
        if iri.contains(filter) {
            return true;
        }
        for (_predicate, value) in &self.properties {
            if value.as_ref().contains(filter) {
                return true;
            }
        }
        false
    }
}

impl Default for Indexers {
    fn default() -> Self {
        Self::new()
    }
}

impl Indexers {
    pub fn new() -> Self {
        let mut indexer = Self {
            predicate_indexer: StringIndexer::new(),
            type_indexer: StringIndexer::new(),
            language_indexer: StringIndexer::new(),
            datatype_indexer: StringIndexer::new(),
        };
        indexer.language_indexer.get_index("en");
        indexer.predicate_indexer.get_index("rdfs:label");
        indexer
    }
    
    pub fn get_type_index(&mut self, type_name: &str) -> IriIndex {
        self.type_indexer.get_index(type_name)
    }
    pub fn get_predicate_index(&mut self, predicate_name: &str) -> IriIndex {
        self.predicate_indexer.get_index(predicate_name)
    }
    pub fn get_language_index(&mut self, language: &str) -> LangIndex {
        self.language_indexer.get_index(language) as LangIndex
    }
    pub fn get_data_type_index(&mut self, data_type: &str) -> DataTypeIndex {
        self.datatype_indexer.get_index(data_type) as DataTypeIndex
    }
    pub fn clean(&mut self) {
        self.predicate_indexer.map.clear();
        self.type_indexer.map.clear();
        self.language_indexer.map.clear();
        self.datatype_indexer.map.clear();
    }
}

impl Default for NodeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeCache {
    pub fn new() -> Self {
        Self {
            cache: IndexMap::new(),
        }
    }
    
    pub fn get_node_by_index(&self, index: IriIndex) -> Option<(&Box<str>, &NObject)> {
        self.cache.get_index(index as usize)
    }
    pub fn get_node_by_index_mut(&mut self, index: IriIndex) -> Option<(&Box<str>, &mut NObject)> {
        self.cache.get_index_mut(index as usize)
    }
    pub fn get_node(&self, iri: &str) -> Option<&NObject> {
        self.cache.get(iri)
    }
    pub fn get_node_mut(&mut self, iri: &str) -> Option<&mut NObject> {
        self.cache.get_mut(iri)
    }
    pub fn get_node_index(&self, iri: &str) -> Option<IriIndex> {
        self.cache.get_full(iri).map(|(index, _, _)| index as IriIndex)
    }
    pub fn get_node_index_or_insert(&mut self, iri: &str, is_blank_node: bool) -> IriIndex {
        if let Some(index) = self.get_node_index(iri) {
            index
        } else {
            self.put_node(iri, NObject {
                types: Vec::new(),
                properties: Vec::new(),
                references: Vec::new(),
                reverse_references: Vec::new(),
                has_subject: false,
                is_blank_node,
            })
        }
    }
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    pub fn iter(&self) -> indexmap::map::Iter<Box<str>, NObject> {
        self.cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<Box<str>, NObject> {
        self.cache.iter_mut()
    }
    pub fn put_node(&mut self, iri: &str, node: NObject) -> IriIndex {
        let new_index = self.cache.len();
        let option = self.cache.insert(iri.into(), node);
        if option.is_some() {
            panic!("Node already exists");
        }
        new_index as IriIndex
    }
    pub fn put_node_replace(&mut self, iri: &str, node: NObject) {
        let option = self.cache.insert(iri.into(), node);
        if option.is_none() {
            panic!("Node can not be replaced");
        }
    }
}

impl Default for NodeData {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeData {
    pub fn new() -> Self {
        Self {
            node_cache: NodeCache::new(),
            indexers : Indexers::new(),
        }
    }
    pub fn get_node_by_index(&self, index: IriIndex) -> Option<(&Box<str>,&NObject)> {
        self.node_cache.get_node_by_index(index)
    }
    pub fn get_node_by_index_mut(&mut self, index: IriIndex) -> Option<(&Box<str>,&mut NObject)> {
        self.node_cache.get_node_by_index_mut(index)
    }
    pub fn get_node(&self, iri: &str) -> Option<&NObject> {
        self.node_cache.get_node(iri)
    }
    pub fn get_node_mut(&mut self, iri: &str) -> Option<&mut NObject> {
        self.node_cache.get_node_mut(iri)
    }
    pub fn get_node_index(&self, iri: &str) -> Option<IriIndex> {
        self.node_cache.cache.get_full(iri).map(|(index, _, _)| index as IriIndex)
    }
    pub fn get_node_index_or_insert(&mut self, iri: &str, is_blank_node: bool) -> IriIndex {
        self.node_cache.get_node_index_or_insert(iri, is_blank_node)
    }
    pub fn len(&self) -> usize {
        self.node_cache.len()
    }
    pub fn is_empty(&self) -> bool {
        self.node_cache.is_empty()
    }
    pub fn iter(&self) -> indexmap::map::Iter<Box<str>, NObject> {
        self.node_cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<Box<str>, NObject> {
        self.node_cache.iter_mut()
    }
    pub fn put_node(&mut self, iri: &str, node: NObject) -> IriIndex {
        self.node_cache.put_node(iri,node)
    }
    pub fn put_node_replace(&mut self, iri: &str, node: NObject) {
        self.node_cache.put_node_replace(iri, node)
    }
    pub fn split_mut(&mut self) -> (&mut Indexers, &mut NodeCache) {
        (&mut self.indexers, &mut self.node_cache)
    }
    pub fn get_type(&self, type_index: IriIndex) -> Option<&str> {
        self.indexers.type_indexer.index_to_str(type_index)
    }
    pub fn get_predicate(&self, predicate_index: IriIndex) -> Option<&str> {
        self.indexers.predicate_indexer.index_to_str(predicate_index)
    }
    pub fn get_type_index(&mut self, type_name: &str) -> IriIndex {
        self.indexers.type_indexer.get_index(type_name)
    }
    pub fn get_predicate_index(&mut self, predicate_name: &str) -> IriIndex {
        self.indexers.predicate_indexer.get_index(predicate_name)
    }
    pub fn get_language(&self, language_index: LangIndex) -> Option<&str> {
        self.indexers.language_indexer.index_to_str(language_index as IriIndex)
    }
    pub fn get_language_index(&mut self, language: &str) -> LangIndex {
        self.indexers.language_indexer.get_index(language) as LangIndex
    }
    pub fn unique_predicates(&self) -> usize {
        self.indexers.predicate_indexer.map.len()
    }
    pub fn unique_types(&self) -> usize {
        self.indexers.type_indexer.map.len()
    }
    pub fn unique_languages(&self) -> usize {
        self.indexers.language_indexer.map.len()
    }
    pub fn unique_data_types(&self) -> usize {
        self.indexers.datatype_indexer.map.len()
    }
    pub fn clean(&mut self) {
        self.node_cache.cache.clear();
        self.indexers.clean();
    }
    pub fn type_label(&self, type_index: IriIndex, language_index: LangIndex) -> Option<&str> {
        let type_iri = self.indexers.type_indexer.index_to_str(type_index);
        if let Some(type_iri) = type_iri {
            if let Some(node) = self.get_node(type_iri) {
                let prop = node.get_property(0, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_ref());
                }
            }
        }
        None
    }
    pub fn predicate_label(&self, type_index: IriIndex, language_index: LangIndex) -> Option<&str> {
        let predicate_iri = self.indexers.predicate_indexer.index_to_str(type_index);
        if let Some(predicate_iri) = predicate_iri {
            if let Some(node) = self.get_node(predicate_iri) {
                let prop = node.get_property(0, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_ref());
                }
            }
        }
        None
    }
    pub fn type_display(&self, type_index: IriIndex, label_context: &LabelContext) -> LabelDisplayValue {
        let type_iri = self.indexers.type_indexer.index_to_str(type_index);
        if let Some(type_iri) = type_iri {
            match label_context.iri_display {
                IriDisplay::Full => {
                    let full_iri = label_context.prefix_manager.get_full_opt(type_iri);
                    if let Some(full_iri) = full_iri {
                        LabelDisplayValue::FullStr(full_iri)
                    } else {
                        LabelDisplayValue::FullRef(type_iri)
                    }
                }
                IriDisplay::Prefixed => {
                    LabelDisplayValue::FullRef(type_iri)
                }
                IriDisplay::Label => {
                    let type_label = self.type_label(type_index, label_context.language_index);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label,type_iri)
                    } else {
                        LabelDisplayValue::FullRef(type_iri)
                    } 
                }
                IriDisplay::Shorten => {
                    LabelDisplayValue::FullRef(short_iri(type_iri))
                }
                IriDisplay::LabelOrShorten => {
                    let type_label = self.type_label(type_index, label_context.language_index);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label,type_iri)
                    } else {
                        LabelDisplayValue::FullRef(short_iri(type_iri))
                    } 
                }
            }
        } else {
            LabelDisplayValue::FullRef("!Unknown")
        }
    }

    pub fn predicate_display(&self, predicate_index: IriIndex, label_context: &LabelContext) -> LabelDisplayValue {
        let predicate_iri = self.indexers.predicate_indexer.index_to_str(predicate_index);
        if let Some(predicate_iri) = predicate_iri {
            match label_context.iri_display {
                IriDisplay::Full => {
                    let full_iri = label_context.prefix_manager.get_full_opt(predicate_iri);
                    if let Some(full_iri) = full_iri {
                        LabelDisplayValue::FullStr(full_iri)
                    } else {
                        LabelDisplayValue::FullRef(predicate_iri)
                    }
                }
                IriDisplay::Prefixed => {
                    LabelDisplayValue::FullRef(predicate_iri)
                }
                IriDisplay::Label => {
                    let type_label = self.type_label(predicate_index, label_context.language_index);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label,predicate_iri)
                    } else {
                        LabelDisplayValue::FullRef(predicate_iri)
                    } 
                }
                IriDisplay::Shorten => {
                    LabelDisplayValue::FullRef(short_iri(predicate_iri))
                }
                IriDisplay::LabelOrShorten => {
                    let type_label = self.type_label(predicate_index, label_context.language_index);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label,predicate_iri)
                    } else {
                        LabelDisplayValue::FullRef(short_iri(predicate_iri))
                    } 
                }
            }
        } else {
            LabelDisplayValue::FullRef("!Unknown")
        }
    }

    pub fn resolve_rdf_lists(&mut self, prefix_manager: &PrefixManager) {
        let predicate_first = self.indexers.predicate_indexer.get_index(&prefix_manager.get_prefixed(rdf::FIRST.as_str()));
        let predicate_rest = self.indexers.predicate_indexer.get_index(&prefix_manager.get_prefixed(rdf::REST.as_str()));
        let node_nil = self.get_node_index(&prefix_manager.get_prefixed(rdf::NIL.as_str())).unwrap_or(0);
        let mut next_list : Vec<(IriIndex,IriIndex)> = Vec::new();
        for (node_index, (_,node)) in self.iter().enumerate() {
            for (predicate,ref_index) in &node.references {
                if *predicate == predicate_rest {
                    next_list.push((node_index as IriIndex,*ref_index));
                }
            }
        }
        let mut head_nodes: Vec<IriIndex> = Vec::new();
        for &(first, _) in &next_list {
            if !next_list.iter().any(|&(_, second)| second == first) {
                head_nodes.push(first);
            }
        }
        for head_node in &head_nodes {
            let mut current_node = Some(*head_node);
            let mut list: Vec<IriIndex> = Vec::new();
            while let Some(c_node) = current_node {
                if c_node != node_nil {
                    list.push(c_node);
                }
                current_node = next_list.iter().find_map(|e| {
                    if e.0 == c_node {
                        return Some(e.1);
                    }
                    None
                });
            }
            let mut list_holders: Vec<(IriIndex,IriIndex)> = Vec::new();
            for node_index in list.iter() {
                let node = self.get_node_by_index_mut(*node_index).unwrap().1;
                let mut literal : Option<Literal> = None;
                let mut reference: Option<IriIndex> = None;
                for (predicate,value) in &node.properties {
                    if *predicate == predicate_first {
                        literal = Some(value.clone());
                        break;
                    }
                }
                if literal.is_none() {
                    for (predicate,value) in &node.references {
                        if *predicate == predicate_first {
                            reference = Some(*value);
                            break;
                        }
                    }
                }
                if list_holders.is_empty() {
                   list_holders = node.reverse_references.clone();
                   if list_holders.is_empty() {
                        continue;
                   }
                }
                for (predicate,holder) in &list_holders {
                    let holder_node: &mut NObject = self.get_node_by_index_mut(*holder).unwrap().1;
                    if let Some(literal) = &literal {
                        holder_node.properties.push((*predicate,literal.clone()));
                    } else if let Some(reference) = reference {
                        holder_node.references.push((*predicate,reference));
                    }
                }
            }
            // Remove reference to rdf list
            for (predicate,holder) in &list_holders {
                let holder_node: &mut NObject = self.get_node_by_index_mut(*holder).unwrap().1;
                holder_node.references.retain(|(ref_predicate,ref_index)| ref_predicate != predicate || ref_index != head_node);
            }
        }
    }
} 

pub struct StringIndexer {
    pub map: IndexMap<Box<str>, IriIndex>,
}

impl Default for StringIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl StringIndexer {
    pub fn new() -> Self {
        Self { map: IndexMap::new() }
    }

    /// Converts a string to an index, assigning a new index if it's unknown
    fn get_index(&mut self, s: &str) -> IriIndex {
        if let Some(&idx) = self.map.get(s) {
            idx as IriIndex
        } else {
            let idx = self.map.len();
            self.map.insert(s.into(), idx as IriIndex);
            idx as IriIndex
        }
    }

    /// Retrieves a string from an index
    fn index_to_str(&self, index: IriIndex) -> Option<&str> {
        self.map.get_index(index as usize).map(|(key, _)| key.as_ref())
    }

    pub fn iter(&self) -> indexmap::map::Iter<Box<str>, IriIndex> {
        self.map.iter()
    }
}

pub struct LabelContext<'a> {
    pub language_index: LangIndex,
    pub iri_display: IriDisplay,
    pub prefix_manager: &'a PrefixManager,
}

impl<'a> LabelContext<'a> {
    pub fn new(language_index: LangIndex, iri_display: IriDisplay, prefix_manager: &'a PrefixManager) -> Self {
        Self {
            language_index,
            iri_display,
            prefix_manager,
        }
    }
}


#[cfg(test)]
mod tests {
    use oxrdf::Triple;
    use crate::prefix_manager::PrefixManager;
    use super::{NodeData, StringIndexer};

    #[test]
    fn test_sting_indexer() {
        let mut string_indexer = StringIndexer::new();
        let index1 = string_indexer.get_index("test");
        let index2 = string_indexer.get_index("test");
        assert_eq!(index1, index2);
        let index3 = string_indexer.get_index("test2");
        assert_ne!(index1, index3);
        assert_eq!(index1+1, index3);
        let s = string_indexer.index_to_str(index2);
        assert!(s.is_some());
        assert_eq!("test",s.unwrap());
        assert!(string_indexer.index_to_str(100).is_none());  
    }

    #[test]
    fn test_node_data() {
        let mut node_data = NodeData::new();
        let prefix_manager = PrefixManager::new();

        let language_filter: Vec<String> = vec![];
        let mut index_cache = crate::rdfwrap::IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        let subject = oxrdf::NamedNode::new("http://example.org#subject").unwrap();
        let rdf_type = oxrdf::NamedNode::new("http://example.org#ClassFoo").unwrap();
        let data_predicate = oxrdf::NamedNode::new("http://example.org#pred").unwrap();

        let mut tcount = 0;
        let triple = Triple::new(
            subject.clone(),
            oxrdf::vocab::rdf::TYPE,
            rdf_type,
        );
        crate::rdfwrap::add_triple(&mut tcount,&mut node_data.indexers, &mut node_data.node_cache,
            triple, &mut index_cache, &language_filter, &prefix_manager);

        crate::rdfwrap::add_triple(&mut tcount,&mut node_data.indexers, &mut node_data.node_cache,
            Triple::new(
                subject.clone(),
                data_predicate.clone(),
                oxrdf::Literal::new_simple_literal("test"),
            ), &mut index_cache, &language_filter, &prefix_manager);

        let pred_index = node_data.indexers.predicate_indexer.get_index(data_predicate.as_str());
        let node = node_data.get_node(subject.as_str());

        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.types.len(), 1);
        assert!(node.has_subject);
        assert!(!node.is_blank_node);
        assert_eq!(0,node.references.len());
        assert_eq!(0,node.references.len());
        assert_eq!(1,node.properties.len());

        let lit = node.get_property(pred_index, 0);
        assert!(lit.is_some());
        assert_eq!(lit.unwrap().as_ref(), "test");

    }
}
