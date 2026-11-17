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
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

use crate::tool_api::{DATASET_PROFILE_RESOURCE_URI, DatasetProfile, Filter, InstanceQueryResult};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowTypeInstancesParams {
    pub type_name: String,
    pub filters: Vec<Filter>,
}

pub enum UiCommand {
    ShowTypeInstances {
        parameters: ShowTypeInstancesParams,
        reply: oneshot::Sender<Result<InstanceQueryResult, String>>,
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
    #[tool(description = "Get the current dataset profile as JSON, matching dataset://current/profile")]
    async fn get_dataset_profile(&self) -> Result<Json<DatasetProfile>, McpError> {
        let profile = self
            .dataset_profile
            .read()
            .map_err(|_| McpError::internal_error("Dataset profile lock is poisoned", None))?;

        Ok(Json(profile.clone()))
    }

    #[tool(description = "Show filtered instances of type in table")]
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
