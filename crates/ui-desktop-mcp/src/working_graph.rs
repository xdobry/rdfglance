use std::collections::{BTreeMap, BTreeSet};

use rdf_glance::{
    IriIndex, RdfGlanceApp,
    graph_algorithms::GraphAlgorithm,
    ui::graph_view::{NeighborPos, update_layout_edges},
};
use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcpapp::resolve_node_index;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NodeListParams {
    pub nodes: Vec<String>,
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
    pub algorithm: Algorithm,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Algorithm {
    DegreeCentrality,
    BetweennessCentrality,
    ClosenessCentrality,
    KCoreCentrality,
    EigenvectorCentrality,
    PageRank,
    ClusteringLouvain,
    ClusteringSpectral,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct IriCount {
    pub iri: String,
    pub count: usize,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct GraphInfo {
    pub node_count: usize,
    pub edge_count: usize,
    pub types: Vec<IriCount>,
    pub predicates: Vec<IriCount>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AlgorithmResult {
    pub algorithm: String,
    pub statistics: Vec<StatisticResult>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct StatisticResult {
    pub name: String,
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
    let mut predicates = BTreeMap::<String, usize>::new();
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
        *predicates.entry(iri.to_string()).or_default() += 1;
    }
    Ok(GraphInfo {
        node_count: nodes.len(),
        edge_count: edges.len(),
        types: types.into_iter().map(|(iri, count)| IriCount { iri, count }).collect(),
        predicates: predicates
            .into_iter()
            .map(|(iri, count)| IriCount { iri, count })
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

pub fn run_algorithm(app: &mut RdfGlanceApp, parameters: AlgorithmParams) -> Result<AlgorithmResult, String> {
    let algorithm = match parameters.algorithm {
        Algorithm::DegreeCentrality => GraphAlgorithm::DegreeCentrality,
        Algorithm::BetweennessCentrality => GraphAlgorithm::BetweennessCentrality,
        Algorithm::ClosenessCentrality => GraphAlgorithm::ClosenessCentrality,
        Algorithm::KCoreCentrality => GraphAlgorithm::KCoreCentrality,
        Algorithm::EigenvectorCentrality => GraphAlgorithm::EigenvectorCentrality,
        Algorithm::PageRank => GraphAlgorithm::PageRank,
        Algorithm::ClusteringLouvain => GraphAlgorithm::ClusteringLouvain,
        Algorithm::ClusteringSpectral => GraphAlgorithm::ClusteringSpectral,
    };
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
    let data = app.rdf_data.read().map_err(|_| "dataset lock is poisoned")?;
    let results = statistics
        .results
        .iter()
        .filter(|result| algorithm.get_statistics_values().contains(&result.statistics_value()))
        .map(|result| {
            let values = statistics
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
            Ok(StatisticResult {
                name: result.statistics_value().to_string(),
                values,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(AlgorithmResult {
        algorithm: algorithm.to_string(),
        statistics: results,
    })
}
