#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;

use clap::{Parser, ValueEnum};
use rdf_glance_desktop_mcp::{
    mcp::{MyMcpServer, UiCommand},
    mcpapp::RdfGlanceMcpApp,
    tool_api::DatasetProfile,
};
use rmcp::{
    ServiceExt,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use tokio::sync::mpsc;

use rmcp::transport::stdio;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum Transport {
    #[default]
    Stdio,
    StreamableHttp,
}

#[derive(Debug, Parser)]
#[command(name = "rdf-glance-mcp")]
struct Cli {
    #[arg(long, value_enum, default_value = "stdio")]
    transport: Transport,
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(value_name = "FILE")]
    file: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    use eframe::egui::ViewportBuilder;
    use rdf_glance::support::uitools::load_icon;
    use tracing_subscriber::EnvFilter;

    let cli = Cli::parse();
    let filter = match EnvFilter::try_from_default_env() {
        Ok(filter) => filter,
        Err(error) if std::env::var_os("RUST_LOG").is_some() => {
            eprintln!("Invalid RUST_LOG filter; using info: {error}");
            EnvFilter::new("info")
        }
        Err(_) => EnvFilter::new("info"),
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default().with_icon(load_icon()),
        ..eframe::NativeOptions::default()
    };
    let args = cli.file.into_iter().collect();
    let transport = cli.transport;
    let host = cli.host;
    let port = cli.port;

    eframe::run_native(
        "rdf-glance",
        options,
        Box::new(|cc| {
            let (ui_rx, dataset_profile) = start_mcp(cc.egui_ctx.clone(), transport, host, port);
            Ok(Box::new(RdfGlanceMcpApp::new(cc.storage, args, ui_rx, dataset_profile)))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {}

fn start_mcp(
    ctx: egui::Context,
    transport: Transport,
    host: String,
    port: u16,
) -> (mpsc::Receiver<UiCommand>, Arc<RwLock<DatasetProfile>>) {
    let (ui_tx, ui_rx) = mpsc::channel::<UiCommand>(64);
    let dataset_profile = Arc::new(RwLock::new(DatasetProfile::default()));
    let server_profile = Arc::clone(&dataset_profile);

    thread::spawn(move || {
        tracing::info!("starting mcp");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Cannot create Tokio runtime");

        runtime.block_on(async move {
            match transport {
                Transport::Stdio => {
                    let server = MyMcpServer::new(ui_tx, ctx, server_profile);
                    let running = server.serve(stdio()).await.expect("Cannot start MCP server");
                    running.waiting().await.expect("MCP server stopped unexpectedly");
                }
                Transport::StreamableHttp => {
                    let listener = match tokio::net::TcpListener::bind((host.as_str(), port)).await {
                        Ok(listener) => listener,
                        Err(error) => {
                            tracing::error!(%error, %host, port, "Cannot bind MCP HTTP server");
                            return;
                        }
                    };
                    let address = match listener.local_addr() {
                        Ok(address) => address,
                        Err(error) => {
                            tracing::error!(%error, "Cannot get MCP HTTP server address");
                            return;
                        }
                    };
                    let service: StreamableHttpService<MyMcpServer, LocalSessionManager> = StreamableHttpService::new(
                        {
                            let ui_tx = ui_tx.clone();
                            let ctx = ctx.clone();
                            let server_profile = Arc::clone(&server_profile);
                            move || {
                                Ok(MyMcpServer::new(
                                    ui_tx.clone(),
                                    ctx.clone(),
                                    Arc::clone(&server_profile),
                                ))
                            }
                        },
                        Default::default(),
                        StreamableHttpServerConfig::default().disable_allowed_hosts(),
                    );
                    let router = axum::Router::new().nest_service("/mcp", service);
                    tracing::info!(%address, "MCP Streamable HTTP server listening at /mcp");
                    if let Err(error) = axum::serve(listener, router).await {
                        tracing::error!(%error, "MCP HTTP server stopped unexpectedly");
                    }
                }
            }
        });
    });
    (ui_rx, dataset_profile)
}

#[cfg(test)]
mod tests {
    use super::{Cli, Transport};
    use clap::Parser;

    #[test]
    fn cli_defaults_to_stdio_and_keeps_optional_file_argument() {
        let cli = Cli::try_parse_from(["rdf-glance-mcp", "data.ttl"]).unwrap();

        assert_eq!(cli.transport, Transport::Stdio);
        assert_eq!(cli.host, "0.0.0.0");
        assert_eq!(cli.port, 8080);
        assert_eq!(cli.file.as_deref(), Some("data.ttl"));
    }

    #[test]
    fn cli_accepts_streamable_http_options() {
        let cli = Cli::try_parse_from([
            "rdf-glance-mcp",
            "--transport",
            "streamable-http",
            "--host",
            "127.0.0.1",
            "--port",
            "9000",
        ])
        .unwrap();

        assert_eq!(cli.transport, Transport::StreamableHttp);
        assert_eq!(cli.host, "127.0.0.1");
        assert_eq!(cli.port, 9000);
    }
}
