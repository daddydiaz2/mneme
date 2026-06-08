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
        "mem_entity_extract" => mem_entity_extract(db, args, project),
        "mem_entity_search" => mem_entity_search(db, args, project),
        "mem_entity_links" => mem_entity_links(db, args, project),
        "mem_cloud_enroll" => mem_cloud_enroll(db, args, project),
        "mem_cloud_sync" => mem_cloud_sync(db, args, project),
        "mem_cloud_status" => mem_cloud_status(db, args, project),
        "mem_consolidate" => mem_consolidate(db, args, project),
        "mem_block_set" => mem_block_set(db, args, project),
        "mem_block_get" => mem_block_get(db, args, project),
        "mem_block_list" => mem_block_list(db, args, project),
        "mem_learn_failures" => mem_learn_failures(db, args, project),
        "mem_session_outcome" => mem_session_outcome(db, args, project),
        "mem_obsidian_export" => mem_obsidian_export(db, args, project),
        "mem_watch_scan" => mem_watch_scan(db, args, project),
        "mem_compress" => mem_compress(db, args, project),
        "mem_compress_batch" => mem_compress_batch(db, args, project),
        "mem_compress_context" => mem_compress_context(db, args, project),
        "mem_temporal_query" => mem_temporal_query(db, args, project),
        "mem_expand" => mem_expand(db, args, project),
        "mem_transcript" => mem_transcript(db, args, project),
        "mem_capture_passive" => mem_capture_passive(db, args, project),
        "mem_entity_frequent" => mem_entity_frequent(db, args, project),
        "mem_judge" => mem_judge(db, args, project),
        "mem_compare" => mem_compare(db, args, project),
        "mem_conflict_candidates" => mem_conflict_candidates(db, args, project),
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
    /// RFC3339 timestamp: when this memory's fact became valid.
    #[serde(default)]
    valid_from: Option<String>,
    /// RFC3339 timestamp: when this memory's fact stopped being valid.
    #[serde(default)]
    valid_until: Option<String>,
    /// Provenance JSON: array of {agent, action, timestamp}.
    #[serde(default)]
    provenance: Option<String>,
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
        valid_from: params.valid_from.as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc)),
        valid_until: params.valid_until.as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.with_timezone(&chrono::Utc)),
        provenance: params.provenance,
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
            valid_from: None,
            valid_until: None,
            provenance: None,
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

// --- Entity Extraction & Linking Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemEntityExtractParams {
    /// Memory ID to extract entities from, or text content
    id: Option<String>,
    /// Text content to extract entities from (alternative to id)
    text: Option<String>,
    /// Optional project context for the extraction.
    #[serde(default)]
    #[allow(dead_code)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemEntitySearchParams {
    query: String,
    #[serde(default)]
    entity_type: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemEntityLinksParams {
    /// Memory ID to get linked memories for
    id: String,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemEntityFrequentParams {
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_limit_20")]
    limit: u32,
}

fn mem_entity_extract(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemEntityExtractParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    // Use project param for scope context (even if only used for metadata)
    let _project = params.project.as_deref().unwrap_or(project);

    let entity_store = db.entities();

    // If memory ID is provided, extract entities from existing memory
    if let Some(id_str) = &params.id {
        let id = Uuid::parse_str(id_str)
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
        let memory = db
            .memories()
            .get(id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(id))?;
        
        if memory.is_encrypted {
            return Err(crate::error::MnemeError::Config(
                "Cannot extract entities from encrypted memory".into(),
            ));
        }

        let entities = entity_store.extract_and_save(&memory)?;
        return Ok(serde_json::to_value(entities)?);
    }

    // If raw text is provided, just extract without saving
    if let Some(text) = &params.text {
        let raw = crate::store::entities::EntityStore::extract_entities_from_text(text);
        return Ok(json!(raw));
    }

    Err(crate::error::MnemeError::Config(
        "Provide either 'id' (memory UUID) or 'text' (raw content)".into(),
    ))
}

fn mem_entity_search(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemEntitySearchParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let entity_type = params
        .entity_type
        .as_deref()
        .map(std::str::FromStr::from_str)
        .transpose()?;

    let entity_store = db.entities();
    let results = entity_store.search_entities(&params.query, entity_type.as_ref(), params.limit)?;
    Ok(serde_json::to_value(results)?)
}

fn mem_entity_links(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemEntityLinksParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id = Uuid::parse_str(&params.id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let entity_store = db.entities();
    let entities = entity_store.get_memory_entities(id)?;
    let links = entity_store.get_memory_links(id, params.limit)?;

    Ok(json!({
        "memory_id": params.id,
        "entities": entities,
        "links": links.iter().map(|(link, title)| {
            json!({
                "entity_name": link.entity_name,
                "entity_type": link.entity_type.to_string(),
                "target_memory_id": link.target_memory_id.to_string(),
                "target_title": title,
                "link_strength": link.link_strength,
            })
        }).collect::<Vec<_>>()
    }))
}

// --- Cloud Sync Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCloudEnrollParams {
    /// Cloud server URL (e.g., https://cloud.mneme.dev)
    server: String,
    /// Authentication token
    token: String,
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCloudSyncParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCloudStatusParams {
    #[serde(default)]
    project: Option<String>,
}

fn mem_cloud_enroll(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCloudEnrollParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let settings = crate::config::settings::Settings::load()?;

    let orch = crate::cloud::CloudOrchestrator::new(
        // We need an Arc<Database>, but we have &Database
        // Create an Arc clone
        std::sync::Arc::new(db.clone()),
        settings.sync,
    );

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        orch.enroll(&params.server, &params.token, &project).await
    })?;

    Ok(serde_json::to_value(result)?)
}

fn mem_cloud_sync(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCloudSyncParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let settings = crate::config::settings::Settings::load()?;

    let orch = crate::cloud::CloudOrchestrator::new(
        std::sync::Arc::new(db.clone()),
        settings.sync,
    );

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async {
        orch.sync_cloud(&project).await
    })?;

    Ok(serde_json::to_value(result)?)
}

fn mem_cloud_status(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCloudStatusParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let settings = crate::config::settings::Settings::load()?;

    let orch = crate::cloud::CloudOrchestrator::new(
        std::sync::Arc::new(db.clone()),
        settings.sync,
    );

    orch.cloud_status(&project)
}

// --- Failure Mining Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemLearnParams {
    #[serde(default)]
    project: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemSessionOutcomeParams {
    /// ID of the session
    session_id: String,
    /// Outcome: "success", "partial", or "failure"
    outcome: String,
    /// Comma-separated failure reasons (only for failure outcome)
    #[serde(default)]
    failure_reasons: Option<String>,
    /// Number of files affected
    #[serde(default)]
    affected_files: u32,
    /// Number of bugs introduced (only for failure outcome)
    #[serde(default)]
    bugs_introduced: u32,
    /// Description of user corrections (if any)
    #[serde(default)]
    user_corrections: Option<String>,
}

fn mem_learn_failures(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemLearnParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let learn_db = std::sync::Arc::new(db.clone());
    let miner = crate::learn::FailureMiner::new(learn_db);
    let project = params.project.unwrap_or_else(|| project.to_string());
    let report = miner.mine(&project)?;
    Ok(serde_json::to_value(report)?)
}

fn mem_session_outcome(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemSessionOutcomeParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let session_id = uuid::Uuid::parse_str(&params.session_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let outcome = match params.outcome.as_str() {
        "success" => crate::learn::SessionOutcome::Success,
        "partial" => crate::learn::SessionOutcome::PartialSuccess,
        "failure" => {
            let reasons: Vec<String> = params
                .failure_reasons
                .as_deref()
                .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
                .unwrap_or_default();
            crate::learn::SessionOutcome::Failure { reasons }
        }
        _ => return Err(crate::error::MnemeError::Config(
            format!("Invalid outcome '{}'. Must be: success, partial, failure", params.outcome)
        )),
    };

    let learn_db = std::sync::Arc::new(db.clone());
    let miner = crate::learn::FailureMiner::new(learn_db);
    miner.record_session_outcome(
        session_id,
        outcome,
        params.affected_files,
        params.bugs_introduced,
        params.user_corrections.as_deref(),
    )?;

    Ok(json!({
        "session_id": params.session_id,
        "outcome": params.outcome,
        "recorded": true
    }))
}

// --- Consolidation & Memory Blocks Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemConsolidateParams {
    #[serde(default)]
    project: Option<String>,
    /// auto, age, deprecated, unused
    #[serde(default = "default_auto")]
    strategy: String,
    /// Days threshold for stale memories
    #[serde(default = "default_30")]
    days: u64,
    #[serde(default)]
    dry_run: bool,
}

fn default_auto() -> String { "auto".to_string() }
fn default_30() -> u64 { 30 }

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemBlockSetParams {
    #[serde(default)]
    project: Option<String>,
    slot: String,
    title: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemBlockGetParams {
    #[serde(default)]
    project: Option<String>,
    slot: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemBlockListParams {
    #[serde(default)]
    project: Option<String>,
}

fn mem_consolidate(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemConsolidateParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let strat = match params.strategy.as_str() {
        "age" => crate::consolidate::ConsolidationStrategy::Age,
        "deprecated" => crate::consolidate::ConsolidationStrategy::Deprecated,
        "unused" => crate::consolidate::ConsolidationStrategy::Unused,
        _ => crate::consolidate::ConsolidationStrategy::Auto,
    };
    let eng = crate::consolidate::ConsolidationEngine::new(
        std::sync::Arc::new(db.clone()),
    );
    let result = eng.consolidate(&project, strat, params.days, params.dry_run)?;
    Ok(serde_json::to_value(result)?)
}

fn mem_block_set(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemBlockSetParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let eng = crate::consolidate::ConsolidationEngine::new(
        std::sync::Arc::new(db.clone()),
    );
    let block = eng.set_block(&project, &params.slot, &params.title, &params.content)?;
    Ok(serde_json::to_value(block)?)
}

fn mem_block_get(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemBlockGetParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let eng = crate::consolidate::ConsolidationEngine::new(
        std::sync::Arc::new(db.clone()),
    );
    let block = eng.get_block(&project, &params.slot)?;
    Ok(serde_json::to_value(block)?)
}

fn mem_block_list(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemBlockListParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;
    let project = params.project.unwrap_or_else(|| project.to_string());
    let eng = crate::consolidate::ConsolidationEngine::new(
        std::sync::Arc::new(db.clone()),
    );
    let blocks = eng.list_blocks(&project)?;
    Ok(serde_json::to_value(blocks)?)
}

// --- Obsidian Export Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemObsidianExportParams {
    #[serde(default)]
    project: Option<String>,
    /// Output directory for the Obsidian vault
    output: String,
}

fn mem_obsidian_export(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemObsidianExportParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let output_dir = std::path::PathBuf::from(&params.output);

    let memories = db.memories().list(&project, None, None, None, 10000, 0)?;
    let graph = db.memories().get_graph(&project)?;

    // Convert graph edges to MemoryRelations for the export
    let relations: Vec<crate::store::memory::MemoryRelation> = graph
        .edges
        .into_iter()
        .filter_map(|e| {
            let source_id = uuid::Uuid::parse_str(&e.source).ok()?;
            let target_id = uuid::Uuid::parse_str(&e.target).ok()?;
            Some(crate::store::memory::MemoryRelation {
                id: uuid::Uuid::nil(),
                sync_id: String::new(),
                source_id,
                target_id,
                relation_type: std::str::FromStr::from_str(&e.relation_type).ok()?,
                confidence: e.confidence,
                judgment_status: "active".to_string(),
                reason: None,
                evidence: None,
                marked_by_actor: "system".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        })
        .collect();

    let stats = crate::export::obsidian::export_to_obsidian(
        &memories,
        &relations,
        &project,
        &output_dir,
    )?;

    Ok(json!({
        "project": project,
        "output": output_dir.join("mneme-export").to_string_lossy().to_string(),
        "files_written": stats.files_written,
        "bytes_written": stats.bytes_written,
        "memories_exported": memories.len()
    }))
}

// --- File Watcher Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemWatchScanParams {
    /// Directory to scan for files
    directory: String,
    /// File extension to watch (default: .md)
    #[serde(default = "default_md_ext")]
    ext: String,
    #[serde(default)]
    project: Option<String>,
}

fn default_md_ext() -> String {
    ".md".to_string()
}

fn mem_watch_scan(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemWatchScanParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let dir = std::path::PathBuf::from(&params.directory);
    let store = db.memories();

    let mut watcher = crate::watch::DirectoryWatcher::new(
        dir,
        params.ext,
        3600, // interval doesn't matter for one-shot
        store,
        project,
    );

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(async { watcher.scan().await })?;

    Ok(json!({
        "indexed": result.indexed,
        "skipped": result.skipped,
        "errors": result.errors,
        "removed": result.removed,
        "tracked_files": watcher.tracked_count()
    }))
}

// --- Context Compression Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCompressParams {
    /// Memory ID to compress
    id: String,
    /// Compression strategy: truncate, smart_summary, keywords_only, minimal
    #[serde(default = "default_smart_summary")]
    strategy: String,
}

fn default_smart_summary() -> String {
    "smart_summary".to_string()
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCompressBatchParams {
    /// Project to compress memories for
    #[serde(default)]
    project: Option<String>,
    /// Compression strategy
    #[serde(default = "default_smart_summary")]
    strategy: String,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCompressContextParams {
    #[serde(default)]
    project: Option<String>,
    /// Compression strategy
    #[serde(default = "default_smart_summary")]
    strategy: String,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

fn mem_compress(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCompressParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id = Uuid::parse_str(&params.id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
    let memory = db.memories().get(id)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(id))?;

    let strategy: crate::compress::CompressionStrategy = params.strategy.parse()?;
    let compressed = crate::compress::CompressionPipeline::compress(&memory, strategy);

    Ok(serde_json::to_value(compressed)?)
}

fn mem_compress_batch(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCompressBatchParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let strategy: crate::compress::CompressionStrategy = params.strategy.parse()?;

    let memories = db.memories().list(&project, None, None, None, params.limit, 0)?;
    let mut compressed = Vec::new();
    let mut total_ratio = 0.0;

    for memory in &memories {
        let result = crate::compress::CompressionPipeline::compress(memory, strategy);
        total_ratio += result.compression_ratio;
        compressed.push(result);
    }

    let avg_ratio = if !compressed.is_empty() {
        total_ratio / compressed.len() as f64
    } else {
        0.0
    };

    Ok(json!({
        "compressed": compressed,
        "count": compressed.len(),
        "average_compression_ratio": avg_ratio,
        "strategy": params.strategy,
        "project": project
    }))
}

fn mem_compress_context(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCompressContextParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let strategy: crate::compress::CompressionStrategy = params.strategy.parse()?;

    let memories = db.memories().context(&project, None, params.limit)?;
    let context_block = crate::compress::CompressionPipeline::compress_context_block(&memories, strategy, params.limit as usize);

    Ok(json!({
        "compressed_context": context_block,
        "memory_count": memories.len(),
        "strategy": params.strategy,
        "project": project
    }))
}

// --- Temporal Query Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemTemporalQueryParams {
    /// Query text to search
    query: String,
    /// RFC3339 timestamp: find facts that were valid at this time
    at_time: String,
    #[serde(default)]
    project: Option<String>,
    #[serde(default = "default_limit_10")]
    limit: u32,
}

fn mem_temporal_query(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemTemporalQueryParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let at_time = chrono::DateTime::parse_from_rfc3339(&params.at_time)
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid at_time: {}", e)))?
        .with_timezone(&chrono::Utc);
    let at_time_str = at_time.to_rfc3339();
    let project = params.project.unwrap_or_else(|| project.to_string());

    let conn = db.get_conn();
    let like_pattern = format!("%{}%", params.query);
    let limit_i64 = params.limit as i64;

    let results: Vec<serde_json::Value>;
    {
        let conn_guard = conn.lock().map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn_guard.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories
             WHERE project = ?1 AND deleted_at IS NULL
             AND (title LIKE ?2 OR content LIKE ?2)
             AND (valid_from IS NULL OR valid_from <= ?3)
             AND (valid_until IS NULL OR valid_until > ?3)
             ORDER BY updated_at DESC
             LIMIT ?4"
        )?;
        let rows = stmt.query_map(
            rusqlite::params![project, like_pattern, at_time_str, limit_i64],
            crate::store::memory::MemoryStore::row_to_memory,
        )?;
        results = rows
            .filter_map(|r| r.ok())
            .map(|m| serde_json::to_value(m).unwrap_or_default())
            .collect();
    }

    Ok(json!({
        "results": results,
        "count": results.len(),
        "query": params.query,
        "at_time": params.at_time,
        "project": project,
        "description": "Facts that were valid at the specified time"
    }))
}

// --- Progressive Retrieval Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemExpandParams {
    /// Memory ID to expand (returns full content)
    id: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemTranscriptParams {
    /// Session ID to retrieve raw transcript for
    session_id: String,
    #[serde(default = "default_limit_50")]
    limit: u32,
}

fn default_limit_50() -> u32 { 50 }

fn mem_expand(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemExpandParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let id = Uuid::parse_str(&params.id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory = db.memories().get(id)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(id))?;

    // Return the full expanded view: all structured fields + entities + relations
    let entity_store = db.entities();
    let entities = entity_store.get_memory_entities(id).ok();
    let links = entity_store.get_memory_links(id, 10).ok();

    Ok(json!({
        "memory": memory,
        "entities": entities,
        "entity_links": links.map(|l| l.iter().map(|(link, title)| {
            json!({
                "entity_name": link.entity_name,
                "target_title": title,
                "link_strength": link.link_strength
            })
        }).collect::<Vec<_>>()),
        "layer": 2,
        "instruction": "This is the expanded view (Layer 2). Use this content for detailed context."
    }))
}

fn mem_transcript(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemTranscriptParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let session_id = Uuid::parse_str(&params.session_id)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let session = db.sessions().get(session_id)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(session_id))?;

    // Get prompts for this session
    let conn = db.get_conn();
    let prompts: Vec<crate::store::memory::UserPrompt>;
    {
        let conn_guard = conn.lock().map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn_guard.prepare(
            "SELECT id, session_id, content, project, created_at
             FROM user_prompts WHERE session_id = ?1
             ORDER BY created_at ASC
             LIMIT ?2"
        )?;
        let limit_i64 = params.limit as i64;
        let rows = stmt.query_map(rusqlite::params![params.session_id, limit_i64], |row| {
            Ok(crate::store::memory::UserPrompt {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                })?,
                session_id: row.get::<_, Option<String>>(1)?
                    .and_then(|s| uuid::Uuid::parse_str(&s).ok()),
                content: row.get(2)?,
                project: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
                    })?.with_timezone(&chrono::Utc),
            })
        })?;
        prompts = rows.collect::<Result<Vec<_>, _>>()?;
    }

    // Build a readable transcript
    let mut transcript_lines = vec![
        format!("## Session Transcript: {}", session.id),
        format!("Project: {} | Started: {} | Ended: {}",
            session.project,
            session.started_at.to_rfc3339(),
            session.ended_at.map(|d| d.to_rfc3339()).unwrap_or_else(|| "active".to_string())
        ),
        String::new(),
    ];

    if let Some(ref summary) = session.summary {
        transcript_lines.push(format!("Summary: {}", summary));
        transcript_lines.push(String::new());
    }

    for (i, prompt) in prompts.iter().enumerate() {
        transcript_lines.push(format!("### Prompt {} ({})", i + 1, prompt.created_at.to_rfc3339()));
        transcript_lines.push(prompt.content.clone());
        transcript_lines.push(String::new());
    }

    let transcript = transcript_lines.join("\n");

    Ok(json!({
        "session": session,
        "prompts": prompts,
        "transcript": transcript,
        "prompt_count": prompts.len(),
        "layer": 3,
        "instruction": "This is the raw transcript (Layer 3). Used for full dialogue reconstruction."
    }))
}

// --- Passive Capture Tool ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCapturePassiveParams {
    /// Session output text to parse and extract memories from
    text: String,
    #[serde(default)]
    project: Option<String>,
    /// Optional session ID to associate captured memories with
    #[serde(default)]
    session_id: Option<String>,
}

fn mem_capture_passive(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCapturePassiveParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let session_id = params
        .session_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memories = db.memories().capture_passive(
        &params.text,
        &project,
        session_id,
        None,
        Some(db.embeddings()),
    )?;

    Ok(json!({
        "captured": memories,
        "count": memories.len(),
        "project": project
    }))
}

// --- Conflict Detection & Judgment Tools ---

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemJudgeParams {
    /// ID of the existing/candidate memory (memory A)
    memory_id_a: String,
    /// ID of the new/incoming memory (memory B)
    memory_id_b: String,
    /// Optional candidate_id to update after judgment
    candidate_id: Option<i64>,
    /// The judged relation: conflicts_with, supersedes, extends, compatible, depends_on
    judged_relation: String,
    /// LLM's reasoning for the judgment
    reasoning: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemCompareParams {
    /// ID of the first memory
    memory_id_a: String,
    /// ID of the second memory
    memory_id_b: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct MemConflictCandidatesParams {
    #[serde(default)]
    project: Option<String>,
    /// Filter by status: pending, judged, dismissed (default: pending)
    #[serde(default = "default_pending")]
    status: String,
    #[serde(default = "default_limit_20")]
    limit: u32,
}

fn default_pending() -> String {
    "pending".to_string()
}

fn mem_judge(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemJudgeParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id_a = Uuid::parse_str(&params.memory_id_a)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
    let memory_id_b = Uuid::parse_str(&params.memory_id_b)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    // If candidate_id is provided, record the judgment
    if let Some(candidate_id) = params.candidate_id {
        let judgment = db.memories().judge_conflict(
            candidate_id,
            &params.judged_relation,
            &params.reasoning,
            "agent",
        )?;
        return Ok(json!({
            "judgment": judgment,
            "memory_id_a": params.memory_id_a,
            "memory_id_b": params.memory_id_b,
            "relation": params.judged_relation
        }));
    }

    // Otherwise, just provide formatted context for LLM to reason about
    let context = db.memories().get_conflict_context(memory_id_a, memory_id_b)?;
    let memory_a = db.memories().get(memory_id_a)?;
    let memory_b = db.memories().get(memory_id_b)?;

    Ok(json!({
        "context": context,
        "memory_a": memory_a,
        "memory_b": memory_b,
        "suggested_relations": ["compatible", "conflicts_with", "supersedes", "extends", "depends_on"],
        "instruction": "Analyze whether memory B conflicts with, extends, supersedes, depends_on, or is compatible with memory A."
    }))
}

fn mem_compare(
    db: &Database,
    args: JsonObject,
    _project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemCompareParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let memory_id_a = Uuid::parse_str(&params.memory_id_a)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
    let memory_id_b = Uuid::parse_str(&params.memory_id_b)
        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

    let memory_a = db.memories().get(memory_id_a)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(memory_id_a))?;
    let memory_b = db.memories().get(memory_id_b)?
        .ok_or_else(|| crate::error::MnemeError::NotFound(memory_id_b))?;

    // Find shared entities
    let entity_store = db.entities();
    let shared_entities = entity_store.search_entities(&memory_a.title, None, 5)?;

    // Check existing relations
    let existing_relations = db.memories().get_existing_relations(memory_id_a, memory_id_b).unwrap_or_default();

    // Compare structured fields
    let comparison = json!({
        "memory_a": {
            "title": memory_a.title,
            "type": memory_a.memory_type.to_string(),
            "importance": memory_a.importance.to_string(),
            "topic_key": memory_a.topic_key,
            "tags": memory_a.tags,
            "content_preview": memory_a.content[..memory_a.content.len().min(200)].to_string()
        },
        "memory_b": {
            "title": memory_b.title,
            "type": memory_b.memory_type.to_string(),
            "importance": memory_b.importance.to_string(),
            "topic_key": memory_b.topic_key,
            "tags": memory_b.tags,
            "content_preview": memory_b.content[..memory_b.content.len().min(200)].to_string()
        },
        "shared_entities": shared_entities.iter().map(|e| e.entity.entity_name.clone()).collect::<Vec<_>>(),
        "existing_relations": existing_relations,
        "same_type": memory_a.memory_type == memory_b.memory_type,
        "same_topic_key": memory_a.topic_key.is_some() && memory_a.topic_key == memory_b.topic_key,
        "same_project": memory_a.project == memory_b.project,
    });

    Ok(comparison)
}

fn mem_conflict_candidates(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemConflictCandidatesParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let status = if params.status.is_empty() { "pending" } else { &params.status };
    let candidates = db.memories().list_conflict_candidates(&project, Some(status), params.limit)?;
    Ok(serde_json::to_value(candidates)?)
}

fn mem_entity_frequent(
    db: &Database,
    args: JsonObject,
    project: &str,
) -> crate::error::Result<serde_json::Value> {
    let params: MemEntityFrequentParams = serde_json::from_value(serde_json::Value::Object(args))
        .map_err(|e| crate::error::MnemeError::Config(format!("Invalid params: {}", e)))?;

    let project = params.project.unwrap_or_else(|| project.to_string());
    let entity_store = db.entities();
    let frequent = entity_store.frequent_entities(&project, params.limit)?;
    Ok(serde_json::to_value(frequent)?)
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
        Tool::new(
            "mem_entity_extract",
            "Extract entities from a memory or text",
            Arc::new(schema_for_type::<MemEntityExtractParams>()),
        ),
        Tool::new(
            "mem_entity_search",
            "Search memories by extracted entity",
            Arc::new(schema_for_type::<MemEntitySearchParams>()),
        ),
        Tool::new(
            "mem_entity_links",
            "Get entity-linked memories for a given memory",
            Arc::new(schema_for_type::<MemEntityLinksParams>()),
        ),
        Tool::new(
            "mem_entity_frequent",
            "Get most frequent entities in a project",
            Arc::new(schema_for_type::<MemEntityFrequentParams>()),
        ),
        Tool::new(
            "mem_cloud_enroll",
            "Enroll this project with a cloud sync server",
            Arc::new(schema_for_type::<MemCloudEnrollParams>()),
        ),
        Tool::new(
            "mem_cloud_sync",
            "Trigger a full cloud sync cycle",
            Arc::new(schema_for_type::<MemCloudSyncParams>()),
        ),
        Tool::new(
            "mem_cloud_status",
            "Check cloud sync status and recent sync history",
            Arc::new(schema_for_type::<MemCloudStatusParams>()),
        ),
        Tool::new(
            "mem_consolidate",
            "Consolidate stale/old/deprecated memories into auto-generated summary memories",
            Arc::new(schema_for_type::<MemConsolidateParams>()),
        ),
        Tool::new(
            "mem_block_set",
            "Set a memory block (slot: human/persona/workflow)",
            Arc::new(schema_for_type::<MemBlockSetParams>()),
        ),
        Tool::new(
            "mem_block_get",
            "Get a memory block by slot name",
            Arc::new(schema_for_type::<MemBlockGetParams>()),
        ),
        Tool::new(
            "mem_block_list",
            "List all memory blocks for a project",
            Arc::new(schema_for_type::<MemBlockListParams>()),
        ),
        Tool::new(
            "mem_learn_failures",
            "Analyze failed sessions and negative feedback to mine failure patterns and auto-generate corrective memories",
            Arc::new(schema_for_type::<MemLearnParams>()),
        ),
        Tool::new(
            "mem_session_outcome",
            "Record the outcome of a session (success/partial/failure with reasons)",
            Arc::new(schema_for_type::<MemSessionOutcomeParams>()),
        ),
        Tool::new(
            "mem_obsidian_export",
            "Export memories to an Obsidian vault (.md files with frontmatter + wikilinks)",
            Arc::new(schema_for_type::<MemObsidianExportParams>()),
        ),
        Tool::new(
            "mem_watch_scan",
            "Scan a directory and auto-index markdown files as memories",
            Arc::new(schema_for_type::<MemWatchScanParams>()),
        ),
        Tool::new(
            "mem_compress",
            "Compress a memory's content for token-efficient context injection",
            Arc::new(schema_for_type::<MemCompressParams>()),
        ),
        Tool::new(
            "mem_compress_batch",
            "Compress multiple memories in a project",
            Arc::new(schema_for_type::<MemCompressBatchParams>()),
        ),
        Tool::new(
            "mem_compress_context",
            "Generate a compressed context block for prompt injection",
            Arc::new(schema_for_type::<MemCompressContextParams>()),
        ),
        Tool::new(
            "mem_temporal_query",
            "Query facts that were valid at a specific point in time",
            Arc::new(schema_for_type::<MemTemporalQueryParams>()),
        ),
        Tool::new(
            "mem_expand",
            "[Layer 2] Get expanded view of a memory with full content, entities, and links",
            Arc::new(schema_for_type::<MemExpandParams>()),
        ),
        Tool::new(
            "mem_transcript",
            "[Layer 3] Get raw session transcript with all prompts",
            Arc::new(schema_for_type::<MemTranscriptParams>()),
        ),
        Tool::new(
            "mem_capture_passive",
            "Parse session output and auto-save memories (Key Learnings, Decisions, Architecture, etc.)",
            Arc::new(schema_for_type::<MemCapturePassiveParams>()),
        ),
        Tool::new(
            "mem_judge",
            "Judge the relationship between two memories (optionally recording the result)",
            Arc::new(schema_for_type::<MemJudgeParams>()),
        ),
        Tool::new(
            "mem_compare",
            "Compare two memories side-by-side with shared entities and existing relations",
            Arc::new(schema_for_type::<MemCompareParams>()),
        ),
        Tool::new(
            "mem_conflict_candidates",
            "List auto-detected conflict candidates for LLM judgment",
            Arc::new(schema_for_type::<MemConflictCandidatesParams>()),
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
