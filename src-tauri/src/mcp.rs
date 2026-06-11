use std::path::PathBuf;

use rmcp::{
    ServerHandler, ServiceExt,
    model::{Implementation, ServerCapabilities, ServerInfo},
};

pub struct QwertMcpServer {
    #[allow(dead_code)]
    vault_root: PathBuf,
}

impl QwertMcpServer {
    pub fn new(vault_root: PathBuf) -> Self {
        Self { vault_root }
    }
}

impl ServerHandler for QwertMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("qwert", env!("CARGO_PKG_VERSION")))
    }
}

pub async fn run_server(vault_root: PathBuf) -> i32 {
    let server = QwertMcpServer::new(vault_root);
    match server.serve(rmcp::transport::stdio()).await {
        Ok(service) => match service.waiting().await {
            Ok(_) => 0,
            Err(e) => {
                eprintln!("MCP service error: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("MCP server startup failed: {e}");
            1
        }
    }
}
