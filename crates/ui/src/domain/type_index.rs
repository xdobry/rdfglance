use egui::Pos2;
use std::collections::HashMap;

use crate::{
    IriIndex, domain::NodeData, ui::table_view::CHAR_WIDTH, uistate::ref_selection::RefSelection,
};

pub const IRI_WIDTH: f32 = 300.0;
pub const REF_COUNT_WIDTH: f32 = 80.0;
const DEFAULT_COLUMN_WIDTH: f32 = 220.0;

pub struct TypeInstanceIndexView {
    pub type_instance_index: rdf_analytics::domain::type_index::TypeInstanceIndex,
    pub selected_type: Option<IriIndex>,
    pub type_cell_action: TypeCellAction,
    pub instance_views: HashMap<IriIndex, InstanceView>,
}

impl Default for TypeInstanceIndexView {
    fn default() -> Self {
        Self {
            type_instance_index: rdf_analytics::domain::type_index::TypeInstanceIndex::default(),
            type_cell_action: TypeCellAction::None,
            instance_views: HashMap::new(),
            selected_type: None,
        }
    }
}

impl TypeInstanceIndexView {
    pub fn update(&mut self, node_data: &NodeData) {
        self.type_instance_index.update(node_data);
        if self.type_instance_index.types_order.is_empty() {
            self.selected_type = None;
        } else {
            self.selected_type = Some(self.type_instance_index.types_order[0]);
        }
        for (type_index, type_data) in &self.type_instance_index.types {
            let mut instance_view = InstanceView::default();
            for (predicate_index, data_characteristics) in type_data.properties.iter() {
                if instance_view.get_column(*predicate_index).is_none() {
                    let predicate_str = node_data.get_predicate(*predicate_index);
                    let column_desc = ColumnDesc {
                        predicate_index: *predicate_index,
                        width: (((data_characteristics.max_len + 1).max(3) as f32) * CHAR_WIDTH)
                            .min(DEFAULT_COLUMN_WIDTH),
                        visible: true,
                    };
                    if let Some(predicate_str) = predicate_str {
                        if predicate_str.contains("label") {
                            instance_view.display_properties.insert(0, column_desc);
                            continue;
                        }
                    }
                    instance_view.display_properties.push(column_desc);
                }
            }
            if !type_data.instances.is_empty() {
                instance_view.selected_idx = Some((type_data.instances[0], 0));
            }
            self.instance_views.insert(*type_index, instance_view);
        }
    }
    pub fn clean(&mut self) {
        self.type_instance_index.clean();
        self.instance_views.clear();
    }
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
    QueryPredicate(Pos2, IriIndex, usize), //  usize is query table row index
    QueryAggregation(Pos2, usize),         // usize is aggregation index global
}

pub enum TableContextMenu {
    None,
    ColumnMenu(Pos2, IriIndex),
    CellMenu(Pos2, IriIndex, IriIndex),
    RefMenu(Pos2, IriIndex),
    IriColumnMenu(Pos2),
    RefColumnMenu(Pos2),
    QueryColumnMenu(Pos2, IriIndex, usize),
    AggregationColumnMenu(Pos2, usize), // usize is aggregation index global
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

