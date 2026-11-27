use std::collections::{BTreeMap, BTreeSet};

use rdf_analytics::graph_algorithms::StatisticValue;
use rdf_glance::{
    IriIndex, RdfGlanceApp,
    graph_algorithms::GraphAlgorithm,
    ui::graph_view::{NeighborPos, update_layout_edges},
};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcpapp::resolve_node_index;
use crate::tool_api::Filter;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NodeListParams {
    pub nodes: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddTypeInstancesParams {
    pub type_name: String,
    #[serde(default)]
    pub filters: Vec<Filter>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetHiddenPredicatesParams {
    pub hidden_predicates: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExpandParams {
    #[serde(default)]
    pub nodes: Vec<String>,
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub predicates: Vec<DirectedPredicate>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DirectedPredicate {
    pub predicate: String,
    pub direction: Direction,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Standard,
    Reverse,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AlgorithmParams {
    pub algorithm: StatisticKind,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticKind {
    DegreeCentrality,
    BetweennessCentrality,
    ClosenessCentrality,
    KCoreCentrality,
    EigenvectorCentrality,
    PageRank,
    ClusteringLouvain,
    ClusteringSpectral,
    FiedlerVector,
}

impl StatisticKind {
    fn graph_algorithm(self) -> GraphAlgorithm {
        match self {
            Self::DegreeCentrality => GraphAlgorithm::DegreeCentrality,
            Self::BetweennessCentrality => GraphAlgorithm::BetweennessCentrality,
            Self::ClosenessCentrality => GraphAlgorithm::ClosenessCentrality,
            Self::KCoreCentrality => GraphAlgorithm::KCoreCentrality,
            Self::EigenvectorCentrality => GraphAlgorithm::EigenvectorCentrality,
            Self::PageRank => GraphAlgorithm::PageRank,
            Self::ClusteringLouvain => GraphAlgorithm::ClusteringLouvain,
            Self::ClusteringSpectral | Self::FiedlerVector => GraphAlgorithm::ClusteringSpectral,
        }
    }

    fn from_statistic_value(value: StatisticValue) -> Self {
        match value {
            StatisticValue::DegreeCentrality => Self::DegreeCentrality,
            StatisticValue::BetweennessCentrality => Self::BetweennessCentrality,
            StatisticValue::ClosenessCentrality => Self::ClosenessCentrality,
            StatisticValue::KCoreCentrality => Self::KCoreCentrality,
            StatisticValue::EigenvectorCentrality => Self::EigenvectorCentrality,
            StatisticValue::PageRank => Self::PageRank,
            StatisticValue::ClusteringLouvain => Self::ClusteringLouvain,
            StatisticValue::ClusteringSpectral => Self::ClusteringSpectral,
            StatisticValue::FiedlerVector => Self::FiedlerVector,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsOrder {
    #[default]
    Highest,
    Lowest,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetStatisticsDataParams {
    pub statistic: StatisticKind,
    #[serde(default)]
    pub order: StatisticsOrder,
    #[serde(default = "default_statistics_limit")]
    pub limit: usize,
}

fn default_statistics_limit() -> usize {
    100
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct IriCount {
    pub iri: String,
    pub count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct PredicateCount {
    pub iri: String,
    pub count: usize,
    pub visible_count: usize,
    pub hidden_count: usize,
    pub hidden: bool,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphInfo {
    pub node_count: usize,
    pub edge_count: usize,
    pub visible_edge_count: usize,
    pub hidden_edge_count: usize,
    pub types: Vec<IriCount>,
    pub predicates: Vec<PredicateCount>,
    pub hidden_predicates: Vec<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatisticsInfo {
    pub row_count: usize,
    pub statistics: Vec<StatisticKind>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatisticsDataResult {
    pub statistic: StatisticKind,
    pub order: StatisticsOrder,
    pub total_rows: usize,
    pub returned_rows: usize,
    pub values: Vec<NodeValue>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct NodeValue {
    pub iri: String,
    pub value: f32,
}

pub fn info(app: &RdfGlanceApp) -> Result<GraphInfo, String> {
    let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
    let nodes = app
        .visible_nodes
        .nodes
        .read()
        .map_err(|_| "graph nodes lock is poisoned")?;
    let edges = app
        .visible_nodes
        .edges
        .read()
        .map_err(|_| "graph edges lock is poisoned")?;
    let mut types = BTreeMap::<String, usize>::new();
    let mut predicates = BTreeMap::<String, (usize, usize, usize)>::new();
    for node in nodes.iter() {
        let (_, object) = data
            .node_data
            .get_node_by_index(node.node_index)
            .ok_or("graph node is unavailable")?;
        for type_index in &object.types {
            let iri = data
                .node_data
                .get_type(*type_index)
                .ok_or("graph node has an invalid type")?;
            *types.entry(iri.to_string()).or_default() += 1;
        }
    }
    for edge in edges.iter() {
        let iri = data
            .node_data
            .get_predicate(edge.predicate)
            .ok_or("graph edge has an invalid predicate")?;
        let counts = predicates.entry(iri.to_string()).or_default();
        counts.0 += 1;
        if app.ui_state.hidden_predicates.contains(edge.predicate) {
            counts.2 += 1;
        } else {
            counts.1 += 1;
        }
    }
    let visible_edge_count = predicates.values().map(|counts| counts.1).sum();
    let hidden_edge_count = predicates.values().map(|counts| counts.2).sum();
    Ok(GraphInfo {
        node_count: nodes.len(),
        edge_count: edges.len(),
        visible_edge_count,
        hidden_edge_count,
        types: types.into_iter().map(|(iri, count)| IriCount { iri, count }).collect(),
        predicates: predicates
            .into_iter()
            .map(|(iri, (count, visible_count, hidden_count))| PredicateCount {
                hidden: hidden_count > 0 && visible_count == 0,
                iri,
                count,
                visible_count,
                hidden_count,
            })
            .collect(),
        hidden_predicates: app
            .ui_state
            .hidden_predicates
            .data
            .iter()
            .filter_map(|index| data.node_data.get_predicate(*index).map(str::to_string))
            .collect(),
    })
}

fn add_targets(app: &mut RdfGlanceApp, targets: &[(IriIndex, IriIndex)]) -> Result<(), String> {
    let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
    let mut neighbors = NeighborPos::new();
    if neighbors.add_many(&mut app.visible_nodes, targets, &app.persistent_data.config_data) {
        update_layout_edges(
            &neighbors,
            &mut app.visible_nodes,
            &data.node_data,
            &app.ui_state.hidden_predicates,
        );
        neighbors.position(&mut app.visible_nodes);
        app.visible_nodes
            .start_layout(&app.persistent_data.config_data, &app.ui_state.hidden_predicates);
    }
    Ok(())
}

pub fn add_nodes(app: &mut RdfGlanceApp, parameters: NodeListParams) -> Result<GraphInfo, String> {
    let targets = {
        let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
        parameters
            .nodes
            .iter()
            .map(|iri| {
                resolve_node_index(&data, iri)
                    .map(|index| (index, index))
                    .ok_or_else(|| format!("node not found: {iri}"))
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    add_targets(app, &targets)?;
    info(app)
}

pub fn add_type_instances(app: &mut RdfGlanceApp, parameters: AddTypeInstancesParams) -> Result<GraphInfo, String> {
    let targets = {
        let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
        let type_index = data
            .node_data
            .indexers
            .type_indexer
            .get_index_opt(&parameters.type_name)
            .or_else(|| {
                let full_iri = data.prefix_manager.get_prefixed(&parameters.type_name);
                data.node_data.indexers.type_indexer.get_index_opt(&full_iri)
            })
            .ok_or_else(|| format!("no type with IRI {} found", parameters.type_name))?;
        let type_data = app
            .type_index
            .type_instance_index
            .types
            .get(&type_index)
            .ok_or_else(|| format!("no instances found for type {}", parameters.type_name))?;
        crate::mcpapp::filter_type_instances(type_data, &data, &parameters.filters)?
            .into_iter()
            .map(|index| (index, index))
            .collect::<Vec<_>>()
    };
    add_targets(app, &targets)?;
    info(app)
}

pub fn set_hidden_predicates(
    app: &mut RdfGlanceApp,
    parameters: SetHiddenPredicatesParams,
) -> Result<GraphInfo, String> {
    let hidden_indices = {
        let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
        parameters
            .hidden_predicates
            .iter()
            .map(|predicate| {
                data.node_data
                    .indexers
                    .predicate_indexer
                    .get_index_opt(predicate)
                    .or_else(|| {
                        let full_iri = data.prefix_manager.get_prefixed(predicate);
                        data.node_data.indexers.predicate_indexer.get_index_opt(&full_iri)
                    })
                    .ok_or_else(|| format!("predicate with IRI {predicate} not found"))
            })
            .collect::<Result<Vec<_>, _>>()?
    };

    app.set_hidden_predicates(&hidden_indices).map_err(str::to_string)?;
    info(app)
}

pub fn expand(app: &mut RdfGlanceApp, parameters: ExpandParams) -> Result<GraphInfo, String> {
    let targets = {
        let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
        let nodes = app
            .visible_nodes
            .nodes
            .read()
            .map_err(|_| "graph nodes lock is poisoned")?;
        let visible: BTreeSet<_> = nodes.iter().map(|node| node.node_index).collect();
        let selected = if parameters.nodes.is_empty() {
            visible.clone()
        } else {
            parameters
                .nodes
                .iter()
                .map(|iri| {
                    let index = resolve_node_index(&data, iri).ok_or_else(|| format!("node not found: {iri}"))?;
                    if !visible.contains(&index) {
                        return Err(format!("node is not in the working graph: {iri}"));
                    }
                    Ok(index)
                })
                .collect::<Result<BTreeSet<_>, String>>()?
        };
        let types = parameters
            .types
            .iter()
            .map(|iri| {
                data.node_data
                    .indexers
                    .type_indexer
                    .get_index_opt(iri)
                    .or_else(|| {
                        data.node_data
                            .indexers
                            .type_indexer
                            .get_index_opt(&data.prefix_manager.get_prefixed(iri))
                    })
                    .ok_or_else(|| format!("type not found: {iri}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let predicates = parameters
            .predicates
            .iter()
            .map(|filter| {
                let index = data
                    .node_data
                    .indexers
                    .predicate_indexer
                    .get_index_opt(&filter.predicate)
                    .or_else(|| {
                        data.node_data
                            .indexers
                            .predicate_indexer
                            .get_index_opt(&data.prefix_manager.get_prefixed(&filter.predicate))
                    })
                    .ok_or_else(|| format!("predicate not found: {}", filter.predicate))?;
                Ok((index, &filter.direction))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut targets = Vec::new();
        for source in selected {
            let (_, node) = data
                .node_data
                .get_node_by_index(source)
                .ok_or("graph node is unavailable")?;
            for (references, reverse) in [(&node.references, false), (&node.reverse_references, true)] {
                for (predicate, target) in references {
                    if app.ui_state.hidden_predicates.contains(*predicate) {
                        continue;
                    }
                    if !predicates.is_empty()
                        && !predicates.iter().any(|(index, direction)| {
                            index == predicate
                                && matches!(
                                    (direction, reverse),
                                    (Direction::Standard, false) | (Direction::Reverse, true)
                                )
                        })
                    {
                        continue;
                    }
                    let (_, target_node) = data
                        .node_data
                        .get_node_by_index(*target)
                        .ok_or("referenced node is unavailable")?;
                    if types.is_empty() || target_node.match_types(&types) {
                        targets.push((source, *target));
                    }
                }
            }
        }
        targets
    };
    add_targets(app, &targets)?;
    info(app)
}

pub fn clean(app: &mut RdfGlanceApp) -> Result<GraphInfo, String> {
    app.visible_nodes.clean_all();
    app.ui_state.selected_nodes.clear();
    app.ui_state.selected_node = None;
    info(app)
}

pub fn reset_statistics(app: &mut RdfGlanceApp) -> Result<GraphInfo, String> {
    if let Some(statistics) = &mut app.statistics_data {
        statistics.results.clear();
    }
    app.visualization_style.use_size_overwrite = false;
    app.visualization_style.use_color_overwrite = false;
    app.visible_nodes.update_node_shapes = true;
    info(app)
}

pub fn statistics_info(app: &RdfGlanceApp) -> Result<StatisticsInfo, String> {
    let Some(statistics) = &app.statistics_data else {
        return Ok(StatisticsInfo {
            row_count: 0,
            statistics: Vec::new(),
        });
    };
    if statistics.data_epoch != app.visible_nodes.data_epoch || statistics.results.is_empty() {
        return Ok(StatisticsInfo {
            row_count: 0,
            statistics: Vec::new(),
        });
    }

    let mut available = Vec::new();
    for result in &statistics.results {
        let statistic = StatisticKind::from_statistic_value(result.statistics_value());
        if !available.contains(&statistic) {
            available.push(statistic);
        }
    }
    Ok(StatisticsInfo {
        row_count: statistics.nodes.len(),
        statistics: available,
    })
}

pub fn get_statistics_data(
    app: &RdfGlanceApp,
    parameters: GetStatisticsDataParams,
) -> Result<StatisticsDataResult, String> {
    let statistics = app
        .statistics_data
        .as_ref()
        .ok_or_else(|| "no statistics are available; run wg_run_algorithm first".to_string())?;
    if statistics.data_epoch != app.visible_nodes.data_epoch {
        return Err("statistics are outdated; run wg_run_algorithm again".to_string());
    }
    let result = statistics
        .results
        .iter()
        .find(|result| StatisticKind::from_statistic_value(result.statistics_value()) == parameters.statistic)
        .ok_or_else(|| {
            format!(
                "statistic {:?} is not available; run wg_run_algorithm for this statistic first",
                parameters.statistic
            )
        })?;
    if statistics.nodes.len() != result.get_data_vec().len() {
        return Err("statistics data does not match the current working graph".to_string());
    }

    let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
    let mut values = statistics
        .nodes
        .iter()
        .zip(result.get_data_vec())
        .map(|((node_index, _), value)| {
            let (iri, _) = data
                .node_data
                .get_node_by_index(*node_index)
                .ok_or("statistics node is unavailable")?;
            Ok(NodeValue {
                iri: iri.to_string(),
                value: *value,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    values.sort_by(|left, right| {
        let order = match parameters.order {
            StatisticsOrder::Highest => right.value.total_cmp(&left.value),
            StatisticsOrder::Lowest => left.value.total_cmp(&right.value),
        };
        order.then_with(|| left.iri.cmp(&right.iri))
    });

    let total_rows = values.len();
    values.truncate(parameters.limit);
    Ok(StatisticsDataResult {
        statistic: parameters.statistic,
        order: parameters.order,
        total_rows,
        returned_rows: values.len(),
        values,
    })
}

pub fn run_algorithm(app: &mut RdfGlanceApp, parameters: AlgorithmParams) -> Result<StatisticsInfo, String> {
    let algorithm = parameters.algorithm.graph_algorithm();
    if app
        .visible_nodes
        .nodes
        .read()
        .map_err(|_| "graph nodes lock is poisoned")?
        .is_empty()
    {
        return Err("working graph is empty".to_string());
    }
    let statistics = app.statistics_data.get_or_insert_with(Default::default);
    app.visible_nodes.run_algorithm(
        algorithm,
        &app.visualization_style,
        statistics,
        &app.persistent_data.config_data,
        &app.ui_state.hidden_predicates,
    );
    app.visualization_style.use_size_overwrite = true;
    app.visualization_style.use_color_overwrite = true;
    statistics_info(app)
}
