use rmcp::{
    handler::server::wrapper::Parameters,
    schemars::JsonSchema,
    tool,
    tool_router,
    ServiceExt,
    transport::stdio,
    ErrorData as McpError,
};

use serde::Deserialize;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetTitleParams {
    title: String,
}

pub enum UiCommand {
    SetTitle {
        title: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

#[derive(Clone)]
pub struct MyMcpServer {
    pub ui_tx: mpsc::Sender<UiCommand>,
    pub egui_ctx: egui::Context,
}

#[tool_router(server_handler)]
impl MyMcpServer {

    #[tool(description = "Change the application window title")]
    async fn set_window_title(
        &self,
        Parameters(args): Parameters<SetTitleParams>,
    ) -> Result<String, McpError> {

        // let (reply_tx, reply_rx) = oneshot::channel();

        /*
        // Send command to egui thread
        self.ui_tx
            .send(UiCommand::SetTitle {
                title: args.title,
                reply: reply_tx,
            })
            .await
            .map_err(|_| {
                McpError::internal_error(
                    "egui application is no longer running",
                    None,
                )
            })?;

        // Wake up egui
        self.egui_ctx.request_repaint();

        // Wait asynchronously for UI response
        let result = reply_rx.await
            .map_err(|_| {
                McpError::internal_error(
                    "egui did not respond",
                    None,
                )
            })?;

        result.map_err(|e| {
            McpError::internal_error(e, None)
        })?;
         */

        Ok("Window title changed".to_string())
    }
}