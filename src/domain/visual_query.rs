use std::{borrow::Cow, collections::VecDeque};
use ordered_float::OrderedFloat;

use crate::{
    IriIndex, domain::{IndexSpan, Indexers, LangIndex, Literal, NObject, RdfData, type_index::{ColumnDesc, InstanceView, TypeData, TypeInstanceIndex, ValueStatistics, ValueTypes}},
};
use egui::Pos2;
use strum_macros::{Display, EnumIter};

pub struct VisualQuery {
    pub root_table: Option<TableQuery>,
    pub instance_view: InstanceView,
    pub instances: Vec<IriIndex>,
    pub aggregated_values: Vec<AggregatedValue>,
    pub selected_table: Option<usize>,
    pub value_statistics: Option<ValueStatistics>,
    pub tables_pro_row: usize,
    pub aggregations_pro_row: usize,
}

pub struct TableQuery {
    pub type_iri: IriIndex,
    pub visible_predicates: Vec<ColumnDesc>,
    pub predicate_filters: Vec<PredicateFilter>,
    pub references: Vec<QueryReference>,
    pub aggregations: Vec<PredicateAggregation>,
    pub add_link_state: TableQueryAddLinkState,
    pub row_index: usize,
    pub aggregation_index: usize,
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

pub struct PredicateAggregation {
    pub predicate_iri: IriIndex,
    pub aggregation_type: AggregationType,
    pub width: f32,
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

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq)]
pub enum AggregationType {
    #[strum(to_string = "Count")]
    Count,
    #[strum(to_string = "Sum")]
    Sum,
    #[strum(to_string = "Avg")]
    Avg,
    #[strum(to_string = "Min")]
    Min,
    #[strum(to_string = "Max")]
    Max,
}

#[derive(Debug, Clone, Copy)]
pub enum AggregatedValue {
    Float(f64),
    Int(i64),
    ShortString(IriIndex),
    LangString(IndexSpan),
    SumCount(f64, i64),
    Empty,
}


impl Default for VisualQuery {
    fn default() -> Self {
        Self {
            root_table: None,
            instance_view: InstanceView::default(),
            tables_pro_row: 1,
            aggregations_pro_row: 0,
            instances: Vec::new(),
            aggregated_values: Vec::new(),
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
            aggregations: vec![],
            row_index: 0,
            aggregation_index: 0,
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

impl Default for PredicateAggregation {
    fn default() -> Self {
        Self {
            predicate_iri: 0,
            aggregation_type: AggregationType::Count,
            to_remove: false,
            width: 150.0,
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
        self.aggregated_values.clear();
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
            sort_from_pairs(&mut self.instances, &mut self.aggregated_values, row_pred, is_asc, self.tables_pro_row, self.aggregations_pro_row);
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
            sort_from_pairs(&mut self.instances, &mut self.aggregated_values,row_pred, is_asc, self.tables_pro_row, self.aggregations_pro_row);
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
                for r in 0..self.aggregations_pro_row {
                    self.aggregated_values.swap(current*self.aggregations_pro_row+r, next*self.aggregations_pro_row+r);
                }
                invert_perm.swap(current, next);
            }
        }
    }
    pub fn sort_aggr(&mut self, aggr_idx: usize, is_asc: bool) {
        let row_pred : Vec<(usize, OrderedFloat<f64>)> = self.aggregated_values.chunks(self.aggregations_pro_row).enumerate().map(| (row_id,agg_values) | {
            let agg_value = agg_values[aggr_idx];
            (row_id, OrderedFloat(agg_value.float_value()))
        }).collect();
        sort_from_pairs(&mut self.instances, &mut self.aggregated_values,row_pred, is_asc, self.tables_pro_row, self.aggregations_pro_row);
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

    pub fn len(&self) -> usize {
        if self.instances.len()>0 {
            self.instances.len()/self.tables_pro_row
        } else if self.aggregated_values.len()>0 {
            self.aggregated_values.len()/self.aggregations_pro_row
        } else {
            0
        }
    }
}

fn sort_from_pairs<T: Ord>(instances: &mut Vec<IriIndex>, aggregated_values: &mut Vec<AggregatedValue>, mut pairs: Vec<(usize,T)>, is_asc: bool, tables_pro_row: usize, aggregation_pro_row: usize) 
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
            for r in 0..aggregation_pro_row {
                aggregated_values.swap(current*aggregation_pro_row+r, next*aggregation_pro_row+r);
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
    pub aggregated_values: &'a mut Vec<AggregatedValue>,
    pub row: &'a mut Vec<IriIndex>,
    pub aggregated_row: &'a mut Vec<AggregatedValue>,
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
    pub fn refresh_table_data(&mut self) -> (usize, usize) {
        let ref_count = self.iter_tables().count();
        let mut aggr_count = 0;
        let mut instances_per_row = 0;
        for (idx, table) in self.iter_tables_mut().enumerate() {
            table.row_index = idx;
            table.is_last = idx == ref_count-1;
            table.aggregation_index = aggr_count;
            aggr_count += table.aggregations.len();
            if aggr_count==0 && table.aggregations.len()==0 {
                instances_per_row += 1;
            }
        }
        for (idx, table) in self.iter_tables_mut().enumerate() {
            if idx == instances_per_row {
                table.is_last = true;
                break;
            }
        }
        (instances_per_row, aggr_count)
    }

    pub fn compute_instances(&self, rdf_data: &RdfData, input_instances: &Vec<IriIndex>) -> (Vec<IriIndex>, Vec<AggregatedValue>) {
        let mut instances = Vec::new();
        let mut aggregated_values = Vec::new();
        let mut aggr_count = 0;
        let mut ref_count = 0;
        for query_table in self.iter_tables() {
            if query_table.aggregations.len()>0 {
                aggr_count += query_table.aggregations.len();
            } else if aggr_count == 0 {
                ref_count += 1;
            }
        }
        let mut row = vec![IriIndex::MAX; ref_count];
        let mut aggregated_row = vec![AggregatedValue::Empty; aggr_count];
        let filter_evaluators : Vec<_> = self.iter_tables().map(|qt| FilterEvaluator::from_table(qt)).collect();
        let mut query_context = QueryContext {
            instances: &mut instances,
            aggregated_values: &mut aggregated_values,
            rdf_data: rdf_data,
            filters: &filter_evaluators,
            row: &mut row,
            aggregated_row: &mut aggregated_row,
        };
        for node_idx in input_instances.iter() {
            self.add_references(*node_idx, &mut query_context, false);
        }
        self.end_aggregation(&mut query_context);
        return (instances, aggregated_values);
    }

    fn end_aggregation(&self, query_context: &mut QueryContext) {
        if self.is_last && self.aggregations.len()>0 {
            for (idx, agg) in self
                .iter_tables()
                .flat_map(|table| table.aggregations.iter())
                .enumerate()
            {
                query_context.aggregated_row[idx] = agg.aggregation_type.final_value(query_context.aggregated_row[idx]);
            }
            for agg_value in query_context.aggregated_row.iter() {
                query_context.aggregated_values.push(*agg_value);
            }
            for agg_value in query_context.aggregated_row.iter_mut() {
                *agg_value = AggregatedValue::Empty;
            }
            for idx in query_context.row.iter() {
                query_context.instances.push(*idx);
            }
        }
    }

    fn add_references(&self, node_idx: IriIndex, query_context: &mut QueryContext, was_aggregation: bool) -> bool {
        if let Some((_iri, nobject)) = query_context.rdf_data.node_data.get_node_by_index(node_idx) {
            if !nobject.types.contains(&self.type_iri) {
                // Just skip if the type not match, 
                // it could be also possible not to do it (so make checks if only the predicate match)
                return false;
            }
            if self.aggregations.len()>0 {
                // Aggregation on type can have filters so we need if the object match the filter before we aggregate it
                if self.row_index>0 {
                    if !query_context.filters[self.row_index].match_nobject(nobject, query_context.rdf_data) {
                        return false;
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
                            // Check all sub references (same level) of this object
                            if reference.table_query.add_references(*ref_index, query_context, true) {
                                has_one_ref = true;
                            }
                            if self.add_sub_reference(1, nobject, query_context, true) {
                                has_one_ref = true;
                            }
                        }
                    }
                    if !has_one_ref {
                        return false;
                    }
                }
                for (agg_idx, aggregation) in self.aggregations.iter().enumerate() {
                    for (prop_iri, prop_value) in nobject.properties.iter() {
                        if aggregation.predicate_iri == *prop_iri {
                            query_context.aggregated_row[agg_idx+self.aggregation_index] =
                                aggregation.aggregation_type.aggregate(query_context.aggregated_row[agg_idx+self.aggregation_index], prop_value, &query_context.rdf_data.node_data.indexers);
                        }
                    }
                }               
                true
            } else {
                if !was_aggregation {
                    query_context.row[self.row_index] = node_idx;
                }
                if self.row_index>0 {
                    if !query_context.filters[self.row_index].match_nobject(nobject, query_context.rdf_data) {
                        return false;
                    }
                }
                // The for is full so emit it to result
                if !was_aggregation && self.is_last {
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
                            // Check all sub references (same level) of this object
                            if reference.table_query.add_references(*ref_index, query_context, was_aggregation) {
                                has_one_ref = true;
                            }
                            if self.add_sub_reference(1, nobject, query_context, was_aggregation) {
                                has_one_ref = true;
                            }
                        }
                    }
                    if !has_one_ref {
                        return false;
                    }
                    for reference in self.references.iter() {
                        reference.table_query.end_aggregation(query_context);
                    }
                }
                true
            }
        } else {
            false
        }
    }

    fn add_sub_reference(&self, ref_idx: usize, nobject: &NObject, query_context: &mut QueryContext, was_aggregation: bool) -> bool {
        if self.references.len() <= ref_idx {
            return false;
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
                if reference.table_query.add_references(*ref_index, query_context, was_aggregation) {
                    has_one_ref = true;
                }
                if self.add_sub_reference(ref_idx+1, nobject, query_context, was_aggregation) {
                    has_one_ref = true;
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

impl AggregatedValue {
    pub fn to_display_str<'a>(&'a self, indexers: &'a Indexers) -> Cow<'a, str> {
        match self {
            AggregatedValue::Float(v) => {
                let s = format!("{:.10}", v);
                let s = s.trim_end_matches('0').trim_end_matches('.');
                Cow::Owned(s.to_string())
            }
            AggregatedValue::Int(v) => Cow::Owned(v.to_string()),
            AggregatedValue::ShortString(index) => {
                Cow::Borrowed(indexers.short_literal_indexer.index_to_str(*index).unwrap())
            }
            AggregatedValue::LangString(str) => {
                Cow::Borrowed(indexers.literal_cache.get_str(*str))
            }
            AggregatedValue::Empty => Cow::Borrowed(""),
            _ => Cow::Borrowed("final missing!"),
        }
    }
    pub fn from_literal(literal: &Literal) -> Self {
        match literal {
            Literal::StringShort(short_index) => {
                AggregatedValue::ShortString(*short_index)
            }
            Literal::String(str) => {
                AggregatedValue::LangString(*str)
            }
            Literal::LangString(_, str) => {
                AggregatedValue::LangString(*str)
            }
            Literal::TypedString(_, str) => {
                AggregatedValue::LangString(*str)
            }
            Literal::NoValue() => {
                AggregatedValue::Empty
            }
        }
    }

    pub fn float_value(&self) -> f64 {
        match self {
            AggregatedValue::Float(v) => *v,
            AggregatedValue::Int(v) => *v as f64,
            _ => 0.0,
        }
    }
}

impl AggregationType {
    pub fn aggregate(&self, old_value: AggregatedValue, literal: &Literal, indexers: &Indexers) -> AggregatedValue {
        match self {
            AggregationType::Count => {
                match old_value {
                    AggregatedValue::Int(old_value) => {
                        AggregatedValue::Int(old_value+1)
                    }
                    _ => {
                        AggregatedValue::Int(1)
                    }
                }
            }
            AggregationType::Sum => {
                let s_value = literal.as_str_ref(indexers);
                match old_value {
                    AggregatedValue::Int(old_ivalue) => {
                        let ivalue = s_value.parse::<i64>();
                        match ivalue {
                            Ok(ivalue) => {
                                AggregatedValue::Int(old_ivalue+ivalue)
                            }
                            Err(_) => {
                                // fallback float
                                let fvalue = s_value.parse::<f64>();
                                match fvalue {
                                    Ok(fvalue) => {
                                        AggregatedValue::Float(old_ivalue as f64+fvalue)
                                    }
                                    Err(_) => {
                                        // fallback float
                                        old_value
                                    }
                                }
                            }
                        }
                    }
                    AggregatedValue::Float(old_fvalue) => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => {
                                AggregatedValue::Float(old_fvalue+fvalue)
                            }
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    _ => {
                        let ivalue = s_value.parse::<i64>();
                        match ivalue {
                            Ok(ivalue) => {
                                AggregatedValue::Int(ivalue)
                            }
                            Err(_) => {
                                // fallback float
                                let fvalue = s_value.parse::<f64>();
                                match fvalue {
                                    Ok(fvalue) => {
                                        AggregatedValue::Float(fvalue)
                                    }
                                    Err(_) => {
                                        // fallback float
                                        old_value
                                    }
                                }
                            }
                        }
                    }
                }
            }
            AggregationType::Min => {
                let s_value = literal.as_str_ref(indexers);
                match old_value {
                    AggregatedValue::Float(old_fvalue) => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => {
                                AggregatedValue::Float(old_fvalue.min(fvalue))
                            }
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    AggregatedValue::Empty => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => {
                                AggregatedValue::Float(fvalue)
                            }
                            Err(_) => {
                                AggregatedValue::from_literal(literal)
                            }
                        }
                    }
                    _ => {
                        let old_str = old_value.to_display_str(indexers);
                        if old_str.as_ref() < s_value {
                            old_value
                        } else {
                            AggregatedValue::from_literal(literal)
                        }
                    }
                }
            }
            AggregationType::Max => {
                let s_value = literal.as_str_ref(indexers);
                match old_value {
                    AggregatedValue::Float(old_fvalue) => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => {
                                AggregatedValue::Float(old_fvalue.max(fvalue))
                            }
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    AggregatedValue::Empty => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => {
                                AggregatedValue::Float(fvalue)
                            }
                            Err(_) => {
                                AggregatedValue::from_literal(literal)
                            }
                        }
                    }
                    _ => {
                        let old_str = old_value.to_display_str(indexers);
                        if old_str.as_ref() > s_value {
                            old_value
                        } else {
                            AggregatedValue::from_literal(literal)
                        }
                    }
                }
            }
            AggregationType::Avg => {
                let s_value = literal.as_str_ref(indexers);
                let fvalue = s_value.parse::<f64>();
                match fvalue {
                    Ok(fvalue) => {
                        match old_value {
                            AggregatedValue::SumCount(sum, count) => {
                                AggregatedValue::SumCount(sum+fvalue, count+1)                                
                            }
                            _ => {
                                AggregatedValue::SumCount(fvalue, 1)                                
                            }
                        }
                    }
                    Err(_) => {
                        // fallback float
                        old_value
                    }
                }   
            }

        }
    }

    pub fn final_value(&self, value: AggregatedValue) -> AggregatedValue {
        match self {
            AggregationType::Avg => {
                match value {
                    AggregatedValue::SumCount(sum, count) => {
                        if count>0 {
                            AggregatedValue::Float(sum/(count as f64))
                        } else {
                            AggregatedValue::Empty
                        }
                    }
                    _ => {
                        value
                    }
                }
            }
            _ => {
                value
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{domain::{NodeData, prefix_manager::PrefixManager}, integration::rdfwrap::RDFWrap};

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

    #[test]
    fn test_visual_query() -> std::io::Result<()> {
        
        let mut rdf_data = RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
        };
        let language_filter: Vec<String> = Vec::new();
        let load_result = RDFWrap::load_file(
                        "sample-rdf-data/programming_languages.ttl".to_string(),
                        &mut rdf_data,
                        &language_filter,
                        None,
                    );
        assert!(load_result.is_ok());
        assert!(load_result.unwrap()>0);

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data.node_data);
        let type_index = rdf_data.node_data.get_type_index("dbo:ProgrammingLanguage");
        let type_desc = type_instance_index.types.get(&type_index);
        assert!(type_desc.is_some());
        let type_data = type_desc.unwrap();
        let mut table_query = TableQuery::default();
        table_query.type_iri = type_index;
        let (refs_per_row, aggr_per_row) = table_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(0, aggr_per_row);

        let instances = type_data.query_instances_for_table_query(&table_query, &rdf_data);

        let (result_instances, result_aggregation) = table_query.compute_instances(&rdf_data, &instances);
        assert_eq!(instances.len(), result_instances.len());
        assert_eq!(0, result_aggregation.len());

        let label_idx = rdf_data.node_data.get_predicate_index("rdfs:label");
        assert!(type_data.properties.get(&label_idx).is_some());

        table_query.aggregations.push(PredicateAggregation { 
            predicate_iri: label_idx,
            aggregation_type: AggregationType::Count,
            ..PredicateAggregation::default()
        });
        let (refs_per_row, aggr_per_row) = table_query.refresh_table_data();
        assert_eq!(0, refs_per_row);
        assert_eq!(1, aggr_per_row);

        let (result_instances, result_aggregation) = table_query.compute_instances(&rdf_data, &instances);
        assert_eq!(0, result_instances.len());
        assert_eq!(1, result_aggregation.len());
        let agg_value = result_aggregation.get(0).unwrap();
        let AggregatedValue::Int(count) = *agg_value else {
            panic!("aggregated value is not int")
        };
        assert!(count>1);
        assert!(count<instances.len() as i64);


        Ok(())
    }


    #[test]
    fn test_visual_query_agg() -> std::io::Result<()> {
        let mut rdf_data = RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
        };
        let language_filter: Vec<String> = Vec::new();
        let load_result = RDFWrap::load_file(
                        "sample-rdf-data/chinook.ttl".to_string(),
                        &mut rdf_data,
                        &language_filter,
                        None,
                    );
        assert!(load_result.is_ok());
        assert!(load_result.unwrap()>0);

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data.node_data);
        let album_index = rdf_data.node_data.get_type_index("http://chinook/class#Album");
        let Some(album_desc) = type_instance_index.types.get(&album_index) else {
            panic!("Album type not found in type_instance_index");
        };

        let track_index = rdf_data.node_data.get_type_index("http://chinook/class#Track");
        let Some(track_desc) = type_instance_index.types.get(&track_index) else {
            panic!("Track type not found in type_instance_index");
        };

        let mut table_query = TableQuery::default();
        table_query.type_iri = album_index;
        table_query.references.push(QueryReference {
            predicate: rdf_data.node_data.get_predicate_index("ns1:AlbumId"),
            table_query: TableQuery {
                type_iri: track_index,
                aggregations: vec![PredicateAggregation {
                    predicate_iri: rdf_data.node_data.get_predicate_index("ns1:UnitPrice"),
                    aggregation_type: AggregationType::Sum,
                    ..PredicateAggregation::default()
                }],
                ..Default::default()
            },
            is_outgoing: false,
            to_remove: false,
        });

        let (refs_per_row, aggr_per_row) = table_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(1, aggr_per_row);

        let instances = album_desc.query_instances_for_table_query(&table_query, &rdf_data);
        let (result_instances, result_aggregation) = table_query.compute_instances(&rdf_data, &instances);

        assert_eq!(instances.len(), result_instances.len());
        assert_eq!(result_instances.len(), result_aggregation.len());

        Ok(())
    }

    #[test]
    fn test_visual_query_agg3() -> std::io::Result<()> {
        let mut rdf_data = RdfData {
                node_data: NodeData::new(),
                prefix_manager: PrefixManager::new(),
        };
        let language_filter: Vec<String> = Vec::new();
        let load_result = RDFWrap::load_file(
                        "sample-rdf-data/chinook.ttl".to_string(),
                        &mut rdf_data,
                        &language_filter,
                        None,
                    );
        assert!(load_result.is_ok());
        assert!(load_result.unwrap()>0);

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data.node_data);
        let artist_index = rdf_data.node_data.get_type_index("http://chinook/class#Artist");
        let Some(artist_desc) = type_instance_index.types.get(&artist_index) else {
            panic!("Artist type not found in type_instance_index");
        };
        let album_index = rdf_data.node_data.get_type_index("http://chinook/class#Album");
        let Some(album_desc) = type_instance_index.types.get(&album_index) else {
            panic!("Album type not found in type_instance_index");
        };
        let track_index = rdf_data.node_data.get_type_index("http://chinook/class#Track");
        let Some(track_desc) = type_instance_index.types.get(&track_index) else {
            panic!("Track type not found in type_instance_index");
        };

        let mut artist_query = TableQuery {
            type_iri: artist_index,
            ..Default::default()
        };
        
        let mut album_query = TableQuery {
            type_iri: album_index,
            aggregations: vec![PredicateAggregation {
                predicate_iri: rdf_data.node_data.get_predicate_index("ns1:Title"),
                aggregation_type: AggregationType::Count,
                ..PredicateAggregation::default()
            }],
            ..Default::default()
        };

        album_query.references.push(QueryReference {
            predicate: rdf_data.node_data.get_predicate_index("ns1:AlbumId"),
            table_query: TableQuery {
                type_iri: track_index,
                aggregations: vec![PredicateAggregation {
                    predicate_iri: rdf_data.node_data.get_predicate_index("ns1:Milliseconds"),
                    aggregation_type: AggregationType::Sum,
                    ..PredicateAggregation::default()
                }],
                ..Default::default()
            },
            is_outgoing: false,
            to_remove: false,
        });

        artist_query.references.push(QueryReference {
            predicate: rdf_data.node_data.get_predicate_index("ns1:ArtistId"),
            table_query: album_query,
            is_outgoing: false,
            to_remove: false,
        });

        let (refs_per_row, aggr_per_row) = artist_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(2, aggr_per_row);

        let instances = artist_desc.query_instances_for_table_query(&artist_query, &rdf_data);
        let (result_instances, result_aggregation) = artist_query.compute_instances(&rdf_data, &instances);

        assert!(instances.len()>=result_instances.len());
        assert!(result_instances.len()>0);
        assert_eq!(result_instances.len()*2, result_aggregation.len());

        if let AggregatedValue::Empty = result_aggregation[1] {
            panic!("No aggregation value for track");
        }

        Ok(())
    }


}
