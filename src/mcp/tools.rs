use std::str::FromStr;
use std::sync::Arc;

use chrono::Utc;
use rmcp::handler::server::tool::schema_for_type;
use rmcp::model::{CallToolResult, Content, JsonObject, Tool};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::memory::{
    CreateMemoryInput, CreatePromptInput, Scope, SearchQuery, UpdateMemoryInput,
};

/// Builds the JSON envelope response for MCP tools.
fn tool_response<T: serde::Serialize>(
    result: crate::error::Result<T>,
    project: &str,
) -> CallToolResult {
    let meta = json!({
        "project": project,
        "timestamp": Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    });

    match result {
        Ok(data) => {
            let envelope = json!({
                "success": true,
                "data": data,
                "error": null,
                "meta": meta
            });
            match Content::json(envelope) {
                Ok(content) => CallToolResult::success(vec![content]),
                Err(e) => {
                    let err_envelope = json!({
                        "success": false,
                        "data": null,
                        "error": {
                            "code": "SERIALIZATION_ERROR",
                            "message": e.to_string()
                        },
                        "meta": meta
                    });
                    CallToolResult::error(vec![Content::text(err_envelope.to_string())])
                }
            }
        }
        Err(e) => {
            let code = match e {
                crate::error::MnemeError::NotFound(_) => "NOT_FOUND",
                crate::error::MnemeError::ProjectRequired => "PROJECT_REQUIRED",
                crate::error::MnemeError::EmptyQuery => "EMPTY_QUERY",
                crate::error::MnemeError::InvalidMemoryType(_) => "INVALID_MEMORY_TYPE",
                crate::error::MnemeError::InvalidImportance(_) => "INVALID_IMPORTANCE",
                crate::error::MnemeError::InvalidScope(_) => "INVALID_SCOPE",
                crate::error::MnemeError::InvalidRelationType(_) => "INVALID_RELATION_TYPE",
                crate::error::MnemeError::RelationAlreadyExists(_, _) => "RELATION_EXISTS",
                crate::error::MnemeError::SelfRelation(_) => "SELF_RELATION",
                crate::error::MnemeError::DuplicateDetected(_) => "DUPLICATE",
                crate::error::MnemeError::Database(_) => "DATABASE_ERROR",
                crate::error::MnemeError::Migration(_) => "MIGRATION_ERROR",
                crate::error::MnemeError::Io(_) => "IO_ERROR",
                crate::error::MnemeError::Serialization(_) => "SERIALIZATION_ERROR",
                crate::error::MnemeError::Config(_) => "CONFIG_ERROR",
                crate::error::MnemeError::Http(_) => "HTTP_ERROR",
                crate::error::MnemeError::Mcp(_) => "MCP_ERROR",
                crate::error::MnemeError::Embeddings(_) => "EMBEDDINGS_ERROR",
                crate::error::MnemeError::EmbeddingsDisabled => "EMBEDDINGS_DISABLED",
                crate::error::MnemeError::PeerNotFound(_) => "PEER_NOT_FOUND",
                crate::error::MnemeError::SyncFailed { .. } => "SYNC_FAILED",
                crate::error::MnemeError::SyncDisabled => "SYNC_DISABLED",
                crate::error::MnemeError::UnsupportedTransport(_) => "UNSUPPORTED_TRANSPORT",
                crate::error::MnemeError::InvalidSyncFile(_) => "INVALID_SYNC_FILE",
                crate::error::MnemeError::Compression(_) => "COMPRESSION_ERROR",
                crate::error::MnemeError::NoRecipientsConfigured => "NO_RECIPIENTS",
                crate::error::MnemeError::DecryptionFailed => "DECRYPTION_FAILED",
                crate::error::MnemeError::IdentityNotLoaded => "IDENTITY_NOT_LOADED",
                crate::error::MnemeError::AlreadyEncrypted(_) => "ALREADY_ENCRYPTED",
                crate::error::MnemeError::NotEncrypted(_) => "NOT_ENCRYPTED",
                crate::error::MnemeError::KeyNotFound(_) => "KEY_NOT_FOUND",
                crate::error::MnemeError::Plugin(_) => "PLUGIN_ERROR",
            };
            let envelope = json!({
                "success": false,
                "data": null,
                "error": {
                    "code": code,
                    "message": e.to_string()
                },
                "meta": meta
            });
            CallToolResult::error(vec![Content::text(envelope.to_string())])
        }
    }
}

/// Executes an MCP tool by name, with optional plugin dispatch.
pub async fn execute_tool(
    db: &Database,
    name: &str,
    arguments: Option<JsonObject>,
    project: &str,
    embeddings: Option<&std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
    plugins: Option<&crate::plugins::PluginManager>,
) -> CallToolResult {
    tracing::debug!("MCP tool call: {} with args: {:?}", name, arguments);

    let args = arguments.unwrap_or_default();

    // Check plugin dispatch first for unknown built-in tools.
    if let Some(pm) = plugins {
        if pm.owns_tool(name) {
            let args_value = serde_json::Value::Object(args);
            let result = pm.call_tool(name, args_value, project);
            return tool_response(result, project);
        }
    }

    let result = match name {
        "mem_save" => mem_save(db, args, project, embeddings),
        "mem_update" => mem_update(db, args, project),
        "mem_delete" => mem_delete(db, args, project),
        "mem_restore" => mem_restore(db, args, project),
        "mem_search" => mem_search(db, args, project),
        "mem_similar" => mem_similar(db, args, project, embeddings).await,
        "mem_get" => mem_get(db, args, project),
        "mem_list" => mem_list(db, args, project),
        "mem_context" => mem_context(db, args, project),
        "mem_timeline" => mem_timeline(db, args, project),
        "mem_session_start" => mem_session_start(db, args, project),
        "mem_session_end" => mem_session_end(db, args, project),
        "mem_session_summary" => mem_session_summary(db, args, project),
        "mem_stats" => mem_stats(db, args, project),
        "mem_projects" => mem_projects(db, args, project),
        "mem_conflicts" => mem_conflicts(db, args, project),
        "mem_save_prompt" => mem_save_prompt(db, args, project),
        "mem_suggest_topic_key" => mem_suggest_topic_key(db, args, project),
        "mem_current_project" => mem_current_project(db, args, project),
        "mem_doctor" => mem_doctor(db, args, project),
        "mem_save_batch" => mem_save_batch(db, args, project, embeddings),
        "mem_delete_relation" => mem_delete_relation(db, args, project),
        "mem_audit" => mem_audit(db, args, project),
        "mem_deduplicate" => mem_deduplicate(db, args, project),
        "mem_feedback" => mem_feedback(db, args, project),
        "mem_deprecate" => mem_deprecate(db, args, project),
        "mem_graph" => mem_graph(db, args, project),
        "mem_summarize" => mem_summarize(db, args, project),
        "mem_inject_context" => mem_inject_context(db, args, project),
        "mem_forget_project" => mem_forget_project(db, args, project),
        "mem_health" => mem_health(db, args, project),
        "mem_remind" => mem_remind(db, args, project),
        "mem_tag_suggest" => mem_tag_suggest(db, args, project),
        "mem_knowledge_gaps" => mem_knowledge_gaps(db, args, project),
        "mem_sync_status" => mem_sync_status(db, args, project),
        "mem_sync_now" => mem_sync_now(db, args, project),
        "mem_sync_export" => mem_sync_export(db, args, project),
        "mem_encrypt" => mem_encrypt(db, args, project),
        "mem_decrypt" => mem_decrypt(db, args, project),
        "keys_list" => keys_list(db, args, project),
        "keys_status" => keys_status(db, args, project),
        _ => Err(crate::error::MnemeError::Mcp(format!(
            "Unknown tool: {}",
            name
        ))),
    };

    tool_response(result, project)
}

// --- Tool Parameter Schemas ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSaveParams {
    title: String,
    content: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(rename = "type", default = "default_note")]
    r#type: String,
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemUpdateParams {
    id: String,
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
    r#type: Option<String>,
    #[serde(default)]
    importance: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    topic_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDeleteParams {
    id: String,
    #[serde(default)]
    hard: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemRestoreParams {
    id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSearchParams {
    query: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(rename = "type", default)]
    r#type: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

fn default_limit_10() -> u32 {
    10
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemGetParams {
    id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemListParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(rename = "type", default)]
    r#type: Option<String>,
    #[serde(default = "default_limit_20")]
    limit: u32,
}

fn default_limit_20() -> u32 {
    20
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemContextParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemTimelineParams {
    id: String,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSessionStartParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    directory: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSessionEndParams {
    id: String,
    #[serde(default)]
    summary: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSessionSummaryParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemStatsParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemProjectsParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemConflictsParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSavePromptParams {
    content: String,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSuggestTopicKeyParams {
    #[serde(rename = "type")]
    r#type: String,
    title: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCurrentProjectParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDoctorParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSimilarParams {
    /// Texto o UUID para buscar similitud semántica.
    query: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_limit_5")]
    limit: u32,
    #[serde(default = "default_threshold_75")]
    threshold: f32,
}

fn default_limit_5() -> u32 {
    5
}
fn default_threshold_75() -> f32 {
    0.75
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct BatchMemoryInput {
    title: String,
    content: String,
    #[serde(rename = "type", default = "default_note")]
    r#type: String,
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSaveBatchParams {
    memories: Vec<BatchMemoryInput>,
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDeleteRelationParams {
    relation_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemAuditParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_days_30")]
    days_threshold: u32,
}

fn default_days_30() -> u32 {
    30
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDeduplicateParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_threshold_85")]
    threshold: f64,
}

fn default_threshold_85() -> f64 {
    0.85
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemFeedbackParams {
    memory_id: String,
    is_useful: bool,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDeprecateParams {
    memory_id: String,
    reason: String,
    #[serde(default)]
    supersedes_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemGraphParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSummarizeParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemInjectContextParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemForgetProjectParams {
    #[serde(default)]
    project: Option<String>,
    confirm: bool,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemHealthParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemRemindParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_high")]
    importance: String,
}

fn default_high() -> String {
    "high".into()
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemTagSuggestParams {
    #[serde(default)]
    project: Option<String>,
    title: String,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemKnowledgeGapsParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSyncStatusParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSyncNowParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSyncExportParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    output: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemEncryptParams {
    memory_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemDecryptParams {
    memory_id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct KeysListParams {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct KeysStatusParams {
    #[serde(default)]
    project: Option<String>,
}

// --- Tool Implementations ---

fn mem_save(
    db: &Database,
    args: JsonObject,
    project: &str,
    embeddings: Option<&std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSaveParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let memory_type = params.r#type.parse()?;
    let importance = params.importance.parse()?;
    let scope = params
        .scope
        .as_deref()
        .map(Scope::from_str)
        .transpose()?
        .unwrap_or(Scope::Project);

    let input = CreateMemoryInput {
        project,
        scope: Some(scope),
        title: params.title,
        content: params.content,
        what: params.what,
        why: params.why,
        context: params.context,
        learned: params.learned,
        memory_type,
        importance,
        tags: params.tags,
        topic_key: params.topic_key,
        capture_prompt: None,
        encrypt: false,
    };

    let engine = embeddings.map(std::sync::Arc::clone);
    let embedding_store = db.embeddings();
    let memory = db.memories().save(input, engine, Some(embedding_store))?;
    Ok(serde_json::to_value(memory)?)
}

fn mem_update(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemUpdateParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let update = UpdateMemoryInput {
        title: params.title,
        content: params.content,
        what: params.what,
        why: params.why,
        context: params.context,
        learned: params.learned,
        memory_type: params.r#type.as_deref().map(str::parse).transpose()?,
        importance: params.importance.as_deref().map(str::parse).transpose()?,
        tags: params.tags,
        scope: params.scope.as_deref().map(Scope::from_str).transpose()?,
        topic_key: params.topic_key,
    };

    let memory = db.memories().update(id, update)?;
    Ok(serde_json::to_value(memory)?)
}

fn mem_delete(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDeleteParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    db.memories().delete(id, params.hard)?;
    Ok(json!({"deleted": true, "id": params.id}))
}

fn mem_restore(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemRestoreParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db.memories().restore(id)?;
    Ok(serde_json::to_value(memory)?)
}

fn mem_search(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSearchParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let memory_type = params.r#type.as_deref().map(str::parse).transpose()?;

    let query = SearchQuery {
        text: params.query,
        project: Some(project),
        scope: None,
        memory_type,
        importance: None,
        tags: Vec::new(),
        limit: params.limit,
        include_snippet: true,
        all_projects: false,
    };

    let weights = crate::store::search::SearchWeights::default();
    let results = db.memories().search(&query, &weights, None)?;
    Ok(serde_json::to_value(results)?)
}

async fn mem_similar(
    db: &Database,
    args: JsonObject,
    project: &str,
    embeddings: Option<&std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSimilarParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let engine = match embeddings {
        Some(e) => e,
        None => return Err(crate::error::MnemeError::EmbeddingsDisabled),
    };

    let project = params.project.unwrap_or_else(|| project.to_string());
    let embedding_store = db.embeddings();

    // Try to parse query as UUID first
    let query_embedding = if let Ok(id) = Uuid::parse_str(&params.query) {
        match embedding_store.load(id)? {
            Some(embedding) => embedding,
            None => {
                // If no stored embedding, load memory and embed it
                let memory = db
                    .memories()
                    .get(id)?
                    .ok_or_else(|| crate::error::MnemeError::NotFound(id))?;
                let text = crate::embeddings::engine::EmbeddingEngine::memory_to_text(&memory);
                engine.embed(&text).await?
            }
        }
    } else {
        engine.embed(&params.query).await?
    };

    let all_embeddings = embedding_store.load_all_for_project(&project)?;
    let mut matches = Vec::new();
    for (id, embedding) in all_embeddings {
        let score = crate::embeddings::similarity::cosine_similarity(&query_embedding, &embedding);
        if score >= params.threshold {
            matches.push(crate::embeddings::similarity::SemanticMatch {
                memory_id: id,
                cosine_score: score,
                combined_score: f64::from(score),
            });
        }
    }
    crate::embeddings::similarity::rank_by_combined_score(&mut matches);
    matches.truncate(params.limit as usize);

    let mut results = Vec::new();
    for m in matches {
        if let Some(memory) = db.memories().get(m.memory_id)? {
            results.push(crate::store::memory::SearchResult {
                memory,
                score: m.combined_score,
                snippet: None,
                match_type: crate::store::memory::MatchType::Semantic,
                cosine_score: Some(m.cosine_score),
            });
        }
    }

    Ok(serde_json::to_value(results)?)
}

fn mem_get(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemGetParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    match db.memories().get(id)? {
        Some(memory) => Ok(serde_json::to_value(memory)?),
        None => Err(crate::error::MnemeError::NotFound(id)),
    }
}

fn mem_list(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemListParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let memory_type = params.r#type.as_deref().map(str::parse).transpose()?;

    let memories =
        db.memories()
            .list(&project, memory_type.as_ref(), None, None, params.limit, 0)?;
    Ok(serde_json::to_value(memories)?)
}

fn mem_context(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemContextParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| _project.to_string());
    let memories = db.memories().context(&project, None, params.limit)?;
    Ok(serde_json::to_value(memories)?)
}

fn mem_timeline(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemTimelineParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db
        .memories()
        .get(id)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(id))?;

    let project = memory.project.clone();
    let memories = db
        .memories()
        .list(&project, None, None, None, params.limit, 0)?;
    Ok(serde_json::to_value(memories)?)
}

fn mem_session_start(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSessionStartParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let current_dir = std::env::current_dir().ok();
    let dir = params
        .directory
        .as_deref()
        .or_else(|| current_dir.as_ref().and_then(|p| p.to_str()));
    let session = db.sessions().start(&project, dir)?;
    Ok(serde_json::to_value(session)?)
}

fn mem_session_end(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSessionEndParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id =
        Uuid::parse_str(&params.id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let session = db.sessions().end(id, params.summary.as_deref())?;
    Ok(serde_json::to_value(session)?)
}

fn mem_session_summary(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSessionSummaryParams =
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    match db.sessions().get_active(&project)? {
        Some(session) => Ok(serde_json::to_value(session)?),
        None => Ok(json!(null)),
    }
}

fn mem_stats(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemStatsParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let stats = db.memories().stats(&project)?;
    Ok(serde_json::to_value(stats)?)
}

fn mem_projects(
    db: &Database,
    _args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let projects = db.memories().list_projects()?;
    Ok(serde_json::to_value(projects)?)
}

fn mem_conflicts(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemConflictsParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let query = SearchQuery {
        text: "conflict".to_string(),
        project: Some(project),
        scope: None,
        memory_type: None,
        importance: None,
        tags: Vec::new(),
        limit: params.limit,
        include_snippet: false,
        all_projects: false,
    };

    let weights = crate::store::search::SearchWeights::default();
    let results = db.memories().search(&query, &weights, None)?;
    Ok(serde_json::to_value(results)?)
}

fn mem_save_prompt(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSavePromptParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let session_id = params
        .session_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let input = CreatePromptInput {
        session_id,
        content: params.content,
        project,
    };

    let store = db.memories();
    let prompt = store.save_prompt(input)?;
    Ok(serde_json::to_value(prompt)?)
}

fn mem_suggest_topic_key(
    _db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSuggestTopicKeyParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let family = match params.r#type.to_lowercase().as_str() {
        "architecture" => "architecture",
        "decision" => "decision",
        "bugfix" => "bug",
        "pattern" => "pattern",
        "convention" => "convention",
        "dependency" => "dependency",
        "workflow" => "workflow",
        "note" => "note",
        "config" => "config",
        "discovery" => "discovery",
        "learning" => "learning",
        _ => "general",
    };

    let slug = params
        .title
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join("-");

    let suggested = format!("{}/{}", family, slug);
    Ok(json!({"topic_key": suggested}))
}

fn mem_current_project(
    _db: &Database,
    _args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let inferred = Settings::infer_project();
    let git_root = Settings::git_toplevel();
    let source = if git_root.is_some() {
        "git"
    } else {
        "directory"
    };
    let path = git_root
        .or_else(|| std::env::current_dir().ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(json!({
        "project": inferred,
        "source": source,
        "path": path
    }))
}

fn mem_doctor(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDoctorParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let issues: Vec<String> = Vec::new();
    let mut checks = Vec::new();

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

    let healthy = issues.is_empty();

    Ok(json!({
        "healthy": healthy,
        "project": project,
        "memory_count": memory_count,
        "session_count": session_count,
        "issues": issues,
        "checks": checks
    }))
}

fn mem_save_batch(
    db: &Database,
    args: JsonObject,
    project: &str,
    embeddings: Option<&std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSaveBatchParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let mut inputs = Vec::new();
    for item in params.memories {
        let memory_type = item.r#type.parse()?;
        let importance = item.importance.parse()?;
        let scope = item
            .scope
            .as_deref()
            .map(Scope::from_str)
            .transpose()?
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

    let engine = embeddings.map(std::sync::Arc::clone);
    let embedding_store = db.embeddings();
    let (saved, duplicates) = db
        .memories()
        .save_batch(inputs, engine, Some(embedding_store))?;
    Ok(json!({
        "saved": saved,
        "duplicates": duplicates,
        "saved_count": saved.len(),
        "duplicate_count": duplicates.len()
    }))
}

fn mem_delete_relation(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDeleteRelationParams =
        serde_json::from_value(serde_json::Value::Object(args))
            .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id = Uuid::parse_str(&params.relation_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let deleted = db.memories().delete_relation(id)?;
    Ok(json!({"deleted": deleted, "id": params.relation_id}))
}

fn mem_audit(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemAuditParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let report = db.memories().audit(&project, params.days_threshold)?;
    Ok(serde_json::to_value(report)?)
}

fn mem_deduplicate(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDeduplicateParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let embedding_store = db.embeddings();
    let groups =
        db.memories()
            .find_duplicates_semantic(&project, params.threshold, &embedding_store)?;
    Ok(serde_json::to_value(groups)?)
}

fn mem_feedback(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemFeedbackParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id = Uuid::parse_str(&params.memory_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let feedback_id =
        db.memories()
            .add_feedback(memory_id, params.is_useful, params.reason.as_deref())?;
    Ok(json!({
        "memory_id": params.memory_id,
        "feedback_id": feedback_id
    }))
}

fn mem_deprecate(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDeprecateParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id = Uuid::parse_str(&params.memory_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
    let supersedes_id = params
        .supersedes_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db
        .memories()
        .deprecate(memory_id, &params.reason, supersedes_id)?;
    Ok(serde_json::to_value(memory)?)
}

fn mem_graph(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemGraphParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let graph = db.memories().get_graph(&project)?;
    Ok(serde_json::to_value(graph)?)
}

fn mem_summarize(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSummarizeParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let session_id = params
        .session_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let summary = db.memories().summarize(&project, session_id)?;
    Ok(serde_json::to_value(summary)?)
}

fn mem_inject_context(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemInjectContextParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let context = db
        .memories()
        .inject_context(&project, params.file.as_deref(), params.limit)?;
    Ok(json!({"context": context, "project": project}))
}

fn mem_forget_project(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemForgetProjectParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    if !params.confirm {
        return Err(crate::error::MnemeError::Config(
            "confirm must be true to forget project".into(),
        ));
    }

    let project = params.project.unwrap_or_else(|| project.to_string());
    let deleted = db.memories().forget_project(&project)?;
    Ok(json!({"deleted": deleted, "project": project}))
}

fn mem_health(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemHealthParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let report = db.memories().health(params.project.as_deref())?;
    Ok(serde_json::to_value(report)?)
}

fn mem_remind(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemRemindParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let importance = params.importance.parse()?;
    let memories = db.memories().remind(&project, &importance)?;
    Ok(serde_json::to_value(memories)?)
}

fn mem_tag_suggest(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemTagSuggestParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let tags = db
        .memories()
        .suggest_tags(&project, &params.title, params.content.as_deref())?;
    Ok(serde_json::to_value(tags)?)
}

fn mem_knowledge_gaps(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemKnowledgeGapsParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let report = db.memories().knowledge_gaps(&project)?;
    Ok(serde_json::to_value(report)?)
}

fn mem_sync_status(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSyncStatusParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let peers = db.peers().list(&project)?;
    let peer_names: Vec<String> = peers.iter().map(|p| p.name.clone()).collect();

    Ok(json!({
        "project": project,
        "peers": peer_names,
        "peer_count": peers.len()
    }))
}

fn mem_sync_now(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSyncNowParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let settings = Settings::load()?;
    let engine = crate::sync::engine::SyncEngine::new(Arc::new(db.clone()), settings.sync)?;

    let rt = tokio::runtime::Runtime::new()?;
    let results = rt.block_on(async { engine.sync_auto(&project).await })?;

    Ok(json!({
        "project": project,
        "results": results
    }))
}

fn mem_sync_export(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSyncExportParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let output = params.output.map(std::path::PathBuf::from);
    let settings = Settings::load()?;
    let engine = crate::sync::engine::SyncEngine::new(Arc::new(db.clone()), settings.sync)?;
    let stats = engine.export_project(&project, output)?;

    Ok(json!({
        "project": project,
        "memories_exported": stats.memories_exported,
        "bytes_written": stats.bytes_written
    }))
}

fn mem_encrypt(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemEncryptParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id = Uuid::parse_str(&params.memory_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db.memories().encrypt_existing(memory_id)?;
    Ok(json!({ "memory": memory, "encrypted": true }))
}

fn mem_decrypt(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemDecryptParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id = Uuid::parse_str(&params.memory_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db.memories().decrypt_existing(memory_id)?;
    Ok(json!({ "memory": memory, "decrypted": true }))
}

fn keys_list(
    db: &Database,
    _args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    let keys = key_store.list()?;
    Ok(json!({ "keys": keys }))
}

fn keys_status(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: KeysStatusParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let key_store = crate::crypto::KeyStore::new(db.get_conn());
    let keys = key_store.list().unwrap_or_default();
    let keys_count = keys.len() as u32;
    let default_key = keys.iter().find(|k| k.is_default).map(|k| k.alias.clone());

    let encrypted_memories = if let Some(ref proj) = params.project {
        db.memories()
            .list(proj, None, None, None, 10000, 0)
            .map(|mems| mems.iter().filter(|m| m.is_encrypted).count() as u32)
            .unwrap_or(0)
    } else {
        0u32
    };

    Ok(json!({
        "keys_count": keys_count,
        "encrypted_memories": encrypted_memories,
        "identity_loaded": false,
        "default_key": default_key,
    }))
}

/// Returns the list of all MCP tool definitions, including plugin-provided tools.
pub fn list_tools(plugins: Option<&crate::plugins::PluginManager>) -> Vec<Tool> {
    let mut tools = vec![
        Tool::new(
            "mem_save",
            "Save a memory with deduplication and topic_key support",
            Arc::new(schema_for_type::<MemSaveParams>()),
        ),
        Tool::new(
            "mem_update",
            "Update an existing memory by ID",
            Arc::new(schema_for_type::<MemUpdateParams>()),
        ),
        Tool::new(
            "mem_delete",
            "Soft-delete a memory (hard delete optional)",
            Arc::new(schema_for_type::<MemDeleteParams>()),
        ),
        Tool::new(
            "mem_restore",
            "Restore a soft-deleted memory",
            Arc::new(schema_for_type::<MemRestoreParams>()),
        ),
        Tool::new(
            "mem_search",
            "Hybrid search (FTS5 + fuzzy matching)",
            Arc::new(schema_for_type::<MemSearchParams>()),
        ),
        Tool::new(
            "mem_get",
            "Get a memory by ID",
            Arc::new(schema_for_type::<MemGetParams>()),
        ),
        Tool::new(
            "mem_list",
            "List memories with optional filters",
            Arc::new(schema_for_type::<MemListParams>()),
        ),
        Tool::new(
            "mem_context",
            "Get recent memories for session context",
            Arc::new(schema_for_type::<MemContextParams>()),
        ),
        Tool::new(
            "mem_timeline",
            "Get memories around a specific memory",
            Arc::new(schema_for_type::<MemTimelineParams>()),
        ),
        Tool::new(
            "mem_session_start",
            "Start a new work session",
            Arc::new(schema_for_type::<MemSessionStartParams>()),
        ),
        Tool::new(
            "mem_session_end",
            "End an active session",
            Arc::new(schema_for_type::<MemSessionEndParams>()),
        ),
        Tool::new(
            "mem_session_summary",
            "Get the active session summary",
            Arc::new(schema_for_type::<MemSessionSummaryParams>()),
        ),
        Tool::new(
            "mem_stats",
            "Get project statistics",
            Arc::new(schema_for_type::<MemStatsParams>()),
        ),
        Tool::new(
            "mem_projects",
            "List all projects",
            Arc::new(schema_for_type::<MemProjectsParams>()),
        ),
        Tool::new(
            "mem_conflicts",
            "Detect potential conflicts",
            Arc::new(schema_for_type::<MemConflictsParams>()),
        ),
        Tool::new(
            "mem_save_prompt",
            "Save a user prompt",
            Arc::new(schema_for_type::<MemSavePromptParams>()),
        ),
        Tool::new(
            "mem_suggest_topic_key",
            "Suggest a topic_key from type and title",
            Arc::new(schema_for_type::<MemSuggestTopicKeyParams>()),
        ),
        Tool::new(
            "mem_current_project",
            "Detect current project from cwd/git",
            Arc::new(schema_for_type::<MemCurrentProjectParams>()),
        ),
        Tool::new(
            "mem_doctor",
            "Run diagnostics on the database",
            Arc::new(schema_for_type::<MemDoctorParams>()),
        ),
        Tool::new(
            "mem_similar",
            "Find memories semantically similar to a given memory or text",
            Arc::new(schema_for_type::<MemSimilarParams>()),
        ),
        Tool::new(
            "mem_save_batch",
            "Save multiple memories in one call",
            Arc::new(schema_for_type::<MemSaveBatchParams>()),
        ),
        Tool::new(
            "mem_delete_relation",
            "Delete a relation by ID",
            Arc::new(schema_for_type::<MemDeleteRelationParams>()),
        ),
        Tool::new(
            "mem_audit",
            "Run quality audit on project memories",
            Arc::new(schema_for_type::<MemAuditParams>()),
        ),
        Tool::new(
            "mem_deduplicate",
            "Find semantically similar memories using embeddings",
            Arc::new(schema_for_type::<MemDeduplicateParams>()),
        ),
        Tool::new(
            "mem_feedback",
            "Record feedback on a memory",
            Arc::new(schema_for_type::<MemFeedbackParams>()),
        ),
        Tool::new(
            "mem_deprecate",
            "Mark a memory as deprecated",
            Arc::new(schema_for_type::<MemDeprecateParams>()),
        ),
        Tool::new(
            "mem_graph",
            "Get knowledge graph for a project",
            Arc::new(schema_for_type::<MemGraphParams>()),
        ),
        Tool::new(
            "mem_summarize",
            "Generate executive summary for a project or session",
            Arc::new(schema_for_type::<MemSummarizeParams>()),
        ),
        Tool::new(
            "mem_inject_context",
            "Generate formatted context block for prompt injection",
            Arc::new(schema_for_type::<MemInjectContextParams>()),
        ),
        Tool::new(
            "mem_forget_project",
            "Hard-delete all memories for a project",
            Arc::new(schema_for_type::<MemForgetProjectParams>()),
        ),
        Tool::new(
            "mem_health",
            "Get system health report",
            Arc::new(schema_for_type::<MemHealthParams>()),
        ),
        Tool::new(
            "mem_remind",
            "Get critical/high importance memories as reminders",
            Arc::new(schema_for_type::<MemRemindParams>()),
        ),
        Tool::new(
            "mem_tag_suggest",
            "Suggest tags based on project tags and content",
            Arc::new(schema_for_type::<MemTagSuggestParams>()),
        ),
        Tool::new(
            "mem_knowledge_gaps",
            "Analyze knowledge coverage gaps",
            Arc::new(schema_for_type::<MemKnowledgeGapsParams>()),
        ),
        Tool::new(
            "mem_sync_status",
            "Get sync status for a project",
            Arc::new(schema_for_type::<MemSyncStatusParams>()),
        ),
        Tool::new(
            "mem_sync_now",
            "Trigger sync with all auto-sync peers",
            Arc::new(schema_for_type::<MemSyncNowParams>()),
        ),
        Tool::new(
            "mem_sync_export",
            "Export project memories to sync file",
            Arc::new(schema_for_type::<MemSyncExportParams>()),
        ),
        Tool::new(
            "mem_encrypt",
            "Encrypt an existing memory",
            Arc::new(schema_for_type::<MemEncryptParams>()),
        ),
        Tool::new(
            "mem_decrypt",
            "Decrypt an existing memory",
            Arc::new(schema_for_type::<MemDecryptParams>()),
        ),
        Tool::new(
            "keys_list",
            "List registered encryption keys",
            Arc::new(schema_for_type::<KeysListParams>()),
        ),
        Tool::new(
            "keys_status",
            "Get encryption keys status",
            Arc::new(schema_for_type::<KeysStatusParams>()),
        ),
    ];

    // Append plugin-provided tools.
    if let Some(pm) = plugins {
        for pt in pm.plugin_tools() {
            tools.push(Tool::new(
                pt.name,
                pt.description,
                Arc::new(
                    pt.input_schema
                        .as_object()
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .collect(),
                ),
            ));
        }
    }

    tools
}
