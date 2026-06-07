use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Categoría de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Architecture,
    Decision,
    Bugfix,
    Pattern,
    Convention,
    Dependency,
    Workflow,
    Note,
    Config,
    Discovery,
    Learning,
    /// Fact generated autonomously by an AI agent (not directly created by user).
    AgentFact,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MemoryType::Architecture => "architecture",
            MemoryType::Decision => "decision",
            MemoryType::Bugfix => "bugfix",
            MemoryType::Pattern => "pattern",
            MemoryType::Convention => "convention",
            MemoryType::Dependency => "dependency",
            MemoryType::Workflow => "workflow",
            MemoryType::Note => "note",
            MemoryType::Config => "config",
            MemoryType::Discovery => "discovery",
            MemoryType::Learning => "learning",
            MemoryType::AgentFact => "agent_fact",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for MemoryType {
    type Err = crate::error::MnemeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "architecture" => Ok(MemoryType::Architecture),
            "decision" => Ok(MemoryType::Decision),
            "bugfix" => Ok(MemoryType::Bugfix),
            "pattern" => Ok(MemoryType::Pattern),
            "convention" => Ok(MemoryType::Convention),
            "dependency" => Ok(MemoryType::Dependency),
            "workflow" => Ok(MemoryType::Workflow),
            "note" => Ok(MemoryType::Note),
            "config" => Ok(MemoryType::Config),
            "discovery" => Ok(MemoryType::Discovery),
            "learning" => Ok(MemoryType::Learning),
            "agent_fact" | "agentfact" | "agent-fact" => Ok(MemoryType::AgentFact),
            other => Err(crate::error::MnemeError::InvalidMemoryType(
                other.to_string(),
            )),
        }
    }
}

/// Nivel de importancia de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Low,
    Medium,
    High,
    Critical,
}

impl Importance {
    /// Factor de ponderación para el algoritmo de relevancia.
    pub fn boost_factor(&self) -> f64 {
        match self {
            Importance::Low => 0.7,
            Importance::Medium => 1.0,
            Importance::High => 1.5,
            Importance::Critical => 2.0,
        }
    }
}

impl std::fmt::Display for Importance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Importance::Low => "low",
            Importance::Medium => "medium",
            Importance::High => "high",
            Importance::Critical => "critical",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for Importance {
    type Err = crate::error::MnemeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Importance::Low),
            "medium" => Ok(Importance::Medium),
            "high" => Ok(Importance::High),
            "critical" => Ok(Importance::Critical),
            other => Err(crate::error::MnemeError::InvalidImportance(
                other.to_string(),
            )),
        }
    }
}

/// Alcance de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Project,
    Personal,
    Global,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Scope::Project => "project",
            Scope::Personal => "personal",
            Scope::Global => "global",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for Scope {
    type Err = crate::error::MnemeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(Scope::Project),
            "personal" => Ok(Scope::Personal),
            "global" => Ok(Scope::Global),
            other => Err(crate::error::MnemeError::InvalidScope(other.to_string())),
        }
    }
}

/// Tipo de relación entre memorias.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    SupersededBy,
    Supersedes,
    ConflictsWith,
    Extends,
    DependsOn,
    RelatedTo,
    Compatible,
    Scoped,
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RelationType::SupersededBy => "superseded_by",
            RelationType::Supersedes => "supersedes",
            RelationType::ConflictsWith => "conflicts_with",
            RelationType::Extends => "extends",
            RelationType::DependsOn => "depends_on",
            RelationType::RelatedTo => "related_to",
            RelationType::Compatible => "compatible",
            RelationType::Scoped => "scoped",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for RelationType {
    type Err = crate::error::MnemeError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "superseded_by" => Ok(RelationType::SupersededBy),
            "supersedes" => Ok(RelationType::Supersedes),
            "conflicts_with" => Ok(RelationType::ConflictsWith),
            "extends" => Ok(RelationType::Extends),
            "depends_on" => Ok(RelationType::DependsOn),
            "related_to" => Ok(RelationType::RelatedTo),
            "compatible" => Ok(RelationType::Compatible),
            "scoped" => Ok(RelationType::Scoped),
            other => Err(crate::error::MnemeError::InvalidRelationType(
                other.to_string(),
            )),
        }
    }
}

/// Representa una memoria persistente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Uuid,
    pub project: String,
    pub scope: Scope,
    pub title: String,
    pub content: String,
    pub what: Option<String>,
    pub why: Option<String>,
    pub context: Option<String>,
    pub learned: Option<String>,
    pub memory_type: MemoryType,
    pub importance: Importance,
    pub tags: Vec<String>,
    pub topic_key: Option<String>,
    pub access_count: u32,
    pub revision_count: u32,
    pub duplicate_count: u32,
    pub normalized_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deprecated_at: Option<DateTime<Utc>>,
    pub deprecated_reason: Option<String>,
    pub supersedes_id: Option<String>,
    pub context_inject_count: u32,
    pub origin_peer: Option<String>,
    pub is_encrypted: bool,
    pub encrypted_for: Option<String>,
    /// When this memory's fact became valid (temporal window start).
    pub valid_from: Option<DateTime<Utc>>,
    /// When this memory's fact stopped being valid (temporal window end).
    pub valid_until: Option<DateTime<Utc>>,
    /// Provenance chain: JSON array of {agent, action, timestamp} describing how this fact was derived.
    pub provenance: Option<String>,
}

/// Relación entre dos memorias.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRelation {
    pub id: Uuid,
    pub sync_id: String,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relation_type: RelationType,
    pub confidence: f32,
    pub judgment_status: String,
    pub reason: Option<String>,
    pub evidence: Option<String>,
    pub marked_by_actor: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Sesión de trabajo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub project: String,
    pub directory: Option<String>,
    pub summary: Option<String>,
    pub memory_ids: Vec<Uuid>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub status: String,
}

/// Resultado de una búsqueda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub memory: Memory,
    pub score: f64,
    pub snippet: Option<String>,
    pub match_type: MatchType,
    pub cosine_score: Option<f32>,
}

/// Tipo de coincidencia en búsqueda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MatchType {
    Fts,
    Fuzzy,
    Exact,
    Semantic,
}

/// Estadísticas de un proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub project: String,
    pub total_memories: u32,
    pub by_type: std::collections::HashMap<String, u32>,
    pub by_importance: std::collections::HashMap<String, u32>,
    pub by_scope: std::collections::HashMap<String, u32>,
    pub total_relations: u32,
    pub total_sessions: u32,
    pub total_prompts: u32,
    pub oldest_memory: Option<DateTime<Utc>>,
    pub newest_memory: Option<DateTime<Utc>>,
    pub most_accessed: Option<String>,
}

/// Resumen de un proyecto.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub memory_count: u32,
    pub session_count: u32,
    pub last_activity: Option<DateTime<Utc>>,
}

/// Entrada para crear una memoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMemoryInput {
    pub project: String,
    pub scope: Option<Scope>,
    pub title: String,
    pub content: String,
    pub what: Option<String>,
    pub why: Option<String>,
    pub context: Option<String>,
    pub learned: Option<String>,
    pub memory_type: MemoryType,
    pub importance: Importance,
    pub tags: Vec<String>,
    pub topic_key: Option<String>,
    pub capture_prompt: Option<bool>,
    #[serde(default)]
    pub encrypt: bool,
    /// Opcional: when this fact becomes valid (defaults to created_at).
    pub valid_from: Option<DateTime<Utc>>,
    /// Opcional: when this fact stops being valid.
    pub valid_until: Option<DateTime<Utc>>,
    /// Opcional: provenance chain JSON.
    pub provenance: Option<String>,
}

/// Entrada para actualizar una memoria.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateMemoryInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub what: Option<String>,
    pub why: Option<String>,
    pub context: Option<String>,
    pub learned: Option<String>,
    pub memory_type: Option<MemoryType>,
    pub importance: Option<Importance>,
    pub tags: Option<Vec<String>>,
    pub scope: Option<Scope>,
    pub topic_key: Option<String>,
}

/// Entrada para crear una relación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationInput {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub relation_type: RelationType,
    pub confidence: Option<f32>,
    pub reason: Option<String>,
}

/// Consulta de búsqueda.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub project: Option<String>,
    pub scope: Option<Scope>,
    pub memory_type: Option<MemoryType>,
    pub importance: Option<Importance>,
    pub tags: Vec<String>,
    pub limit: u32,
    pub include_snippet: bool,
    pub all_projects: bool,
}

/// Prompt de usuario registrado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPrompt {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub content: String,
    pub project: String,
    pub created_at: DateTime<Utc>,
}

/// Entrada para crear un prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromptInput {
    pub session_id: Option<Uuid>,
    pub content: String,
    pub project: String,
}

/// Estadísticas de reindexación de embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexStats {
    /// Total de memorias procesadas.
    pub total: u32,
    /// Memorias indexadas exitosamente.
    pub indexed: u32,
    /// Memorias omitidas (ya tenían embedding).
    pub skipped: u32,
    /// Memorias que fallaron.
    pub failed: u32,
    /// Duración en milisegundos.
    pub duration_ms: u64,
}

/// Resultado de una auditoría de calidad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Memorias sin acceso reciente.
    pub stale_memories: Vec<Memory>,
    /// Memorias sin tags.
    pub untagged_memories: Vec<Memory>,
    /// Memorias con contenido muy corto.
    pub short_memories: Vec<Memory>,
    /// Distribución por tipo.
    pub type_distribution: HashMap<String, u32>,
    /// Promedio de revisiones.
    pub average_revisions: f64,
    /// Cantidad de grupos con duplicados.
    pub duplicate_groups: u32,
}

/// Grupo de memorias duplicadas semánticamente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// IDs de memorias en el grupo.
    pub memory_ids: Vec<String>,
    /// Títulos de memorias en el grupo.
    pub titles: Vec<String>,
    /// Score coseno máximo dentro del grupo.
    pub cosine_score: f32,
}

/// Nodo del grafo de conocimiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// ID de la memoria.
    pub id: String,
    /// Título de la memoria.
    pub title: String,
    /// Tipo de memoria.
    pub memory_type: String,
    /// Nivel de importancia.
    pub importance: String,
}

/// Arista del grafo de conocimiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// ID origen.
    pub source: String,
    /// ID destino.
    pub target: String,
    /// Tipo de relación.
    pub relation_type: String,
    /// Confianza de la relación.
    pub confidence: f32,
}

/// Datos del grafo de conocimiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    /// Nodos del grafo.
    pub nodes: Vec<GraphNode>,
    /// Aristas del grafo.
    pub edges: Vec<GraphEdge>,
}

/// Resultado de un resumen ejecutivo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryResult {
    /// Texto del resumen.
    pub summary: String,
    /// Cantidad de memorias.
    pub memory_count: u32,
    /// Conteo por tipo.
    pub by_type: HashMap<String, u32>,
}

/// Reporte de salud del sistema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Tamaño de la base de datos en MB.
    pub db_size_mb: f64,
    /// Total de memorias.
    pub total_memories: u32,
    /// Memorias huérfanas.
    pub orphaned_memories: u32,
    /// Memorias sin embedding.
    pub unindexed_embeddings: u32,
    /// Última sincronización.
    pub last_sync: Option<String>,
    /// Modelo de embeddings.
    pub embedding_model: String,
    /// Versión de mneme.
    pub version: String,
}

/// Brecha de conocimiento detectada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGap {
    /// Área subrepresentada.
    pub area: String,
    /// Cantidad de memorias en el área.
    pub count: u32,
    /// Sugerencia para cubrir el gap.
    pub suggestion: String,
}

/// Reporte de brechas de conocimiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGapsReport {
    /// Lista de brechas.
    pub gaps: Vec<KnowledgeGap>,
    /// Score de cobertura (0-1).
    pub coverage_score: f64,
}

/// Store para operaciones de memoria.
#[allow(dead_code)]
pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
    crypto: Option<Arc<Mutex<crate::crypto::CryptoEngine>>>,
}

impl MemoryStore {
    /// Crea un nuevo MemoryStore.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn, crypto: None }
    }

    /// Asigna el motor de encriptación.
    pub fn with_crypto(mut self, crypto: Arc<Mutex<crate::crypto::CryptoEngine>>) -> Self {
        self.crypto = Some(crypto);
        self
    }

    /// Compute normalized hash for deduplication.
    /// Combines: project + scope + memory_type + title (lowercased, whitespace normalized).
    fn compute_hash(project: &str, scope: &Scope, memory_type: &MemoryType, title: &str) -> String {
        let normalized = format!(
            "{}:{}:{}:{}",
            project.to_lowercase(),
            scope.to_string().to_lowercase(),
            memory_type.to_string().to_lowercase(),
            title
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        );
        format!("{:x}", md5::compute(normalized))
    }

    /// Check for duplicates in a rolling window (last 24 hours).
    fn find_duplicate(&self, hash: &str, project: &str) -> crate::error::Result<Option<Memory>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let cutoff = (Utc::now() - chrono::Duration::hours(24)).to_rfc3339();
        let mut stmt = conn.prepare(
            "SELECT id FROM memories
             WHERE normalized_hash = ?1 AND project = ?2 AND deleted_at IS NULL
             AND created_at > ?3
             LIMIT 1",
        )?;
        let id: Result<String, _> =
            stmt.query_row(params![hash, project, cutoff], |row| row.get(0));
        match id {
            Ok(id) => {
                let id = Uuid::parse_str(&id)
                    .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
                drop(stmt);
                drop(conn);
                self.get(id)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Upsert a memory with topic_key (evolutionary updates).
    fn upsert_by_topic_key(
        &self,
        input: &CreateMemoryInput,
    ) -> crate::error::Result<Option<Memory>> {
        let topic_key = match &input.topic_key {
            Some(tk) => tk,
            None => return Ok(None),
        };

        let id = {
            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let scope_str = input.scope.as_ref().unwrap_or(&Scope::Project).to_string();
            let mut stmt = conn.prepare(
                "SELECT id FROM memories
                 WHERE project = ?1 AND scope = ?2 AND topic_key = ?3 AND deleted_at IS NULL
                 LIMIT 1",
            )?;
            let id: Result<String, _> = stmt
                .query_row(params![input.project, scope_str, topic_key], |row| {
                    row.get(0)
                });
            match id {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(e.into()),
            }
        };

        let id =
            Uuid::parse_str(&id).map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

        let update = UpdateMemoryInput {
            title: Some(input.title.clone()),
            content: Some(input.content.clone()),
            what: input.what.clone(),
            why: input.why.clone(),
            context: input.context.clone(),
            learned: input.learned.clone(),
            memory_type: Some(input.memory_type.clone()),
            importance: Some(input.importance.clone()),
            tags: Some(input.tags.clone()),
            scope: input.scope.clone(),
            topic_key: input.topic_key.clone(),
        };
        self.update(id, update).map(Some)
    }

    /// Indexa embedding para una memoria.
    pub async fn index_embedding(
        &self,
        memory: &Memory,
        engine: &std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>,
        embedding_store: &crate::embeddings::store::EmbeddingStore,
    ) -> crate::error::Result<()> {
        let text = crate::embeddings::engine::EmbeddingEngine::memory_to_text(memory);
        let embedding = engine.embed(&text).await?;
        embedding_store.save(memory.id, &embedding, engine.model_name())?;
        tracing::info!(memory_id = %memory.id, "indexed embedding");
        Ok(())
    }

    /// Reindexa todas las memorias sin embedding.
    pub async fn reindex_embeddings(
        &self,
        project: &str,
        engine: &std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>,
        embedding_store: &crate::embeddings::store::EmbeddingStore,
    ) -> crate::error::Result<ReindexStats> {
        let start = std::time::Instant::now();
        let unindexed = embedding_store.find_unindexed(project)?;
        let total = unindexed.len() as u32;
        let mut indexed = 0u32;
        let mut failed = 0u32;

        for id in unindexed {
            match self.get(id)? {
                Some(memory) => {
                    match self.index_embedding(&memory, engine, embedding_store).await {
                        Ok(()) => indexed += 1,
                        Err(e) => {
                            tracing::warn!(memory_id = %id, error = %e, "failed to index embedding");
                            failed += 1;
                        }
                    }
                }
                None => {
                    failed += 1;
                }
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        let skipped = total.saturating_sub(indexed + failed);
        Ok(ReindexStats {
            total,
            indexed,
            skipped,
            failed,
            duration_ms,
        })
    }

    /// Save a new memory. Handles dedupe, topic_key upserts, and returns the memory.
    /// Si engine y embedding_store son proporcionados, intenta indexar el embedding
    /// de forma no bloqueante.
    pub fn save(
        &self,
        input: CreateMemoryInput,
        engine: Option<std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
        embedding_store: Option<crate::embeddings::store::EmbeddingStore>,
    ) -> crate::error::Result<Memory> {
        if let Some(existing) = self.upsert_by_topic_key(&input)? {
            tracing::info!("upserted memory via topic_key: {}", existing.id);
            return Ok(existing);
        }

        let scope = input.scope.clone().unwrap_or(Scope::Project);
        let hash = Self::compute_hash(&input.project, &scope, &input.memory_type, &input.title);
        if let Some(mut existing) = self.find_duplicate(&hash, &input.project)? {
            existing.duplicate_count += 1;
            existing.last_seen_at = Some(Utc::now());
            existing.updated_at = Utc::now();

            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            conn.execute(
                "UPDATE memories SET duplicate_count = ?1, last_seen_at = ?2, updated_at = ?3
                 WHERE id = ?4",
                params![
                    existing.duplicate_count,
                    existing.last_seen_at.map(|d| d.to_rfc3339()),
                    existing.updated_at.to_rfc3339(),
                    existing.id.to_string()
                ],
            )?;
            tracing::info!("detected duplicate memory: {}", existing.id);
            return Ok(existing);
        }

        let id = Uuid::new_v4();
        let now = Utc::now();

        let (
            content_to_save,
            what_to_save,
            why_to_save,
            context_to_save,
            learned_to_save,
            is_encrypted,
            encrypted_for,
        ) = if input.encrypt {
            if let Some(crypto_arc) = &self.crypto {
                let crypto = crypto_arc
                    .lock()
                    .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
                if crypto.has_recipients() {
                    let enc_content = crypto.encrypt_str(&input.content)?;
                    let enc_what = input
                        .what
                        .as_deref()
                        .map(|s| crypto.encrypt_str(s))
                        .transpose()?;
                    let enc_why = input
                        .why
                        .as_deref()
                        .map(|s| crypto.encrypt_str(s))
                        .transpose()?;
                    let enc_ctx = input
                        .context
                        .as_deref()
                        .map(|s| crypto.encrypt_str(s))
                        .transpose()?;
                    let enc_learned = input
                        .learned
                        .as_deref()
                        .map(|s| crypto.encrypt_str(s))
                        .transpose()?;
                    let label = crypto.encrypted_for_label();
                    (
                        enc_content,
                        enc_what,
                        enc_why,
                        enc_ctx,
                        enc_learned,
                        true,
                        Some(label),
                    )
                } else {
                    return Err(crate::error::MnemeError::NoRecipientsConfigured);
                }
            } else {
                return Err(crate::error::MnemeError::NoRecipientsConfigured);
            }
        } else {
            (
                input.content.clone(),
                input.what.clone(),
                input.why.clone(),
                input.context.clone(),
                input.learned.clone(),
                false,
                None,
            )
        };

        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO memories (
                id, project, scope, title, content, what, why, context, learned,
                memory_type, importance, tags, topic_key, access_count, revision_count,
                duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at,
                is_encrypted, encrypted_for, valid_from, valid_until, provenance
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
            params![
                id.to_string(),
                &input.project,
                scope.to_string(),
                &input.title,
                &content_to_save,
                what_to_save.as_deref(),
                why_to_save.as_deref(),
                context_to_save.as_deref(),
                learned_to_save.as_deref(),
                input.memory_type.to_string(),
                input.importance.to_string(),
                serde_json::to_string(&input.tags)?,
                input.topic_key.as_deref(),
                0u32,
                1u32,
                0u32,
                &hash,
                now.to_rfc3339(),
                now.to_rfc3339(),
                Option::<String>::None,
                now.to_rfc3339(),
                is_encrypted,
                encrypted_for.as_deref(),
                input.valid_from.map(|d| d.to_rfc3339()),  // valid_from
                input.valid_until.map(|d| d.to_rfc3339()),  // valid_until
                input.provenance,  // provenance
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO memories_fts(rowid, title, content, what, why, context, learned, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                rowid,
                &input.title,
                if is_encrypted { "" } else { &content_to_save },
                what_to_save.as_deref(),
                why_to_save.as_deref(),
                context_to_save.as_deref(),
                learned_to_save.as_deref(),
                serde_json::to_string(&input.tags)?,
            ],
        )?;
        drop(conn);

        // Auto-index embedding non-blocking
        if let (Some(engine), Some(_store)) = (engine, embedding_store) {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let memory_id = id;
                let store_conn = self.conn.clone();
                handle.spawn(async move {
                    let embed_store = crate::embeddings::store::EmbeddingStore::new(store_conn.clone());
                    let mem_store = MemoryStore::new(store_conn);
                    if let Ok(Some(memory)) = mem_store.get(memory_id) {
                        let text = crate::embeddings::engine::EmbeddingEngine::memory_to_text(&memory);
                        match engine.embed(&text).await {
                            Ok(embedding) => {
                                if let Err(e) = embed_store.save(memory_id, &embedding, engine.model_name()) {
                                    tracing::warn!(memory_id = %memory_id, error = %e, "auto-index failed");
                                }
                            }
                            Err(e) => {
                                tracing::warn!(memory_id = %memory_id, error = %e, "auto-index embed failed");
                            }
                        }
                    }
                });
            } else {
                tracing::debug!("no tokio runtime available for auto-index");
            }
        }

        // Auto-extract entities (non-blocking)
        if !is_encrypted {
            let entity_store = crate::store::entities::EntityStore::new(self.conn.clone());
            if let Ok(Some(saved_memory)) = self.get(id) {
                if let Err(e) = entity_store.extract_and_save(&saved_memory) {
                    tracing::warn!(memory_id = %id, error = %e, "entity extraction failed");
                }

                // Auto-detect conflict candidates
                if let Ok(candidates) = self.detect_conflict_candidates(&saved_memory) {
                    if !candidates.is_empty() {
                        tracing::info!(
                            memory_id = %id, candidate_count = candidates.len(),
                            "detected potential conflicts"
                        );
                    }
                }
            }
        }

        // Optionally set valid_from (if not provided, defaults to created_at)
        if input.valid_from.is_some() {
            // Already handled above via the INSERT
        }

        tracing::info!("saved new memory: {}", id);
        self.get(id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(id))
    }

    /// Update a memory by ID. Increments revision_count.
    pub fn update(&self, id: Uuid, input: UpdateMemoryInput) -> crate::error::Result<Memory> {
        {
            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let deleted: Option<Option<String>> = conn
                .query_row(
                    "SELECT deleted_at FROM memories WHERE id = ?1",
                    params![id.to_string()],
                    |row| row.get::<_, Option<String>>(0),
                )
                .ok();

            match deleted {
                None => return Err(crate::error::MnemeError::NotFound(id)),
                Some(Some(_)) => return Err(crate::error::MnemeError::NotFound(id)),
                Some(None) => {}
            }
        }

        {
            let mut conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let tx = conn.transaction()?;

            if let Some(title) = &input.title {
                tx.execute(
                    "UPDATE memories SET title = ?1 WHERE id = ?2",
                    params![title, id.to_string()],
                )?;
            }
            if let Some(content) = &input.content {
                tx.execute(
                    "UPDATE memories SET content = ?1 WHERE id = ?2",
                    params![content, id.to_string()],
                )?;
            }
            if let Some(what) = &input.what {
                tx.execute(
                    "UPDATE memories SET what = ?1 WHERE id = ?2",
                    params![what, id.to_string()],
                )?;
            }
            if let Some(why) = &input.why {
                tx.execute(
                    "UPDATE memories SET why = ?1 WHERE id = ?2",
                    params![why, id.to_string()],
                )?;
            }
            if let Some(context) = &input.context {
                tx.execute(
                    "UPDATE memories SET context = ?1 WHERE id = ?2",
                    params![context, id.to_string()],
                )?;
            }
            if let Some(learned) = &input.learned {
                tx.execute(
                    "UPDATE memories SET learned = ?1 WHERE id = ?2",
                    params![learned, id.to_string()],
                )?;
            }
            if let Some(memory_type) = &input.memory_type {
                tx.execute(
                    "UPDATE memories SET memory_type = ?1 WHERE id = ?2",
                    params![memory_type.to_string(), id.to_string()],
                )?;
            }
            if let Some(importance) = &input.importance {
                tx.execute(
                    "UPDATE memories SET importance = ?1 WHERE id = ?2",
                    params![importance.to_string(), id.to_string()],
                )?;
            }
            if let Some(tags) = &input.tags {
                let tags_json = serde_json::to_string(tags)?;
                tx.execute(
                    "UPDATE memories SET tags = ?1 WHERE id = ?2",
                    params![tags_json, id.to_string()],
                )?;
            }
            if let Some(scope) = &input.scope {
                tx.execute(
                    "UPDATE memories SET scope = ?1 WHERE id = ?2",
                    params![scope.to_string(), id.to_string()],
                )?;
            }
            if let Some(topic_key) = &input.topic_key {
                tx.execute(
                    "UPDATE memories SET topic_key = ?1 WHERE id = ?2",
                    params![topic_key, id.to_string()],
                )?;
            }

            tx.execute(
                "UPDATE memories SET revision_count = revision_count + 1, updated_at = ?1 WHERE id = ?2",
                params![Utc::now().to_rfc3339(), id.to_string()],
            )?;

            tx.commit()?;

            // Update FTS5 index
            let rowid: i64 = conn.query_row(
                "SELECT rowid FROM memories WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )?;
            let (title, content, what, why, context, learned, tags_json) = conn.query_row(
                "SELECT title, content, what, why, context, learned, tags FROM memories WHERE id = ?1",
                params![id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                },
            )?;
            conn.execute(
                "INSERT INTO memories_fts(rowid, title, content, what, why, context, learned, tags)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![rowid, title, content, what, why, context, learned, tags_json],
            )?;
        }

        tracing::info!("updated memory: {}", id);
        self.get(id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(id))
    }

    /// Soft-delete a memory (default). Hard-delete if hard=true.
    pub fn delete(&self, id: Uuid, hard: bool) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        if hard {
            let rowid: i64 = conn.query_row(
                "SELECT rowid FROM memories WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )?;
            conn.execute("DELETE FROM memories_fts WHERE rowid = ?1", params![rowid])?;
            conn.execute(
                "DELETE FROM memories WHERE id = ?1",
                params![id.to_string()],
            )?;
            tracing::info!("hard-deleted memory: {}", id);
        } else {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE memories SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
                params![now, id.to_string()],
            )?;
            let rowid: i64 = conn.query_row(
                "SELECT rowid FROM memories WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )?;
            conn.execute("DELETE FROM memories_fts WHERE rowid = ?1", params![rowid])?;
            tracing::info!("soft-deleted memory: {}", id);
        }
        Ok(())
    }

    /// Restore a soft-deleted memory.
    pub fn restore(&self, id: Uuid) -> crate::error::Result<Memory> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "UPDATE memories SET deleted_at = NULL WHERE id = ?1",
            params![id.to_string()],
        )?;
        let rowid: i64 = conn.query_row(
            "SELECT rowid FROM memories WHERE id = ?1",
            params![id.to_string()],
            |row| row.get(0),
        )?;
        let (title, content, what, why, context, learned, tags_json) = conn.query_row(
            "SELECT title, content, what, why, context, learned, tags FROM memories WHERE id = ?1",
            params![id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;
        conn.execute(
            "INSERT INTO memories_fts(rowid, title, content, what, why, context, learned, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![rowid, title, content, what, why, context, learned, tags_json],
        )?;
        drop(conn);
        tracing::info!("restored memory: {}", id);
        self.get(id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(id))
    }

    /// Get a memory by ID, incrementing access_count. Ignores soft-deleted.
    pub fn get(&self, id: Uuid) -> crate::error::Result<Option<Memory>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "UPDATE memories SET access_count = access_count + 1, last_accessed_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![Utc::now().to_rfc3339(), id.to_string()],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE id = ?1 AND deleted_at IS NULL",
        )?;

        let result = stmt.query_row(params![id.to_string()], Self::row_to_memory);

        match result {
            Ok(mut memory) => {
                if memory.is_encrypted {
                    if let Some(crypto_arc) = &self.crypto {
                        let mut crypto = crypto_arc.lock().map_err(|_| {
                            crate::error::MnemeError::Config("mutex poisoned".into())
                        })?;
                        if crypto.can_decrypt() {
                            memory.content = crypto
                                .decrypt_str(&memory.content)
                                .unwrap_or_else(|_| "[ENCRIPTADO]".to_string());
                            if let Some(ref s) = memory.what {
                                memory.what = Some(
                                    crypto
                                        .decrypt_str(s)
                                        .unwrap_or_else(|_| "[ENCRIPTADO]".to_string()),
                                );
                            }
                            if let Some(ref s) = memory.why {
                                memory.why = Some(
                                    crypto
                                        .decrypt_str(s)
                                        .unwrap_or_else(|_| "[ENCRIPTADO]".to_string()),
                                );
                            }
                            if let Some(ref s) = memory.context {
                                memory.context = Some(
                                    crypto
                                        .decrypt_str(s)
                                        .unwrap_or_else(|_| "[ENCRIPTADO]".to_string()),
                                );
                            }
                            if let Some(ref s) = memory.learned {
                                memory.learned = Some(
                                    crypto
                                        .decrypt_str(s)
                                        .unwrap_or_else(|_| "[ENCRIPTADO]".to_string()),
                                );
                            }
                        } else {
                            memory.content = "[ENCRIPTADO]".to_string();
                            memory.what = memory.what.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                            memory.why = memory.why.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                            memory.context =
                                memory.context.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                            memory.learned =
                                memory.learned.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                        }
                    } else {
                        memory.content = "[ENCRIPTADO]".to_string();
                        memory.what = memory.what.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                        memory.why = memory.why.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                        memory.context =
                            memory.context.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                        memory.learned =
                            memory.learned.as_ref().map(|_| "[ENCRIPTADO]".to_string());
                    }
                }
                Ok(Some(memory))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List memories for a project with optional filters. Ignores soft-deleted.
    pub fn list(
        &self,
        project: &str,
        memory_type: Option<&MemoryType>,
        importance: Option<&Importance>,
        scope: Option<&Scope>,
        limit: u32,
        offset: u32,
    ) -> crate::error::Result<Vec<Memory>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let mut conditions = vec!["project = ?1", "deleted_at IS NULL"];
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        param_values.push(Box::new(project.to_string()));

        if let Some(mt) = memory_type {
            conditions.push("memory_type = ?");
            param_values.push(Box::new(mt.to_string()));
        }
        if let Some(imp) = importance {
            conditions.push("importance = ?");
            param_values.push(Box::new(imp.to_string()));
        }
        if let Some(sc) = scope {
            conditions.push("scope = ?");
            param_values.push(Box::new(sc.to_string()));
        }

        let sql = format!(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE {}
             ORDER BY updated_at DESC LIMIT ? OFFSET ?",
            conditions.join(" AND ")
        );
        param_values.push(Box::new(limit as i64));
        param_values.push(Box::new(offset as i64));

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_memory)?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        Ok(memories)
    }

    /// Get recent memories for context (session injection). Ignores soft-deleted.
    pub fn context(
        &self,
        project: &str,
        scope: Option<&Scope>,
        limit: u32,
    ) -> crate::error::Result<Vec<Memory>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let mut sql = String::from(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE project = ?1 AND deleted_at IS NULL",
        );
        let mut param_values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project.to_string())];

        if let Some(sc) = scope {
            sql.push_str(" AND scope = ?");
            param_values.push(Box::new(sc.to_string()));
        }

        sql.push_str(
            " ORDER BY last_accessed_at IS NULL, last_accessed_at DESC, updated_at DESC LIMIT ?",
        );
        param_values.push(Box::new(limit as i64));

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_memory)?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        Ok(memories)
    }

    /// Get stats for a project.
    pub fn stats(&self, project: &str) -> crate::error::Result<MemoryStats> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let total: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
            params![project],
            |row| row.get(0),
        )?;

        let mut by_type = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT memory_type, COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL GROUP BY memory_type",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            by_type.insert(k, v);
        }

        let mut by_importance = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT importance, COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL GROUP BY importance",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            by_importance.insert(k, v);
        }

        let mut by_scope = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT scope, COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL GROUP BY scope",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            by_scope.insert(k, v);
        }

        let total_relations: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memory_relations r
             JOIN memories m ON r.source_id = m.id
             WHERE m.project = ?1 AND m.deleted_at IS NULL",
            params![project],
            |row| row.get(0),
        )?;

        let total_sessions: u32 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE project = ?1",
            params![project],
            |row| row.get(0),
        )?;

        let total_prompts: u32 = conn.query_row(
            "SELECT COUNT(*) FROM user_prompts WHERE project = ?1",
            params![project],
            |row| row.get(0),
        )?;

        let oldest: Option<String> = conn
            .query_row(
                "SELECT MIN(created_at) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
                params![project],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let newest: Option<String> = conn
            .query_row(
                "SELECT MAX(created_at) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
                params![project],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let most_accessed: Option<String> = conn
            .query_row(
                "SELECT title FROM memories WHERE project = ?1 AND deleted_at IS NULL ORDER BY access_count DESC LIMIT 1",
                params![project],
                |row| row.get(0),
            )
            .ok();

        Ok(MemoryStats {
            project: project.to_string(),
            total_memories: total,
            by_type,
            by_importance,
            by_scope,
            total_relations,
            total_sessions,
            total_prompts,
            oldest_memory: oldest
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            newest_memory: newest
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            most_accessed,
        })
    }

    /// List all projects with summary.
    pub fn list_projects(&self) -> crate::error::Result<Vec<ProjectSummary>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let mut stmt = conn.prepare(
            "SELECT project, COUNT(*), MAX(updated_at)
             FROM memories WHERE deleted_at IS NULL
             GROUP BY project ORDER BY project",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectSummary {
                name: row.get(0)?,
                memory_count: row.get(1)?,
                session_count: 0u32,
                last_activity: row
                    .get::<_, Option<String>>(2)?
                    .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                    .map(|d| d.with_timezone(&Utc)),
            })
        })?;

        let mut projects: Vec<ProjectSummary> = Vec::new();
        for row in rows {
            projects.push(row?);
        }

        for project in &mut projects {
            let count: u32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sessions WHERE project = ?1",
                    params![&project.name],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            project.session_count = count;
        }

        Ok(projects)
    }

    /// Check if a project exists.
    pub fn project_exists(&self, project: &str) -> crate::error::Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let count: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL LIMIT 1",
            params![project],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Search memories using the hybrid search engine.
    /// Si se proporcionan scores semánticos, se integran en la puntuación.
    pub fn search(
        &self,
        query: &SearchQuery,
        weights: &crate::store::search::SearchWeights,
        semantic_scores: Option<&HashMap<Uuid, f32>>,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let engine = crate::store::search::SearchEngine::new();
        engine.search(&conn, query, weights, semantic_scores)
    }

    /// Search with cross-encoder reranking.
    /// When embeddings feature is enabled, re-ranks top results using semantic refinement.
    pub fn search_reranked(
        &self,
        query: &SearchQuery,
        weights: &crate::store::search::SearchWeights,
        semantic_scores: Option<&HashMap<Uuid, f32>>,
        engine: &std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let search_engine = crate::store::search::SearchEngine::new();
        let mut results = search_engine.search(&conn, query, weights, semantic_scores)?;

        // Rerank top results using semantic similarity refinement
        if !results.is_empty() {
            crate::embeddings::rerank::rerank_search_results(
                &query.text,
                &mut results,
                Some(engine),
                weights,
            );
        }

        Ok(results)
    }

    /// Maps a SQL row to a Memory struct. Public for use by MCP tools.
    pub fn row_to_memory(row: &rusqlite::Row) -> Result<Memory, rusqlite::Error> {
        Ok(Memory {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            project: row.get(1)?,
            scope: Scope::from_str(&row.get::<_, String>(2)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            title: row.get(3)?,
            content: row.get(4)?,
            what: row.get(5)?,
            why: row.get(6)?,
            context: row.get(7)?,
            learned: row.get(8)?,
            memory_type: MemoryType::from_str(&row.get::<_, String>(9)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            importance: Importance::from_str(&row.get::<_, String>(10)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    10,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            tags: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or_default(),
            topic_key: row.get(12)?,
            access_count: row.get(13)?,
            revision_count: row.get(14)?,
            duplicate_count: row.get(15)?,
            normalized_hash: row.get(16)?,
            created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(17)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        17,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(18)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        18,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&Utc),
            last_accessed_at: row
                .get::<_, Option<String>>(19)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            last_seen_at: row
                .get::<_, Option<String>>(20)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            deleted_at: row
                .get::<_, Option<String>>(21)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            deprecated_at: row
                .get::<_, Option<String>>(22)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            deprecated_reason: row.get(23)?,
            supersedes_id: row.get(24)?,
            context_inject_count: row.get(25)?,
            origin_peer: row.get(26)?,
            is_encrypted: row.get(27).unwrap_or(false),
            encrypted_for: row.get(28).unwrap_or(None),
            valid_from: row
                .get::<_, Option<String>>(29)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            valid_until: row
                .get::<_, Option<String>>(30)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            provenance: row.get(31)?,
        })
    }

    /// Guarda un prompt de usuario.
    pub fn save_prompt(&self, input: CreatePromptInput) -> crate::error::Result<UserPrompt> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO user_prompts (id, session_id, content, project, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                input.session_id.map(|u| u.to_string()),
                input.content,
                input.project,
                now.to_rfc3339()
            ],
        )?;
        tracing::info!("saved prompt: {}", id);
        Ok(UserPrompt {
            id,
            session_id: input.session_id,
            content: input.content,
            project: input.project,
            created_at: now,
        })
    }

    /// Crea una relación entre dos memorias.
    pub fn create_relation(
        &self,
        input: CreateRelationInput,
    ) -> crate::error::Result<MemoryRelation> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        if input.source_id == input.target_id {
            return Err(crate::error::MnemeError::SelfRelation(input.source_id));
        }

        let existing: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memory_relations
             WHERE source_id = ?1 AND target_id = ?2",
            params![input.source_id.to_string(), input.target_id.to_string()],
            |row| row.get(0),
        )?;

        if existing > 0 {
            return Err(crate::error::MnemeError::RelationAlreadyExists(
                input.source_id,
                input.target_id,
            ));
        }

        conn.execute(
            "INSERT INTO memory_relations (
                id, sync_id, source_id, target_id, relation_type, confidence,
                judgment_status, reason, evidence, marked_by_actor, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                id.to_string(),
                id.to_string(),
                input.source_id.to_string(),
                input.target_id.to_string(),
                input.relation_type.to_string(),
                input.confidence,
                "active",
                input.reason.as_deref(),
                Option::<String>::None,
                "user",
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;

        tracing::info!(
            "created relation: {} -> {}",
            input.source_id,
            input.target_id
        );
        Ok(MemoryRelation {
            id,
            sync_id: id.to_string(),
            source_id: input.source_id,
            target_id: input.target_id,
            relation_type: input.relation_type,
            confidence: input.confidence.unwrap_or(1.0),
            judgment_status: "active".to_string(),
            reason: input.reason,
            evidence: None,
            marked_by_actor: "user".to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Guarda un lote de memorias, detectando duplicados.
    pub fn save_batch(
        &self,
        inputs: Vec<CreateMemoryInput>,
        engine: Option<std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
        embedding_store: Option<crate::embeddings::store::EmbeddingStore>,
    ) -> crate::error::Result<(Vec<Memory>, Vec<Memory>)> {
        let mut saved = Vec::new();
        let mut duplicates = Vec::new();
        for input in inputs {
            match self.save(input, engine.clone(), embedding_store.clone()) {
                Ok(memory) => {
                    if memory.duplicate_count > 0 {
                        duplicates.push(memory.clone());
                    }
                    saved.push(memory);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "batch save failed for one item");
                }
            }
        }
        tracing::info!(
            saved = saved.len(),
            duplicates = duplicates.len(),
            "batch save complete"
        );
        Ok((saved, duplicates))
    }

    /// Elimina una relación por su ID.
    pub fn delete_relation(&self, relation_id: Uuid) -> crate::error::Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let affected = conn.execute(
            "DELETE FROM memory_relations WHERE id = ?1",
            params![relation_id.to_string()],
        )?;
        tracing::info!(relation_id = %relation_id, affected_rows = affected, "deleted relation");
        Ok(affected > 0)
    }

    /// Ejecuta una auditoría de calidad sobre un proyecto.
    pub fn audit(&self, project: &str, days_threshold: u32) -> crate::error::Result<AuditReport> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let cutoff = (Utc::now() - chrono::Duration::days(i64::from(days_threshold))).to_rfc3339();

        // Stale memories
        let mut stmt = conn.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE project = ?1 AND deleted_at IS NULL
             AND (last_accessed_at IS NULL OR last_accessed_at < ?2)
             ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![project, cutoff], Self::row_to_memory)?;
        let mut stale_memories = Vec::new();
        for row in rows {
            stale_memories.push(row?);
        }

        // Untagged memories
        let mut stmt = conn.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE project = ?1 AND deleted_at IS NULL
             AND (tags = '[]' OR tags = '')
             ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![project], Self::row_to_memory)?;
        let mut untagged_memories = Vec::new();
        for row in rows {
            untagged_memories.push(row?);
        }

        // Short memories
        let mut stmt = conn.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE project = ?1 AND deleted_at IS NULL
             AND LENGTH(content) < 20
             ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![project], Self::row_to_memory)?;
        let mut short_memories = Vec::new();
        for row in rows {
            short_memories.push(row?);
        }

        // Type distribution
        let mut type_distribution = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT memory_type, COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL GROUP BY memory_type"
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            type_distribution.insert(k, v);
        }

        // Average revisions
        let avg_revisions: f64 = conn.query_row(
            "SELECT COALESCE(AVG(revision_count), 0.0) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
            params![project],
            |row| row.get(0),
        ).unwrap_or(0.0);

        // Duplicate groups
        let duplicate_groups: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL AND duplicate_count > 0",
            params![project],
            |row| row.get(0),
        ).unwrap_or(0);

        Ok(AuditReport {
            stale_memories,
            untagged_memories,
            short_memories,
            type_distribution,
            average_revisions: avg_revisions,
            duplicate_groups,
        })
    }

    /// Encuentra duplicados semánticos usando embeddings.
    pub fn find_duplicates_semantic(
        &self,
        project: &str,
        threshold: f64,
        embedding_store: &crate::embeddings::store::EmbeddingStore,
    ) -> crate::error::Result<Vec<DuplicateGroup>> {
        let all = embedding_store.load_all_for_project(project)?;
        if all.len() < 2 {
            return Ok(Vec::new());
        }

        let threshold_f32 = threshold as f32;
        let mut adjacency: std::collections::HashMap<usize, Vec<usize>> =
            std::collections::HashMap::new();

        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                let score = crate::embeddings::similarity::cosine_similarity(&all[i].1, &all[j].1);
                if score >= threshold_f32 {
                    adjacency.entry(i).or_default().push(j);
                    adjacency.entry(j).or_default().push(i);
                }
            }
        }

        if adjacency.is_empty() {
            return Ok(Vec::new());
        }

        // Transitive closure via BFS
        let mut visited = vec![false; all.len()];
        let mut groups = Vec::new();

        for start in 0..all.len() {
            if visited[start] || !adjacency.contains_key(&start) {
                continue;
            }
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(start);
            visited[start] = true;
            let mut component = Vec::new();
            let mut max_score = 0.0f32;

            while let Some(node) = queue.pop_front() {
                component.push(node);
                if let Some(neighbors) = adjacency.get(&node) {
                    for &neighbor in neighbors {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            queue.push_back(neighbor);
                        }
                        let score = crate::embeddings::similarity::cosine_similarity(
                            &all[node].1,
                            &all[neighbor].1,
                        );
                        if score > max_score {
                            max_score = score;
                        }
                    }
                }
            }

            if component.len() >= 2 {
                let mut memory_ids = Vec::new();
                let mut titles = Vec::new();
                for &idx in &component {
                    memory_ids.push(all[idx].0.to_string());
                    if let Ok(Some(mem)) = self.get(all[idx].0) {
                        titles.push(mem.title);
                    } else {
                        titles.push(String::new());
                    }
                }
                groups.push(DuplicateGroup {
                    memory_ids,
                    titles,
                    cosine_score: max_score,
                });
            }
        }

        Ok(groups)
    }

    /// Registra feedback sobre una memoria.
    pub fn add_feedback(
        &self,
        memory_id: Uuid,
        is_useful: bool,
        reason: Option<&str>,
    ) -> crate::error::Result<i64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO memory_feedback (memory_id, is_useful, reason, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                memory_id.to_string(),
                if is_useful { 1 } else { 0 },
                reason,
                now
            ],
        )?;
        let id = conn.last_insert_rowid();
        tracing::info!(memory_id = %memory_id, feedback_id = id, "added feedback");
        Ok(id)
    }

    /// Marca una memoria como deprecada.
    pub fn deprecate(
        &self,
        memory_id: Uuid,
        reason: &str,
        supersedes_id: Option<Uuid>,
    ) -> crate::error::Result<Memory> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE memories SET deprecated_at = ?1, deprecated_reason = ?2, supersedes_id = ?3, updated_at = ?4
             WHERE id = ?5 AND deleted_at IS NULL",
            params![
                now,
                reason,
                supersedes_id.map(|u| u.to_string()),
                now,
                memory_id.to_string()
            ],
        )?;
        drop(conn);
        tracing::info!(memory_id = %memory_id, "deprecated memory");
        self.get(memory_id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(memory_id))
    }

    /// Obtiene el grafo de conocimiento de un proyecto.
    pub fn get_graph(&self, project: &str) -> crate::error::Result<GraphData> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let mut nodes = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT id, title, memory_type, importance FROM memories
             WHERE project = ?1 AND deleted_at IS NULL AND deprecated_at IS NULL",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(GraphNode {
                id: row.get(0)?,
                title: row.get(1)?,
                memory_type: row.get(2)?,
                importance: row.get(3)?,
            })
        })?;
        for row in rows {
            nodes.push(row?);
        }

        let mut edges = Vec::new();
        let mut stmt = conn.prepare(
            "SELECT r.source_id, r.target_id, r.relation_type, r.confidence
             FROM memory_relations r
             JOIN memories m1 ON r.source_id = m1.id
             JOIN memories m2 ON r.target_id = m2.id
             WHERE m1.project = ?1 AND m1.deleted_at IS NULL
             AND m2.project = ?1 AND m2.deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok(GraphEdge {
                source: row.get(0)?,
                target: row.get(1)?,
                relation_type: row.get(2)?,
                confidence: row.get(3)?,
            })
        })?;
        for row in rows {
            edges.push(row?);
        }

        Ok(GraphData { nodes, edges })
    }

    /// Genera un resumen ejecutivo de un proyecto o sesión.
    pub fn summarize(
        &self,
        project: &str,
        session_id: Option<Uuid>,
    ) -> crate::error::Result<SummaryResult> {
        let memories = if let Some(sid) = session_id {
            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let memory_ids_json: String = conn.query_row(
                "SELECT memory_ids FROM sessions WHERE id = ?1 AND project = ?2",
                params![sid.to_string(), project],
                |row| row.get(0),
            )?;
            let ids: Vec<Uuid> = serde_json::from_str(&memory_ids_json).map_err(|e| {
                crate::error::MnemeError::Config(format!("invalid session memory_ids: {}", e))
            })?;
            let mut result = Vec::new();
            for id in ids {
                if let Some(mem) = self.get(id)? {
                    result.push(mem);
                }
            }
            result
        } else {
            self.list(project, None, None, None, 1000, 0)?
        };

        let memory_count = memories.len() as u32;
        let mut by_type = HashMap::new();
        let mut decisions = Vec::new();
        let mut bugs = Vec::new();

        for mem in &memories {
            *by_type.entry(mem.memory_type.to_string()).or_insert(0u32) += 1;
            match mem.memory_type {
                MemoryType::Decision => decisions.push(mem.title.clone()),
                MemoryType::Bugfix => bugs.push(mem.title.clone()),
                _ => {}
            }
        }

        let mut summary_parts = vec![format!("Resumen del proyecto '{}'", project)];
        summary_parts.push(format!("Total de memorias: {}", memory_count));

        if !decisions.is_empty() {
            summary_parts.push(format!("\nDecisiones tomadas ({}):", decisions.len()));
            for d in decisions {
                summary_parts.push(format!("- {}", d));
            }
        }

        if !bugs.is_empty() {
            summary_parts.push(format!("\nBugs corregidos ({}):", bugs.len()));
            for b in bugs {
                summary_parts.push(format!("- {}", b));
            }
        }

        Ok(SummaryResult {
            summary: summary_parts.join("\n"),
            memory_count,
            by_type,
        })
    }

    /// Genera un bloque de contexto formateado para inyección en prompts.
    pub fn inject_context(
        &self,
        project: &str,
        file: Option<&str>,
        limit: u32,
    ) -> crate::error::Result<String> {
        let mut lines = vec![
            format!("## Contexto del proyecto: {}", project),
            String::new(),
        ];

        // Critical/high importance memories
        let critical = self.list(project, None, Some(&Importance::Critical), None, limit, 0)?;
        let high = self.list(project, None, Some(&Importance::High), None, limit, 0)?;
        let mut important = critical;
        important.extend(high);
        important.truncate(limit as usize);

        if !important.is_empty() {
            lines.push("### Decisiones arquitectónicas críticas".to_string());
            for mem in &important {
                lines.push(format!(
                    "- {} ({}): {}",
                    mem.title,
                    mem.memory_type,
                    &mem.content[..mem.content.len().min(120)]
                ));
            }
            lines.push(String::new());
        }

        // Recent memories related to file
        if let Some(file_path) = file {
            let related = self.list(project, None, None, None, limit, 0)?;
            let file_related: Vec<_> = related
                .into_iter()
                .filter(|m| {
                    m.context
                        .as_ref()
                        .map(|c| c.contains(file_path))
                        .unwrap_or(false)
                })
                .take(limit as usize)
                .collect();
            if !file_related.is_empty() {
                lines.push("### Memorias recientes relevantes".to_string());
                for mem in &file_related {
                    lines.push(format!(
                        "- {} ({}): {}",
                        mem.title,
                        mem.importance,
                        &mem.content[..mem.content.len().min(120)]
                    ));
                }
                lines.push(String::new());
            }
        }

        // Architecture decisions and conventions
        let arch = self.list(
            project,
            Some(&MemoryType::Architecture),
            None,
            None,
            limit,
            0,
        )?;
        let conventions =
            self.list(project, Some(&MemoryType::Convention), None, None, limit, 0)?;
        let mut patterns = arch;
        patterns.extend(conventions);
        patterns.truncate(limit as usize);

        if !patterns.is_empty() {
            lines.push("### Convenciones y patrones".to_string());
            for mem in &patterns {
                lines.push(format!(
                    "- {}: {}",
                    mem.title,
                    &mem.content[..mem.content.len().min(120)]
                ));
            }
            lines.push(String::new());
        }

        // Update context_inject_count
        {
            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let ids: Vec<String> = important
                .iter()
                .chain(patterns.iter())
                .map(|m| m.id.to_string())
                .collect();
            for id in &ids {
                let _ = conn.execute(
                    "UPDATE memories SET context_inject_count = context_inject_count + 1 WHERE id = ?1",
                    params![id],
                );
            }
        }

        Ok(lines.join("\n"))
    }

    /// Elimina todas las memorias de un proyecto (hard delete).
    pub fn forget_project(&self, project: &str) -> crate::error::Result<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare("SELECT rowid FROM memories WHERE project = ?1")?;
        let rows = stmt.query_map(params![project], |row| row.get::<_, i64>(0))?;
        let mut rowids = Vec::new();
        for row in rows {
            rowids.push(row?);
        }
        for rowid in rowids {
            conn.execute("DELETE FROM memories_fts WHERE rowid = ?1", params![rowid])?;
        }
        let affected = conn.execute("DELETE FROM memories WHERE project = ?1", params![project])?;
        tracing::info!(project = project, deleted = affected, "forgot project");
        Ok(affected as u32)
    }

    /// Reporte de salud del sistema.
    pub fn health(&self, project: Option<&str>) -> crate::error::Result<HealthReport> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let db_size_mb = (page_count * page_size) as f64 / (1024.0 * 1024.0);

        let total_memories: u32 = if let Some(proj) = project {
            conn.query_row(
                "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
                params![proj],
                |row| row.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM memories WHERE deleted_at IS NULL",
                [],
                |row| row.get(0),
            )?
        };

        // Orphaned memories: those with empty project
        let orphaned_memories: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE (project IS NULL OR project = '') AND deleted_at IS NULL",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        // Unindexed embeddings
        let unindexed_embeddings: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM memories m
             LEFT JOIN memory_embeddings e ON m.id = e.memory_id
             WHERE m.deleted_at IS NULL AND e.memory_id IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Try to get embedding model
        let embedding_model: String = conn
            .query_row(
                "SELECT model_name FROM memory_embeddings ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(HealthReport {
            db_size_mb,
            total_memories,
            orphaned_memories,
            unindexed_embeddings,
            last_sync: None,
            embedding_model,
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    /// Retorna memorias críticas/high como recordatorios.
    pub fn remind(
        &self,
        project: &str,
        importance: &Importance,
    ) -> crate::error::Result<Vec<Memory>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let mut sql = String::from(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE project = ?1 AND deleted_at IS NULL AND deprecated_at IS NULL"
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project.to_string())];

        match importance {
            Importance::Critical => {
                sql.push_str(" AND importance = 'critical'");
            }
            Importance::High => {
                sql.push_str(" AND importance IN ('high', 'critical')");
            }
            _ => {
                sql.push_str(" AND importance = ?2");
                params_vec.push(Box::new(importance.to_string()));
            }
        }

        sql.push_str(
            " ORDER BY CASE importance WHEN 'critical' THEN 1 WHEN 'high' THEN 2 ELSE 3 END,
                      last_accessed_at IS NULL, last_accessed_at ASC LIMIT 50",
        );

        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), Self::row_to_memory)?;

        let mut memories = Vec::new();
        for row in rows {
            memories.push(row?);
        }
        Ok(memories)
    }

    /// Sugiere tags basados en tags existentes y contenido.
    pub fn suggest_tags(
        &self,
        project: &str,
        title: &str,
        content: Option<&str>,
    ) -> crate::error::Result<Vec<String>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        // Get all tags from project
        let mut stmt =
            conn.prepare("SELECT tags FROM memories WHERE project = ?1 AND deleted_at IS NULL")?;
        let rows = stmt.query_map(params![project], |row| row.get::<_, String>(0))?;

        let mut tag_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for row in rows {
            let tags_json = row?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            for tag in tags {
                *tag_counts.entry(tag.to_lowercase()).or_insert(0) += 1;
            }
        }

        // Sort by frequency and take top 20
        let mut sorted_tags: Vec<(String, u32)> = tag_counts.into_iter().collect();
        sorted_tags.sort_by_key(|b| std::cmp::Reverse(b.1));
        let top_tags: Vec<String> = sorted_tags.into_iter().take(20).map(|(t, _)| t).collect();

        // Extract keywords from title + content
        let text = format!("{} {}", title, content.unwrap_or("")).to_lowercase();
        let stopwords: std::collections::HashSet<&str> = [
            "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "of", "in", "for",
            "on", "with", "at", "by", "from", "as", "to", "and", "or", "but",
        ]
        .iter()
        .copied()
        .collect();

        let words: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2 && !stopwords.contains(w))
            .map(|w| w.to_string())
            .collect();

        // Suggest tags that appear in both top tags and keywords, or are similar
        let mut suggestions = Vec::new();
        for tag in &top_tags {
            if words.iter().any(|w| w.contains(tag) || tag.contains(w)) {
                suggestions.push(tag.clone());
            }
        }

        // Also add keywords that look like tags (already exist in top_tags)
        for word in words {
            if top_tags.contains(&word) && !suggestions.contains(&word) {
                suggestions.push(word);
            }
        }

        suggestions.truncate(10);
        Ok(suggestions)
    }

    /// Analiza brechas de conocimiento en un proyecto.
    pub fn knowledge_gaps(&self, project: &str) -> crate::error::Result<KnowledgeGapsReport> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let total: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL",
            params![project],
            |row| row.get(0),
        )?;

        if total == 0 {
            return Ok(KnowledgeGapsReport {
                gaps: Vec::new(),
                coverage_score: 0.0,
            });
        }

        let mut counts = HashMap::new();
        let mut stmt = conn.prepare(
            "SELECT memory_type, COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL GROUP BY memory_type"
        )?;
        let rows = stmt.query_map(params![project], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            counts.insert(k, v);
        }

        // Ideal distribution percentages
        let ideals: std::collections::HashMap<&str, f64> = [
            ("architecture", 0.15),
            ("decision", 0.15),
            ("bugfix", 0.10),
            ("pattern", 0.10),
            ("convention", 0.10),
            ("dependency", 0.05),
            ("workflow", 0.05),
            ("note", 0.10),
            ("config", 0.05),
            ("discovery", 0.05),
            ("learning", 0.10),
            ("agent_fact", 0.05),
        ]
        .iter()
        .copied()
        .collect();

        let mut gaps = Vec::new();
        let mut covered = 0.0;

        for (area, ideal_pct) in &ideals {
            let count = counts.get(*area).copied().unwrap_or(0);
            let actual_pct = f64::from(count) / f64::from(total);
            if actual_pct < ideal_pct * 0.5 && count < 5 {
                gaps.push(KnowledgeGap {
                    area: area.to_string(),
                    count,
                    suggestion: format!("Considera documentar más items de tipo '{}'", area),
                });
            } else if actual_pct >= ideal_pct * 0.8 {
                covered += ideal_pct;
            } else {
                covered += ideal_pct * (actual_pct / ideal_pct);
            }
        }

        // Check topic_key coverage
        let without_topic_key: u32 = conn.query_row(
            "SELECT COUNT(*) FROM memories WHERE project = ?1 AND deleted_at IS NULL AND topic_key IS NULL",
            params![project],
            |row| row.get(0),
        ).unwrap_or(0);

        if without_topic_key > total / 2 {
            gaps.push(KnowledgeGap {
                area: "topic_key".to_string(),
                count: without_topic_key,
                suggestion:
                    "Muchas memorias carecen de topic_key; esto dificulta la evolución organizada"
                        .to_string(),
            });
        }

        let coverage_score = (covered / ideals.values().sum::<f64>()).clamp(0.0, 1.0);

        Ok(KnowledgeGapsReport {
            gaps,
            coverage_score,
        })
    }

    /// Encripta una memoria existente in-place.
    pub fn encrypt_existing(&self, memory_id: Uuid) -> crate::error::Result<Memory> {
        let memory = self
            .get(memory_id)?
            .ok_or(crate::error::MnemeError::NotFound(memory_id))?;
        if memory.is_encrypted {
            return Err(crate::error::MnemeError::AlreadyEncrypted(memory_id));
        }
        let crypto_arc = self
            .crypto
            .as_ref()
            .ok_or(crate::error::MnemeError::NoRecipientsConfigured)?;
        let crypto = crypto_arc
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        if !crypto.has_recipients() {
            return Err(crate::error::MnemeError::NoRecipientsConfigured);
        }
        let enc_content = crypto.encrypt_str(&memory.content)?;
        let enc_what = memory
            .what
            .as_deref()
            .map(|s| crypto.encrypt_str(s))
            .transpose()?;
        let enc_why = memory
            .why
            .as_deref()
            .map(|s| crypto.encrypt_str(s))
            .transpose()?;
        let enc_ctx = memory
            .context
            .as_deref()
            .map(|s| crypto.encrypt_str(s))
            .transpose()?;
        let enc_learned = memory
            .learned
            .as_deref()
            .map(|s| crypto.encrypt_str(s))
            .transpose()?;
        let label = crypto.encrypted_for_label();
        drop(crypto);

        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "UPDATE memories SET content=?1, what=?2, why=?3, context=?4, learned=?5, is_encrypted=1, encrypted_for=?6, updated_at=?7 WHERE id=?8",
            params![enc_content, enc_what, enc_why, enc_ctx, enc_learned, label, Utc::now().to_rfc3339(), memory_id.to_string()],
        )?;
        drop(conn);
        self.get(memory_id)?
            .ok_or(crate::error::MnemeError::NotFound(memory_id))
    }

    /// Desencripta una memoria encriptada (permanentemente).
    pub fn decrypt_existing(&self, memory_id: Uuid) -> crate::error::Result<Memory> {
        let memory = self
            .get(memory_id)?
            .ok_or(crate::error::MnemeError::NotFound(memory_id))?;
        if !memory.is_encrypted {
            return Err(crate::error::MnemeError::NotEncrypted(memory_id));
        }
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "UPDATE memories SET content=?1, what=?2, why=?3, context=?4, learned=?5, is_encrypted=0, encrypted_for=NULL, updated_at=?6 WHERE id=?7",
            params![memory.content, memory.what, memory.why, memory.context, memory.learned, Utc::now().to_rfc3339(), memory_id.to_string()],
        )?;
        drop(conn);
        self.get(memory_id)?
            .ok_or(crate::error::MnemeError::NotFound(memory_id))
    }

    // --- Passive Capture ---

    /// Parsea texto de output de sesión y extrae memorias automáticamente.
    /// Busca secciones como ## Key Learnings, ## Decisions, ## Architecture, etc.
    pub fn capture_passive(
        &self,
        text: &str,
        project: &str,
        session_id: Option<Uuid>,
        engine: Option<std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
        embedding_store: Option<crate::embeddings::store::EmbeddingStore>,
    ) -> crate::error::Result<Vec<Memory>> {
        let mut saved = Vec::new();

        // Parse sections from markdown-style headings
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Detect markdown headings with known section markers
            if let Some(section_type) = Self::detect_section_type(line) {
                let title = line.trim_start_matches('#').trim().to_string();
                let mut content_parts: Vec<String> = Vec::new();
                i += 1;

                // Collect content until next heading or end
                while i < lines.len() && !lines[i].trim().starts_with('#') && !lines[i].trim().is_empty() {
                    content_parts.push(lines[i].to_string());
                    i += 1;
                }

                if content_parts.is_empty() {
                    continue;
                }

                let content = content_parts.join("\n").trim().to_string();
                if content.len() < 10 {
                    continue;
                }

                let (memory_type, importance, what, why, context, learned, topic_key) = Self::section_to_metadata(section_type, &title, &content);

                let input = CreateMemoryInput {
                    project: project.to_string(),
                    scope: Some(Scope::Project),
                    title: title.clone(),
                    content: content.clone(),
                    what: what.map(|s| s.to_string()),
                    why: why.map(|s| s.to_string()),
                    context: context.map(|s| s.to_string()),
                    learned: learned.map(|s| s.to_string()),
                    memory_type,
                    importance,
                    tags: Vec::new(),
                    topic_key,
                    capture_prompt: session_id.map(|_| true),
                    encrypt: false,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                };

                match self.save(input, engine.clone(), embedding_store.clone()) {
                    Ok(memory) => {
                        if let Some(sid) = session_id {
                            let session_store = SessionStore::new(self.conn.clone());
                            let _ = session_store.add_memory(sid, memory.id);
                        }
                        saved.push(memory);
                    }
                    Err(e) => {
                        tracing::warn!(section = %title, error = %e, "passive capture save failed");
                    }
                }
            } else {
                i += 1;
            }
        }

        tracing::info!(captured = saved.len(), "passive capture complete");
        Ok(saved)
    }

    /// Detecta el tipo de sección basado en el contenido del heading.
    fn detect_section_type(line: &str) -> Option<&'static str> {
        let lower = line.to_lowercase();
        let markers = [
            ("key learnings", "learning"),
            ("decisions", "decision"),
            ("architecture", "architecture"),
            ("bugfix", "bugfix"),
            ("bug fix", "bugfix"),
            ("bugs fixed", "bugfix"),
            ("patterns", "pattern"),
            ("conventions", "convention"),
            ("dependencies", "dependency"),
            ("discoveries", "discovery"),
            ("discovery", "discovery"),
            ("workflow", "workflow"),
            ("config changes", "config"),
            ("config", "config"),
            ("notes", "note"),
            ("note", "note"),
            ("summary", "note"),
        ];
        for (keyword, section_type) in &markers {
            if lower.contains(keyword) {
                return Some(section_type);
            }
        }
        None
    }

    /// Convierte una sección parseada en metadatos de CreateMemoryInput.
    fn section_to_metadata(
        section_type: &str,
        title: &str,
        content: &str,
    ) -> (MemoryType, Importance, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>) {
        let memory_type = MemoryType::from_str(section_type).unwrap_or(MemoryType::Note);
        let importance = match section_type {
            "architecture" | "decision" => Importance::High,
            "bugfix" => Importance::Medium,
            "config" => Importance::Low,
            _ => Importance::Medium,
        };

        // Extract structured fields from content
        let (what, why, context, learned) = Self::extract_structured_fields(content);

        let topic_key = if !section_type.is_empty() && !title.is_empty() {
            let slug = title
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != ' ', "")
                .split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join("-");
            Some(format!("{}/{}", section_type, slug))
        } else {
            None
        };

        (memory_type, importance, what, why, context, learned, topic_key)
    }

    /// Extrae campos What/Why/Context/Learned de contenido estructurado.
    fn extract_structured_fields(content: &str) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
        let mut what = None;
        let mut why = None;
        let mut context = None;
        let mut learned = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(val) = trimmed.strip_prefix("**What:**").or_else(|| trimmed.strip_prefix("**What:** ")) {
                what = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("**Why:**").or_else(|| trimmed.strip_prefix("**Why:** ")) {
                why = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("**Context:**").or_else(|| trimmed.strip_prefix("**Context:** ")) {
                context = Some(val.trim().to_string());
            } else if let Some(val) = trimmed.strip_prefix("**Learned:**").or_else(|| trimmed.strip_prefix("**Learned:** ")) {
                learned = Some(val.trim().to_string());
            }
        }

        (what, why, context, learned)
    }

    // --- Conflict Detection ---

    /// Detecta candidatos de conflicto para una memoria recién guardada.
    /// Busca por: topic_key compartido, título similar, y mismo project+type.
    pub fn detect_conflict_candidates(&self, memory: &Memory) -> crate::error::Result<Vec<ConflictCandidate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut candidates = Vec::new();
        let matcher = fuzzy_matcher::skim::SkimMatcherV2::default();

        // 1. Check topic_key overlap
        if let Some(ref topic_key) = memory.topic_key {
            let mut stmt = conn.prepare(
                "SELECT id, title, memory_type FROM memories
                 WHERE project = ?1 AND topic_key = ?2 AND id != ?3
                 AND deleted_at IS NULL AND deprecated_at IS NULL
                 LIMIT 5"
            )?;
            let rows = stmt.query_map(params![memory.project, topic_key, memory.id.to_string()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            for row in rows {
                let (id_str, _title, _type) = row?;
                if let Ok(target_id) = Uuid::parse_str(&id_str) {
                    candidates.push(ConflictCandidate {
                        id: 0,
                        source_id: memory.id,
                        target_id,
                        reason: format!("Same topic_key: '{}'", topic_key),
                        match_score: 0.7,
                        candidate_type: "topic_key".to_string(),
                        judgment_status: "pending".to_string(),
                        judged_relation: None,
                        judged_reason: None,
                        created_at: Utc::now(),
                    });
                }
            }
        }

        // 2. Check title similarity (fuzzy match >= 80)
        {
            let mut stmt = conn.prepare(
                "SELECT id, title FROM memories
                 WHERE project = ?1 AND id != ?2
                 AND deleted_at IS NULL AND deprecated_at IS NULL
                 LIMIT 50"
            )?;
            let rows = stmt.query_map(params![memory.project, memory.id.to_string()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (id_str, title) = row?;
                if let (Ok(target_id), Some(score)) = (Uuid::parse_str(&id_str), matcher.fuzzy_match(&title, &memory.title)) {
                    let normalized = (score as f32).abs() / 100.0;
                    if normalized >= 0.80 {
                        candidates.push(ConflictCandidate {
                            id: 0,
                            source_id: memory.id,
                            target_id,
                            reason: format!("Similar title: '{}' vs '{}' (score: {:.2})", memory.title, title, normalized),
                            match_score: normalized,
                            candidate_type: "title".to_string(),
                            judgment_status: "pending".to_string(),
                            judged_relation: None,
                            judged_reason: None,
                            created_at: Utc::now(),
                        });
                    }
                }
            }
        }

        // Deduplicate by (source_id, target_id, candidate_type)
        let mut seen = std::collections::HashSet::new();
        candidates.retain(|c| seen.insert((c.source_id, c.target_id, c.candidate_type.clone())));

        // Save to DB
        for c in &candidates {
            conn.execute(
                "INSERT OR IGNORE INTO relation_candidates
                 (source_id, target_id, reason, match_score, candidate_type, judgment_status, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'pending', ?6)",
                params![
                    c.source_id.to_string(),
                    c.target_id.to_string(),
                    c.reason,
                    c.match_score,
                    c.candidate_type,
                    Utc::now().to_rfc3339(),
                ],
            )?;
        }

        Ok(candidates)
    }

    /// Lista candidatos de conflicto pendientes.
    pub fn list_conflict_candidates(
        &self,
        project: &str,
        status: Option<&str>,
        limit: u32,
    ) -> crate::error::Result<Vec<ConflictCandidate>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let status_filter = status.unwrap_or("pending");
        let limit_i64 = limit as i64;

        let mut stmt = conn.prepare(
            "SELECT c.id, c.source_id, c.target_id, c.reason, c.match_score, c.candidate_type,
                    c.judgment_status, c.judged_relation, c.judged_reason, c.created_at
             FROM relation_candidates c
             JOIN memories m1 ON m1.id = c.source_id
             JOIN memories m2 ON m2.id = c.target_id
             WHERE c.judgment_status = ?1 AND m1.project = ?2 AND m1.deleted_at IS NULL AND m2.deleted_at IS NULL
             ORDER BY c.match_score DESC
             LIMIT ?3"
        )?;

        let rows = stmt.query_map(params![status_filter, project, limit_i64], |row| {
            Ok(ConflictCandidate {
                id: row.get(0)?,
                source_id: Uuid::parse_str(&row.get::<_, String>(1)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
                })?,
                target_id: Uuid::parse_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
                })?,
                reason: row.get(3)?,
                match_score: row.get(4)?,
                candidate_type: row.get(5)?,
                judgment_status: row.get(6)?,
                judged_relation: row.get(7)?,
                judged_reason: row.get(8)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(e))
                    })?
                    .with_timezone(&Utc),
            })
        })?;

        let mut candidates = Vec::new();
        for row in rows {
            candidates.push(row?);
        }
        Ok(candidates)
    }

    /// Registra el juicio de un LLM/agente sobre un candidato.
    /// Si el juicio es conflicts_with/supersedes, actualiza la relación automáticamente.
    pub fn judge_conflict(
        &self,
        candidate_id: i64,
        judged_relation: &str,
        reasoning: &str,
        judged_by: &str,
    ) -> crate::error::Result<ConflictJudgment> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        // Get candidate details
        let (source_id_str, target_id_str): (String, String) = conn.query_row(
            "SELECT source_id, target_id FROM relation_candidates WHERE id = ?1",
            params![candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let source_id = Uuid::parse_str(&source_id_str)
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
        let target_id = Uuid::parse_str(&target_id_str)
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
        let now = Utc::now().to_rfc3339();

        // Update candidate status
        conn.execute(
            "UPDATE relation_candidates SET judgment_status = 'judged', judged_relation = ?1, judged_reason = ?2, judged_at = ?3
             WHERE id = ?4",
            params![judged_relation, reasoning, now, candidate_id],
        )?;

        // Record the judgment
        conn.execute(
            "INSERT INTO conflict_judgments (candidate_id, memory_id_a, memory_id_b, proposed_relation, confidence, reasoning, judged_by, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                candidate_id,
                source_id_str,
                target_id_str,
                judged_relation,
                1.0,
                reasoning,
                judged_by,
                now,
            ],
        )?;
        let judgment_id = conn.last_insert_rowid();

        // If relation is conflicts_with, supersedes, or extends — create the actual relation
        if judged_relation == "conflicts_with" || judged_relation == "supersedes" || judged_relation == "extends" {
            let relation_type = match judged_relation {
                "conflicts_with" => RelationType::ConflictsWith,
                "supersedes" => RelationType::Supersedes,
                "extends" => RelationType::Extends,
                _ => RelationType::ConflictsWith,
            };

            // Check if relation already exists
            let existing: u32 = conn.query_row(
                "SELECT COUNT(*) FROM memory_relations WHERE source_id = ?1 AND target_id = ?2",
                params![source_id_str, target_id_str],
                |row| row.get(0),
            ).unwrap_or(0);

            if existing == 0 {
                let rel_id = Uuid::new_v4();
                conn.execute(
                    "INSERT INTO memory_relations (id, sync_id, source_id, target_id, relation_type, confidence, judgment_status, reason, evidence, marked_by_actor, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?8, ?9, ?10, ?11)",
                    params![
                        rel_id.to_string(),
                        rel_id.to_string(),
                        source_id_str,
                        target_id_str,
                        relation_type.to_string(),
                        1.0f32,
                        reasoning,
                        serde_json::to_string(&vec![reasoning])?,
                        judged_by,
                        now,
                        now,
                    ],
                )?;
            }

            // If supersedes, mark old memory as deprecated
            if judged_relation == "supersedes" {
                conn.execute(
                    "UPDATE memories SET deprecated_at = ?1, deprecated_reason = ?2, supersedes_id = ?3, updated_at = ?4
                     WHERE id = ?5 AND deleted_at IS NULL AND deprecated_at IS NULL",
                    params![now, reasoning, source_id_str, now, target_id_str],
                )?;
            }
        }

        Ok(ConflictJudgment {
            id: judgment_id,
            candidate_id,
            memory_id_a: source_id,
            memory_id_b: target_id,
            proposed_relation: judged_relation.to_string(),
            confidence: 1.0,
            reasoning: Some(reasoning.to_string()),
            evidence: None,
            judged_by: judged_by.to_string(),
            created_at: Utc::now(),
        })
    }

    /// Obtiene contexto formateado para que un LLM juzgue un par de memorias.
    /// Obtiene las relaciones existentes entre dos memorias.
    pub fn get_existing_relations(&self, memory_id_a: Uuid, memory_id_b: Uuid) -> crate::error::Result<Vec<MemoryRelation>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, sync_id, source_id, target_id, relation_type, confidence, judgment_status,
                    reason, evidence, marked_by_actor, created_at, updated_at
             FROM memory_relations
             WHERE (source_id = ?1 AND target_id = ?2) OR (source_id = ?2 AND target_id = ?1)
             LIMIT 10"
        )?;
        let rows = stmt.query_map(params![memory_id_a.to_string(), memory_id_b.to_string()], |row| {
            Ok(MemoryRelation {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
                })?,
                sync_id: row.get(1)?,
                source_id: Uuid::parse_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
                })?,
                target_id: Uuid::parse_str(&row.get::<_, String>(3)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(e))
                })?,
                relation_type: std::str::FromStr::from_str(&row.get::<_, String>(4)?).map_err(|e: crate::error::MnemeError| {
                    rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
                })?,
                confidence: row.get(5)?,
                judgment_status: row.get(6)?,
                reason: row.get(7)?,
                evidence: row.get(8)?,
                marked_by_actor: row.get(9)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(10, rusqlite::types::Type::Text, Box::new(e))
                    })?
                    .with_timezone(&Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(e))
                    })?
                    .with_timezone(&Utc),
            })
        })?;
        let mut relations = Vec::new();
        for row in rows {
            relations.push(row?);
        }
        Ok(relations)
    }

    pub fn get_conflict_context(&self, source_id: Uuid, target_id: Uuid) -> crate::error::Result<String> {
        let source = self.get(source_id)?.ok_or_else(|| crate::error::MnemeError::NotFound(source_id))?;
        let target = self.get(target_id)?.ok_or_else(|| crate::error::MnemeError::NotFound(target_id))?;

        Ok(format!(
            r#"## Memoria A (existente)
- **ID:** {}
- **Título:** {}
- **Tipo:** {}
- **Importancia:** {}
- **Contenido:** {}

## Memoria B (nueva)
- **ID:** {}
- **Título:** {}
- **Tipo:** {}
- **Importancia:** {}
- **Contenido:** {}

## Tarea
Analiza si la Memoria B **conflicta**, **extiende**, **reemplaza (supersedes)** o es **compatible** con la Memoria A.
Responde con una de estas relaciones: `compatible`, `conflicts_with`, `supersedes`, `extends`, `depends_on`
Provee una razón breve.
"#,
            source.id, source.title, source.memory_type, source.importance, &source.content[..source.content.len().min(300)],
            target.id, target.title, target.memory_type, target.importance, &target.content[..target.content.len().min(300)],
        ))
    }
}

/// Candidato de conflicto detectado automáticamente.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictCandidate {
    pub id: i64,
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub reason: String,
    pub match_score: f32,
    pub candidate_type: String,
    pub judgment_status: String,
    pub judged_relation: Option<String>,
    pub judged_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Juicio de conflicto realizado por el LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictJudgment {
    pub id: i64,
    pub candidate_id: i64,
    pub memory_id_a: Uuid,
    pub memory_id_b: Uuid,
    pub proposed_relation: String,
    pub confidence: f32,
    pub reasoning: Option<String>,
    pub evidence: Option<String>,
    pub judged_by: String,
    pub created_at: DateTime<Utc>,
}

/// Store para operaciones de sesión.
#[allow(dead_code)]
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    /// Crea un nuevo SessionStore.
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Start a new session.
    pub fn start(&self, project: &str, directory: Option<&str>) -> crate::error::Result<Session> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO sessions (id, project, directory, summary, memory_ids, started_at, ended_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.to_string(),
                project,
                directory,
                Option::<String>::None,
                "[]",
                now.to_rfc3339(),
                Option::<String>::None,
                "active"
            ],
        )?;

        tracing::info!("started session: {} for project: {}", id, project);
        Ok(Session {
            id,
            project: project.to_string(),
            directory: directory.map(|s| s.to_string()),
            summary: None,
            memory_ids: Vec::new(),
            started_at: now,
            ended_at: None,
            status: "active".to_string(),
        })
    }

    /// End a session.
    pub fn end(&self, session_id: Uuid, summary: Option<&str>) -> crate::error::Result<Session> {
        let now = Utc::now();
        {
            let conn = self
                .conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            conn.execute(
                "UPDATE sessions SET ended_at = ?1, summary = ?2, status = ?3 WHERE id = ?4",
                params![now.to_rfc3339(), summary, "ended", session_id.to_string()],
            )?;
        }

        tracing::info!("ended session: {}", session_id);
        self.get(session_id)?
            .ok_or_else(|| crate::error::MnemeError::NotFound(session_id))
    }

    /// Add a memory to a session.
    pub fn add_memory(&self, session_id: Uuid, memory_id: Uuid) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let memory_ids_json: String = conn.query_row(
            "SELECT memory_ids FROM sessions WHERE id = ?1",
            params![session_id.to_string()],
            |row| row.get(0),
        )?;

        let mut memory_ids: Vec<String> = serde_json::from_str(&memory_ids_json)?;
        memory_ids.push(memory_id.to_string());
        let updated = serde_json::to_string(&memory_ids)?;

        conn.execute(
            "UPDATE sessions SET memory_ids = ?1 WHERE id = ?2",
            params![updated, session_id.to_string()],
        )?;

        tracing::debug!("added memory {} to session {}", memory_id, session_id);
        Ok(())
    }

    /// Get the active session for a project.
    pub fn get_active(&self, project: &str) -> crate::error::Result<Option<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let result = conn.query_row(
            "SELECT id, project, directory, summary, memory_ids, started_at, ended_at, status
             FROM sessions WHERE project = ?1 AND status = 'active' ORDER BY started_at DESC LIMIT 1",
            params![project],
            |row| self.row_to_session(row),
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a session by ID.
    pub fn get(&self, session_id: Uuid) -> crate::error::Result<Option<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let result = conn.query_row(
            "SELECT id, project, directory, summary, memory_ids, started_at, ended_at, status
             FROM sessions WHERE id = ?1",
            params![session_id.to_string()],
            |row| self.row_to_session(row),
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List sessions for a project.
    pub fn list(&self, project: &str, limit: u32) -> crate::error::Result<Vec<Session>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, project, directory, summary, memory_ids, started_at, ended_at, status
             FROM sessions WHERE project = ?1 ORDER BY started_at DESC LIMIT ?",
        )?;
        let limit_i64 = limit as i64;
        let rows = stmt.query_map(params![project, limit_i64], |row| self.row_to_session(row))?;

        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    fn row_to_session(&self, row: &rusqlite::Row) -> Result<Session, rusqlite::Error> {
        Ok(Session {
            id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?,
            project: row.get(1)?,
            directory: row.get(2)?,
            summary: row.get(3)?,
            memory_ids: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
            started_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?
                .with_timezone(&Utc),
            ended_at: row
                .get::<_, Option<String>>(6)?
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            status: row.get(7)?,
        })
    }
}
