use std::collections::HashMap;

use egui::Pos2;
use rdf_analytics::domain::type_index::ValueStatistics;

use crate::domain::type_index::{ColumnDesc, InstanceView};

pub use rdf_analytics::domain::visual_query::*;

#[derive(Default)]
pub struct TableQueryView {
    pub position: Pos2,
    pub visible_predicates: Vec<ColumnDesc>,
    pub aggregation_widths: Vec<f32>,
    pub to_remove: bool,
}

#[derive(Default)]
pub struct VisualQueryView {
    pub query: VisualQuery,
    pub instance_view: InstanceView,
    pub selected_table: Option<TableQueryId>,
    pub value_statistics: Option<ValueStatistics>,
    pub table_views: HashMap<TableQueryId, TableQueryView>,
    pub(crate) next_id: u64,
}

impl VisualQueryView {
    pub fn allocate_table_id(&mut self) -> TableQueryId {
        Self::next_table_id(&mut self.next_id)
    }

    pub(crate) fn next_table_id(next_id: &mut u64) -> TableQueryId {
        let id = TableQueryId(*next_id);
        *next_id = next_id.checked_add(1).expect("table query ID overflow");
        id
    }

    pub fn synchronize_table_views(&mut self) {
        if let Some(root) = &self.query.root_table {
            self.table_views
                .retain(|id, _| root.iter_tables().any(|table| table.id == *id));
            if self
                .selected_table
                .is_some_and(|id| !self.table_views.contains_key(&id))
            {
                self.selected_table = Some(root.id);
            }
        } else {
            self.table_views.clear();
            self.selected_table = None;
        }
    }

    pub fn clean(&mut self) {
        self.query.clean();
        self.instance_view = InstanceView::default();
        self.selected_table = None;
        self.value_statistics = None;
        self.table_views.clear();
    }

    pub fn clear_instances(&mut self) {
        self.query.clear_instances();
        self.instance_view.pos = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_views_follow_stable_ids_after_removal_and_clean() {
        let mut view = VisualQueryView::default();
        let root_id = view.allocate_table_id();
        let removed_id = view.allocate_table_id();
        let retained_id = view.allocate_table_id();
        for id in [root_id, removed_id, retained_id] {
            view.table_views.insert(id, TableQueryView::default());
        }
        view.table_views
            .get_mut(&retained_id)
            .unwrap()
            .aggregation_widths
            .push(225.0);
        view.query.root_table = Some(TableQuery {
            id: root_id,
            references: vec![
                QueryReference {
                    table_query: TableQuery {
                        id: removed_id,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                QueryReference {
                    table_query: TableQuery {
                        id: retained_id,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        view.selected_table = Some(retained_id);
        let root = view.query.root_table.as_mut().unwrap();
        root.references.remove(0);
        root.refresh_table_data();
        view.synchronize_table_views();
        assert!(!view.table_views.contains_key(&removed_id));
        assert_eq!(view.selected_table, Some(retained_id));
        assert_eq!(view.table_views[&retained_id].aggregation_widths, vec![225.0]);
        view.selected_table = Some(removed_id);
        view.synchronize_table_views();
        assert_eq!(view.selected_table, Some(root_id));
        view.clean();
        assert!(view.table_views.is_empty());
        assert!(view.selected_table.is_none());
        assert_ne!(view.allocate_table_id(), root_id);
    }
}
