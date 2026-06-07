use std::sync::Arc;

use mneme::http::router::create_router;
use mneme::store::db::Database;
use uuid::Uuid;

fn setup_db() -> Database {
    let path = std::path::PathBuf::from(format!("/tmp/mneme_http_int_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

async fn spawn_server(db: Database) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
    let db = Arc::new(db);
    let router = create_router(db, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    // Brief wait for server to be ready
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    (addr, handle)
}

#[tokio::test]
async fn test_http_health_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_create_and_list_memory() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;
    let base = format!("http://{addr}");

    // Create a memory
    let create_resp = reqwest::Client::new()
        .post(format!("{base}/api/v1/memories"))
        .json(&serde_json::json!({
            "project": "http-test",
            "title": "HTTP Created",
            "content": "Via HTTP test",
            "memory_type": "note",
            "importance": "high"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), 200, "create should succeed");

    // List memories
    let list_resp = reqwest::get(format!("{base}/api/v1/memories?project=http-test"))
        .await
        .unwrap();
    assert_eq!(list_resp.status(), 200);
    let data: serde_json::Value = list_resp.json().await.unwrap();
    assert!(data.is_array());
    assert!(!data.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_http_create_memory_invalid_json() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/api/v1/memories"))
        .header("content-type", "application/json")
        .body("not valid json")
        .send()
        .await
        .unwrap();
    // Should return 400 or 422 for invalid JSON
    assert!(resp.status().is_client_error());
}

#[tokio::test]
async fn test_http_get_memory_not_found() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!(
        "http://{addr}/api/v1/memories/00000000-0000-0000-0000-000000000000"
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_http_stats_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/api/v1/stats?project=http-test"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_projects_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/api/v1/projects"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_graph_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/api/v1/graph?project=http-test"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_search_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::Client::new()
        .post(format!("http://{addr}/api/v1/memories/search"))
        .json(&serde_json::json!({
            "text": "test",
            "project": "http-test"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_health_v1_returns_200() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/api/v1/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_http_not_found_returns_404() {
    let db = setup_db();
    let (addr, _handle) = spawn_server(db).await;

    let resp = reqwest::get(format!("http://{addr}/api/v1/nonexistent"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
