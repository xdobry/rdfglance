use std::io;

use crate::{IriIndex, domain::{LabelContext, NodeData, visual_query::{AggregatedValue, VisualQuery}}};

impl VisualQuery {
    pub fn export_csv<W: io::Write>(
        &self,
        wtr: &mut csv::Writer<W>,
        node_data: &NodeData,
        label_context: &LabelContext,
    ) -> std::io::Result<()> {
        if let Some(root_table) = self.root_table.as_ref() {
            let mut row: Vec<String> = Vec::new();
            let mut has_aggregation = false;
            for table in root_table.iter_tables() {
                if table.aggregations.len()==0 {
                    if !has_aggregation {
                        for column in table.visible_predicates.iter() {
                            if column.visible {
                                let predicate_label =
                                    node_data.predicate_display(column.predicate_index, &label_context, &node_data.indexers);
                                row.push(predicate_label.as_str().to_string());
                            }
                        }
                    }
                } else {
                    has_aggregation = true;
                    for aggr in table.aggregations.iter() {
                        let predicate_label =
                            node_data.predicate_display(aggr.predicate_iri, &label_context, &node_data.indexers);
                        let aggr_label = format!("{}({})",aggr.aggregation_type,predicate_label.as_str());
                        row.push(aggr_label);
                    }
                }
            }
            wtr.write_record(&row)?;

            if self.instances.len() == 0 && self.aggregated_values.len() == 0 {
                panic!("no data to csv export");
            }

            let instances_chunks: Box<dyn Iterator<Item = &[IriIndex]>> =
            if self.tables_pro_row == 0 {
                Box::new(std::iter::repeat([].as_slice()).take(self.aggregated_values.len()/self.aggregations_pro_row))
            } else {
                Box::new(self.instances.chunks(self.tables_pro_row))
            };


            let aggregated_chunks: Box<dyn Iterator<Item = &[AggregatedValue]>> =
            if self.aggregations_pro_row == 0 {
                Box::new(std::iter::repeat([].as_slice()).take(self.instances.len()/self.tables_pro_row))
            } else {
                Box::new(self.aggregated_values.chunks(self.aggregations_pro_row))
            };

            for (instances, aggregated_values) in instances_chunks.zip(aggregated_chunks) {
                row.clear();
                for (table_query, instance_index) in root_table.iter_tables().zip(instances) {
                    if table_query.aggregations.len()==0 {
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
                    } else {
                        break;
                    }
                }
                for agg_value in aggregated_values.iter() {
                    let str_value = agg_value.to_display_str(&node_data.indexers).to_string();
                    row.push(str_value);
                }
                wtr.write_record(&row)?;
            }
        }     
        wtr.flush()?;
        
        Ok(())
    }
}