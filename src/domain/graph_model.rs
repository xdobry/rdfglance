use indexmap::IndexMap;
use oxrdf::vocab::rdf;

use crate::domain::{config::IriDisplay, graph_styles::GVisualizationStyle, prefix_manager::PrefixManager, string_indexer::{IndexSpan, StringCache, StringIndexer}};

pub type IriIndex = u32;
pub type LangIndex = u16;
pub type DataTypeIndex = u16;

#[derive(Clone, PartialEq, Eq)]
pub enum Literal {
    StringShort(IriIndex),
    String(IndexSpan),
    LangString(LangIndex, IndexSpan),
    TypedString(DataTypeIndex, IndexSpan),
}

impl Literal {
    pub fn as_str_ref<'a>(&self, indexers: &'a Indexers) -> &'a str {
        match self {
            Literal::StringShort(index) => {
                let str = indexers.short_literal_indexer.index_to_str(*index).unwrap();
                str
            }
            Literal::String(str) => {
                let str = indexers.literal_cache.get_str(*str);
                str
            }
            Literal::LangString(_index, str) => {
                let str = indexers.literal_cache.get_str(*str);
                str
            }
            Literal::TypedString(_type, str) => {
                let str = indexers.literal_cache.get_str(*str);
                str
            }
        }
    }
}

pub type ObjectType = Literal;
pub type PredicateLiteral = (IriIndex, ObjectType);
pub type PredicateReference = (IriIndex, IriIndex); // (predicate_index, referenced_node_index)

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
    pub short_literal_indexer: StringIndexer,
    pub literal_cache: StringCache,
}

pub enum LabelDisplayValue<'a> {
    FullStr(Box<str>),
    FullRef(&'a str),
    ShortAndIri(&'a str, &'a str),
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
        return &iri[last_hash + 1..];
    } else {
        let last_slash = iri.rfind('/');
        if let Some(last_slash) = last_slash {
            return &iri[last_slash + 1..];
        }
    }
    let first_colon = iri.find(':');
    if let Some(first_colon) = first_colon {
        return &iri[first_colon + 1..];
    }
    iri
}

impl NObject {
    pub fn has_same_type(&self, types: &Vec<IriIndex>) -> bool {
        for types in types {
            if self.types.contains(types) {
                return true;
            }
        }
        false
    }

    pub fn node_label<'a>(
        &'a self,
        iri: &'a str,
        styles: &GVisualizationStyle,
        should_short_iri: bool,
        language_index: LangIndex,
        indexers: &'a Indexers,
    ) -> &'a str {
        let label_opt = self.node_label_opt(styles, language_index, indexers);
        if let Some(label) = label_opt {
            return label;
        }
        if should_short_iri {
            return short_iri(iri);
        }
        iri
    }

    pub fn node_label_opt<'a>(
        &self,
        styles: &GVisualizationStyle,
        language_index: LangIndex,
        indexers: &'a Indexers,
    ) -> Option<&'a str> {
        for type_index in self.types.iter() {
            if let Some(type_style) = styles.node_styles.get(type_index) {
                let prop = self.get_property(type_style.label_index, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_str_ref(indexers));
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
                        if *lang == language_index {
                            return Some(value);
                        }
                        if *lang == 0 {
                            fallback_lang = Some(value);
                        }
                    }
                    ObjectType::String(_) | ObjectType::TypedString(_, _) | ObjectType::StringShort(_) => {
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

    pub fn get_property_count(
        &self,
        predicate_index: IriIndex,
        language_index: LangIndex,
    ) -> Option<(&ObjectType, u32)> {
        let mut no_lang: Option<&ObjectType> = None;
        let mut fallback_lang: Option<&ObjectType> = None;
        let mut lang_value: Option<&ObjectType> = None;
        let mut count: u32 = 0;
        for (predicate, value) in &self.properties {
            if predicate == &predicate_index {
                count += 1;
                match value {
                    ObjectType::LangString(lang, _) => {
                        if *lang == language_index && lang_value.is_none() {
                            lang_value = Some(value);
                        }
                        if *lang == 0 && fallback_lang.is_none() {
                            fallback_lang = Some(value);
                        }
                    }
                    ObjectType::String(_) | ObjectType::TypedString(_, _) | ObjectType::StringShort(_) => {
                        if no_lang.is_none() {
                            no_lang = Some(value);
                        }
                    }
                }
            }
        }
        if let Some(lang) = lang_value {
            return Some((lang, count));
        }
        if let Some(fallback_lang) = fallback_lang {
            return Some((fallback_lang, count));
        }
        if let Some(no_lang) = no_lang {
            return Some((no_lang, count));
        }
        None
    }

    pub fn apply_filter(&self, filter: &str, iri: &str, indexers: &Indexers) -> bool {
        if iri.contains(filter) {
            return true;
        }
        for (_predicate, value) in &self.properties {
            if value.as_str_ref(indexers).contains(filter) {
                return true;
            }
        }
        false
    }

    pub fn match_types(&self, types: &[IriIndex]) -> bool {
        for type_index in self.types.iter() {
            if types.contains(type_index) {
                return true;
            }
        }
        false
    }

    pub fn has_property(&self, predicate_index: IriIndex) -> bool {
        for (predicate, _value) in &self.properties {
            if predicate == &predicate_index {
                return true;
            }
        }
        false
    }

    pub fn highest_priority_types(&self, styles: &GVisualizationStyle) -> Vec<IriIndex> {
        let mut max_priority: u32 = 0;
        let mut result: Vec<IriIndex> = Vec::new();
        for type_index in self.types.iter() {
            if let Some(type_style) = styles.node_styles.get(type_index) {
                if type_style.priority > max_priority {
                    max_priority = type_style.priority;
                    result.clear();
                    result.push(*type_index);
                } else if type_style.priority == max_priority {
                    result.push(*type_index);
                }
            }
        }
        result
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
            short_literal_indexer: StringIndexer::new(),
            literal_cache: StringCache::default(),
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
        self.predicate_indexer = StringIndexer::new();
        self.type_indexer = StringIndexer::new();
        self.language_indexer = StringIndexer::new();
        self.datatype_indexer = StringIndexer::new();
        self.language_indexer.get_index("en");
        self.predicate_indexer.get_index("rdfs:label");
    }
}

impl Default for NodeCache {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeCache {
    pub fn new() -> Self {
        Self { cache: IndexMap::new() }
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
            self.put_node(
                iri,
                NObject {
                    types: Vec::new(),
                    properties: Vec::new(),
                    references: Vec::new(),
                    reverse_references: Vec::new(),
                    has_subject: false,
                    is_blank_node,
                },
            )
        }
    }
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    pub fn iter(&self) -> indexmap::map::Iter<'_, Box<str>, NObject> {
        self.cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, Box<str>, NObject> {
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
            indexers: Indexers::new(),
        }
    }
    pub fn get_node_by_index(&self, index: IriIndex) -> Option<(&Box<str>, &NObject)> {
        self.node_cache.get_node_by_index(index)
    }
    pub fn get_node_by_index_mut(&mut self, index: IriIndex) -> Option<(&Box<str>, &mut NObject)> {
        self.node_cache.get_node_by_index_mut(index)
    }
    pub fn get_node(&self, iri: &str) -> Option<&NObject> {
        self.node_cache.get_node(iri)
    }
    pub fn get_node_mut(&mut self, iri: &str) -> Option<&mut NObject> {
        self.node_cache.get_node_mut(iri)
    }
    pub fn get_node_index(&self, iri: &str) -> Option<IriIndex> {
        self.node_cache
            .cache
            .get_full(iri)
            .map(|(index, _, _)| index as IriIndex)
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
    pub fn iter(&self) -> indexmap::map::Iter<'_, Box<str>, NObject> {
        self.node_cache.iter()
    }
    pub fn iter_mut(&mut self) -> indexmap::map::IterMut<'_, Box<str>, NObject> {
        self.node_cache.iter_mut()
    }
    pub fn put_node(&mut self, iri: &str, node: NObject) -> IriIndex {
        self.node_cache.put_node(iri, node)
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
    pub fn type_label<'a>(
        &self,
        type_index: IriIndex,
        language_index: LangIndex,
        indexers: &'a Indexers,
    ) -> Option<&'a str> {
        let type_iri = self.indexers.type_indexer.index_to_str(type_index);
        if let Some(type_iri) = type_iri {
            if let Some(node) = self.get_node(type_iri) {
                let prop = node.get_property(0, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_str_ref(indexers));
                }
            }
        }
        None
    }
    pub fn predicate_label<'a>(
        &self,
        type_index: IriIndex,
        language_index: LangIndex,
        indexers: &'a Indexers,
    ) -> Option<&'a str> {
        let predicate_iri = self.indexers.predicate_indexer.index_to_str(type_index);
        if let Some(predicate_iri) = predicate_iri {
            if let Some(node) = self.get_node(predicate_iri) {
                let prop = node.get_property(0, language_index);
                if let Some(prop) = prop {
                    return Some(prop.as_str_ref(indexers));
                }
            }
        }
        None
    }
    pub fn type_display<'a>(
        &'a self,
        type_index: IriIndex,
        label_context: &'a LabelContext,
        indexers: &'a Indexers,
    ) -> LabelDisplayValue<'a> {
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
                IriDisplay::Prefixed => LabelDisplayValue::FullRef(type_iri),
                IriDisplay::Label => {
                    let type_label = self.type_label(type_index, label_context.language_index, indexers);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label, type_iri)
                    } else {
                        LabelDisplayValue::FullRef(type_iri)
                    }
                }
                IriDisplay::Shorten => LabelDisplayValue::FullRef(short_iri(type_iri)),
                IriDisplay::LabelOrShorten => {
                    let type_label = self.type_label(type_index, label_context.language_index, indexers);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label, type_iri)
                    } else {
                        LabelDisplayValue::FullRef(short_iri(type_iri))
                    }
                }
            }
        } else {
            LabelDisplayValue::FullRef("!Unknown")
        }
    }

    pub fn predicate_display<'a>(
        &'a self,
        predicate_index: IriIndex,
        label_context: &LabelContext,
        indexers: &'a Indexers,
    ) -> LabelDisplayValue<'a> {
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
                IriDisplay::Prefixed => LabelDisplayValue::FullRef(predicate_iri),
                IriDisplay::Label => {
                    let type_label = self.predicate_label(predicate_index, label_context.language_index, indexers);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label, predicate_iri)
                    } else {
                        LabelDisplayValue::FullRef(predicate_iri)
                    }
                }
                IriDisplay::Shorten => LabelDisplayValue::FullRef(short_iri(predicate_iri)),
                IriDisplay::LabelOrShorten => {
                    let type_label = self.predicate_label(predicate_index, label_context.language_index, indexers);
                    if let Some(type_label) = type_label {
                        LabelDisplayValue::ShortAndIri(type_label, predicate_iri)
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
        let predicate_first = self
            .indexers
            .predicate_indexer
            .get_index(&prefix_manager.get_prefixed(rdf::FIRST.as_str()));
        let predicate_rest = self
            .indexers
            .predicate_indexer
            .get_index(&prefix_manager.get_prefixed(rdf::REST.as_str()));
        let node_nil = self
            .get_node_index(&prefix_manager.get_prefixed(rdf::NIL.as_str()))
            .unwrap_or(0);
        let mut next_list: Vec<(IriIndex, IriIndex)> = Vec::new();
        for (node_index, (_, node)) in self.iter().enumerate() {
            for (predicate, ref_index) in &node.references {
                if *predicate == predicate_rest {
                    next_list.push((node_index as IriIndex, *ref_index));
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
            let mut list_holders: Vec<(IriIndex, IriIndex)> = Vec::new();
            for node_index in list.iter() {
                let node = self.get_node_by_index_mut(*node_index).unwrap().1;
                let mut literal: Option<Literal> = None;
                let mut reference: Option<IriIndex> = None;
                for (predicate, value) in &node.properties {
                    if *predicate == predicate_first {
                        literal = Some(value.clone());
                        break;
                    }
                }
                if literal.is_none() {
                    for (predicate, value) in &node.references {
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
                for (predicate, holder) in &list_holders {
                    let holder_node: &mut NObject = self.get_node_by_index_mut(*holder).unwrap().1;
                    if let Some(literal) = &literal {
                        holder_node.properties.push((*predicate, literal.clone()));
                    } else if let Some(reference) = reference {
                        holder_node.references.push((*predicate, reference));
                    }
                }
            }
            // Remove reference to rdf list
            for (predicate, holder) in &list_holders {
                let holder_node: &mut NObject = self.get_node_by_index_mut(*holder).unwrap().1;
                holder_node
                    .references
                    .retain(|(ref_predicate, ref_index)| ref_predicate != predicate || ref_index != head_node);
            }
        }
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
    use super::NodeData;
    use crate::{domain::config::IriDisplay, domain::LabelContext, domain::prefix_manager::PrefixManager};
    use oxrdf::Triple;

    #[test]
    fn test_node_data() {
        let mut node_data = NodeData::new();
        let prefix_manager = PrefixManager::new();

        let language_filter: Vec<String> = vec![];
        let mut index_cache = crate::integration::rdfwrap::IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        let subject = oxrdf::NamedNode::new("http://example.org#subject").unwrap();
        let rdf_type = oxrdf::NamedNode::new("http://example.org#ClassFoo").unwrap();
        let data_predicate = oxrdf::NamedNode::new("http://example.org#pred").unwrap();

        let mut tcount = 0;
        let triple = Triple::new(subject.clone(), oxrdf::vocab::rdf::TYPE, rdf_type);
        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            triple,
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(
                subject.clone(),
                data_predicate.clone(),
                oxrdf::Literal::new_simple_literal("test"),
            ),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        let pred_index = node_data.indexers.predicate_indexer.get_index(data_predicate.as_str());
        let node = node_data.get_node(subject.as_str());

        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.types.len(), 1);
        assert!(node.has_subject);
        assert!(!node.is_blank_node);
        assert_eq!(0, node.references.len());
        assert_eq!(0, node.references.len());
        assert_eq!(1, node.properties.len());

        let lit = node.get_property(pred_index, 0);
        assert!(lit.is_some());
        assert_eq!(lit.unwrap().as_str_ref(&node_data.indexers), "test");
    }

    #[test]
    fn test_node_data_labels() {
        let mut node_data = NodeData::new();
        let prefix_manager = PrefixManager::new();

        let language_filter: Vec<String> = vec![];
        let mut index_cache = crate::integration::rdfwrap::IndexCache {
            index: 0,
            iri: String::with_capacity(100),
        };
        let subject = oxrdf::NamedNode::new("http://example.org#subject").unwrap();
        let rdf_type = oxrdf::NamedNode::new("http://example.org#ClassFoo").unwrap();
        let data_predicate = oxrdf::NamedNode::new("http://example.org#pred").unwrap();
        let owl_data_property = oxrdf::NamedNode::new("http://www.w3.org/2002/07/owl#DatatypeProperty").unwrap();
        let owl_class = oxrdf::NamedNode::new("http://www.w3.org/2002/07/owl#Class").unwrap();

        let mut tcount = 0;

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(subject.clone(), oxrdf::vocab::rdf::TYPE, rdf_type.clone()),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(
                subject.clone(),
                data_predicate.clone(),
                oxrdf::Literal::new_simple_literal("test"),
            ),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(
                data_predicate.clone(),
                oxrdf::vocab::rdf::TYPE,
                owl_data_property.clone(),
            ),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(
                data_predicate.clone(),
                oxrdf::vocab::rdfs::LABEL,
                oxrdf::Literal::new_simple_literal("mypred"),
            ),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(rdf_type.clone(), oxrdf::vocab::rdf::TYPE, owl_class.clone()),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        crate::integration::rdfwrap::add_triple(
            &mut tcount,
            &mut node_data.indexers,
            &mut node_data.node_cache,
            Triple::new(
                rdf_type.clone(),
                oxrdf::vocab::rdfs::LABEL,
                oxrdf::Literal::new_simple_literal("MyClass"),
            ),
            &mut index_cache,
            &language_filter,
            &prefix_manager,
        );

        let node = node_data.get_node(subject.as_str());
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.types.len(), 1);

        let type_index = node.types.get(0).unwrap();
        assert_eq!(
            rdf_type.as_str(),
            node_data.indexers.type_indexer.index_to_str(*type_index).unwrap()
        );

        let type_label = node_data.type_label(*type_index, 0, &node_data.indexers);
        assert!(type_label.is_some());
        let type_label = type_label.unwrap();
        assert_eq!(type_label, "MyClass");

        assert_eq!(node.properties.len(), 1);
        let (prop_index, prop_literal) = node.properties.get(0).unwrap();
        assert_eq!(
            data_predicate.as_str(),
            node_data.indexers.predicate_indexer.index_to_str(*prop_index).unwrap()
        );
        let prop_str = prop_literal.as_str_ref(&node_data.indexers);
        assert_eq!(prop_str, "test");

        let prop_label = node_data.predicate_label(*prop_index, 0, &node_data.indexers);
        assert!(prop_label.is_some());
        let prop_label = prop_label.unwrap();
        assert_eq!(prop_label, "mypred");

        let label_context = LabelContext::new(0, IriDisplay::Label, &prefix_manager);
        let type_display = node_data.type_display(*type_index, &label_context, &node_data.indexers);
        assert_eq!("MyClass", type_display.as_str());
        let type_display = node_data.predicate_display(*prop_index, &label_context, &node_data.indexers);
        assert_eq!("mypred", type_display.as_str());

        let label_context = LabelContext::new(0, IriDisplay::LabelOrShorten, &prefix_manager);
        let type_display = node_data.type_display(*type_index, &label_context, &node_data.indexers);
        assert_eq!("MyClass", type_display.as_str());
        let type_display = node_data.predicate_display(*prop_index, &label_context, &node_data.indexers);
        assert_eq!("mypred", type_display.as_str());

        let label_context = LabelContext::new(0, IriDisplay::Shorten, &prefix_manager);
        let type_display = node_data.type_display(*type_index, &label_context, &node_data.indexers);
        assert_eq!("ClassFoo", type_display.as_str());
        let type_display = node_data.predicate_display(*prop_index, &label_context, &node_data.indexers);
        assert_eq!("pred", type_display.as_str());

        let label_context = LabelContext::new(0, IriDisplay::Full, &prefix_manager);
        let type_display = node_data.type_display(*type_index, &label_context, &node_data.indexers);
        assert_eq!("http://example.org#ClassFoo", type_display.as_str());
        let type_display = node_data.predicate_display(*prop_index, &label_context, &node_data.indexers);
        assert_eq!("http://example.org#pred", type_display.as_str());
    }
}
