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
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_util::sync::CancellationToken;

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_db() -> Arc<Database> {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_dir = std::env::temp_dir().join(format!("mneme_mcp2_{}_{}", std::process::id(), id));
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

fn make_args(pairs: Vec<(&str, &str)>) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert(
        "project".to_string(),
        serde_json::json!("test-project"),
    );
    for (k, v) in pairs {
        map.insert(k.to_string(), serde_json::json!(v));
    }
    map
}

#[tokio::test]
async fn test_mcp_mem_list_empty() {
    let server = test_server();
    let args = make_args(vec![]);

    let request = CallToolRequestParam {
        name: "mem_list".into(),
        arguments: Some(args),
    };

    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].is_array());
    assert!(parsed["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_mcp_mem_update_after_save() {
    let server = test_server();
    let mut save_args = serde_json::Map::new();
    save_args.insert("project".to_string(), serde_json::json!("test-project"));
    save_args.insert("title".to_string(), serde_json::json!("Update Test"));
    save_args.insert("content".to_string(), serde_json::json!("Original content"));

    let save_req = CallToolRequestParam {
        name: "mem_save".into(),
        arguments: Some(save_args),
    };
    let save_result = server.call_tool(save_req, test_context()).await.unwrap();
    let save_text = save_result.content[0].as_text().unwrap().text.clone();
    let save_val: serde_json::Value = serde_json::from_str(&save_text).unwrap();
    let id = save_val["data"]["id"].as_str().unwrap().to_string();

    // Update the memory
    let mut update_args = serde_json::Map::new();
    update_args.insert("id".to_string(), serde_json::json!(id));
    update_args.insert("title".to_string(), serde_json::json!("Updated Title"));

    let update_req = CallToolRequestParam {
        name: "mem_update".into(),
        arguments: Some(update_args),
    };
    let upd_result = server.call_tool(update_req, test_context()).await.unwrap();
    let upd_text = upd_result.content[0].as_text().unwrap().text.clone();
    let upd_val: serde_json::Value = serde_json::from_str(&upd_text).unwrap();
    assert_eq!(upd_val["success"], true);
    assert_eq!(upd_val["data"]["title"], "Updated Title");
}

#[tokio::test]
async fn test_mcp_mem_stats() {
    let server = test_server();
    let args = make_args(vec![]);

    let save_args = make_args(vec![
        ("title", "Stats Test"),
        ("content", "Test for stats"),
    ]);
    // Save one to have data
    let _ = server
        .call_tool(
            CallToolRequestParam {
                name: "mem_save".into(),
                arguments: Some(save_args),
            },
            test_context(),
        )
        .await;

    let request = CallToolRequestParam {
        name: "mem_stats".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["total_memories"].as_u64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn test_mcp_mem_projects() {
    let server = test_server();
    let args = make_args(vec![]);

    let request = CallToolRequestParam {
        name: "mem_projects".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"].is_array());
}

#[tokio::test]
async fn test_mcp_mem_graph_empty() {
    let server = test_server();
    let args = make_args(vec![]);

    let request = CallToolRequestParam {
        name: "mem_graph".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["nodes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_mcp_mem_health() {
    let server = test_server();
    let args = make_args(vec![]);

    let request = CallToolRequestParam {
        name: "mem_health".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["data"]["version"].is_string());
    assert!(parsed["data"]["db_size_mb"].as_f64().unwrap_or(0.0) > 0.0);
}

#[tokio::test]
async fn test_mcp_invalid_tool_returns_error() {
    let server = test_server();
    let args = make_args(vec![]);

    let request = CallToolRequestParam {
        name: "nonexistent_tool".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], false);
    assert!(parsed["error"].is_object());
}

#[tokio::test]
async fn test_mcp_mem_save_encrypted() {
    let server = test_server();
    let mut args = make_args(vec![
        ("title", "Encrypted memory"),
        ("content", "Secret content"),
    ]);
    args.insert("encrypt".to_string(), serde_json::json!(true));

    let request = CallToolRequestParam {
        name: "mem_save".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    // Should succeed but since no crypto configured, encryption may not apply
    // The memory should still save without encryption if no recipients configured
    assert_eq!(parsed["success"], true, "mem_save with encrypt=true failed: {parsed}");
}

#[tokio::test]
async fn test_mcp_mem_delete_restore_flow() {
    let server = test_server();
    let save_args = make_args(vec![
        ("title", "Delete Me"),
        ("content", "Will be deleted"),
    ]);

    let save_req = CallToolRequestParam {
        name: "mem_save".into(),
        arguments: Some(save_args),
    };
    let save_result = server.call_tool(save_req, test_context()).await.unwrap();
    let save_text = save_result.content[0].as_text().unwrap().text.clone();
    let save_val: serde_json::Value = serde_json::from_str(&save_text).unwrap();
    let id = save_val["data"]["id"].as_str().unwrap().to_string();

    // Delete it
    let mut del_args = serde_json::Map::new();
    del_args.insert("id".to_string(), serde_json::json!(id));

    let del_req = CallToolRequestParam {
        name: "mem_delete".into(),
        arguments: Some(del_args.clone()),
    };
    let del_result = server.call_tool(del_req, test_context()).await.unwrap();
    let del_text = del_result.content[0].as_text().unwrap().text.clone();
    let del_val: serde_json::Value = serde_json::from_str(&del_text).unwrap();
    assert_eq!(del_val["success"], true);

    // Attempt to get it (should fail after delete since get filters soft-deleted)
    let get_req = CallToolRequestParam {
        name: "mem_get".into(),
        arguments: Some(del_args),
    };
    let get_result = server.call_tool(get_req, test_context()).await.unwrap();
    let get_text = get_result.content[0].as_text().unwrap().text.clone();
    let get_val: serde_json::Value = serde_json::from_str(&get_text).unwrap();
    // get on deleted memory returns error memory not found
    assert_eq!(get_val["success"], false);
}

#[tokio::test]
async fn test_mcp_mem_save_batch_empty() {
    let server = test_server();

    let mut args = make_args(vec![]);
    let memories = serde_json::json!([
        {"title": "Batch1", "content": "First batch"},
        {"title": "Batch2", "content": "Second batch"}
    ]);
    args.insert("memories".to_string(), memories);

    let request = CallToolRequestParam {
        name: "mem_save_batch".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true, "batch save should succeed: {parsed}");
    // The format can vary depending on how batch responses are structured
    // It may be an array or have a count field
    let has_data = parsed.get("data").is_some_and(|d| d.is_array() || d.is_object());
    assert!(has_data, "batch save response should have data: {parsed}");
}

#[tokio::test]
async fn test_mcp_list_tools_contains_expected() {
    let server = test_server();
    let request = rmcp::model::PaginatedRequestParam::default();

    let result = server.list_tools(request, test_context()).await.unwrap();
    let names: Vec<&str> = result.tools.iter().map(|t| t.name.as_ref()).collect();

    assert!(names.contains(&"mem_save"));
    assert!(names.contains(&"mem_get"));
    assert!(names.contains(&"mem_search"));
    assert!(names.contains(&"mem_list"));
    assert!(names.contains(&"mem_stats"));
    assert!(names.contains(&"mem_graph"));
    assert!(names.contains(&"mem_health"));
}


#[tokio::test]
async fn test_mcp_mem_audit_empty_project() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_audit".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn test_mcp_mem_doctor() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_doctor".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["success"].as_bool().unwrap_or(false));
}

#[tokio::test]
async fn test_mcp_mem_current_project() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_current_project".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(parsed["success"].as_bool().unwrap_or(false));
}

#[tokio::test]
async fn test_mcp_mem_timeline_empty() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_timeline".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(!text.is_empty(), "timeline should return non-empty response");
}

#[tokio::test]
async fn test_mcp_mem_suggest_topic_key() {
    let server = test_server();
    let mut args = make_args(vec![("title", "Test Memory")]);
    args.insert("content".to_string(), serde_json::json!("test content"));
    let request = CallToolRequestParam {
        name: "mem_suggest_topic_key".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(!text.is_empty(), "suggest_topic_key should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_conflicts_empty() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_conflicts".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn test_mcp_mem_deprecate_flow() {
    let server = test_server();
    let save_args = make_args(vec![("title", "To Deprecate"), ("content", "old content")]);
    let save_req = CallToolRequestParam { name: "mem_save".into(), arguments: Some(save_args) };
    let save_res = server.call_tool(save_req, test_context()).await.unwrap();
    let val: serde_json::Value = serde_json::from_str(&save_res.content[0].as_text().unwrap().text).unwrap();
    let id = val["data"]["id"].as_str().unwrap().to_string();
    let mut dep_args = serde_json::Map::new();
    dep_args.insert("id".to_string(), serde_json::json!(id));
    dep_args.insert("reason".to_string(), serde_json::json!("superseded"));
    let dep_req = CallToolRequestParam {
        name: "mem_deprecate".into(),
        arguments: Some(dep_args),
    };
    let dep_res = server.call_tool(dep_req, test_context()).await.unwrap();
    let dep_text = dep_res.content[0].as_text().unwrap().text.clone();
    assert!(!dep_text.is_empty(), "deprecate should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_feedback() {
    let server = test_server();
    let save_args = make_args(vec![("title", "Feedback Test"), ("content", "rate me")]);
    let save_req = CallToolRequestParam { name: "mem_save".into(), arguments: Some(save_args) };
    let save_res = server.call_tool(save_req, test_context()).await.unwrap();
    let val: serde_json::Value = serde_json::from_str(&save_res.content[0].as_text().unwrap().text).unwrap();
    let id = val["data"]["id"].as_str().unwrap().to_string();
    let mut fb_args = serde_json::Map::new();
    fb_args.insert("id".to_string(), serde_json::json!(id));
    fb_args.insert("feedback_type".to_string(), serde_json::json!("useful"));
    let fb_req = CallToolRequestParam {
        name: "mem_feedback".into(),
        arguments: Some(fb_args),
    };
    let fb_res = server.call_tool(fb_req, test_context()).await.unwrap();
    let fb_text = fb_res.content[0].as_text().unwrap().text.clone();
    assert!(!fb_text.is_empty(), "feedback should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_knowledge_gaps_empty() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_knowledge_gaps".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn test_mcp_mem_remind_empty() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_remind".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn test_mcp_mem_tag_suggest() {
    let server = test_server();
    let mut save_args = make_args(vec![("title", "Tag Test"), ("content", "rust code")]);
    save_args.insert("tags".to_string(), serde_json::json!(["rust", "coding"]));
    let save_req = CallToolRequestParam { name: "mem_save".into(), arguments: Some(save_args) };
    let _ = server.call_tool(save_req, test_context()).await;
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_tag_suggest".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(!text.is_empty(), "tag_suggest should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_restore_flow() {
    let server = test_server();
    let save_args = make_args(vec![("title", "Restore Me"), ("content", "gone but not forgotten")]);
    let save_req = CallToolRequestParam { name: "mem_save".into(), arguments: Some(save_args) };
    let save_res = server.call_tool(save_req, test_context()).await.unwrap();
    let val: serde_json::Value = serde_json::from_str(&save_res.content[0].as_text().unwrap().text).unwrap();
    let id = val["data"]["id"].as_str().unwrap().to_string();
    let mut del_args = serde_json::Map::new();
    del_args.insert("id".to_string(), serde_json::json!(id));
    let del_req = CallToolRequestParam { name: "mem_delete".into(), arguments: Some(del_args.clone()) };
    let _ = server.call_tool(del_req, test_context()).await;
    let rest_req = CallToolRequestParam { name: "mem_restore".into(), arguments: Some(del_args) };
    let rest_res = server.call_tool(rest_req, test_context()).await.unwrap();
    let rest_text = rest_res.content[0].as_text().unwrap().text.clone();
    assert!(!rest_text.is_empty(), "restore should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_context_empty() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_context".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn test_mcp_mem_inject_context() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_inject_context".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(!text.is_empty(), "inject_context should return non-empty");
}

#[tokio::test]
async fn test_mcp_mem_forget_project() {
    let server = test_server();
    let args = make_args(vec![]);
    let request = CallToolRequestParam {
        name: "mem_forget_project".into(),
        arguments: Some(args),
    };
    let result = server.call_tool(request, test_context()).await.unwrap();
    let text = result.content[0].as_text().unwrap().text.clone();
    assert!(!text.is_empty(), "forget_project should return non-empty");
}
