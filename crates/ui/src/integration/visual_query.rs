use std::io;

use crate::{
    IriIndex,
    domain::{
        LabelContext, NodeData,
        visual_query::{AggregatedValue, VisualQueryView},
    },
};

impl VisualQueryView {
    pub fn export_csv<W: io::Write>(
        &self,
        wtr: &mut csv::Writer<W>,
        node_data: &NodeData,
        label_context: &LabelContext,
    ) -> std::io::Result<()> {
        if let Some(root_table) = self.query.root_table.as_ref() {
            let mut row: Vec<String> = Vec::new();
            let mut has_aggregation = false;
            for table in root_table.iter_tables() {
                if table.aggregations.len() == 0 {
                    if !has_aggregation {
                        for column in self.table_views[&table.id].visible_predicates.iter() {
                            if column.visible {
                                let predicate_label = node_data.predicate_display(
                                    column.predicate_index,
                                    &label_context,
                                    &node_data.indexers,
                                );
                                row.push(predicate_label.as_str().to_string());
                            }
                        }
                    }
                } else {
                    has_aggregation = true;
                    for aggr in table.aggregations.iter() {
                        let predicate_label =
                            node_data.predicate_display(aggr.predicate_iri, &label_context, &node_data.indexers);
                        let aggr_label = format!("{}({})", aggr.aggregation_type, predicate_label.as_str());
                        row.push(aggr_label);
                    }
                }
            }
            wtr.write_record(&row)?;

            if self.query.instances.len() == 0 && self.query.aggregated_values.len() == 0 {
                panic!("no data to csv export");
            }

            let instances_chunks: Box<dyn Iterator<Item = &[IriIndex]>> = if self.query.tables_pro_row == 0 {
                Box::new(
                    std::iter::repeat([].as_slice())
                        .take(self.query.aggregated_values.len() / self.query.aggregations_pro_row),
                )
            } else {
                Box::new(self.query.instances.chunks(self.query.tables_pro_row))
            };

            let aggregated_chunks: Box<dyn Iterator<Item = &[AggregatedValue]>> = if self.query.aggregations_pro_row
                == 0
            {
                Box::new(std::iter::repeat([].as_slice()).take(self.query.instances.len() / self.query.tables_pro_row))
            } else {
                Box::new(self.query.aggregated_values.chunks(self.query.aggregations_pro_row))
            };

            let columns_by_table: Vec<_> = root_table
                .iter_tables()
                .take_while(|table| table.aggregations.is_empty())
                .map(|table| self.table_views[&table.id].visible_predicates.as_slice())
                .collect();
            for (instances, aggregated_values) in instances_chunks.zip(aggregated_chunks) {
                row.clear();
                for (columns, instance_index) in columns_by_table.iter().zip(instances) {
                    let node = node_data.get_node_by_index(*instance_index);
                    if let Some((_node_iri, node)) = node {
                        for column_desc in columns.iter().filter(|p| p.visible) {
                            let property =
                                node.get_property_count(column_desc.predicate_index, label_context.language_index);
                            if let Some((property, _count)) = property {
                                let value = property.as_str_ref(&node_data.indexers);
                                row.push(value.to_string());
                            } else {
                                row.push("".to_string());
                            }
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            RdfData,
            prefix_manager::PrefixManager,
            type_index::ColumnDesc,
            visual_query::{TableQuery, TableQueryView},
        },
        integration::rdfwrap::RDFWrap,
    };
    use rdf_core::IriDisplay;

    #[test]
    fn csv_export_uses_visible_columns_from_table_view() {
        let mut rdf_data = RdfData {
            node_data: NodeData::new(),
            prefix_manager: PrefixManager::new(),
        };
        RDFWrap::load_file(
            "../../sample-rdf-data/programming_languages.ttl",
            &mut rdf_data,
            &[],
            None,
        )
        .unwrap();
        let node_data = &mut rdf_data.node_data;
        let label = node_data.get_predicate_index("rdfs:label");
        let date = node_data.get_predicate_index("dbo:latestReleaseDate");
        let english = node_data.get_language_index("en");
        let instance = node_data.get_node_index("dbr:.QL").unwrap();
        let mut view = VisualQueryView::default();
        let id = view.allocate_table_id();
        view.query.root_table = Some(TableQuery {
            id,
            ..Default::default()
        });
        view.query.instances.push(instance);
        view.table_views.insert(
            id,
            TableQueryView {
                visible_predicates: vec![
                    ColumnDesc {
                        predicate_index: label,
                        width: 100.0,
                        visible: true,
                    },
                    ColumnDesc {
                        predicate_index: date,
                        width: 100.0,
                        visible: false,
                    },
                ],
                ..Default::default()
            },
        );
        let context = LabelContext::new(english, IriDisplay::Prefixed, &rdf_data.prefix_manager);
        let mut writer = csv::Writer::from_writer(Vec::new());
        view.export_csv(&mut writer, node_data, &context).unwrap();
        let data = writer.into_inner().unwrap();
        let mut reader = csv::Reader::from_reader(data.as_slice());
        assert_eq!(reader.headers().unwrap().iter().collect::<Vec<_>>(), vec!["rdfs:label"]);
        assert_eq!(
            reader.records().next().unwrap().unwrap().iter().collect::<Vec<_>>(),
            vec![".QL"]
        );
    }
}
