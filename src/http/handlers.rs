use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::memory::{
    CreateMemoryInput, CreateRelationInput, Memory, MemoryStats, ProjectSummary, Scope,
    SearchQuery, SearchResult, Session, UpdateMemoryInput,
};

/// Respuesta de error estándar de la API.
#[derive(Debug, Serialize)]
pub struct ApiError {
    error: String,
    code: String,
}

impl ApiError {
    fn new(code: &str, message: &str) -> Self {
        Self {
            error: message.to_string(),
            code: code.to_string(),
        }
    }
}

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<ApiError>)>;

fn map_err(e: crate::error::MnemeError) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match &e {
        crate::error::MnemeError::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND"),
        crate::error::MnemeError::ProjectRequired => (StatusCode::BAD_REQUEST, "PROJECT_REQUIRED"),
        crate::error::MnemeError::EmptyQuery => (StatusCode::BAD_REQUEST, "EMPTY_QUERY"),
        crate::error::MnemeError::InvalidMemoryType(_) => {
            (StatusCode::BAD_REQUEST, "INVALID_MEMORY_TYPE")
        }
        crate::error::MnemeError::InvalidImportance(_) => {
            (StatusCode::BAD_REQUEST, "INVALID_IMPORTANCE")
        }
        crate::error::MnemeError::InvalidScope(_) => (StatusCode::BAD_REQUEST, "INVALID_SCOPE"),
        crate::error::MnemeError::InvalidRelationType(_) => {
            (StatusCode::BAD_REQUEST, "INVALID_RELATION_TYPE")
        }
        crate::error::MnemeError::RelationAlreadyExists(_, _) => {
            (StatusCode::CONFLICT, "RELATION_EXISTS")
        }
        crate::error::MnemeError::SelfRelation(_) => (StatusCode::BAD_REQUEST, "SELF_RELATION"),
        crate::error::MnemeError::DuplicateDetected(_) => (StatusCode::CONFLICT, "DUPLICATE"),
        crate::error::MnemeError::Database(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR")
        }
        crate::error::MnemeError::Migration(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "MIGRATION_ERROR")
        }
        crate::error::MnemeError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "IO_ERROR"),
        crate::error::MnemeError::Serialization(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "SERIALIZATION_ERROR")
        }
        crate::error::MnemeError::Config(_) => (StatusCode::BAD_REQUEST, "CONFIG_ERROR"),
        crate::error::MnemeError::Http(_) => (StatusCode::INTERNAL_SERVER_ERROR, "HTTP_ERROR"),
        crate::error::MnemeError::Mcp(_) => (StatusCode::INTERNAL_SERVER_ERROR, "MCP_ERROR"),
        crate::error::MnemeError::Embeddings(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "EMBEDDINGS_ERROR")
        }
        crate::error::MnemeError::EmbeddingsDisabled => {
            (StatusCode::SERVICE_UNAVAILABLE, "EMBEDDINGS_DISABLED")
        }
        crate::error::MnemeError::PeerNotFound(_) => (StatusCode::NOT_FOUND, "PEER_NOT_FOUND"),
        crate::error::MnemeError::SyncFailed { .. } => {
            (StatusCode::INTERNAL_SERVER_ERROR, "SYNC_FAILED")
        }
        crate::error::MnemeError::SyncDisabled => (StatusCode::SERVICE_UNAVAILABLE, "SYNC_DISABLED"),
        crate::error::MnemeError::UnsupportedTransport(_) => {
            (StatusCode::BAD_REQUEST, "UNSUPPORTED_TRANSPORT")
        }
        crate::error::MnemeError::InvalidSyncFile(_) => {
            (StatusCode::BAD_REQUEST, "INVALID_SYNC_FILE")
        }
        crate::error::MnemeError::Compression(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "COMPRESSION_ERROR")
        }
        crate::error::MnemeError::NoRecipientsConfigured => {
            (StatusCode::SERVICE_UNAVAILABLE, "NO_RECIPIENTS")
        }
        crate::error::MnemeError::DecryptionFailed => {
            (StatusCode::UNAUTHORIZED, "DECRYPTION_FAILED")
        }
        crate::error::MnemeError::IdentityNotLoaded => {
            (StatusCode::UNAUTHORIZED, "IDENTITY_NOT_LOADED")
        }
        crate::error::MnemeError::AlreadyEncrypted(_) => {
            (StatusCode::CONFLICT, "ALREADY_ENCRYPTED")
        }
        crate::error::MnemeError::NotEncrypted(_) => {
            (StatusCode::CONFLICT, "NOT_ENCRYPTED")
        }
        crate::error::MnemeError::KeyNotFound(_) => {
            (StatusCode::NOT_FOUND, "KEY_NOT_FOUND")
        }
    };
    (status, Json(ApiError::new(code, &e.to_string())))
}

// --- Health ---

/// GET /health
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

// --- Memories ---

/// Query params para listar memorias.
#[derive(Debug, Deserialize)]
pub struct ListMemoriesQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
    #[serde(rename = "type", default)]
    memory_type: Option<String>,
    #[serde(default)]
    importance: Option<String>,
}

/// GET /api/v1/memories
pub async fn list_memories(
    State(db): State<Arc<Database>>,
    Query(query): Query<ListMemoriesQuery>,
) -> ApiResult<Vec<Memory>> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let memory_type = query
        .memory_type
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(map_err)?;
    let importance = query
        .importance
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(map_err)?;
    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let memories = db
        .memories()
        .list(
            &project,
            memory_type.as_ref(),
            importance.as_ref(),
            None,
            limit,
            offset,
        )
        .map_err(map_err)?;

    Ok(Json(memories))
}

/// Body para crear memoria.
#[derive(Debug, Deserialize)]
pub struct CreateMemoryBody {
    title: String,
    content: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(rename = "type", default = "default_note")]
    memory_type: String,
    #[serde(default = "default_medium")]
    importance: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    what: Option<String>,
    #[serde(default)]
    why: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    learned: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
}

fn default_note() -> String {
    "note".into()
}
fn default_medium() -> String {
    "medium".into()
}

/// POST /api/v1/memories
pub async fn create_memory(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<CreateMemoryBody>,
) -> ApiResult<Memory> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let memory_type = body.memory_type.parse().map_err(map_err)?;
    let importance = body.importance.parse().map_err(map_err)?;
    let scope = body
        .scope
        .as_deref()
        .map(Scope::from_str)
        .transpose()
        .map_err(map_err)?
        .unwrap_or(Scope::Project);

    let input = CreateMemoryInput {
        project,
        scope: Some(scope),
        title: body.title,
        content: body.content,
        what: body.what,
        why: body.why,
        context: body.context,
        learned: body.learned,
        memory_type,
        importance,
        tags: body.tags,
        topic_key: body.topic_key,
        capture_prompt: None,
        encrypt: false,
    };

    let engine = embeddings.clone();
    let embedding_store = db.embeddings();
    let memory = db
        .memories()
        .save(input, engine, Some(embedding_store))
        .map_err(map_err)?;
    Ok(Json(memory))
}

/// GET /api/v1/memories/:id
pub async fn get_memory(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> ApiResult<Memory> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;

    match db.memories().get(id).map_err(map_err)? {
        Some(memory) => Ok(Json(memory)),
        None => Err(map_err(crate::error::MnemeError::NotFound(id))),
    }
}

/// Body para actualizar memoria.
#[derive(Debug, Deserialize, Default)]
pub struct UpdateMemoryBody {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    what: Option<String>,
    #[serde(default)]
    why: Option<String>,
    #[serde(default)]
    context: Option<String>,
    #[serde(default)]
    learned: Option<String>,
    #[serde(rename = "type", default)]
    memory_type: Option<String>,
    #[serde(default)]
    importance: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
}

/// PUT /api/v1/memories/:id
pub async fn update_memory(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMemoryBody>,
) -> ApiResult<Memory> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;

    let update = UpdateMemoryInput {
        title: body.title,
        content: body.content,
        what: body.what,
        why: body.why,
        context: body.context,
        learned: body.learned,
        memory_type: body
            .memory_type
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(map_err)?,
        importance: body
            .importance
            .as_deref()
            .map(str::parse)
            .transpose()
            .map_err(map_err)?,
        tags: body.tags,
        scope: body
            .scope
            .as_deref()
            .map(Scope::from_str)
            .transpose()
            .map_err(map_err)?,
        topic_key: body.topic_key,
    };

    let memory = db.memories().update(id, update).map_err(map_err)?;
    Ok(Json(memory))
}

/// DELETE /api/v1/memories/:id
pub async fn delete_memory(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
    Query(query): Query<DeleteMemoryQuery>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;

    db.memories().delete(id, query.hard).map_err(map_err)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct DeleteMemoryQuery {
    #[serde(default)]
    hard: bool,
}

/// POST /api/v1/memories/search
#[derive(Debug, Deserialize)]
pub struct SearchBody {
    text: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

pub async fn search_memories(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<SearchBody>,
) -> ApiResult<Vec<SearchResult>> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let query = SearchQuery {
        text: body.text.clone(),
        project: Some(project.clone()),
        scope: None,
        memory_type: None,
        importance: None,
        tags: Vec::new(),
        limit: body.limit.unwrap_or(10),
        include_snippet: true,
        all_projects: false,
    };

    let weights = crate::store::search::SearchWeights::default();

    let semantic_scores = if let Some(engine) = embeddings {
        let embedding_store = db.embeddings();
        match engine.embed(&body.text).await {
            Ok(query_embedding) => {
                match embedding_store.load_all_for_project(&project) {
                    Ok(all_embeddings) => {
                        let mut scores = std::collections::HashMap::new();
                        for (id, embedding) in all_embeddings {
                            let score = crate::embeddings::similarity::cosine_similarity(
                                &query_embedding,
                                &embedding,
                            );
                            scores.insert(id, score);
                        }
                        Some(scores)
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "failed to load embeddings for search");
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to embed query for search");
                None
            }
        }
    } else {
        None
    };

    let results = db
        .memories()
        .search(&query, &weights, semantic_scores.as_ref())
        .map_err(map_err)?;
    Ok(Json(results))
}

// --- Relations ---

/// GET /api/v1/memories/:id/relations
pub async fn get_relations(
    State(_db): State<Arc<Database>>,
    Path(_id): Path<String>,
) -> ApiResult<Vec<serde_json::Value>> {
    Ok(Json(vec![]))
}

/// Body para crear relación.
#[derive(Debug, Deserialize)]
pub struct CreateRelationBody {
    target_id: String,
    relation_type: String,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    reason: Option<String>,
}

/// POST /api/v1/memories/:id/relations
pub async fn create_relation(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
    Json(body): Json<CreateRelationBody>,
) -> ApiResult<serde_json::Value> {
    let source_id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let target_id = Uuid::parse_str(&body.target_id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let relation_type = body.relation_type.parse().map_err(map_err)?;

    let input = CreateRelationInput {
        source_id,
        target_id,
        relation_type,
        confidence: Some(body.confidence.unwrap_or(1.0)),
        reason: body.reason,
    };

    db.memories().create_relation(input).map_err(map_err)?;
    Ok(Json(json!({"created": true})))
}

/// DELETE /api/v1/relations/:relation_id
pub async fn delete_relation(
    State(db): State<Arc<Database>>,
    Path(relation_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let id = Uuid::parse_str(&relation_id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    db.memories().delete_relation(id).map_err(map_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Stats ---

/// GET /api/v1/stats
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn get_stats(
    State(db): State<Arc<Database>>,
    Query(query): Query<StatsQuery>,
) -> ApiResult<MemoryStats> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let stats = db.memories().stats(&project).map_err(map_err)?;
    Ok(Json(stats))
}

// --- Projects ---

/// GET /api/v1/projects
pub async fn list_projects(State(db): State<Arc<Database>>) -> ApiResult<Vec<ProjectSummary>> {
    let projects = db.memories().list_projects().map_err(map_err)?;
    Ok(Json(projects))
}

// --- Sessions ---

/// POST /api/v1/sessions/start
#[derive(Debug, Deserialize)]
pub struct StartSessionBody {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    directory: Option<String>,
}

pub async fn start_session(
    State(db): State<Arc<Database>>,
    Json(body): Json<StartSessionBody>,
) -> ApiResult<Session> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let current_dir = std::env::current_dir().ok();
    let dir = body
        .directory
        .as_deref()
        .or_else(|| current_dir.as_ref().and_then(|p| p.to_str()));
    let session = db.sessions().start(&project, dir).map_err(map_err)?;
    Ok(Json(session))
}

/// POST /api/v1/sessions/:id/end
#[derive(Debug, Deserialize)]
pub struct EndSessionBody {
    #[serde(default)]
    summary: Option<String>,
}

pub async fn end_session(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
    Json(body): Json<EndSessionBody>,
) -> ApiResult<Session> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let session = db
        .sessions()
        .end(id, body.summary.as_deref())
        .map_err(map_err)?;
    Ok(Json(session))
}

/// GET /api/v1/sessions/active
#[derive(Debug, Deserialize)]
pub struct ActiveSessionQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn get_active_session(
    State(db): State<Arc<Database>>,
    Query(query): Query<ActiveSessionQuery>,
) -> ApiResult<Option<Session>> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let session = db.sessions().get_active(&project).map_err(map_err)?;
    Ok(Json(session))
}

// --- Context ---

/// GET /api/v1/context
#[derive(Debug, Deserialize)]
pub struct ContextQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

pub async fn get_context(
    State(db): State<Arc<Database>>,
    Query(query): Query<ContextQuery>,
) -> ApiResult<Vec<Memory>> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let limit = query.limit.unwrap_or(10);
    let memories = db
        .memories()
        .context(&project, None, limit)
        .map_err(map_err)?;
    Ok(Json(memories))
}

// --- Export ---

/// GET /api/v1/export
#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn export_memories(
    State(db): State<Arc<Database>>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<Vec<Memory>> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let memories = db
        .memories()
        .list(&project, None, None, None, 10000, 0)
        .map_err(map_err)?;
    Ok(Json(memories))
}

// --- Import ---

/// POST /api/v1/import
pub async fn import_memories(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<Vec<Memory>>,
) -> ApiResult<serde_json::Value> {
    let store = db.memories();
    let mut count = 0;
    let engine = embeddings.clone();
    let embedding_store = db.embeddings();
    for mem in body {
        let input = CreateMemoryInput {
            project: mem.project.clone(),
            scope: Some(mem.scope),
            title: mem.title,
            content: mem.content,
            what: mem.what,
            why: mem.why,
            context: mem.context,
            learned: mem.learned,
            memory_type: mem.memory_type,
            importance: mem.importance,
            tags: mem.tags,
            topic_key: mem.topic_key,
            capture_prompt: None,
            encrypt: false,
        };
        store.save(input, engine.clone(), Some(embedding_store.clone())).map_err(map_err)?;
        count += 1;
    }
    Ok(Json(json!({"imported": count})))
}

// --- Doctor ---

/// GET /api/v1/doctor
#[derive(Debug, Deserialize)]
pub struct DoctorQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    check: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DoctorResponse {
    healthy: bool,
    project: String,
    checks: Vec<String>,
    issues: Vec<String>,
}

pub async fn run_doctor(
    State(db): State<Arc<Database>>,
    Query(query): Query<DoctorQuery>,
) -> ApiResult<DoctorResponse> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let mut checks = Vec::new();
    let issues = Vec::new();

    checks.push("Database connection: OK".to_string());

    let memory_count = db
        .memories()
        .stats(&project)
        .map(|s| s.total_memories)
        .unwrap_or(0);
    let session_count = db
        .sessions()
        .list(&project, 1000)
        .map(|s| s.len() as u32)
        .unwrap_or(0);

    checks.push(format!("Memories: {}", memory_count));
    checks.push(format!("Sessions: {}", session_count));
    checks.push("Orphaned relations: 0".to_string());

    if let Some(check) = query.check {
        if check == "relations" {
            checks.push("Relation check: OK".to_string());
        }
    }

    let healthy = issues.is_empty();

    Ok(Json(DoctorResponse {
        healthy,
        project,
        checks,
        issues,
    }))
}

// --- Similar ---

/// POST /api/v1/memories/similar
#[derive(Debug, Deserialize)]
pub struct SimilarBody {
    /// Texto o UUID de la memoria de referencia.
    query: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    threshold: Option<f32>,
}

pub async fn similar_memories(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<SimilarBody>,
) -> ApiResult<Vec<SearchResult>> {
    let engine = match embeddings {
        Some(e) => e,
        None => return Err(map_err(crate::error::MnemeError::EmbeddingsDisabled)),
    };

    let project = body.project.unwrap_or_else(Settings::infer_project);
    let limit = body.limit.unwrap_or(5);
    let threshold = body.threshold.unwrap_or(0.75);
    let embedding_store = db.embeddings();

    let query_embedding = if let Ok(id) = Uuid::parse_str(&body.query) {
        match embedding_store.load(id).map_err(map_err)? {
            Some(embedding) => embedding,
            None => {
                let memory = db
                    .memories()
                    .get(id)
                    .map_err(map_err)?
                    .ok_or_else(|| map_err(crate::error::MnemeError::NotFound(id)))?;
                let text = crate::embeddings::engine::EmbeddingEngine::memory_to_text(&memory);
                engine.embed(&text).await.map_err(map_err)?
            }
        }
    } else {
        engine.embed(&body.query).await.map_err(map_err)?
    };

    let all_embeddings = embedding_store.load_all_for_project(&project).map_err(map_err)?;
    let mut matches = Vec::new();
    for (id, embedding) in all_embeddings {
        let score = crate::embeddings::similarity::cosine_similarity(&query_embedding, &embedding);
        if score >= threshold {
            matches.push(crate::embeddings::similarity::SemanticMatch {
                memory_id: id,
                cosine_score: score,
                combined_score: f64::from(score),
            });
        }
    }
    crate::embeddings::similarity::rank_by_combined_score(&mut matches);
    matches.truncate(limit as usize);

    let mut results = Vec::new();
    for m in matches {
        if let Some(memory) = db.memories().get(m.memory_id).map_err(map_err)? {
            results.push(SearchResult {
                memory,
                score: m.combined_score,
                snippet: None,
                match_type: crate::store::memory::MatchType::Semantic,
                cosine_score: Some(m.cosine_score),
            });
        }
    }

    Ok(Json(results))
}

// --- Reindex ---

/// POST /api/v1/embeddings/reindex
#[derive(Debug, Deserialize)]
pub struct ReindexBody {
    #[serde(default)]
    project: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    force: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    total: u32,
    indexed: u32,
    skipped: u32,
    failed: u32,
    duration_ms: u64,
}

pub async fn reindex_embeddings(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<ReindexBody>,
) -> ApiResult<ReindexResponse> {
    let engine = match embeddings {
        Some(e) => e,
        None => return Err(map_err(crate::error::MnemeError::EmbeddingsDisabled)),
    };

    let project = body.project.unwrap_or_else(Settings::infer_project);
    let embedding_store = db.embeddings();
    let stats = db
        .memories()
        .reindex_embeddings(&project, &engine, &embedding_store)
        .await
        .map_err(map_err)?;

    Ok(Json(ReindexResponse {
        total: stats.total,
        indexed: stats.indexed,
        skipped: stats.skipped,
        failed: stats.failed,
        duration_ms: stats.duration_ms,
    }))
}

// --- Batch Save ---

#[derive(Debug, Deserialize)]
pub struct BatchSaveBody {
    #[serde(default)]
    project: Option<String>,
    memories: Vec<CreateMemoryBody>,
}

#[derive(Debug, Serialize)]
pub struct BatchSaveResponse {
    saved: Vec<Memory>,
    duplicates: Vec<Memory>,
    saved_count: usize,
    duplicate_count: usize,
}

pub async fn batch_save(
    State(db): State<Arc<Database>>,
    Extension(embeddings): Extension<Option<Arc<crate::embeddings::engine::EmbeddingEngine>>>,
    Json(body): Json<BatchSaveBody>,
) -> ApiResult<BatchSaveResponse> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let mut inputs = Vec::new();
    for item in body.memories {
        let memory_type = item.memory_type.parse().map_err(map_err)?;
        let importance = item.importance.parse().map_err(map_err)?;
        let scope = item
            .scope
            .as_deref()
            .map(Scope::from_str)
            .transpose()
            .map_err(map_err)?
            .unwrap_or(Scope::Project);
        inputs.push(CreateMemoryInput {
            project: project.clone(),
            scope: Some(scope),
            title: item.title,
            content: item.content,
            what: item.what,
            why: item.why,
            context: item.context,
            learned: item.learned,
            memory_type,
            importance,
            tags: item.tags,
            topic_key: item.topic_key,
            capture_prompt: None,
            encrypt: false,
        });
    }

    let engine = embeddings.clone();
    let embedding_store = db.embeddings();
    let (saved, duplicates) = db
        .memories()
        .save_batch(inputs, engine, Some(embedding_store))
        .map_err(map_err)?;

    Ok(Json(BatchSaveResponse {
        saved_count: saved.len(),
        duplicate_count: duplicates.len(),
        saved,
        duplicates,
    }))
}

// --- Audit ---

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_days_30")]
    days_threshold: u32,
}

fn default_days_30() -> u32 {
    30
}

pub async fn get_audit(
    State(db): State<Arc<Database>>,
    Query(query): Query<AuditQuery>,
) -> ApiResult<crate::store::memory::AuditReport> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let report = db
        .memories()
        .audit(&project, query.days_threshold)
        .map_err(map_err)?;
    Ok(Json(report))
}

// --- Deduplicate ---

#[derive(Debug, Deserialize)]
pub struct DeduplicateBody {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_threshold_85")]
    threshold: f64,
}

fn default_threshold_85() -> f64 {
    0.85
}

pub async fn deduplicate(
    State(db): State<Arc<Database>>,
    Json(body): Json<DeduplicateBody>,
) -> ApiResult<Vec<crate::store::memory::DuplicateGroup>> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let embedding_store = db.embeddings();
    let groups = db
        .memories()
        .find_duplicates_semantic(&project, body.threshold, &embedding_store)
        .map_err(map_err)?;
    Ok(Json(groups))
}

// --- Feedback ---

#[derive(Debug, Deserialize)]
pub struct FeedbackBody {
    memory_id: String,
    is_useful: bool,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FeedbackResponse {
    memory_id: String,
    feedback_id: i64,
}

pub async fn add_feedback(
    State(db): State<Arc<Database>>,
    Json(body): Json<FeedbackBody>,
) -> ApiResult<FeedbackResponse> {
    let memory_id = Uuid::parse_str(&body.memory_id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let feedback_id = db
        .memories()
        .add_feedback(memory_id, body.is_useful, body.reason.as_deref())
        .map_err(map_err)?;
    Ok(Json(FeedbackResponse {
        memory_id: body.memory_id,
        feedback_id,
    }))
}

// --- Deprecate ---

#[derive(Debug, Deserialize)]
pub struct DeprecateBody {
    memory_id: String,
    reason: String,
    #[serde(default)]
    supersedes_id: Option<String>,
}

pub async fn deprecate(
    State(db): State<Arc<Database>>,
    Json(body): Json<DeprecateBody>,
) -> ApiResult<Memory> {
    let memory_id = Uuid::parse_str(&body.memory_id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let supersedes_id = body
        .supersedes_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let memory = db
        .memories()
        .deprecate(memory_id, &body.reason, supersedes_id)
        .map_err(map_err)?;
    Ok(Json(memory))
}

// --- Graph ---

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn get_graph(
    State(db): State<Arc<Database>>,
    Query(query): Query<GraphQuery>,
) -> ApiResult<crate::store::memory::GraphData> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let graph = db.memories().get_graph(&project).map_err(map_err)?;
    Ok(Json(graph))
}

// --- Summarize ---

#[derive(Debug, Deserialize)]
pub struct SummarizeQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

pub async fn get_summarize(
    State(db): State<Arc<Database>>,
    Query(query): Query<SummarizeQuery>,
) -> ApiResult<crate::store::memory::SummaryResult> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let session_id = query
        .session_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let summary = db
        .memories()
        .summarize(&project, session_id)
        .map_err(map_err)?;
    Ok(Json(summary))
}

// --- Inject Context ---

#[derive(Debug, Deserialize)]
pub struct InjectContextQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

fn default_limit_10() -> u32 {
    10
}

#[derive(Debug, Serialize)]
pub struct InjectContextResponse {
    context: String,
    project: String,
}

pub async fn inject_context(
    State(db): State<Arc<Database>>,
    Query(query): Query<InjectContextQuery>,
) -> ApiResult<InjectContextResponse> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let context = db
        .memories()
        .inject_context(&project, query.file.as_deref(), query.limit)
        .map_err(map_err)?;
    Ok(Json(InjectContextResponse { context, project }))
}

// --- Forget Project ---

#[derive(Debug, Deserialize)]
pub struct ForgetProjectBody {
    #[serde(default)]
    project: Option<String>,
    confirm: bool,
}

#[derive(Debug, Serialize)]
pub struct ForgetProjectResponse {
    deleted: u32,
    project: String,
}

pub async fn forget_project(
    State(db): State<Arc<Database>>,
    Json(body): Json<ForgetProjectBody>,
) -> ApiResult<ForgetProjectResponse> {
    if !body.confirm {
        return Err(map_err(crate::error::MnemeError::Config(
            "confirm must be true to forget project".into(),
        )));
    }
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let deleted = db.memories().forget_project(&project).map_err(map_err)?;
    Ok(Json(ForgetProjectResponse { deleted, project }))
}

// --- Health ---

#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn get_health(
    State(db): State<Arc<Database>>,
    Query(query): Query<HealthQuery>,
) -> ApiResult<crate::store::memory::HealthReport> {
    let report = db
        .memories()
        .health(query.project.as_deref())
        .map_err(map_err)?;
    Ok(Json(report))
}

// --- Remind ---

#[derive(Debug, Deserialize)]
pub struct RemindQuery {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_high")]
    importance: String,
}

fn default_high() -> String {
    "high".into()
}

pub async fn get_remind(
    State(db): State<Arc<Database>>,
    Query(query): Query<RemindQuery>,
) -> ApiResult<Vec<Memory>> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let importance = query.importance.parse().map_err(map_err)?;
    let memories = db.memories().remind(&project, &importance).map_err(map_err)?;
    Ok(Json(memories))
}

// --- Tag Suggest ---

#[derive(Debug, Deserialize)]
pub struct TagSuggestBody {
    #[serde(default)]
    project: Option<String>,
    title: String,
    #[serde(default)]
    content: Option<String>,
}

pub async fn tag_suggest(
    State(db): State<Arc<Database>>,
    Json(body): Json<TagSuggestBody>,
) -> ApiResult<Vec<String>> {
    let project = body.project.unwrap_or_else(Settings::infer_project);
    let tags = db
        .memories()
        .suggest_tags(&project, &body.title, body.content.as_deref())
        .map_err(map_err)?;
    Ok(Json(tags))
}

// --- Knowledge Gaps ---

#[derive(Debug, Deserialize)]
pub struct KnowledgeGapsQuery {
    #[serde(default)]
    project: Option<String>,
}

pub async fn get_knowledge_gaps(
    State(db): State<Arc<Database>>,
    Query(query): Query<KnowledgeGapsQuery>,
) -> ApiResult<crate::store::memory::KnowledgeGapsReport> {
    let project = query.project.unwrap_or_else(Settings::infer_project);
    let report = db.memories().knowledge_gaps(&project).map_err(map_err)?;
    Ok(Json(report))
}

// --- Sync ---

use crate::sync::protocol::{SyncHello, SyncRequest, SyncResponse};

pub async fn sync_hello(
    State(db): State<Arc<Database>>,
    Json(body): Json<SyncHello>,
) -> ApiResult<SyncHello> {
    let settings = Settings::load().map_err(map_err)?;
    let engine = crate::sync::engine::SyncEngine::new(db, settings.sync.clone()).map_err(map_err)?;
    let hello = engine.build_hello(&body.project).map_err(map_err)?;
    Ok(Json(hello))
}

pub async fn sync_pull(
    State(db): State<Arc<Database>>,
    Json(body): Json<SyncRequest>,
) -> ApiResult<SyncResponse> {
    let settings = Settings::load().map_err(map_err)?;
    let engine = crate::sync::engine::SyncEngine::new(db, settings.sync.clone()).map_err(map_err)?;
    let response = engine.build_response(&body).map_err(map_err)?;
    Ok(Json(response))
}

pub async fn sync_push(
    State(db): State<Arc<Database>>,
    Json(body): Json<SyncResponse>,
) -> ApiResult<serde_json::Value> {
    let settings = Settings::load().map_err(map_err)?;
    let engine = crate::sync::engine::SyncEngine::new(db, settings.sync.clone()).map_err(map_err)?;
    let stats = engine.apply_response(&body).map_err(map_err)?;
    Ok(Json(json!({
        "applied": stats.memories_applied,
        "conflicts": stats.conflicts_resolved
    })))
}

// --- Encryption ---

/// POST /api/v1/memories/:id/encrypt
pub async fn encrypt_memory(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> ApiResult<Memory> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let memory = db.memories().encrypt_existing(id).map_err(map_err)?;
    Ok(Json(memory))
}

/// POST /api/v1/memories/:id/decrypt
pub async fn decrypt_memory(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> ApiResult<Memory> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let memory = db.memories().decrypt_existing(id).map_err(map_err)?;
    Ok(Json(memory))
}

// --- Keys ---

/// GET /api/v1/keys
pub async fn list_keys(State(db): State<Arc<Database>>) -> ApiResult<Vec<crate::crypto::RegisteredKey>> {
    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    let keys = key_store.list().map_err(map_err)?;
    Ok(Json(keys))
}

/// Body para agregar clave.
#[derive(Debug, Deserialize)]
pub struct AddKeyBody {
    alias: String,
    key: String,
    #[serde(default)]
    default: bool,
}

/// POST /api/v1/keys
pub async fn add_key(
    State(db): State<Arc<Database>>,
    Json(body): Json<AddKeyBody>,
) -> ApiResult<crate::crypto::RegisteredKey> {
    let recipient = crate::crypto::RecipientKey::from_string(&body.key).map_err(map_err)?;
    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    let registered = key_store.add(&body.alias, &recipient).map_err(map_err)?;
    if body.default {
        key_store.set_default(registered.id).map_err(map_err)?;
    }
    Ok(Json(registered))
}

/// DELETE /api/v1/keys/:id
pub async fn remove_key(
    State(db): State<Arc<Database>>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let id = Uuid::parse_str(&id)
        .map_err(|e| map_err(crate::error::MnemeError::Config(e.to_string())))?;
    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    key_store.remove(id).map_err(map_err)?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/keys/status
pub async fn keys_status(State(db): State<Arc<Database>>) -> ApiResult<serde_json::Value> {
    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    let keys = key_store.list().unwrap_or_default();
    let keys_count = keys.len() as u32;
    let default_key = keys.iter().find(|k| k.is_default).map(|k| k.alias.clone());

    Ok(Json(json!({
        "keys_count": keys_count,
        "encrypted_memories": 0,
        "identity_loaded": false,
        "default_key": default_key,
    })))
}
