#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use rmcp::{transport::stdio, ServiceExt};
    flowchart_mcp_server::FlowchartServer::new()
        .serve(stdio())
        .await?
        .waiting()
        .await?;
    Ok(())
}
