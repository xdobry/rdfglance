use std::collections::VecDeque;
use ordered_float::OrderedFloat;

use crate::{
    IriIndex,
    domain::{LangIndex, Literal, NObject, RdfData, type_index::{ColumnDesc, InstanceView, TypeData, TypeInstanceIndex, ValueStatistics, ValueTypes}},
};
use egui::Pos2;
use strum_macros::{Display, EnumIter};

pub struct VisualQuery {
    pub root_table: Option<TableQuery>,
    pub instance_view: InstanceView,
    pub tables_pro_row: usize,
    pub instances: Vec<IriIndex>,
    pub selected_table: Option<usize>,
    pub value_statistics: Option<ValueStatistics>,
}

pub struct TableQuery {
    pub type_iri: IriIndex,
    pub visible_predicates: Vec<ColumnDesc>,
    pub predicate_filters: Vec<PredicateFilter>,
    pub references: Vec<QueryReference>,
    pub instances: Vec<IriIndex>,
    pub add_link_state: TableQueryAddLinkState,
    pub row_index: usize,
    pub position: Pos2,
    pub is_last: bool,
    pub to_remove: bool,
}

pub enum TableQueryAddLinkState {
    None,
    AddReference,
    AddRevReference,
}

pub struct PredicateFilter {
    pub predicate_iri: IriIndex,
    pub filter_type: FilterType,
    pub filter_value: String,
    pub to_remove: bool,
}

pub struct QueryReference {
    pub predicate: IriIndex,
    pub table_query: TableQuery,
    pub is_outgoing: bool,
    pub to_remove: bool,
}

pub struct TableQueryIter<'a> {
    stack: VecDeque<&'a TableQuery>,
}

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq)]
pub enum FilterType {
    #[strum(to_string = "Contains")]
    Contains,
    #[strum(to_string = "=")]
    Equals,
    #[strum(to_string = "= no case")]
    EqualsNoCase,
    #[strum(to_string = "Starts With")]
    StartsWith,
    #[strum(to_string = "Ends With")]
    EndsWith,
    #[strum(to_string = ">")]
    GraterThan,
    #[strum(to_string = "<")]
    LessThan,
    #[strum(to_string = "Exists")]
    Exists,
    #[strum(to_string = "Not Exists")]
    NotExists,
}


impl Default for VisualQuery {
    fn default() -> Self {
        Self {
            root_table: None,
            instance_view: InstanceView::default(),
            tables_pro_row: 1,
            instances: Vec::new(),
            selected_table: None,
            value_statistics: None,
        }
    }
}


impl Default for TableQuery {
    fn default() -> Self {
        Self {
            type_iri: 0,
            visible_predicates: vec![],
            predicate_filters: vec![],
            references: vec![],
            instances: vec![],
            row_index: 0,
            to_remove: false,
            is_last: false,
            position: Pos2::ZERO,
            add_link_state: TableQueryAddLinkState::None,
        }
    }
}

impl Default for QueryReference {
    fn default() -> Self {
        Self {
            predicate: 0,
            table_query: TableQuery::default(),
            is_outgoing: true,
            to_remove: false,
        }
    }
}

impl Default for PredicateFilter {
    fn default() -> Self {
        Self {
            predicate_iri: 0,
            filter_type: FilterType::Equals,
            filter_value: String::new(),
            to_remove: false,
        }
    }
}

impl VisualQuery {
    pub fn clean(&mut self) {
        self.root_table = None;
        self.instances.clear();
        self.tables_pro_row = 1;
    }
    pub fn clear_instances(&mut self) {
        self.instances.clear();
        self.instance_view.pos = 0.0;
    }
    pub fn sort_instances(&mut self, table_idx: usize, predicate: IriIndex, rdf_data: &RdfData, 
        value_type: ValueTypes,
        is_asc: bool, lang_index: LangIndex) {
        if value_type == ValueTypes::INTEGER {
            let row_pred : Vec<(usize, i64)> = self.instances.chunks(self.tables_pro_row).enumerate().map(| (row_id,instances) | {
            let instance_idx = instances[table_idx];
            if let Some((_, nobject)) = rdf_data.node_data.get_node_by_index(instance_idx) {
                if let Some(literal) = nobject.get_property(predicate, lang_index) {
                    (row_id, literal.as_str_ref(&rdf_data.node_data.indexers).parse::<i64>().unwrap_or(0))
                } else {
                    (row_id, 0)
                }
            } else {
                (row_id, 0)
            }
            }).collect();
            sort_from_pairs(&mut self.instances, row_pred, is_asc, self.tables_pro_row);
            return;
        } else if value_type == ValueTypes::DOUBLE {
            let row_pred : Vec<(usize, OrderedFloat<f64>)> = self.instances.chunks(self.tables_pro_row).enumerate().map(| (row_id,instances) | {
            let instance_idx = instances[table_idx];
            if let Some((_, nobject)) = rdf_data.node_data.get_node_by_index(instance_idx) {
                if let Some(literal) = nobject.get_property(predicate, lang_index) {
                    (row_id, OrderedFloat(literal.as_str_ref(&rdf_data.node_data.indexers).parse::<f64>().unwrap_or(0.0)))
                } else {
                    (row_id, OrderedFloat(0.0))
                }
            } else {
                (row_id, OrderedFloat(0.0))
            }
            }).collect();
            sort_from_pairs(&mut self.instances, row_pred, is_asc, self.tables_pro_row);
            return;
        }
        let mut row_pred : Vec<(usize, Literal)> = self.instances.chunks(self.tables_pro_row).enumerate().map(| (row_id,instances) | {
            let instance_idx = instances[table_idx];
            if let Some((_, nobject)) = rdf_data.node_data.get_node_by_index(instance_idx) {
                if let Some(literal) = nobject.get_property(predicate, lang_index) {
                    (row_id, literal.clone())
                } else {
                    (row_id, Literal::NoValue())
                }
            } else {
                (row_id, Literal::NoValue())
            }
        }).collect();
        // we use stable version because use can apply several sorts on different columns
        row_pred.sort_by(|a, b| {
            let a_str = a.1.as_str_ref(&rdf_data.node_data.indexers);
            let b_str = b.1.as_str_ref(&rdf_data.node_data.indexers);
            let cmp = a_str.cmp(b_str);
            if is_asc {
                cmp
            } else {
                cmp.reverse()
            }
        });
        let mut invert_perm = vec![0; row_pred.len()];
        for (idx, row) in row_pred.iter().enumerate() {
            invert_perm[row.0] = idx; 
        }
        for i in 0..invert_perm.len() {
            let current = i;
            // While the element is not in the correct place
            while invert_perm[current] != current {
                let next = invert_perm[current];
                for r in 0..self.tables_pro_row {
                    self.instances.swap(current*self.tables_pro_row+r, next*self.tables_pro_row+r);
                }
                invert_perm.swap(current, next);
            }
        }
    }
    pub fn value_type(&self, table_idx: usize, predicate: IriIndex, type_index: &TypeInstanceIndex) -> ValueTypes {
        if let Some(root_table) = &self.root_table {
            if let Some(type_iri) = root_table.iter_tables().find(|t| t.row_index == table_idx).map(|t| t.type_iri) {
                if let Some(type_data) = type_index.types.get(&type_iri) {
                    type_data.properties.get(&predicate).map_or(ValueTypes::empty(), |d| d.value_types)
                } else {
                    ValueTypes::empty()
                }
            } else {
                ValueTypes::empty()
            }
        } else {
            ValueTypes::empty()
        }
    }
}

fn sort_from_pairs<T: Ord>(instances: &mut Vec<IriIndex>, mut pairs: Vec<(usize,T)>, is_asc: bool, tables_pro_row: usize) 
{
    // we use stable version because use can apply several sorts on different columns
    pairs.sort_by(|a, b| {
        if is_asc {
            a.1.cmp(&b.1)
        } else {
            b.1.cmp(&a.1)
        }
    });
    let mut invert_perm = vec![0; pairs.len()];
    for (idx, row) in pairs.iter().enumerate() {
        invert_perm[row.0] = idx; 
    }
    for i in 0..invert_perm.len() {
        let current = i;
        // While the element is not in the correct place
        while invert_perm[current] != current {
            let next = invert_perm[current];
            for r in 0..tables_pro_row {
                instances.swap(current*tables_pro_row+r, next*tables_pro_row+r);
            }
            invert_perm.swap(current, next);
        }
    }
}


impl TypeData {
    pub fn query_instances_for_table_query(&self, table_query: &TableQuery, rdf_data: &RdfData) -> Vec<IriIndex> {
        let filter_evaluator = FilterEvaluator::from_table(&table_query);
        self.instances.iter().copied().filter(|iri_index| filter_evaluator.match_object(*iri_index, &rdf_data)).collect()
    }
}

pub struct TableQueryIterMut<'a> {
    stack: VecDeque<*mut TableQuery>,
    _marker: std::marker::PhantomData<&'a mut TableQuery>,
}

pub struct QueryContext<'a> {
    pub rdf_data: &'a RdfData,
    pub filters: &'a Vec<FilterEvaluator<'a>>,
    pub instances: &'a mut Vec<IriIndex>,
    pub row: &'a mut Vec<IriIndex>,

}

impl TableQuery {
    pub fn iter_tables(&self) -> TableQueryIter<'_> {
        TableQueryIter {
            stack: VecDeque::from([self]), // start with root
        }
    }
    pub fn iter_tables_mut(&mut self) -> TableQueryIterMut<'_> {
        let mut stack = VecDeque::new();
        stack.push_back(self as *mut TableQuery);
        TableQueryIterMut {
            stack,
            _marker: std::marker::PhantomData,
        }
    }
    pub fn refresh_table_data(&mut self) -> usize {
        let ref_count = self.iter_tables().count();
        for (idx, table) in self.iter_tables_mut().enumerate() {
            table.row_index = idx;
            table.is_last = idx == ref_count-1;
        }
        ref_count
    }

    pub fn compute_instances(&self, rdf_data: &RdfData) -> Vec<IriIndex> {
        let mut instances = Vec::new();
        let ref_count = self.iter_tables().count();
        let mut row = vec![IriIndex::MAX; ref_count];
        let filter_evaluators : Vec<_> = self.iter_tables().map(|qt| FilterEvaluator::from_table(qt)).collect();
        let mut query_context = QueryContext {
            instances: &mut instances,
            rdf_data: rdf_data,
            filters: &filter_evaluators,
            row: &mut row,
        };
        for node_idx in self.instances.iter() {
            self.add_references(*node_idx, &mut query_context);
        }
        return instances;
    }
    fn add_references(&self, node_idx: IriIndex, query_context: &mut QueryContext) -> bool {
        if let Some((_iri, nobject)) = query_context.rdf_data.node_data.get_node_by_index(node_idx) {
            query_context.row[self.row_index] = node_idx;
            if self.row_index>0 {
                if !query_context.filters[self.row_index].match_nobject(nobject, query_context.rdf_data) {
                    return false;
                }
            }
            // The for is full so emit it to result
            if self.is_last {
                for idx in query_context.row.iter() {
                    query_context.instances.push(*idx);
                }
            }
            if !self.references.is_empty() {
                let ref_iter_count = 0;
                let reference = self.references.get(ref_iter_count).unwrap();
                let mut has_one_ref = false;
                let references = if reference.is_outgoing {
                    &nobject.references
                } else {
                    &nobject.reverse_references
                };
                for (predicate_idx,ref_index) in references {
                    if *predicate_idx == reference.predicate {
                        has_one_ref = true;
                        // Check all sub references (same level) of this object
                        if !reference.table_query.add_references(*ref_index, query_context) {
                            return false;
                        }
                        if !self.add_sub_reference(1, nobject, query_context) {
                            return false;
                        }
                    }
                }
                if !has_one_ref {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn add_sub_reference(&self, ref_idx: usize, nobject: &NObject, query_context: &mut QueryContext) -> bool {
        if self.references.len() <= ref_idx {
            return true
        }
        let reference = self.references.get(ref_idx).unwrap();
        let mut has_one_ref = false;
        let references = if reference.is_outgoing {
            &nobject.references
        } else {
            &nobject.reverse_references
        };
        for (predicate_idx,ref_index) in references {
            if *predicate_idx == reference.predicate {
                has_one_ref = true;
                if !reference.table_query.add_references(*ref_index, query_context) {
                    return false;
                }
                if !self.add_sub_reference(ref_idx+1, nobject, query_context) {
                    return false;
                }
            }
        }
        has_one_ref
    }
}

impl<'a> Iterator for TableQueryIter<'a> {
    type Item = &'a TableQuery;

    /**
     * Do DFS (Depth First Search)on query structure
     */
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.stack.pop_back() {
            for reference in current.references.iter().rev() {
                self.stack.push_back(&reference.table_query);
            }
            return Some(current);
        }
        None
    }
}


impl<'a> Iterator for TableQueryIterMut<'a> {
    type Item = &'a mut TableQuery;

    /**
     * Do DFS on query structure
     */
    fn next(&mut self) -> Option<Self::Item> {
        let current_ptr = self.stack.pop_back()?;
        // SAFETY: We ensure unique mutable access by never aliasing pointers
        let current: &mut TableQuery = unsafe { &mut *current_ptr };

        // Push children onto the stack
        for reference in &mut current.references.iter_mut().rev() {
            self.stack.push_back(&mut reference.table_query as *mut TableQuery);
        }

        Some(current)
    }
}

pub struct FilterEvaluator<'a> {
    table: &'a TableQuery 
}

impl<'a> FilterEvaluator<'a> {
    pub fn from_table(table_query: &'a TableQuery) -> Self {
        Self {
            table: table_query
        }
    }
    pub fn match_object(&self, iri_idx: IriIndex, rdf_data: &RdfData) -> bool {
        if self.table.predicate_filters.is_empty() {
            return true;
        }
        if let Some((_iri,nobject)) = rdf_data.node_data.get_node_by_index(iri_idx) {
            return self.match_nobject(nobject, rdf_data);
        } else {
            return false;
        }
    }

    pub fn match_nobject(&self, nobject: &NObject, rdf_data: &RdfData) -> bool {
        for predicate_filter in self.table.predicate_filters.iter() {
            let mut has_match = false;
            for (predicate_idx, literal) in nobject.properties.iter() {
                if *predicate_idx == predicate_filter.predicate_iri {
                    let literal_str = literal.as_str_ref(&rdf_data.node_data.indexers);
                    match predicate_filter.filter_type {
                        FilterType::Equals => {
                            if predicate_filter.filter_value != literal_str {
                                return false;
                            }
                        },
                        FilterType::EqualsNoCase => {
                            if !predicate_filter.filter_value.eq_ignore_ascii_case(literal_str) {
                                return false;
                            }
                        },
                        FilterType::Contains => {
                            if !literal_str.contains(&predicate_filter.filter_value) {
                                return false;
                            }
                        },
                        FilterType::StartsWith => {
                            if !literal_str.starts_with(&predicate_filter.filter_value) {
                                return false;
                            }
                        },
                        FilterType::EndsWith => {
                            if !literal_str.ends_with(&predicate_filter.filter_value) {
                                return false;
                            }
                        },
                        FilterType::GraterThan => {
                            match literal {
                                Literal::TypedString(_type_idx,_span) => {
                                    if literal.value_type(&rdf_data.node_data.indexers).intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER) {
                                        if let Ok(filter_isize) = predicate_filter.filter_value.parse() as Result<f64, _> {
                                            if let Ok(value_isize) = literal_str.parse() as Result<f64, _>  {
                                                if filter_isize>=value_isize {
                                                    return false;
                                                }
                                            } else {
                                                return false;
                                            }
                                        } else {
                                            return false;
                                        }
                                    } else {
                                        return literal.as_str_ref(&rdf_data.node_data.indexers).cmp(&predicate_filter.filter_value).is_ge();
                                    }
                                },
                                _ => {
                                    return literal.as_str_ref(&rdf_data.node_data.indexers).cmp(&predicate_filter.filter_value).is_ge();
                                }
                            }
                        },
                        FilterType::LessThan => {
                            match literal {
                                Literal::TypedString(_type_idx,_span) => {
                                    if literal.value_type(&rdf_data.node_data.indexers).intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER) {
                                        if let Ok(filter_isize) = predicate_filter.filter_value.parse() as Result<f64, _> {
                                            if let Ok(value_isize) = literal_str.parse() as Result<f64, _>  {
                                                if filter_isize<=value_isize {
                                                    return false;
                                                }
                                            } else {
                                                return false;
                                            }
                                        } else {
                                            return false;
                                        }
                                    } else {
                                        return literal.as_str_ref(&rdf_data.node_data.indexers).cmp(&predicate_filter.filter_value).is_le();    
                                    }
                                },
                                _ => {
                                    return literal.as_str_ref(&rdf_data.node_data.indexers).cmp(&predicate_filter.filter_value).is_le();
                                }
                            }
                        }
                        _ => {

                        }
                    }
                    has_match = true;
                }
            }
            if !has_match ^ matches!(predicate_filter.filter_type, FilterType::NotExists) {
                return false;
            }
        }
        true
     }
}



#[cfg(test)]
mod tests {
    use super::*;

    fn table_query() -> TableQuery {
        TableQuery {
            type_iri: 1,
            references: vec![
                QueryReference {
                    predicate: 2,
                    table_query: TableQuery {
                        type_iri: 3,
                        references: vec![QueryReference {
                            predicate: 2,
                            table_query: TableQuery {
                                type_iri: 6,
                                ..Default::default()
                            },
                            is_outgoing: true,
                            to_remove: false,
                        }],
                        ..Default::default()
                    },
                    is_outgoing: true,
                    to_remove: false,
                },
                QueryReference {
                    predicate: 4,
                    table_query: TableQuery {
                        type_iri: 5,
                        ..Default::default()
                    },
                    is_outgoing: false,
                    to_remove: false,
                },
            ],
            ..Default::default()
        }
    }
    
    #[test]
    fn test_table_query_iterator() {
        let table_query = table_query();
        for t in table_query.iter_tables() {
            println!("t iri {}",t.type_iri)
        }

        let mut iter = table_query.iter_tables();
        assert_eq!(iter.next().unwrap().type_iri, 1);
        assert_eq!(iter.next().unwrap().type_iri, 3);
        assert_eq!(iter.next().unwrap().type_iri, 6);
        assert_eq!(iter.next().unwrap().type_iri, 5);
        assert!(iter.next().is_none());
    }

        #[test]
    fn test_table_query_mut_iterator() {
        let mut table_query = table_query();

        let mut iter = table_query.iter_tables_mut();
        assert_eq!(iter.next().unwrap().type_iri, 1);
        assert_eq!(iter.next().unwrap().type_iri, 3);
        assert_eq!(iter.next().unwrap().type_iri, 6);
        assert_eq!(iter.next().unwrap().type_iri, 5);
        assert!(iter.next().is_none());
    }

}
