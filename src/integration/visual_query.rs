use std::io;

use crate::domain::{LabelContext, NodeData, visual_query::VisualQuery};

impl VisualQuery {
    pub fn export_csv<W: io::Write>(
        &self,
        wtr: &mut csv::Writer<W>,
        node_data: &NodeData,
        label_context: &LabelContext,
    ) -> std::io::Result<()> {
        if let Some(root_table) = self.root_table.as_ref() {
            let mut row: Vec<String> = Vec::new();
            for table in root_table.iter_tables() {
                for column in table.visible_predicates.iter() {
                    if column.visible {
                        let predicate_label =
                            node_data.predicate_display(column.predicate_index, &label_context, &node_data.indexers);
                        row.push(predicate_label.as_str().to_string());
                    }
                }
            }
            wtr.write_record(&row)?;
            for instances in self.instances.chunks(self.tables_pro_row) {
                row.clear();
                for (table_query, instance_index) in root_table.iter_tables().zip(instances) {
                    let node = node_data.get_node_by_index(*instance_index);
                    if let Some((_node_iri, node)) = node {
                        for column_desc in table_query.visible_predicates
                            .iter()
                            .filter(|p| p.visible) {
                                let property = node.get_property_count(column_desc.predicate_index, label_context.language_index);
                                if let Some((property, _count)) = property {
                                    let value = property.as_str_ref(&node_data.indexers);
                                    row.push(value.to_string());
                                } else {
                                    row.push("".to_string());
                                }
                        }
                    }
                }
                wtr.write_record(&row)?;
            }
        }     
        wtr.flush()?;
        
        Ok(())
    }
}