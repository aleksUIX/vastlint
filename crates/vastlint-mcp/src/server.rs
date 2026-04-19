use crate::tools::VastlintServer;
use rmcp::{transport::stdio, ServiceExt};

pub async fn run() {
    let server = VastlintServer;
    let service = server.serve(stdio()).await.expect("MCP server failed");
    service
        .waiting()
        .await
        .expect("MCP server exited with error");
}
