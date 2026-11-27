use ordered_float::OrderedFloat;
use rayon::prelude::*;
use std::{borrow::Cow, collections::VecDeque};

use crate::domain::type_index::{TypeData, TypeInstanceIndex, ValueTypes};
use rdf_core::{IndexSpan, Indexers, IriIndex, LangIndex, Literal, NObject, NodeData};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

pub struct VisualQuery {
    pub root_table: Option<TableQuery>,
    pub instances: Vec<IriIndex>,
    pub aggregated_values: Vec<AggregatedValue>,
    pub tables_pro_row: usize,
    pub aggregations_pro_row: usize,
}

pub struct TableQuery {
    pub id: TableQueryId,
    pub type_iri: IriIndex,
    pub predicate_filters: Vec<PredicateFilter>,
    pub references: Vec<QueryReference>,
    pub aggregations: Vec<PredicateAggregation>,
    pub row_index: usize,
    pub aggregation_index: usize,
    pub is_last: bool,
}

#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
pub struct TableQueryId(pub u64);

pub struct PredicateFilter {
    pub predicate_iri: IriIndex,
    pub filter_type: FilterType,
    pub filter_value: String,
}

pub struct PredicateAggregation {
    pub predicate_iri: IriIndex,
    pub aggregation_type: AggregationType,
}

pub struct QueryReference {
    pub predicate: IriIndex,
    pub table_query: TableQuery,
    pub is_outgoing: bool,
}

pub struct TableQueryIter<'a> {
    stack: VecDeque<&'a TableQuery>,
}

#[derive(Debug, Clone, Copy, EnumIter, Display, PartialEq, Serialize, Deserialize, JsonSchema)]
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
    AvgCount(f64, f64),
    Empty,
}

impl Default for VisualQuery {
    fn default() -> Self {
        Self {
            root_table: None,
            tables_pro_row: 1,
            aggregations_pro_row: 0,
            instances: Vec::new(),
            aggregated_values: Vec::new(),
        }
    }
}

impl Default for TableQuery {
    fn default() -> Self {
        Self {
            id: TableQueryId::default(),
            type_iri: 0,
            predicate_filters: vec![],
            references: vec![],
            aggregations: vec![],
            row_index: 0,
            aggregation_index: 0,
            is_last: false,
        }
    }
}

impl Default for QueryReference {
    fn default() -> Self {
        Self {
            predicate: 0,
            table_query: TableQuery::default(),
            is_outgoing: true,
        }
    }
}

impl Default for PredicateFilter {
    fn default() -> Self {
        Self {
            predicate_iri: 0,
            filter_type: FilterType::Equals,
            filter_value: String::new(),
        }
    }
}

impl Default for PredicateAggregation {
    fn default() -> Self {
        Self {
            predicate_iri: 0,
            aggregation_type: AggregationType::Count,
        }
    }
}

impl VisualQuery {
    pub fn clean(&mut self) {
        self.root_table = None;
        self.clear_instances();
        self.tables_pro_row = 1;
        self.aggregations_pro_row = 0;
    }
    pub fn clear_instances(&mut self) {
        self.instances.clear();
        self.aggregated_values.clear();
    }
    pub fn sort_instances(
        &mut self,
        table_idx: usize,
        predicate: IriIndex,
        rdf_data: &NodeData,
        value_type: ValueTypes,
        is_asc: bool,
        lang_index: LangIndex,
    ) {
        if value_type == ValueTypes::INTEGER {
            let row_pred: Vec<(usize, i64)> = self
                .instances
                .chunks(self.tables_pro_row)
                .enumerate()
                .map(|(row_id, instances)| {
                    let instance_idx = instances[table_idx];
                    if let Some((_, nobject)) = rdf_data.get_node_by_index(instance_idx) {
                        if let Some(literal) = nobject.get_property(predicate, lang_index) {
                            (
                                row_id,
                                literal.as_str_ref(&rdf_data.indexers).parse::<i64>().unwrap_or(0),
                            )
                        } else {
                            (row_id, 0)
                        }
                    } else {
                        (row_id, 0)
                    }
                })
                .collect();
            sort_from_pairs(
                &mut self.instances,
                &mut self.aggregated_values,
                row_pred,
                is_asc,
                self.tables_pro_row,
                self.aggregations_pro_row,
            );
            return;
        } else if value_type == ValueTypes::DOUBLE {
            let row_pred: Vec<(usize, OrderedFloat<f64>)> = self
                .instances
                .chunks(self.tables_pro_row)
                .enumerate()
                .map(|(row_id, instances)| {
                    let instance_idx = instances[table_idx];
                    if let Some((_, nobject)) = rdf_data.get_node_by_index(instance_idx) {
                        if let Some(literal) = nobject.get_property(predicate, lang_index) {
                            (
                                row_id,
                                OrderedFloat(literal.as_str_ref(&rdf_data.indexers).parse::<f64>().unwrap_or(0.0)),
                            )
                        } else {
                            (row_id, OrderedFloat(0.0))
                        }
                    } else {
                        (row_id, OrderedFloat(0.0))
                    }
                })
                .collect();
            sort_from_pairs(
                &mut self.instances,
                &mut self.aggregated_values,
                row_pred,
                is_asc,
                self.tables_pro_row,
                self.aggregations_pro_row,
            );
            return;
        }
        let mut row_pred: Vec<(usize, Literal)> = self
            .instances
            .chunks(self.tables_pro_row)
            .enumerate()
            .map(|(row_id, instances)| {
                let instance_idx = instances[table_idx];
                if let Some((_, nobject)) = rdf_data.get_node_by_index(instance_idx) {
                    if let Some(literal) = nobject.get_property(predicate, lang_index) {
                        (row_id, literal.clone())
                    } else {
                        (row_id, Literal::NoValue())
                    }
                } else {
                    (row_id, Literal::NoValue())
                }
            })
            .collect();
        // we use stable version because use can apply several sorts on different columns
        row_pred.par_sort_by(|a, b| {
            let a_str = a.1.as_str_ref(&rdf_data.indexers);
            let b_str = b.1.as_str_ref(&rdf_data.indexers);
            let cmp = a_str.cmp(b_str);
            if is_asc { cmp } else { cmp.reverse() }
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
                    self.instances
                        .swap(current * self.tables_pro_row + r, next * self.tables_pro_row + r);
                }
                for r in 0..self.aggregations_pro_row {
                    self.aggregated_values.swap(
                        current * self.aggregations_pro_row + r,
                        next * self.aggregations_pro_row + r,
                    );
                }
                invert_perm.swap(current, next);
            }
        }
    }
    pub fn sort_aggr(&mut self, aggr_idx: usize, is_asc: bool) {
        let row_pred: Vec<(usize, OrderedFloat<f64>)> = self
            .aggregated_values
            .chunks(self.aggregations_pro_row)
            .enumerate()
            .map(|(row_id, agg_values)| {
                let agg_value = agg_values[aggr_idx];
                (row_id, OrderedFloat(agg_value.float_value()))
            })
            .collect();
        sort_from_pairs(
            &mut self.instances,
            &mut self.aggregated_values,
            row_pred,
            is_asc,
            self.tables_pro_row,
            self.aggregations_pro_row,
        );
    }

    pub fn value_type(&self, table_idx: usize, predicate: IriIndex, type_index: &TypeInstanceIndex) -> ValueTypes {
        if let Some(root_table) = &self.root_table {
            if let Some(type_iri) = root_table
                .iter_tables()
                .find(|t| t.row_index == table_idx)
                .map(|t| t.type_iri)
            {
                if let Some(type_data) = type_index.types.get(&type_iri) {
                    type_data
                        .properties
                        .get(&predicate)
                        .map_or(ValueTypes::empty(), |d| d.value_types)
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
        if self.instances.len() > 0 {
            self.instances.len() / self.tables_pro_row
        } else if self.aggregated_values.len() > 0 {
            self.aggregated_values.len() / self.aggregations_pro_row
        } else {
            0
        }
    }
}

fn sort_from_pairs<T: Ord + Send>(
    instances: &mut Vec<IriIndex>,
    aggregated_values: &mut Vec<AggregatedValue>,
    mut pairs: Vec<(usize, T)>,
    is_asc: bool,
    tables_pro_row: usize,
    aggregation_pro_row: usize,
) {
    // we use stable version because use can apply several sorts on different columns
    pairs.par_sort_by(|a, b| if is_asc { a.1.cmp(&b.1) } else { b.1.cmp(&a.1) });
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
                instances.swap(current * tables_pro_row + r, next * tables_pro_row + r);
            }
            for r in 0..aggregation_pro_row {
                aggregated_values.swap(current * aggregation_pro_row + r, next * aggregation_pro_row + r);
            }
            invert_perm.swap(current, next);
        }
    }
}

pub fn query_instances_for_table_query(
    type_data: &TypeData,
    table_query: &TableQuery,
    rdf_data: &NodeData,
) -> Vec<IriIndex> {
    let filter_evaluator = FilterEvaluator::from_table(&table_query);
    type_data
        .instances
        .par_iter()
        .copied()
        .filter(|iri_index| filter_evaluator.match_object(*iri_index, &rdf_data))
        .collect()
}

pub struct QueryContext<'a> {
    pub rdf_data: &'a NodeData,
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
    pub fn table_mut(&mut self, row_index: usize) -> Option<&mut TableQuery> {
        if self.row_index == row_index {
            return Some(self);
        }
        self.references
            .iter_mut()
            .find_map(|reference| reference.table_query.table_mut(row_index))
    }

    pub fn table_by_id_mut(&mut self, id: TableQueryId) -> Option<&mut TableQuery> {
        if self.id == id {
            return Some(self);
        }
        self.references
            .iter_mut()
            .find_map(|reference| reference.table_query.table_by_id_mut(id))
    }

    fn for_each_table_mut(&mut self, visit: &mut impl FnMut(&mut TableQuery)) {
        visit(self);
        for reference in &mut self.references {
            reference.table_query.for_each_table_mut(visit);
        }
    }
    pub fn refresh_table_data(&mut self) -> (usize, usize) {
        let ref_count = self.iter_tables().count();
        let mut aggr_count = 0;
        let mut instances_per_row = 0;
        let mut idx = 0;
        self.for_each_table_mut(&mut |table| {
            table.row_index = idx;
            table.is_last = idx == ref_count - 1;
            table.aggregation_index = aggr_count;
            aggr_count += table.aggregations.len();
            if aggr_count == 0 && table.aggregations.len() == 0 {
                instances_per_row += 1;
            }
            idx += 1;
        });
        self.for_each_table_mut(&mut |table| {
            if table.row_index == instances_per_row {
                table.is_last = true;
            }
        });
        (instances_per_row, aggr_count)
    }

    pub fn compute_instances(
        &self,
        rdf_data: &NodeData,
        input_instances: &Vec<IriIndex>,
    ) -> (Vec<IriIndex>, Vec<AggregatedValue>) {
        let mut instances = Vec::new();
        let mut aggregated_values = Vec::new();
        let mut aggr_count = 0;
        let mut ref_count = 0;
        for query_table in self.iter_tables() {
            if query_table.aggregations.len() > 0 {
                aggr_count += query_table.aggregations.len();
            } else if aggr_count == 0 {
                ref_count += 1;
            }
        }
        let mut row = vec![IriIndex::MAX; ref_count];
        let mut aggregated_row = vec![AggregatedValue::Empty; aggr_count];
        let filter_evaluators: Vec<_> = self.iter_tables().map(|qt| FilterEvaluator::from_table(qt)).collect();
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
        if self.is_last && self.aggregations.len() > 0 {
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
        if let Some((_iri, nobject)) = query_context.rdf_data.get_node_by_index(node_idx) {
            if !nobject.types.contains(&self.type_iri) {
                // Just skip if the type not match,
                // it could be also possible not to do it (so make checks if only the predicate match)
                return false;
            }
            if self.aggregations.len() > 0 {
                // Aggregation on type can have filters so we need if the object match the filter before we aggregate it
                if self.row_index > 0 {
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
                    for (predicate_idx, ref_index) in references {
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
                            query_context.aggregated_row[agg_idx + self.aggregation_index] =
                                aggregation.aggregation_type.aggregate(
                                    query_context.aggregated_row[agg_idx + self.aggregation_index],
                                    prop_value,
                                    &query_context.rdf_data.indexers,
                                );
                        }
                    }
                }
                true
            } else {
                if !was_aggregation {
                    query_context.row[self.row_index] = node_idx;
                }
                if self.row_index > 0 {
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
                    for (predicate_idx, ref_index) in references {
                        if *predicate_idx == reference.predicate {
                            // Check all sub references (same level) of this object
                            if reference
                                .table_query
                                .add_references(*ref_index, query_context, was_aggregation)
                            {
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

    fn add_sub_reference(
        &self,
        ref_idx: usize,
        nobject: &NObject,
        query_context: &mut QueryContext,
        was_aggregation: bool,
    ) -> bool {
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
        for (predicate_idx, ref_index) in references {
            if *predicate_idx == reference.predicate {
                if reference
                    .table_query
                    .add_references(*ref_index, query_context, was_aggregation)
                {
                    has_one_ref = true;
                }
                if self.add_sub_reference(ref_idx + 1, nobject, query_context, was_aggregation) {
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

pub struct FilterEvaluator<'a> {
    predicate_filters: &'a [PredicateFilter],
}

impl<'a> FilterEvaluator<'a> {
    pub fn from_table(table_query: &'a TableQuery) -> Self {
        Self::from_predicate_filters(&table_query.predicate_filters)
    }

    pub fn from_predicate_filters(predicate_filters: &'a [PredicateFilter]) -> Self {
        Self { predicate_filters }
    }

    pub fn match_object(&self, iri_idx: IriIndex, rdf_data: &NodeData) -> bool {
        if self.predicate_filters.is_empty() {
            return true;
        }
        if let Some((_iri, nobject)) = rdf_data.get_node_by_index(iri_idx) {
            return self.match_nobject(nobject, rdf_data);
        } else {
            return false;
        }
    }

    pub fn match_nobject(&self, nobject: &NObject, rdf_data: &NodeData) -> bool {
        for predicate_filter in self.predicate_filters.iter() {
            let mut has_match = false;
            for (predicate_idx, literal) in nobject.properties.iter() {
                if *predicate_idx == predicate_filter.predicate_iri {
                    let literal_str = literal.as_str_ref(&rdf_data.indexers);
                    match predicate_filter.filter_type {
                        FilterType::Equals => {
                            if predicate_filter.filter_value != literal_str {
                                return false;
                            }
                        }
                        FilterType::EqualsNoCase => {
                            if !predicate_filter.filter_value.eq_ignore_ascii_case(literal_str) {
                                return false;
                            }
                        }
                        FilterType::Contains => {
                            if !literal_str.contains(&predicate_filter.filter_value) {
                                return false;
                            }
                        }
                        FilterType::StartsWith => {
                            if !literal_str.starts_with(&predicate_filter.filter_value) {
                                return false;
                            }
                        }
                        FilterType::EndsWith => {
                            if !literal_str.ends_with(&predicate_filter.filter_value) {
                                return false;
                            }
                        }
                        FilterType::GraterThan => match literal {
                            Literal::TypedString(_type_idx, _span) => {
                                if literal
                                    .value_type(&rdf_data.indexers)
                                    .intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER)
                                {
                                    if let Ok(filter_isize) = predicate_filter.filter_value.parse() as Result<f64, _> {
                                        if let Ok(value_isize) = literal_str.parse() as Result<f64, _> {
                                            if filter_isize >= value_isize {
                                                return false;
                                            }
                                        } else {
                                            return false;
                                        }
                                    } else {
                                        return false;
                                    }
                                } else {
                                    return literal
                                        .as_str_ref(&rdf_data.indexers)
                                        .cmp(&predicate_filter.filter_value)
                                        .is_ge();
                                }
                            }
                            _ => {
                                return literal
                                    .as_str_ref(&rdf_data.indexers)
                                    .cmp(&predicate_filter.filter_value)
                                    .is_ge();
                            }
                        },
                        FilterType::LessThan => match literal {
                            Literal::TypedString(_type_idx, _span) => {
                                if literal
                                    .value_type(&rdf_data.indexers)
                                    .intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER)
                                {
                                    if let Ok(filter_isize) = predicate_filter.filter_value.parse() as Result<f64, _> {
                                        if let Ok(value_isize) = literal_str.parse() as Result<f64, _> {
                                            if filter_isize <= value_isize {
                                                return false;
                                            }
                                        } else {
                                            return false;
                                        }
                                    } else {
                                        return false;
                                    }
                                } else {
                                    return literal
                                        .as_str_ref(&rdf_data.indexers)
                                        .cmp(&predicate_filter.filter_value)
                                        .is_le();
                                }
                            }
                            _ => {
                                return literal
                                    .as_str_ref(&rdf_data.indexers)
                                    .cmp(&predicate_filter.filter_value)
                                    .is_le();
                            }
                        },
                        _ => {}
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
            AggregatedValue::LangString(str) => Cow::Borrowed(indexers.literal_cache.get_str(*str)),
            AggregatedValue::Empty => Cow::Borrowed(""),
            _ => Cow::Borrowed("final missing!"),
        }
    }
    pub fn from_literal(literal: &Literal) -> Self {
        match literal {
            Literal::StringShort(short_index) => AggregatedValue::ShortString(*short_index),
            Literal::String(str) => AggregatedValue::LangString(*str),
            Literal::LangString(_, str) => AggregatedValue::LangString(*str),
            Literal::LangShortString(_, short_index) => AggregatedValue::ShortString(*short_index),
            Literal::TypedString(_, str) => AggregatedValue::LangString(*str),
            Literal::NoValue() => AggregatedValue::Empty,
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
            AggregationType::Count => match old_value {
                AggregatedValue::Int(old_value) => AggregatedValue::Int(old_value + 1),
                _ => AggregatedValue::Int(1),
            },
            AggregationType::Sum => {
                let s_value = literal.as_str_ref(indexers);
                match old_value {
                    AggregatedValue::Int(old_ivalue) => {
                        let ivalue = s_value.parse::<i64>();
                        match ivalue {
                            Ok(ivalue) => AggregatedValue::Int(old_ivalue + ivalue),
                            Err(_) => {
                                // fallback float
                                let fvalue = s_value.parse::<f64>();
                                match fvalue {
                                    Ok(fvalue) => AggregatedValue::Float(old_ivalue as f64 + fvalue),
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
                            Ok(fvalue) => AggregatedValue::Float(old_fvalue + fvalue),
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    _ => {
                        let ivalue = s_value.parse::<i64>();
                        match ivalue {
                            Ok(ivalue) => AggregatedValue::Int(ivalue),
                            Err(_) => {
                                // fallback float
                                let fvalue = s_value.parse::<f64>();
                                match fvalue {
                                    Ok(fvalue) => AggregatedValue::Float(fvalue),
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
                            Ok(fvalue) => AggregatedValue::Float(old_fvalue.min(fvalue)),
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    AggregatedValue::Empty => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => AggregatedValue::Float(fvalue),
                            Err(_) => AggregatedValue::from_literal(literal),
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
                            Ok(fvalue) => AggregatedValue::Float(old_fvalue.max(fvalue)),
                            Err(_) => {
                                // fallback float
                                old_value
                            }
                        }
                    }
                    AggregatedValue::Empty => {
                        let fvalue = s_value.parse::<f64>();
                        match fvalue {
                            Ok(fvalue) => AggregatedValue::Float(fvalue),
                            Err(_) => AggregatedValue::from_literal(literal),
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
                    Ok(fvalue) => match old_value {
                        AggregatedValue::AvgCount(avg, count) => {
                            let count = count + 1.0;
                            let avg = avg + (fvalue - avg) / count;
                            AggregatedValue::AvgCount(avg, count)
                        }
                        _ => AggregatedValue::AvgCount(fvalue, 1.0),
                    },
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
            AggregationType::Avg => match value {
                AggregatedValue::AvgCount(avg, count) => {
                    if count > 0.0 {
                        AggregatedValue::Float(avg)
                    } else {
                        AggregatedValue::Empty
                    }
                }
                _ => value,
            },
            _ => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use oxttl::TurtleParser;
    use rdf_core::{IndexCache, PrefixManager, add_triple};
    use std::{fs::File, io::BufReader, path::Path};

    use super::*;

    fn load_sample(name: &str) -> NodeData {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../sample-rdf-data")
            .join(name);
        let mut parser = TurtleParser::new().for_reader(BufReader::new(File::open(path).unwrap()));
        let mut prefixes = PrefixManager::new();
        let mut data = NodeData::new();
        let mut index_cache = IndexCache {
            index: 0,
            iri: String::new(),
        };
        let mut count = 0;
        while let Some(triple) = parser.next() {
            for (prefix, iri) in parser.prefixes() {
                prefixes.add_prefix(prefix, iri);
            }
            let (indexers, cache) = data.split_mut();
            add_triple(
                &mut count,
                indexers,
                cache,
                triple.unwrap(),
                &mut index_cache,
                &[],
                &prefixes,
            );
        }
        assert!(count > 0);
        data
    }

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
                        }],
                        ..Default::default()
                    },
                    is_outgoing: true,
                },
                QueryReference {
                    predicate: 4,
                    table_query: TableQuery {
                        type_iri: 5,
                        ..Default::default()
                    },
                    is_outgoing: false,
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn test_table_query_iterator() {
        let table_query = table_query();
        for t in table_query.iter_tables() {
            println!("t iri {}", t.type_iri)
        }

        let mut iter = table_query.iter_tables();
        assert_eq!(iter.next().unwrap().type_iri, 1);
        assert_eq!(iter.next().unwrap().type_iri, 3);
        assert_eq!(iter.next().unwrap().type_iri, 6);
        assert_eq!(iter.next().unwrap().type_iri, 5);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_table_query_mut_lookup() {
        let mut table_query = table_query();
        table_query.refresh_table_data();
        assert_eq!(table_query.table_mut(0).unwrap().type_iri, 1);
        assert_eq!(table_query.table_mut(1).unwrap().type_iri, 3);
        assert_eq!(table_query.table_mut(2).unwrap().type_iri, 6);
        assert_eq!(table_query.table_mut(3).unwrap().type_iri, 5);
        assert!(table_query.table_mut(4).is_none());
    }

    #[test]
    fn test_visual_query() -> std::io::Result<()> {
        let mut rdf_data = load_sample("programming_languages.ttl");

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data);
        let type_index = rdf_data.get_type_index("dbo:ProgrammingLanguage");
        let type_desc = type_instance_index.types.get(&type_index);
        assert!(type_desc.is_some());
        let type_data = type_desc.unwrap();
        let mut table_query = TableQuery::default();
        table_query.type_iri = type_index;
        let (refs_per_row, aggr_per_row) = table_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(0, aggr_per_row);

        let instances = query_instances_for_table_query(&type_data, &table_query, &rdf_data);

        let (result_instances, result_aggregation) = table_query.compute_instances(&rdf_data, &instances);
        assert_eq!(instances.len(), result_instances.len());
        assert_eq!(0, result_aggregation.len());

        let label_idx = rdf_data.get_predicate_index("rdfs:label");
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
        assert!(count > 1);
        assert!(count < instances.len() as i64);

        Ok(())
    }

    #[test]
    fn programming_languages_filter_and_reference_query() {
        let mut node_data = load_sample("programming_languages.ttl");
        let mut index = TypeInstanceIndex::default();
        index.update(&node_data);
        let language_type = node_data.get_type_index("dbo:ProgrammingLanguage");
        let label = node_data.get_predicate_index("rdfs:label");
        let influenced_by = node_data.get_predicate_index("dbo:influencedBy");
        let mut query = TableQuery {
            type_iri: language_type,
            predicate_filters: vec![PredicateFilter {
                predicate_iri: label,
                filter_type: FilterType::Equals,
                filter_value: ".QL".to_string(),
            }],
            references: vec![QueryReference {
                predicate: influenced_by,
                table_query: TableQuery {
                    type_iri: language_type,
                    ..Default::default()
                },
                is_outgoing: true,
            }],
            ..Default::default()
        };
        assert_eq!(query.refresh_table_data(), (2, 0));
        let candidates = query_instances_for_table_query(&index.types[&language_type], &query, &node_data);
        assert_eq!(candidates.len(), 1);
        let (rows, aggregates) = query.compute_instances(&node_data, &candidates);
        assert_eq!(
            rows,
            vec![
                node_data.get_node_index("dbr:.QL").unwrap(),
                node_data.get_node_index("dbr:Datalog").unwrap()
            ]
        );
        assert!(aggregates.is_empty());
    }

    #[test]
    fn programming_languages_sort_query_results() {
        let mut node_data = load_sample("programming_languages.ttl");
        let mut index = TypeInstanceIndex::default();
        index.update(&node_data);
        let language_type = node_data.get_type_index("dbo:ProgrammingLanguage");
        let label = node_data.get_predicate_index("rdfs:label");
        let language = node_data.get_language_index("en");
        let mut root = TableQuery {
            type_iri: language_type,
            predicate_filters: vec![PredicateFilter {
                predicate_iri: label,
                filter_type: FilterType::Contains,
                filter_value: "BASIC".to_string(),
            }],
            ..Default::default()
        };
        let (tables_pro_row, aggregations_pro_row) = root.refresh_table_data();
        let candidates = query_instances_for_table_query(&index.types[&language_type], &root, &node_data);
        assert!(candidates.len() > 2);
        let (instances, aggregated_values) = root.compute_instances(&node_data, &candidates);
        let mut query = VisualQuery {
            root_table: Some(root),
            instances,
            aggregated_values,
            tables_pro_row,
            aggregations_pro_row,
        };
        query.sort_instances(0, label, &node_data, ValueTypes::empty(), true, language);
        let labels: Vec<_> = query
            .instances
            .iter()
            .map(|id| {
                node_data
                    .get_node_by_index(*id)
                    .unwrap()
                    .1
                    .get_property(label, language)
                    .unwrap()
                    .as_str_ref(&node_data.indexers)
            })
            .collect();
        assert!(labels.windows(2).all(|pair| pair[0] <= pair[1]));
        query.clean();
        assert_eq!(query.len(), 0);
        assert!(query.aggregated_values.is_empty());
        assert_eq!(query.aggregations_pro_row, 0);
    }

    #[test]
    fn test_visual_query_agg() -> std::io::Result<()> {
        let mut rdf_data = load_sample("chinook.ttl");

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data);
        let album_index = rdf_data.get_type_index("http://chinook/class#Album");
        let Some(album_desc) = type_instance_index.types.get(&album_index) else {
            panic!("Album type not found in type_instance_index");
        };

        let track_index = rdf_data.get_type_index("http://chinook/class#Track");
        let Some(_track_desc) = type_instance_index.types.get(&track_index) else {
            panic!("Track type not found in type_instance_index");
        };

        let mut table_query = TableQuery::default();
        table_query.type_iri = album_index;
        table_query.references.push(QueryReference {
            predicate: rdf_data.get_predicate_index("ns1:AlbumId"),
            table_query: TableQuery {
                type_iri: track_index,
                aggregations: vec![PredicateAggregation {
                    predicate_iri: rdf_data.get_predicate_index("ns1:UnitPrice"),
                    aggregation_type: AggregationType::Sum,
                    ..PredicateAggregation::default()
                }],
                ..Default::default()
            },
            is_outgoing: false,
        });

        let (refs_per_row, aggr_per_row) = table_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(1, aggr_per_row);

        let instances = query_instances_for_table_query(&album_desc, &table_query, &rdf_data);
        let (result_instances, result_aggregation) = table_query.compute_instances(&rdf_data, &instances);

        assert_eq!(instances.len(), result_instances.len());
        assert_eq!(result_instances.len(), result_aggregation.len());

        Ok(())
    }

    #[test]
    fn test_visual_query_agg3() -> std::io::Result<()> {
        let mut rdf_data = load_sample("chinook.ttl");

        let mut type_instance_index = TypeInstanceIndex::default();
        type_instance_index.update(&rdf_data);
        let artist_index = rdf_data.get_type_index("http://chinook/class#Artist");
        let Some(artist_desc) = type_instance_index.types.get(&artist_index) else {
            panic!("Artist type not found in type_instance_index");
        };
        let album_index = rdf_data.get_type_index("http://chinook/class#Album");
        let Some(_album_desc) = type_instance_index.types.get(&album_index) else {
            panic!("Album type not found in type_instance_index");
        };
        let track_index = rdf_data.get_type_index("http://chinook/class#Track");
        let Some(_track_desc) = type_instance_index.types.get(&track_index) else {
            panic!("Track type not found in type_instance_index");
        };

        let mut artist_query = TableQuery {
            type_iri: artist_index,
            ..Default::default()
        };

        let mut album_query = TableQuery {
            type_iri: album_index,
            aggregations: vec![PredicateAggregation {
                predicate_iri: rdf_data.get_predicate_index("ns1:Title"),
                aggregation_type: AggregationType::Count,
                ..PredicateAggregation::default()
            }],
            ..Default::default()
        };

        album_query.references.push(QueryReference {
            predicate: rdf_data.get_predicate_index("ns1:AlbumId"),
            table_query: TableQuery {
                type_iri: track_index,
                aggregations: vec![PredicateAggregation {
                    predicate_iri: rdf_data.get_predicate_index("ns1:Milliseconds"),
                    aggregation_type: AggregationType::Sum,
                    ..PredicateAggregation::default()
                }],
                ..Default::default()
            },
            is_outgoing: false,
        });

        artist_query.references.push(QueryReference {
            predicate: rdf_data.get_predicate_index("ns1:ArtistId"),
            table_query: album_query,
            is_outgoing: false,
        });

        let (refs_per_row, aggr_per_row) = artist_query.refresh_table_data();
        assert_eq!(1, refs_per_row);
        assert_eq!(2, aggr_per_row);

        let instances = query_instances_for_table_query(&artist_desc, &artist_query, &rdf_data);
        let (result_instances, result_aggregation) = artist_query.compute_instances(&rdf_data, &instances);

        assert!(instances.len() >= result_instances.len());
        assert!(result_instances.len() > 0);
        assert_eq!(result_instances.len() * 2, result_aggregation.len());

        if let AggregatedValue::Empty = result_aggregation[1] {
            panic!("No aggregation value for track");
        }

        Ok(())
    }
}
