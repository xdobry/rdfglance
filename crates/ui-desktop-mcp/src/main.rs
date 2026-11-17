#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;

use rmcp::ServiceExt;
use tokio::sync::mpsc;

use crate::mcp::{MyMcpServer, UiCommand};
use crate::mcpapp::RdfGlanceMcpApp;
use rmcp::transport::stdio;
use std::sync::{Arc, RwLock};

pub mod mcp;
pub mod mcpapp;
pub mod tool_api;

use crate::tool_api::DatasetProfile;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    use eframe::egui::ViewportBuilder;
    use rdf_glance::support::uitools::load_icon;

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_icon(load_icon()),
        ..eframe::NativeOptions::default()
    };
    let args = std::env::args().skip(1).collect();

    eframe::run_native(
        "rdf-glance",
        options,
        Box::new(|cc| {
            let (ui_rx, dataset_profile) = start_mcp(cc.egui_ctx.clone());
            Ok(Box::new(RdfGlanceMcpApp::new(cc.storage, args, ui_rx, dataset_profile)))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}

fn start_mcp(ctx: egui::Context) -> (mpsc::Receiver<UiCommand>, Arc<RwLock<DatasetProfile>>) {
    let (ui_tx, ui_rx) = mpsc::channel::<UiCommand>(64);
    let dataset_profile = Arc::new(RwLock::new(DatasetProfile::default()));
    let server_profile = Arc::clone(&dataset_profile);

    thread::spawn(move || {
        eprintln!("starting mcp");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Cannot create Tokio runtime");

        runtime.block_on(async move {
            let server = MyMcpServer::new(ui_tx, ctx, server_profile);

            let running = server.serve(stdio()).await.expect("Cannot start MCP server");

            running.waiting().await.expect("MCP server stopped unexpectedly");
        });
    });
    (ui_rx, dataset_profile)
}
