use eframe::Storage;
use egui::Ui;
use rdf_analytics::domain::type_index::TypeData;
use rdf_glance::IriIndex;
use rdf_glance::domain::{
    RdfData,
    graph_model::short_iri,
    type_index::{InstanceView, TypeInstanceIndexView},
    visual_query::{FilterEvaluator, PredicateFilter},
};
use rdf_glance::uistate::actions::NodeAction;
use rdf_glance::{
    RdfGlanceApp,
    uistate::{DataLoading, SystemMessage, events::AppEvent},
};
use std::sync::{Arc, RwLock, atomic::Ordering};
use tokio::sync::{mpsc::Receiver, oneshot};

use crate::mcp::{GetTypeInstanceRangeParams, UiCommand};
use crate::tool_api::{
    DatasetProfile, Filter, InstanceQueryRangeResult, InstanceQueryResult, NodeInfo, NodePropertyInfo,
    NodeReferenceInfo, dataset_profile_from_index, value_type_names,
};

pub struct RdfGlanceMcpApp {
    ui_rx: Receiver<UiCommand>,
    my_app: RdfGlanceApp,
    dataset_profile: Arc<RwLock<DatasetProfile>>,
    pending_load: Option<(oneshot::Sender<Result<DatasetProfile, String>>, Arc<DataLoading>)>,
}

struct PropertyColumn {
    name: String,
    iri: String,
    predicate_index: IriIndex,
}

impl RdfGlanceMcpApp {
    pub fn new(
        storage: Option<&dyn Storage>,
        args: Vec<String>,
        ui_rx: Receiver<UiCommand>,
        dataset_profile: Arc<RwLock<DatasetProfile>>,
    ) -> Self {
        let my_app = RdfGlanceApp::new(storage, args);

        Self {
            ui_rx,
            my_app,
            dataset_profile,
            pending_load: None,
        }
    }

    fn process_mcp_commands(&mut self, _ctx: &egui::Context) {
        while self.my_app.load_handle.is_none() && self.my_app.data_loading.is_none() {
            let Ok(command) = self.ui_rx.try_recv() else { break };
            match command {
                UiCommand::WgAddNodes { parameters, reply } => {
                    let _ = reply.send(crate::working_graph::add_nodes(&mut self.my_app, parameters));
                }
                UiCommand::WgExpand { parameters, reply } => {
                    let _ = reply.send(crate::working_graph::expand(&mut self.my_app, parameters));
                }
                UiCommand::WgInfo { reply } => {
                    let _ = reply.send(crate::working_graph::info(&self.my_app));
                }
                UiCommand::WgClean { reply } => {
                    let _ = reply.send(crate::working_graph::clean(&mut self.my_app));
                }
                UiCommand::WgRunAlgorithm { parameters, reply } => {
                    let _ = reply.send(crate::working_graph::run_algorithm(&mut self.my_app, parameters));
                }
                UiCommand::WgResetStatistics { reply } => {
                    let _ = reply.send(crate::working_graph::reset_statistics(&mut self.my_app));
                }
                UiCommand::LoadData { parameters, reply } => {
                    if self.my_app.load_handle.is_some() || self.my_app.data_loading.is_some() {
                        let _ = reply.send(Err("loading is already in progress".to_string()));
                    } else if self.my_app.system_message.has_message() {
                        let _ = reply.send(Err("dismiss the current application message before loading".to_string()));
                    } else if !std::path::Path::new(&parameters.file).is_file() {
                        let _ = reply.send(Err(format!("file not found: {}", parameters.file)));
                    } else {
                        self.my_app.load_ttl(&parameters.file, false);
                        if let Some(loading) = &self.my_app.data_loading {
                            self.pending_load = Some((reply, Arc::clone(loading)));
                        } else {
                            let _ = reply.send(Err("loading could not be started".to_string()));
                        }
                    }
                }
                UiCommand::ResetData { reply } => {
                    let _ = reply.send(reset_data(&mut self.my_app, &self.dataset_profile));
                }
                UiCommand::ShowTypeInstances { parameters, reply } => {
                    let rdf_data = match self.my_app.rdf_data.read() {
                        Ok(rdf_data) => rdf_data,
                        Err(_) => {
                            let _ = reply.send(Err("no data lock".to_string()));
                            continue;
                        }
                    };
                    let Some(type_index) = rdf_data
                        .node_data
                        .indexers
                        .type_indexer
                        .get_index_opt(&parameters.type_name)
                    else {
                        let _ = reply.send(Err(format!(
                            "no type with name {} found. Consult resource again",
                            parameters.type_name
                        )));
                        continue;
                    };
                    let Some(type_data) = self.my_app.type_index.type_instance_index.types.get(&type_index) else {
                        let _ = reply.send(Err(format!("no instances found for type {}", parameters.type_name)));
                        continue;
                    };
                    let instances = match filter_type_instances(type_data, &rdf_data, &parameters.filters) {
                        Ok(instances) => instances,
                        Err(error) => {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                    };
                    let count = instances.len();
                    let columns = type_property_columns(type_data, &rdf_data);
                    let result_columns = columns.iter().map(|column| column.name.clone()).collect();
                    drop(rdf_data);
                    self.my_app
                        .queue_node_action(NodeAction::ShowTypeInstances(type_index, instances));
                    let _ = reply.send(Ok(InstanceQueryResult {
                        columns: result_columns,
                        count,
                    }));
                }
                UiCommand::GetTypeInstanceRange { parameters, reply } => {
                    let rdf_data = match self.my_app.rdf_data.read() {
                        Ok(rdf_data) => rdf_data,
                        Err(_) => {
                            let _ = reply.send(Err("no data lock".to_string()));
                            continue;
                        }
                    };
                    let result = instance_query_range(&self.my_app.type_index, &rdf_data, &parameters);
                    let _ = reply.send(result);
                }
                UiCommand::ShowNode { parameters, reply } => {
                    let rdf_data = match self.my_app.rdf_data.read() {
                        Ok(rdf_data) => rdf_data,
                        Err(_) => {
                            let _ = reply.send(Err("no data lock".to_string()));
                            continue;
                        }
                    };
                    let Some(node_index) = resolve_node_index(&rdf_data, &parameters.iri) else {
                        let _ = reply.send(Err(format!("no node with IRI {} found", parameters.iri)));
                        continue;
                    };
                    let Some((node_iri, _)) = rdf_data.node_data.get_node_by_index(node_index) else {
                        let _ = reply.send(Err(format!("no node with IRI {} found", parameters.iri)));
                        continue;
                    };
                    let node_iri = node_iri.to_string();
                    drop(rdf_data);
                    self.my_app.queue_node_action(NodeAction::BrowseNode(node_index));
                    let _ = reply.send(Ok(node_iri));
                }
                UiCommand::GetNodeInfo { parameters, reply } => {
                    let rdf_data = match self.my_app.rdf_data.read() {
                        Ok(rdf_data) => rdf_data,
                        Err(_) => {
                            let _ = reply.send(Err("no data lock".to_string()));
                            continue;
                        }
                    };
                    let result = parameters
                        .node_iris
                        .iter()
                        .map(|iri| node_info(&rdf_data, iri))
                        .collect();
                    let _ = reply.send(result);
                }
            }
        }
    }
}

fn reset_data(app: &mut RdfGlanceApp, dataset_profile: &RwLock<DatasetProfile>) -> Result<DatasetProfile, String> {
    if app.load_handle.is_some() || app.data_loading.is_some() {
        return Err("cannot reset data while loading is in progress".to_string());
    }
    let mut profile = dataset_profile
        .write()
        .map_err(|_| "dataset profile lock is poisoned".to_string())?;
    app.clean_data();
    app.statistics_data = None;
    *profile = DatasetProfile::default();
    Ok(profile.clone())
}

fn node_info(rdf_data: &RdfData, requested_iri: &str) -> Result<NodeInfo, String> {
    let node_index =
        resolve_node_index(rdf_data, requested_iri).ok_or_else(|| format!("no node with IRI {requested_iri} found"))?;
    let (iri, node) = rdf_data
        .node_data
        .get_node_by_index(node_index)
        .ok_or_else(|| format!("node with IRI {requested_iri} is unavailable"))?;

    let types = node
        .types
        .iter()
        .map(|type_index| {
            rdf_data
                .node_data
                .get_type(*type_index)
                .map(str::to_string)
                .ok_or_else(|| format!("node {iri} has an invalid type index {type_index}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let properties = node
        .properties
        .iter()
        .map(|(predicate_index, literal)| {
            let predicate = rdf_data
                .node_data
                .get_predicate(*predicate_index)
                .ok_or_else(|| format!("node {iri} has an invalid predicate index {predicate_index}"))?;
            let datatype = match literal {
                rdf_glance::domain::Literal::TypedString(datatype_index, _) => Some(
                    rdf_data
                        .node_data
                        .indexers
                        .datatype_indexer
                        .index_to_str(*datatype_index as IriIndex)
                        .ok_or_else(|| format!("node {iri} has an invalid datatype index {datatype_index}"))?
                        .to_string(),
                ),
                _ => None,
            };
            Ok(NodePropertyInfo {
                predicate: predicate.to_string(),
                value: literal.as_str_ref(&rdf_data.node_data.indexers).to_string(),
                datatype,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(NodeInfo {
        iri: iri.to_string(),
        types,
        properties,
        references: node_reference_infos(&rdf_data.node_data, &node.references, iri)?,
        reverse_references: node_reference_infos(&rdf_data.node_data, &node.reverse_references, iri)?,
        is_blank_node: node.is_blank_node,
        has_subject: node.has_subject,
    })
}

fn node_reference_infos(
    node_data: &rdf_glance::domain::NodeData,
    references: &[(IriIndex, IriIndex)],
    subject_iri: &str,
) -> Result<Vec<NodeReferenceInfo>, String> {
    references
        .iter()
        .map(|(predicate_index, node_index)| {
            let predicate = node_data.get_predicate(*predicate_index).ok_or_else(|| {
                format!("node {subject_iri} has an invalid reference predicate index {predicate_index}")
            })?;
            let (node_iri, _) = node_data
                .get_node_by_index(*node_index)
                .ok_or_else(|| format!("node {subject_iri} has an invalid reference node index {node_index}"))?;
            Ok(NodeReferenceInfo {
                predicate: predicate.to_string(),
                node_iri: node_iri.to_string(),
            })
        })
        .collect()
}

pub(crate) fn resolve_node_index(rdf_data: &RdfData, iri: &str) -> Option<IriIndex> {
    rdf_data.node_data.get_node_index(iri).or_else(|| {
        rdf_data
            .node_data
            .get_node_index(&rdf_data.prefix_manager.get_prefixed(iri))
            .or_else(|| {
                let full_iri = rdf_data.prefix_manager.get_full_opt(iri)?;
                rdf_data.node_data.get_node_index(&full_iri)
            })
    })
}

fn type_property_columns(type_data: &TypeData, rdf_data: &RdfData) -> Vec<PropertyColumn> {
    let mut columns: Vec<PropertyColumn> = type_data
        .properties
        .keys()
        .filter_map(|predicate_index| {
            let iri = rdf_data.node_data.get_predicate(*predicate_index)?;
            Some(PropertyColumn {
                name: short_iri(iri).to_string(),
                iri: iri.to_string(),
                predicate_index: *predicate_index,
            })
        })
        .collect();
    columns.sort_by(|left, right| left.name.cmp(&right.name).then(left.iri.cmp(&right.iri)));
    columns
}

fn visible_property_columns(instance_view: &InstanceView, rdf_data: &RdfData) -> Vec<PropertyColumn> {
    instance_view
        .display_properties
        .iter()
        .filter(|column| column.visible)
        .filter_map(|column| {
            let iri = rdf_data.node_data.get_predicate(column.predicate_index)?;
            Some(PropertyColumn {
                name: short_iri(iri).to_string(),
                iri: iri.to_string(),
                predicate_index: column.predicate_index,
            })
        })
        .collect()
}

fn instance_query_range(
    type_index_view: &TypeInstanceIndexView,
    rdf_data: &RdfData,
    parameters: &GetTypeInstanceRangeParams,
) -> Result<InstanceQueryRangeResult, String> {
    if parameters.to < parameters.from {
        return Err("to must be greater than or equal to from".to_string());
    }
    let selected_type = type_index_view
        .selected_type
        .ok_or_else(|| "no type is currently selected".to_string())?;
    let type_data = type_index_view
        .type_instance_index
        .types
        .get(&selected_type)
        .ok_or_else(|| "the selected type has no instance data".to_string())?;
    let instance_view = type_index_view
        .instance_views
        .get(&selected_type)
        .ok_or_else(|| "the selected type has no table view".to_string())?;
    let property_columns = visible_property_columns(instance_view, rdf_data);

    let selected_columns: Vec<&PropertyColumn> = match &parameters.columns {
        None => property_columns.iter().collect(),
        Some(names) => {
            let mut selected = Vec::new();
            for name in names {
                if name == "iri" {
                    continue;
                }
                let matching_columns: Vec<&PropertyColumn> = property_columns
                    .iter()
                    .filter(|column| column.name == name.as_str() || column.iri == name.as_str())
                    .collect();
                match matching_columns.as_slice() {
                    [column] => selected.push(*column),
                    [] => return Err(format!("unknown property column {name}")),
                    _ => return Err(format!("property column {name} is ambiguous; use its full IRI")),
                }
            }
            selected
        }
    };

    let mut columns = Vec::with_capacity(selected_columns.len() + 1);
    columns.push("iri".to_string());
    columns.extend(selected_columns.iter().map(|column| column.name.clone()));

    let instances = &type_data.filtered_instances;
    let start = parameters.from.min(instances.len());
    let end = parameters
        .to
        .min(parameters.from.saturating_add(100))
        .min(instances.len());
    let mut rows = Vec::with_capacity(end.saturating_sub(start));
    for instance_index in &instances[start..end] {
        let Some((iri, node)) = rdf_data.node_data.get_node_by_index(*instance_index) else {
            return Err(format!("instance {instance_index} is no longer available"));
        };
        let mut row = Vec::with_capacity(columns.len());
        row.push(iri.to_string());
        for column in &selected_columns {
            let value = node
                .get_property(column.predicate_index, 0)
                .map(|literal| literal.as_str_ref(&rdf_data.node_data.indexers).to_string())
                .unwrap_or_default();
            row.push(value);
        }
        rows.push(row);
    }

    Ok(InstanceQueryRangeResult { columns, rows })
}

fn filter_type_instances(
    type_data: &TypeData,
    rdf_data: &RdfData,
    filters: &[Filter],
) -> Result<Vec<IriIndex>, String> {
    let predicate_filters: Result<Vec<PredicateFilter>, String> = filters
        .iter()
        .map(|filter| {
            let predicate_indexer = &rdf_data.node_data.indexers.predicate_indexer;
            let predicate_iri = predicate_indexer
                .get_index_opt(&filter.attribute)
                .or_else(|| {
                    let prefixed_iri = rdf_data.prefix_manager.get_prefixed(&filter.attribute);
                    predicate_indexer.get_index_opt(&prefixed_iri)
                })
                .ok_or_else(|| format!("no predicate with IRI {} found", filter.attribute))?;
            if !type_data.properties.contains_key(&predicate_iri) {
                return Err(format!("property {} is not used for the type", filter.attribute));
            }
            Ok(PredicateFilter {
                predicate_iri,
                filter_type: filter.operator,
                filter_value: filter.value.clone(),
                to_remove: false,
            })
        })
        .collect();
    let predicate_filters = predicate_filters?;
    let evaluator = FilterEvaluator::from_predicate_filters(&predicate_filters);

    Ok(type_data
        .instances
        .iter()
        .copied()
        .filter(|instance| evaluator.match_object(*instance, rdf_data))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdf_glance::domain::visual_query::FilterType;
    use std::path::PathBuf;

    #[test]
    fn reset_data_clears_dataset_and_profile() {
        let data_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample-rdf-data/programming_languages.ttl");
        let mut app = RdfGlanceApp::new(None, vec![]);
        app.load_ttl(data_path.to_str().unwrap(), true);
        app.join_load(false);
        assert!(!app.is_empty());
        let profile = RwLock::new(DatasetProfile {
            nodes: 42,
            ..Default::default()
        });

        let result = reset_data(&mut app, &profile).expect("reset should succeed");
        assert!(app.is_empty());
        assert_eq!(result.nodes, 0);
        assert_eq!(profile.read().unwrap().nodes, 0);
    }

    #[test]
    fn filter_type_instances_uses_visual_query_filtering() {
        let data_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sample-rdf-data/programming_languages.ttl");
        let data_path = data_path.to_str().expect("sample data path should be valid UTF-8");
        let mut app = RdfGlanceApp::new(None, vec![]);
        app.load_ttl(data_path, true);
        app.join_load(false);

        let rdf_data = app.rdf_data.read().expect("RDF data lock should be available");
        let type_index = app
            .type_index
            .type_instance_index
            .types
            .iter()
            .find_map(|(type_index, _)| {
                rdf_data
                    .node_data
                    .get_type(*type_index)
                    .filter(|type_iri| type_iri.ends_with("ProgrammingLanguage"))
                    .map(|_| *type_index)
            })
            .expect("programming language type should be indexed");
        let type_data = app
            .type_index
            .type_instance_index
            .types
            .get(&type_index)
            .expect("programming language type data should be indexed");
        let columns = type_property_columns(type_data, &rdf_data);
        assert!(columns.iter().any(|column| column.name == "label"));

        let label_predicate = (0..rdf_data.node_data.unique_predicates())
            .map(|predicate_index| predicate_index as IriIndex)
            .find(|predicate_index| {
                rdf_data
                    .node_data
                    .get_predicate(*predicate_index)
                    .is_some_and(|predicate_iri| predicate_iri.ends_with("label"))
            })
            .expect("label predicate should be indexed");
        let label_iri = rdf_data.node_data.get_predicate(label_predicate).unwrap();
        let label_iri = rdf_data
            .prefix_manager
            .get_full_opt(label_iri)
            .map(|iri| iri.into_string())
            .unwrap_or_else(|| label_iri.to_string());

        let unfiltered =
            filter_type_instances(type_data, &rdf_data, &[]).expect("empty filters should return all instances");
        assert_eq!(unfiltered.len(), type_data.instances.len());

        let filtered = filter_type_instances(
            type_data,
            &rdf_data,
            &[Filter {
                attribute: label_iri.to_string(),
                value: "Rust".to_string(),
                operator: FilterType::Equals,
            }],
        )
        .expect("known predicate should filter instances");
        assert_eq!(filtered.len(), 1);
        let (_, rust_node) = rdf_data
            .node_data
            .get_node_by_index(filtered[0])
            .expect("filtered instance should still exist");
        assert!(rust_node.properties.iter().any(|(predicate, literal)| {
            *predicate == label_predicate && literal.as_str_ref(&rdf_data.node_data.indexers) == "Rust"
        }));
        let rust_iri = rdf_data
            .node_data
            .get_node_by_index(filtered[0])
            .expect("filtered instance should still exist")
            .0
            .to_string();
        assert_eq!(resolve_node_index(&rdf_data, &rust_iri), Some(filtered[0]));
        let compact_rust_iri = rdf_data.prefix_manager.get_prefixed(&rust_iri);
        assert_eq!(resolve_node_index(&rdf_data, &compact_rust_iri), Some(filtered[0]));
        assert_eq!(resolve_node_index(&rdf_data, "urn:missing-node"), None);
        let rust_info = node_info(&rdf_data, &compact_rust_iri).expect("known node info should be available");
        assert_eq!(rust_info.iri, rust_iri);
        assert!(!rust_info.types.is_empty());
        assert!(rust_info.properties.iter().any(|property| {
            property.predicate.ends_with("label") && property.value == "Rust" && !property.value_types.is_empty()
        }));
        assert_eq!(rust_info.is_blank_node, rust_node.is_blank_node);
        assert_eq!(rust_info.has_subject, rust_node.has_subject);

        let error = filter_type_instances(
            type_data,
            &rdf_data,
            &[Filter {
                attribute: "urn:missing-predicate".to_string(),
                value: "x".to_string(),
                operator: FilterType::Equals,
            }],
        )
        .expect_err("unknown predicates should return an error");
        assert!(error.contains("urn:missing-predicate"));

        let error = filter_type_instances(
            type_data,
            &rdf_data,
            &[Filter {
                attribute: "dbp:nationality".to_string(),
                value: "American".to_string(),
                operator: FilterType::Equals,
            }],
        )
        .expect_err("known predicates unused by the type should return an error");
        assert_eq!(error, "property dbp:nationality is not used for the type");

        let type_index_view = &mut app.type_index;
        type_index_view.selected_type = Some(type_index);
        type_index_view
            .type_instance_index
            .types
            .get_mut(&type_index)
            .expect("selected type data should be available")
            .filtered_instances = vec![filtered[0]; 101];
        let instance_view = type_index_view
            .instance_views
            .get_mut(&type_index)
            .expect("selected type table view should be available");
        for column in &mut instance_view.display_properties {
            column.visible = column.predicate_index == label_predicate;
        }

        let selected_column = instance_query_range(
            type_index_view,
            &rdf_data,
            &GetTypeInstanceRangeParams {
                from: 0,
                to: 1,
                columns: Some(vec!["label".to_string()]),
            },
        )
        .expect("selected property column should be returned");
        assert_eq!(selected_column.columns, vec!["iri", "label"]);
        assert_eq!(selected_column.rows[0][1], "Rust");

        let page = instance_query_range(
            type_index_view,
            &rdf_data,
            &GetTypeInstanceRangeParams {
                from: 1,
                to: 3,
                columns: None,
            },
        )
        .expect("an omitted selection should return all visible columns in the requested range");
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.columns, vec!["iri", "label"]);

        let iri_only = instance_query_range(
            type_index_view,
            &rdf_data,
            &GetTypeInstanceRangeParams {
                from: 0,
                to: 1,
                columns: Some(Vec::new()),
            },
        )
        .expect("an empty property selection should return IRIs");
        assert_eq!(iri_only.columns, vec!["iri"]);
        assert_eq!(iri_only.rows[0].len(), 1);

        let capped = instance_query_range(
            type_index_view,
            &rdf_data,
            &GetTypeInstanceRangeParams {
                from: 0,
                to: usize::MAX,
                columns: Some(Vec::new()),
            },
        )
        .expect("large ranges should be capped");
        assert_eq!(capped.rows.len(), 100);
    }
}

impl eframe::App for RdfGlanceMcpApp {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Some(app_event) = self.my_app.app_event.take() {
            match app_event {
                AppEvent::DataSetChange => {
                    if let Ok(rdf_data) = self.my_app.rdf_data.read() {
                        let profile =
                            dataset_profile_from_index(&rdf_data, &self.my_app.type_index.type_instance_index);
                        if let Ok(mut dataset_profile) = self.dataset_profile.write() {
                            *dataset_profile = profile;
                        }
                    }
                }
            }
        }
        if self.pending_load.is_some() && self.my_app.load_handle.is_none() {
            let (reply, loading) = self.pending_load.take().unwrap();
            let result = if loading.stop_loading.load(Ordering::Relaxed) {
                Err("loading was stopped".to_string())
            } else if let SystemMessage::Error(error) = &self.my_app.system_message {
                Err(error.clone())
            } else {
                self.dataset_profile
                    .read()
                    .map(|profile| profile.clone())
                    .map_err(|_| "dataset profile lock is poisoned".to_string())
            };
            let _ = reply.send(result);
        }
        self.process_mcp_commands(ctx);
        if self.my_app.load_handle.is_none() && !self.ui_rx.is_empty() {
            ctx.request_repaint();
        }
        self.my_app.logic(ctx, frame);
    }

    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.my_app.ui(ui, frame);
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        self.my_app.save(storage);
    }
}
