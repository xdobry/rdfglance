use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{
        tool::ToolRouter,
        wrapper::{Json, Parameters},
    },
    model::{Implementation, ServerCapabilities, ServerConfig},
    schemars::JsonSchema,
    tool, tool_handler, tool_router,
};

use rmcp::RoleServer;
use rmcp::model::{
    ListResourcesResult, PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse, ReadResourceResult,
    Resource, ResourceContents,
};
use rmcp::service::RequestContext;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

use crate::tool_api::{
    DATASET_PROFILE_RESOURCE_URI, DatasetProfile, Filter, InstanceQueryRangeResult, InstanceQueryResult, NodeInfo,
    SearchNodeResult,
};
use crate::working_graph::{
    AddTypeInstancesParams, AlgorithmParams, ExpandParams, GetStatisticsDataParams, GraphInfo, NodeListParams,
    SetHiddenPredicatesParams, StatisticsDataResult, StatisticsInfo,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LoadDataParams {
    pub file: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowTypeInstancesParams {
    pub type_name: String,
    #[serde(default)]
    pub filters: Vec<Filter>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetTypeInstanceRangeParams {
    pub from: usize,
    pub to: usize,
    #[serde(default)]
    pub columns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowNodeParams {
    pub iri: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNodeInfoParams {
    pub node_iris: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    pub search_text: String,
    pub max_result: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TakeScreenShotParams {
    pub target_file_location: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayTypeName {
    Browse,
    Graph,
    Table,
    Prefixes,
    Configuration,
    MetaGraph,
    Statistics,
    VisualQuery,
    ReferenceResolver,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SwitchDisplayParams {
    pub display_type: DisplayTypeName,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_type_instances_filters_default_to_empty() {
        let parameters: ShowTypeInstancesParams = serde_json::from_value(serde_json::json!({
            "type_name": "dbo:ProgrammingLanguage"
        }))
        .expect("filters should be optional");

        assert!(parameters.filters.is_empty());

        let schema = serde_json::to_value(rmcp::schemars::schema_for!(ShowTypeInstancesParams))
            .expect("request schema should serialize");
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|required| required.iter().all(|field| field != "filters"))
        );
    }

    #[test]
    fn get_type_instance_range_distinguishes_omitted_and_empty_columns() {
        let all_columns: GetTypeInstanceRangeParams = serde_json::from_value(serde_json::json!({
            "from": 0,
            "to": 10
        }))
        .expect("range columns should be optional");
        assert!(all_columns.columns.is_none());

        let iri_only: GetTypeInstanceRangeParams = serde_json::from_value(serde_json::json!({
            "from": 0,
            "to": 10,
            "columns": []
        }))
        .expect("an empty column selection should be accepted");
        assert_eq!(iri_only.columns, Some(Vec::new()));
    }

    #[test]
    fn get_node_info_accepts_multiple_iris() {
        let parameters: GetNodeInfoParams = serde_json::from_value(serde_json::json!({
            "node_iris": ["https://example.org/a", "https://example.org/b"]
        }))
        .expect("node IRIs should deserialize");

        assert_eq!(parameters.node_iris.len(), 2);
    }

    #[test]
    fn search_requires_text_and_result_limit() {
        let parameters: SearchParams = serde_json::from_value(serde_json::json!({
            "search_text": "Rust",
            "max_result": 25
        }))
        .expect("search parameters should deserialize");

        assert_eq!(parameters.search_text, "Rust");
        assert_eq!(parameters.max_result, 25);
        assert!(serde_json::from_value::<SearchParams>(serde_json::json!({ "search_text": "Rust" })).is_err());
    }

    #[test]
    fn take_screen_shot_requires_target_file_location() {
        let parameters: TakeScreenShotParams = serde_json::from_value(serde_json::json!({
            "target_file_location": "screenshots/current.png"
        }))
        .expect("screenshot path should deserialize");
        assert_eq!(parameters.target_file_location, "screenshots/current.png");

        let schema = serde_json::to_value(rmcp::schemars::schema_for!(TakeScreenShotParams))
            .expect("request schema should serialize");
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|field| field == "target_file_location"))
        );
    }

    #[test]
    fn switch_display_accepts_display_type_names() {
        let parameters: SwitchDisplayParams = serde_json::from_value(serde_json::json!({
            "display_type": "visual_query"
        }))
        .expect("display type should deserialize");
        assert_eq!(parameters.display_type, DisplayTypeName::VisualQuery);

        let schema = serde_json::to_value(rmcp::schemars::schema_for!(SwitchDisplayParams))
            .expect("request schema should serialize");
        assert!(
            schema["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|field| field == "display_type"))
        );
    }

    #[test]
    fn statistics_data_parameters_apply_defaults() {
        let parameters: GetStatisticsDataParams = serde_json::from_value(serde_json::json!({
            "statistic": "page_rank"
        }))
        .expect("statistic parameters should deserialize with defaults");
        assert_eq!(parameters.statistic, crate::working_graph::StatisticKind::PageRank);
        assert_eq!(parameters.order, crate::working_graph::StatisticsOrder::Highest);
        assert_eq!(parameters.limit, 100);
    }

    #[tokio::test]
    async fn take_screen_shot_repaints_until_reply() {
        let (ui_tx, mut ui_rx) = mpsc::channel(1);
        let (repaint_tx, mut repaint_rx) = mpsc::unbounded_channel();
        let ctx = egui::Context::default();
        ctx.set_repaint_observer(move |_| {
            let _ = repaint_tx.send(());
        });
        let server = MyMcpServer::new(ui_tx, ctx, Arc::new(RwLock::new(DatasetProfile::default())));
        let screenshot = tokio::spawn(async move {
            server
                .take_screen_shot(Parameters(TakeScreenShotParams {
                    target_file_location: "screenshot.png".to_string(),
                }))
                .await
        });
        let UiCommand::TakeScreenShot { reply, .. } = ui_rx.recv().await.expect("screenshot command") else {
            panic!("expected screenshot command");
        };

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            for _ in 0..3 {
                repaint_rx.recv().await.expect("repaint while screenshot is pending");
            }
        })
        .await
        .expect("follow-up repaint without user input");
        reply.send(Ok("screenshot.png".to_string())).expect("screenshot receiver");
        let Json(path) = screenshot.await.expect("screenshot task").expect("screenshot response");
        assert_eq!(path, "screenshot.png");

        while repaint_rx.try_recv().is_ok() {}
        assert_ne!(
            tokio::time::timeout(std::time::Duration::from_millis(50), repaint_rx.recv()).await,
            Ok(Some(())),
            "repaints must stop after the reply"
        );
    }
}

pub enum UiCommand {
    WgAddNodes {
        parameters: NodeListParams,
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgAddTypeInstances {
        parameters: AddTypeInstancesParams,
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgSetHiddenPredicates {
        parameters: SetHiddenPredicatesParams,
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgExpand {
        parameters: ExpandParams,
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgInfo {
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgClean {
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    WgRunAlgorithm {
        parameters: AlgorithmParams,
        reply: oneshot::Sender<Result<StatisticsInfo, String>>,
    },
    StatisticsInfo {
        reply: oneshot::Sender<Result<StatisticsInfo, String>>,
    },
    GetStatisticsData {
        parameters: GetStatisticsDataParams,
        reply: oneshot::Sender<Result<StatisticsDataResult, String>>,
    },
    WgResetStatistics {
        reply: oneshot::Sender<Result<GraphInfo, String>>,
    },
    SwitchDisplay {
        display_type: DisplayTypeName,
        reply: oneshot::Sender<Result<DisplayTypeName, String>>,
    },
    LoadData {
        parameters: LoadDataParams,
        reply: oneshot::Sender<Result<DatasetProfile, String>>,
    },
    ResetData {
        reply: oneshot::Sender<Result<DatasetProfile, String>>,
    },
    ShowTypeInstances {
        parameters: ShowTypeInstancesParams,
        reply: oneshot::Sender<Result<InstanceQueryResult, String>>,
    },
    GetTypeInstanceRange {
        parameters: GetTypeInstanceRangeParams,
        reply: oneshot::Sender<Result<InstanceQueryRangeResult, String>>,
    },
    ShowNode {
        parameters: ShowNodeParams,
        reply: oneshot::Sender<Result<String, String>>,
    },
    GetNodeInfo {
        parameters: GetNodeInfoParams,
        reply: oneshot::Sender<Result<Vec<NodeInfo>, String>>,
    },
    Search {
        parameters: SearchParams,
        reply: oneshot::Sender<Result<Vec<SearchNodeResult>, String>>,
    },
    GetPrefixes {
        reply: oneshot::Sender<Result<BTreeMap<String, String>, String>>,
    },
    TakeScreenShot {
        parameters: TakeScreenShotParams,
        reply: oneshot::Sender<Result<String, String>>,
    },
}

#[derive(Clone)]
pub struct MyMcpServer {
    pub ui_tx: mpsc::Sender<UiCommand>,
    pub egui_ctx: egui::Context,
    pub tool_router: ToolRouter<Self>,
    pub dataset_profile: Arc<RwLock<DatasetProfile>>,
}

impl MyMcpServer {
    pub fn new(
        ui_tx: mpsc::Sender<UiCommand>,
        egui_ctx: egui::Context,
        dataset_profile: Arc<RwLock<DatasetProfile>>,
    ) -> Self {
        Self {
            ui_tx,
            egui_ctx,
            tool_router: Self::tool_router(),
            dataset_profile,
        }
    }
}

#[tool_router]
impl MyMcpServer {
    #[tool(description = "Add RDF nodes to the working graph by IRI")]
    async fn wg_add_nodes(
        &self,
        Parameters(parameters): Parameters<NodeListParams>,
    ) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgAddNodes { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(
        description = "Add all instances of an RDF type to the working graph, optionally filtered by data properties"
    )]
    async fn wg_add_type_instances(
        &self,
        Parameters(parameters): Parameters<AddTypeInstancesParams>,
    ) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgAddTypeInstances { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(
        description = "Replace the hidden RDF predicates used by working graph rendering and algorithms; an empty list shows all predicates"
    )]
    async fn wg_set_hidden_predicates(
        &self,
        Parameters(parameters): Parameters<SetHiddenPredicatesParams>,
    ) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgSetHiddenPredicates { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Expand working graph nodes by target types and directed predicates; empty lists mean all")]
    async fn wg_expand(&self, Parameters(parameters): Parameters<ExpandParams>) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgExpand { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Get working graph node and edge counts grouped by type and predicate")]
    async fn wg_info(&self) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgInfo { reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Remove all nodes and edges from the working graph")]
    async fn wg_clean(&self) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgClean { reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Run a graph algorithm on the current working graph")]
    async fn wg_run_algorithm(
        &self,
        Parameters(parameters): Parameters<AlgorithmParams>,
    ) -> Result<Json<StatisticsInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgRunAlgorithm { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Show the number of rows and statistic kinds currently available for the working graph")]
    async fn statistics_info(&self) -> Result<Json<StatisticsInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::StatisticsInfo { reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Return working graph node values for a statistic, sorted highest or lowest first")]
    async fn get_statistics_data(
        &self,
        Parameters(parameters): Parameters<GetStatisticsDataParams>,
    ) -> Result<Json<StatisticsDataResult>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::GetStatisticsData { parameters, reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Clear calculated working graph statistics and reset statistics styling")]
    async fn wg_reset_statistics(&self) -> Result<Json<GraphInfo>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::WgResetStatistics { reply })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(description = "Switch the application to a display type")]
    async fn switch_display(
        &self,
        Parameters(parameters): Parameters<SwitchDisplayParams>,
    ) -> Result<Json<DisplayTypeName>, McpError> {
        let (reply, receiver) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::SwitchDisplay {
                display_type: parameters.display_type,
                reply,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        Ok(Json(
            receiver
                .await
                .map_err(|_| McpError::internal_error("egui did not respond", None))?
                .map_err(|error| McpError::internal_error(error, None))?,
        ))
    }

    #[tool(
        description = "Add a local file to the current dataset and return the updated profile. Supported extensions: ttl, rdf, nt, trig, nq, jsonld, csv, json, xml, ndjson."
    )]
    async fn load_data(&self, Parameters(args): Parameters<LoadDataParams>) -> Result<Json<DatasetProfile>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::LoadData {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        let profile = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(Json(profile))
    }

    #[tool(description = "Clear the current dataset and its derived views")]
    async fn reset_data(&self) -> Result<Json<DatasetProfile>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::ResetData { reply: reply_tx })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();
        let profile = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(Json(profile))
    }

    #[tool(description = "Get the current dataset profile as JSON, matching dataset://current/profile")]
    async fn get_dataset_profile(&self) -> Result<Json<DatasetProfile>, McpError> {
        let profile = self
            .dataset_profile
            .read()
            .map_err(|_| McpError::internal_error("Dataset profile lock is poisoned", None))?;

        Ok(Json(profile.clone()))
    }

    #[tool(description = "Return a map of dataset prefixes to their full IRIs")]
    async fn get_prefixes(&self) -> Result<Json<BTreeMap<String, String>>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::GetPrefixes { reply: reply_tx })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();

        let prefixes = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(Json(prefixes))
    }

    #[tool(description = "Save a PNG screenshot of the current application viewport to a local file path")]
    async fn take_screen_shot(
        &self,
        Parameters(parameters): Parameters<TakeScreenShotParams>,
    ) -> Result<Json<String>, McpError> {
        let (reply_tx, mut reply_rx) = oneshot::channel();
        self.ui_tx
            .send(UiCommand::TakeScreenShot {
                parameters,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;
        self.egui_ctx.request_repaint();

        let mut repaint_interval = tokio::time::interval(std::time::Duration::from_millis(16));
        repaint_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let response = loop {
            tokio::select! {
                response = &mut reply_rx => break response,
                _ = repaint_interval.tick() => self.egui_ctx.request_repaint(),
            }
        };
        let path = response
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;
        Ok(Json(path))
    }

    #[tool(description = "Show filtered instances of type in table. Only data properties filter are allowed")]
    async fn show_type_instances(
        &self,
        Parameters(args): Parameters<ShowTypeInstancesParams>,
    ) -> Result<Json<InstanceQueryResult>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        // Send command to egui thread
        self.ui_tx
            .send(UiCommand::ShowTypeInstances {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;

        // Wake up egui
        self.egui_ctx.request_repaint();

        // Wait asynchronously for UI response
        let result = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?;

        let result = result.map_err(|e| McpError::internal_error(e, None))?;

        Ok(Json(result))
    }

    #[tool(
        description = "Get a range of rows from the most recently shown type instances. The range is [from, to), limited to 100 rows. The IRI column is always included. Omit columns to return all properties, or pass an empty list to return only IRIs. Select properties by column name or full IRI."
    )]
    async fn get_type_instance_range(
        &self,
        Parameters(args): Parameters<GetTypeInstanceRangeParams>,
    ) -> Result<Json<InstanceQueryRangeResult>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.ui_tx
            .send(UiCommand::GetTypeInstanceRange {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;

        self.egui_ctx.request_repaint();

        let result = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?;
        let result = result.map_err(|error| McpError::internal_error(error, None))?;

        Ok(Json(result))
    }

    #[tool(description = "Show the RDF node with the given IRI in the browse view")]
    async fn show_node(&self, Parameters(args): Parameters<ShowNodeParams>) -> Result<Json<String>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.ui_tx
            .send(UiCommand::ShowNode {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;

        self.egui_ctx.request_repaint();

        let iri = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;

        Ok(Json(iri))
    }

    #[tool(description = "Full-text search RDF literals and show up to max_result matches in the browse view")]
    async fn search(&self, Parameters(args): Parameters<SearchParams>) -> Result<Json<Vec<SearchNodeResult>>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.ui_tx
            .send(UiCommand::Search {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;

        self.egui_ctx.request_repaint();

        let results = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;

        Ok(Json(results))
    }

    #[tool(
        description = "Get RDF types, properties and their value types, references, reverse references, and node flags for one or more node IRIs"
    )]
    async fn get_node_info(
        &self,
        Parameters(args): Parameters<GetNodeInfoParams>,
    ) -> Result<Json<Vec<NodeInfo>>, McpError> {
        let (reply_tx, reply_rx) = oneshot::channel();

        self.ui_tx
            .send(UiCommand::GetNodeInfo {
                parameters: args,
                reply: reply_tx,
            })
            .await
            .map_err(|_| McpError::internal_error("egui application is no longer running", None))?;

        self.egui_ctx.request_repaint();

        let nodes = reply_rx
            .await
            .map_err(|_| McpError::internal_error("egui did not respond", None))?
            .map_err(|error| McpError::internal_error(error, None))?;

        Ok(Json(nodes))
    }
}

#[tool_handler]
impl ServerHandler for MyMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_resources_list_changed()
                .build(),
        )
        .with_server_info(Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
        .with_instructions(include_str!("mcp-instructions.md"))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult::with_all_items(vec![
            Resource::new(DATASET_PROFILE_RESOURCE_URI, "dataset-profile")
                .with_title("Current dataset profile")
                .with_description("Mapped TypeInstanceIndex for the dataset currently loaded in RDFGlance")
                .with_mime_type("application/json"),
        ]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, McpError> {
        if request.uri != DATASET_PROFILE_RESOURCE_URI {
            return Err(McpError::invalid_params(
                format!("Unknown resource URI: {}", request.uri),
                None,
            ));
        }

        let profile = self
            .dataset_profile
            .read()
            .map_err(|_| McpError::internal_error("Dataset profile lock is poisoned", None))?;
        let json = serde_json::to_string_pretty(&*profile)
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        let contents = ResourceContents::text(json, DATASET_PROFILE_RESOURCE_URI).with_mime_type("application/json");
        Ok(ReadResourceResult::new(vec![contents]).into())
    }
}
