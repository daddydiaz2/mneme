use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParam, CallToolResult, Implementation, ListToolsResult, PaginatedRequestParam,
    ServerCapabilities, ServerInfo,
};
use rmcp::service::{RequestContext, RoleServer};
use tokio::sync::RwLock;

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::memory::Session;

/// Servidor MCP para Mneme.
#[derive(Debug, Clone)]
pub struct MnemeServer {
    db: Arc<Database>,
    #[allow(dead_code)]
    config: Arc<Settings>,
    current_project: Arc<RwLock<String>>,
    #[allow(dead_code)]
    current_session: Arc<RwLock<Option<Session>>>,
    embeddings: Option<Arc<crate::embeddings::engine::EmbeddingEngine>>,
    plugins: Arc<crate::plugins::PluginManager>,
}

impl MnemeServer {
    /// Crea un nuevo MnemeServer.
    pub fn new(
        db: Arc<Database>,
        config: Arc<Settings>,
        embeddings: Option<Arc<crate::embeddings::engine::EmbeddingEngine>>,
    ) -> Self {
        let project = config.mcp.default_project.clone();
        Self {
            db,
            config,
            current_project: Arc::new(RwLock::new(project)),
            current_session: Arc::new(RwLock::new(None)),
            embeddings,
            plugins: Arc::new(
                crate::plugins::PluginManager::load_from_default_dir().unwrap_or_else(|e| {
                    tracing::warn!(error = %e, "plugin loading failed, continuing without plugins");
                    crate::plugins::PluginManager::empty()
                }),
            ),
        }
    }

    /// Ejecuta el servidor MCP sobre stdio.
    pub async fn run_stdio(self) -> crate::error::Result<()> {
        let (stdin, stdout) = rmcp::transport::io::stdio();
        let transport = (stdin, stdout);
        rmcp::service::serve_server(self, transport)
            .await
            .map_err(|e| crate::error::MnemeError::Mcp(e.to_string()))?;
        Ok(())
    }

    async fn current_project(&self) -> String {
        self.current_project.read().await.clone()
    }
}

impl ServerHandler for MnemeServer {
    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::Error> {
        let project = self.current_project().await;
        Ok(crate::mcp::tools::execute_tool(
            &self.db,
            &request.name,
            request.arguments,
            &project,
            self.embeddings.as_ref(),
            Some(&self.plugins),
        )
        .await)
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::Error> {
        Ok(ListToolsResult {
            next_cursor: None,
            tools: crate::mcp::tools::list_tools(Some(&self.plugins)),
        })
    }

    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mneme".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Mneme MCP server — persistent memory for AI agents".to_string()),
        }
    }
}
