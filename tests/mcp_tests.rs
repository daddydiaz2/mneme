use std::sync::Arc;

use mneme::config::settings::Settings;
use mneme::mcp::server::MnemeServer;
use mneme::store::db::Database;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParam, ClientCapabilities, Implementation, InitializeRequestParam,
    ProtocolVersion,
};
use rmcp::service::{AtomicU32RequestIdProvider, Peer, RequestContext};
use tokio_util::sync::CancellationToken;

use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Creates a test database in a temporary directory.
fn test_db() -> Arc<Database> {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("mneme_test_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");
    Arc::new(Database::open(&db_path).unwrap())
}

fn test_server() -> MnemeServer {
    let db = test_db();
    let config = Arc::new(Settings::default());
    MnemeServer::new(db, config, None)
}

fn test_context() -> RequestContext<rmcp::service::RoleServer> {
    let peer_info = InitializeRequestParam {
        protocol_version: ProtocolVersion::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation::default(),
    };
    RequestContext {
        ct: CancellationToken::new(),
        id: rmcp::model::RequestId::Number(1),
        peer: Peer::new(Arc::new(AtomicU32RequestIdProvider::default()), peer_info).0,
    }
}

#[tokio::test]
async fn test_mcp_mem_save_returns_valid_json() {
    let server = test_server();
    let mut args = serde_json::Map::new();
    args.insert("title".to_string(), serde_json::json!("Test Memory"));
    args.insert("content".to_string(), serde_json::json!("This is a test"));
    args.insert("project".to_string(), serde_json::json!("test-project"));

    let request = CallToolRequestParam {
        name: "mem_save".into(),
        arguments: Some(args),
    };

    let result = server.call_tool(request, test_context()).await.unwrap();
    assert!(!result.content.is_empty());

    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].is_object());
    assert_eq!(parsed["meta"]["version"], "0.1.0");
}

#[tokio::test]
async fn test_mcp_mem_search_returns_results() {
    let server = test_server();

    // First save a memory
    let mut save_args = serde_json::Map::new();
    save_args.insert("title".to_string(), serde_json::json!("Rust Memory"));
    save_args.insert("content".to_string(), serde_json::json!("Rust is great"));
    save_args.insert("project".to_string(), serde_json::json!("test-project"));

    let save_request = CallToolRequestParam {
        name: "mem_save".into(),
        arguments: Some(save_args),
    };

    server
        .call_tool(save_request, test_context())
        .await
        .unwrap();

    // Then search for it
    let mut search_args = serde_json::Map::new();
    search_args.insert("query".to_string(), serde_json::json!("Rust"));
    search_args.insert("project".to_string(), serde_json::json!("test-project"));

    let search_request = CallToolRequestParam {
        name: "mem_search".into(),
        arguments: Some(search_args),
    };

    let result = server
        .call_tool(search_request, test_context())
        .await
        .unwrap();
    assert!(!result.content.is_empty());

    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].is_array());
}

#[tokio::test]
async fn test_mcp_error_response_format() {
    let server = test_server();

    let mut args = serde_json::Map::new();
    args.insert(
        "id".to_string(),
        serde_json::json!("00000000-0000-0000-0000-000000000000"),
    );

    let request = CallToolRequestParam {
        name: "mem_get".into(),
        arguments: Some(args),
    };

    let result = server.call_tool(request, test_context()).await.unwrap();
    assert!(!result.content.is_empty());

    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"].is_object());
    assert!(parsed["error"]["code"].is_string());
    assert!(parsed["error"]["message"].is_string());
    assert!(parsed["meta"].is_object());
}
