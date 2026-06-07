use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::http::handlers;
use crate::store::db::Database;

/// Crea el router HTTP para la API REST.
pub fn create_router(
    db: Arc<Database>,
    embeddings: Option<Arc<crate::embeddings::engine::EmbeddingEngine>>,
) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/memories", get(handlers::list_memories))
        .route("/api/v1/memories", post(handlers::create_memory))
        .route("/api/v1/memories/search", post(handlers::search_memories))
        .route("/api/v1/memories/similar", post(handlers::similar_memories))
        .route("/api/v1/memories/:id", get(handlers::get_memory))
        .route("/api/v1/memories/:id", put(handlers::update_memory))
        .route("/api/v1/memories/:id", delete(handlers::delete_memory))
        .route(
            "/api/v1/memories/:id/relations",
            get(handlers::get_relations),
        )
        .route(
            "/api/v1/memories/:id/relations",
            post(handlers::create_relation),
        )
        .route(
            "/api/v1/relations/:relation_id",
            delete(handlers::delete_relation),
        )
        .route("/api/v1/stats", get(handlers::get_stats))
        .route("/api/v1/projects", get(handlers::list_projects))
        .route("/api/v1/sessions/start", post(handlers::start_session))
        .route("/api/v1/sessions/:id/end", post(handlers::end_session))
        .route("/api/v1/sessions/active", get(handlers::get_active_session))
        .route("/api/v1/context", get(handlers::get_context))
        .route("/api/v1/export", get(handlers::export_memories))
        .route("/api/v1/import", post(handlers::import_memories))
        .route("/api/v1/doctor", get(handlers::run_doctor))
        .route(
            "/api/v1/embeddings/reindex",
            post(handlers::reindex_embeddings),
        )
        .route("/api/v1/memories/batch", post(handlers::batch_save))
        .route("/api/v1/audit", get(handlers::get_audit))
        .route("/api/v1/deduplicate", post(handlers::deduplicate))
        .route("/api/v1/feedback", post(handlers::add_feedback))
        .route("/api/v1/deprecate", post(handlers::deprecate))
        .route("/api/v1/graph", get(handlers::get_graph))
        .route("/api/v1/summarize", get(handlers::get_summarize))
        .route("/api/v1/inject-context", get(handlers::inject_context))
        .route("/api/v1/forget-project", post(handlers::forget_project))
        .route("/api/v1/health", get(handlers::get_health))
        .route("/api/v1/remind", get(handlers::get_remind))
        .route("/api/v1/tag-suggest", post(handlers::tag_suggest))
        .route("/api/v1/knowledge-gaps", get(handlers::get_knowledge_gaps))
        .route(
            "/api/v1/memories/:id/encrypt",
            post(handlers::encrypt_memory),
        )
        .route(
            "/api/v1/memories/:id/decrypt",
            post(handlers::decrypt_memory),
        )
        .route("/api/v1/keys", get(handlers::list_keys))
        .route("/api/v1/keys", post(handlers::add_key))
        .route("/api/v1/keys/:id", delete(handlers::remove_key))
        .route("/api/v1/keys/status", get(handlers::keys_status))
        .route("/api/v1/sync/hello", post(handlers::sync_hello))
        .route("/api/v1/sync/pull", post(handlers::sync_pull))
        .route("/api/v1/sync/push", post(handlers::sync_push))
        .layer(Extension(embeddings))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(db)
}
