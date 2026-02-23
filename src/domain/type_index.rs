use core::{f64, num};
use std::{collections::HashMap};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
use bitflags::bitflags;
use ordered_float::OrderedFloat;
use egui::Pos2;

use crate::{IriIndex, domain::{LabelContext, LangIndex, Literal, NodeData, RdfData}, ui::table_view::CHAR_WIDTH, uistate::ref_selection::RefSelection};

use rayon::prelude::*;

pub const IRI_WIDTH: f32 = 300.0;
pub const REF_COUNT_WIDTH: f32 = 80.0;
const DEFAULT_COLUMN_WIDTH: f32 = 220.0;

pub struct TypeInstanceIndex {
    pub nodes: usize,
    pub unique_predicates: usize,
    pub unique_types: usize,
    pub properties: usize,
    pub references: usize,
    pub blank_nodes: usize,
    pub max_instance_type_count: usize,
    pub min_instance_type_count: usize,
    pub unresolved_references: usize,
    pub types: HashMap<IriIndex, TypeData>,
    pub types_order: Vec<IriIndex>,
    pub types_filtered: Vec<IriIndex>,
    pub selected_type: Option<IriIndex>,
    pub types_filter: String,
    pub type_cell_action: TypeCellAction,
    pub value_statistics: Option<ValueStatistics>,
}

pub struct ValueStatistics {
    pub count: usize,
    pub missing: usize,
    pub most_frequent_values: Vec<(u32, u32)>,
    pub num_statistic: Option<NumStatistics>,
}

pub struct NumStatistics {
    pub max: f64,
    pub count: f64,
    pub min: f64,
    pub avg: f64,
    pub sum: f64,
}

pub enum TypeCellAction {
    None,
    ShowRefTypes(Pos2, IriIndex),
    ShowValueStatistics(Pos2),
}

impl TypeCellAction {
    pub fn pos(&self) -> Pos2 {
        match self {
            TypeCellAction::ShowRefTypes(pos, _) => *pos,
            TypeCellAction::ShowValueStatistics(pos) => *pos,
            TypeCellAction::None => Pos2::new(0.0, 0.0),
        }
    }
}
pub struct DataPropCharacteristics {
    pub count: u32,
    pub max_len: u32,
    pub max_cardinality: u32,
    pub min_cardinality: u32,
    pub value_types: ValueTypes,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ValueTypes: u32 {
        const STRING       = 0b1;
        const SHORT_STRING = 0b10;
        const LANG_STRING  = 0b100;
        const INTEGER      = 0b1000;
        const DOUBLE       = 0b10000;
        const XML          = 0b100000;
        const JSON         = 0b1000000;
        const DATE_TIME    = 0b10000000;
        const DATE         = 0b100000000;
        const TIME         = 0b1000000000;
        const DURATION     = 0b10000000000;
        const BOOLEAN      = 0b100000000000;
        const UNKNOWN      = 0b1000000000000;
    }
}

pub struct ReferenceCharacteristics {
    pub count: u32,
    pub max_cardinality: u32,
    pub min_cardinality: u32,
    pub types: Vec<IriIndex>,
}

pub struct TypeData {
    pub instances: Vec<IriIndex>,
    pub filtered_instances: Vec<IriIndex>,
    pub properties: HashMap<IriIndex, DataPropCharacteristics>,
    pub references: HashMap<IriIndex, ReferenceCharacteristics>,
    pub rev_references: HashMap<IriIndex, ReferenceCharacteristics>,
    pub instance_view: InstanceView,
}

pub struct InstanceView {
    // Used for Y ScrollBar
    pub pos: f32,
    pub drag_pos: Option<f32>,
    pub display_properties: Vec<ColumnDesc>,
    pub instance_filter: String,
    pub context_menu: TableContextMenu,
    pub column_pos: u32,
    pub column_resize: InstanceColumnResize,
    pub iri_width: f32,
    pub ref_count_width: f32,
    pub selected_idx: Option<(IriIndex, usize)>,
    pub ref_selection: RefSelection,
}

pub enum InstanceColumnResize {
    None,
    Predicate(Pos2, IriIndex),
    Iri(Pos2),
    Refs(Pos2),
    QueryPredicate(Pos2, IriIndex, usize), //  u32 is query table row index
}

pub enum TableContextMenu {
    None,
    ColumnMenu(Pos2, IriIndex),
    CellMenu(Pos2, IriIndex, IriIndex),
    RefMenu(Pos2, IriIndex),
    IriColumnMenu(Pos2),
    RefColumnMenu(Pos2),
    QueryColumnMenu(Pos2, IriIndex, usize),
}

impl Default for InstanceView {
    fn default() -> Self {
        Self {
            pos: 0.0,
            drag_pos: None,
            column_pos: 0,
            display_properties: vec![],
            instance_filter: String::new(),
            context_menu: TableContextMenu::None,
            column_resize: InstanceColumnResize::None,
            iri_width: IRI_WIDTH,
            ref_count_width: REF_COUNT_WIDTH,
            selected_idx: None,
            ref_selection: RefSelection::None,
        }
    }
}

impl Default for TypeInstanceIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DataPropCharacteristics {
    fn default() -> Self {
        Self {
            count: 0,
            max_len: 0,
            min_cardinality: u32::MAX,
            max_cardinality: 0,
            value_types: ValueTypes::empty()
        }
    }
}

impl Default for NumStatistics {
    fn default() -> Self {
        Self {
            min: f64::NAN,
            max: f64::NAN,
            count: 0.0,
            avg: 0.0,
            sum: 0.0,
        }
    }
}

impl TypeData {
    pub fn new(_type_index: IriIndex) -> Self {
        Self {
            instances: Vec::new(),
            filtered_instances: Vec::new(),
            properties: HashMap::new(),
            references: HashMap::new(),
            rev_references: HashMap::new(),
            instance_view: InstanceView::default(),
        }
    }
    
    pub fn calculate_value_statistics(&self, predicate: IriIndex, node_data: &NodeData) -> ValueStatistics {
        let value_type = self.properties.get(&predicate).map_or(ValueTypes::empty(), |d| d.value_types);
        ValueStatistics::calculate_value_statistics(predicate, value_type, node_data, &self.filtered_instances)
    }

    pub fn sort_instances(&mut self, predicate_to_sort: IriIndex, is_asc: bool, rdf_data: &RdfData, language_index: LangIndex) {
        let prop_desc = self.properties.get(&predicate_to_sort);
        if let Some(prop_desc) = prop_desc {
            if prop_desc.value_types == ValueTypes::INTEGER {
                let row_pred : Vec<(usize, i64)> = self.filtered_instances.iter().enumerate().map(| (row_id,instance_idx) | {
                    if let Some((_, nobject)) = rdf_data.node_data.get_node_by_index(*instance_idx) {
                        if let Some(literal) = nobject.get_property(predicate_to_sort, language_index) {
                            (row_id, literal.as_str_ref(&rdf_data.node_data.indexers).parse::<i64>().unwrap_or(0))
                        } else {
                            (row_id, 0)
                        }
                    } else {
                        (row_id, 0)
                    }
                }).collect();
                sort_from_pairs(&mut self.filtered_instances, row_pred, is_asc);
                return
            } else if prop_desc.value_types == ValueTypes::DOUBLE {
                let row_pred : Vec<(usize, OrderedFloat<f64>)> = self.filtered_instances.iter().enumerate().map(| (row_id,instance_idx) | {
                    if let Some((_, nobject)) = rdf_data.node_data.get_node_by_index(*instance_idx) {
                        if let Some(literal) = nobject.get_property(predicate_to_sort, language_index) {
                            (row_id, OrderedFloat(literal.as_str_ref(&rdf_data.node_data.indexers).parse::<f64>().unwrap_or(0.0)))
                        } else {
                            (row_id, OrderedFloat(0.0))
                        }
                    } else {
                        (row_id, OrderedFloat(0.0))
                    }
                }).collect();
                sort_from_pairs(&mut self.filtered_instances, row_pred, is_asc);
                return
            }
        }
        let asc_greater = if is_asc { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less};
        let asc_less = if is_asc { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater};
        self.filtered_instances.sort_by(|a, b| {
            let node_a = rdf_data.node_data.get_node_by_index(*a);
            let node_b = rdf_data.node_data.get_node_by_index(*b);
            if let Some((_, node_a)) = node_a {
                if let Some((_, node_b)) = node_b {
                    let a_value =
                        &node_a.get_property(predicate_to_sort, language_index);
                    let b_value =
                        &node_b.get_property(predicate_to_sort, language_index);
                    if let Some(a_value) = a_value {
                        if let Some(b_value) = b_value {
                            let a_value = a_value.as_str_ref(&rdf_data.node_data.indexers);
                            let b_value = b_value.as_str_ref(&rdf_data.node_data.indexers);
                            if is_asc {
                                a_value.cmp(b_value)
                            } else {
                                b_value.cmp(a_value)
                            }
                        } else {
                            std::cmp::Ordering::Less
                        }
                    } else if let Some(_b_value) = b_value {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                } else {
                    asc_less
                }
            } else {
                asc_greater
            }
        });        
    } 
}

fn sort_from_pairs<T: Ord>(instances: &mut Vec<IriIndex>, mut pairs: Vec<(usize,T)>, is_asc: bool) 
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
            instances.swap(current,next);
            invert_perm.swap(current, next);
        }
    }
}

impl ValueStatistics {
    pub fn calculate_value_statistics<'a, I>(predicate: u32, value_type: ValueTypes, node_data: &NodeData, iter: I) -> ValueStatistics
    where I: IntoIterator<Item = &'a IriIndex>  {
        let mut count = 0;
        let mut missing = 0;
        let mut freq: HashMap<u32, u32> = HashMap::new();
        let mut num_statistics : Option<NumStatistics> = if value_type.intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER) {
            Some(NumStatistics::default())
        } else {
            None
        };
        for instance_index in iter {
            if let Some((_iri, nobject)) = node_data.get_node_by_index(*instance_index) {
                let mut found = false;
                for (property_index, value) in &nobject.properties {
                    if *property_index == predicate {
                        count += 1;
                        found = true;
                        match value {
                            Literal::StringShort(s) => {
                                *freq.entry(*s).or_insert(0) += 1;
                            },
                            _ => {
                            }
                        }
                        if let Some(num_statistics) = num_statistics.as_mut() {
                            let literal_value_type = value.value_type(&node_data.indexers);
                            if literal_value_type.intersects(ValueTypes::DOUBLE | ValueTypes::INTEGER) {
                                let double_value = value.as_str_ref(&node_data.indexers).parse::<f64>().unwrap_or(0.0);
                                if num_statistics.min.is_nan() || double_value < num_statistics.min {
                                    num_statistics.min = double_value;
                                }
                                if num_statistics.max.is_nan() || double_value > num_statistics.max {
                                    num_statistics.max = double_value;
                                }
                                num_statistics.count += 1.0;
                                num_statistics.avg += (double_value-num_statistics.avg) / num_statistics.count;
                                num_statistics.sum += double_value;
                            }
                        }
                    }
                }
                if !found {
                    missing += 1;
                }
            }
        }
        let mut freq_vec: Vec<(u32, u32)> = freq
            .into_iter()
            .map(|(s_idx, count)| (s_idx, count))
            .collect();
        freq_vec.sort_unstable_by(|a, b| b.1.cmp(&a.1));
        let most_frequent_values = freq_vec.into_iter().take(10).collect();
        ValueStatistics { 
            count: count,
            missing: missing, 
            most_frequent_values,
            num_statistic: num_statistics
        }
    }
}

fn count_type_references(type_references: &mut HashMap<IriIndex, ReferenceCharacteristics>, references: &Vec<(IriIndex, IriIndex)>, node_data: &NodeData) {
    let mut ref_counts: Vec<(IriIndex, u32, Vec<IriIndex>)> = Vec::new();
    for (predicate_index, ref_index) in references {
        let ref_node = node_data.get_node_by_index(*ref_index);
        if let Some((_str, ref_node)) = ref_node {
            let mut found = false;
            for (predicate_count_index, predicate_count, types) in ref_counts.iter_mut() {
                if *predicate_count_index == *predicate_index {
                    *predicate_count += 1;
                    found = true;
                    for type_index in &ref_node.types {
                        if !types.contains(type_index) {
                            types.push(*type_index);
                        }
                    }
                    break;
                }
            }
            if !found {
                ref_counts.push((*predicate_index, 1, ref_node.types.clone()));
            }
        }
    }
    // Search unknown references (set count to 0)
    for predicate_index in type_references.keys() {
        if !ref_counts.iter().any(|(index, _, _)| *index == *predicate_index) {
            ref_counts.push((*predicate_index, 0, vec![]));
        }
    }
    for (predicate_index, count, types) in ref_counts {
        let reference_characteristics = type_references.get_mut(&predicate_index);
        if let Some(reference_characteristics) = reference_characteristics {
            reference_characteristics.count += count;
            reference_characteristics.max_cardinality =
                reference_characteristics.max_cardinality.max(count);
            reference_characteristics.min_cardinality =
                reference_characteristics.min_cardinality.min(count);
            for type_index in types.iter() {
                if !reference_characteristics.types.contains(type_index) {
                    reference_characteristics.types.push(*type_index)
                }
            }
        } else {
            type_references.insert(
                predicate_index,
                ReferenceCharacteristics {
                    count,
                    min_cardinality: count,
                    max_cardinality: count,
                    types,
                },
            );
        }
    }        
}


impl InstanceView {
    pub fn get_column(&self, predicate_index: IriIndex) -> Option<&ColumnDesc> {
        self.display_properties
            .iter()
            .find(|column_desc| column_desc.predicate_index == predicate_index)
    }
    pub fn visible_columns(&self) -> u32 {
        let mut count = 0;
        for column_desc in &self.display_properties {
            if column_desc.visible {
                count += 1;
            }
        }
        count
    }
}


#[derive(Clone)]
pub struct ColumnDesc {
    pub predicate_index: IriIndex,
    pub width: f32,
    pub visible: bool,
}

impl TypeInstanceIndex {
    pub fn new() -> Self {
        Self {
            nodes: 0,
            unique_predicates: 0,
            unique_types: 0,
            properties: 0,
            references: 0,
            blank_nodes: 0,
            unresolved_references: 0,
            max_instance_type_count: 0,
            min_instance_type_count: 0,
            types: HashMap::new(),
            types_order: Vec::new(),
            types_filtered: Vec::new(),
            selected_type: None,
            types_filter: String::new(),
            type_cell_action: TypeCellAction::None,
            value_statistics: None,
        }
    }

    pub fn clean(&mut self) {
        self.nodes = 0;
        self.unique_predicates = 0;
        self.unique_types = 0;
        self.properties = 0;
        self.references = 0;
        self.blank_nodes = 0;
        self.unresolved_references = 0;
        self.max_instance_type_count = 0;
        self.min_instance_type_count = 0;
        self.types.clear();
        self.types_order.clear();
    }

    pub fn update(&mut self, node_data: &NodeData) {
        self.clean();
        #[cfg(not(target_arch = "wasm32"))]
        let start = Instant::now();
        let node_len = node_data.len();
        // TODO concurrent optimization
        // 1. partition the instances in groups (count  rayon::current_num_threads()) in dependency to type
        // 2. build hash map of each group (there are disjuct)
        // 3. merge all hash maps
        for (node_index, (_node_iri, node)) in node_data.iter().enumerate() {
            if node.has_subject {
                self.nodes += 1;
            } else {
                self.unresolved_references += 1;
            }
            if node.is_blank_node {
                self.blank_nodes += 1;
            }
            for type_index in &node.types {
                let type_data = self
                    .types
                    .entry(*type_index)
                    .or_insert_with(|| TypeData::new(*type_index));
                type_data.instances.push(node_index as IriIndex);
                for (property_index, property_stat) in type_data.properties.iter_mut() {
                    let mut property_card = 0;
                    for (predicate_index, value) in &node.properties {
                        if *property_index == *predicate_index {
                            property_stat.count += 1;
                            property_stat.value_types |= value.value_type(&node_data.indexers);
                            property_card += 1;
                            property_stat.max_len = property_stat
                                .max_len
                                .max(value.as_str_ref(&node_data.indexers).len() as u32);
                        }
                    }
                    property_stat.max_cardinality = property_stat.max_cardinality.max(property_card);
                    property_stat.min_cardinality = property_stat.min_cardinality.min(property_card);
                }
                let mut unknown_properties = vec![];
                for (predicate_index, _value) in &node.properties {
                    if !type_data.properties.contains_key(predicate_index) {
                        unknown_properties.push(*predicate_index);
                    }
                }
                for predicate_index in unknown_properties {
                    let mut property_card = 0;
                    let mut property_stat = DataPropCharacteristics::default();
                    for (property_index, value) in &node.properties {
                        if *property_index == predicate_index {
                            property_stat.count += 1;
                            property_card += 1;
                            property_stat.max_len = property_stat
                                .max_len
                                .max(value.as_str_ref(&node_data.indexers).len() as u32);
                        }
                    }
                    property_stat.max_cardinality = property_card;
                    property_stat.min_cardinality = property_card;
                    type_data.properties.insert(predicate_index, property_stat);
                }
                count_type_references(&mut type_data.references, &node.references, node_data);
                count_type_references(&mut type_data.rev_references, &node.reverse_references, node_data);
            }
            self.references += node.references.len();
            self.properties += node.properties.len();
        }
        self.unique_predicates = node_data.unique_predicates();
        self.unique_types = node_data.unique_types();
        for (type_index, type_data) in self.types.iter_mut() {
            self.types_order.push(*type_index);
            if self.min_instance_type_count == 0 && self.max_instance_type_count == 0 {
                self.min_instance_type_count = type_data.instances.len();
                self.max_instance_type_count = type_data.instances.len();
            } else {
                self.min_instance_type_count = self.min_instance_type_count.min(type_data.instances.len());
                self.max_instance_type_count = self.max_instance_type_count.max(type_data.instances.len());
            }
            for (predicate_index, data_characteristics) in type_data.properties.iter() {
                if type_data.instance_view.get_column(*predicate_index).is_none() {
                    let predicate_str = node_data.get_predicate(*predicate_index);
                    let column_desc = ColumnDesc {
                        predicate_index: *predicate_index,
                        width: (((data_characteristics.max_len + 1).max(3) as f32) * CHAR_WIDTH)
                            .min(DEFAULT_COLUMN_WIDTH),
                        visible: true,
                    };
                    if let Some(predicate_str) = predicate_str {
                        if predicate_str.contains("label") {
                            type_data.instance_view.display_properties.insert(0, column_desc);
                            continue;
                        }
                    }
                    type_data.instance_view.display_properties.push(column_desc);
                }
            }
            type_data.filtered_instances = type_data.instances.clone();
            if !type_data.instances.is_empty() {
                type_data.instance_view.selected_idx = Some((type_data.instances[0], 0));
            }
        }
        self.types_order.sort_by(|a, b| {
            let a_data = self.types.get(a).unwrap();
            let b_data = self.types.get(b).unwrap();
            b_data.instances.len().cmp(&a_data.instances.len())
        });
        if self.types_order.is_empty() {
            self.selected_type = None;
        } else {
            self.selected_type = Some(self.types_order[0]);
        }
        self.types_filter.clear();
        self.types_filtered = self.types_order.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let duration = start.elapsed();
            println!("Time taken to index {} nodes: {:?}", node_len, duration);
            println!("Nodes per second: {}", node_len as f64 / duration.as_secs_f64());
        }
    }

    pub fn apply_filter(&mut self, node_data: &mut NodeData, label_context: &LabelContext) {
        if self.types_filter.is_empty() {
            self.types_filtered = self.types_order.clone();
        } else {
            let filter = self.types_filter.to_lowercase();
            self.types_filtered = self
                .types_order
                .par_iter()
                .filter(|type_index| {
                    let label = node_data.type_display(**type_index, label_context, &node_data.indexers);
                    label.as_str().to_lowercase().contains(&filter)
                })
                .cloned()
                .collect();
        }
    }
}