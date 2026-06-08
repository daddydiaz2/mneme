//! Memory consolidation and memory blocks (Letta-inspired).
//!
//! Consolidation: compact stale/old/unused memories into auto-generated summary memories.
//! Memory blocks: `human`, `persona`, `workflow` slots per project — inspired by Letta's
//! memory blocks architecture.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::store::db::Database;
use crate::store::memory::{
    CreateMemoryInput, Importance, Memory, MemoryType, Scope,
};

/// Resultado de una corrida de consolidación.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConsolidationResult {
    pub project: String,
    pub total_analyzed: u32,
    pub total_consolidated: u32,
    pub total_removed: u32,
    pub summary_memory_title: Option<String>,
    pub summary_memory_id: Option<String>,
    pub details: Vec<String>,
}

/// Un block de memoria (Letta-style slot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBlock {
    pub id: i64,
    pub project: String,
    pub slot: String,
    pub memory_id: String,
    pub title: String,
    pub content: String,
    pub updated_at: DateTime<Utc>,
}

/// Estrategia de consolidación.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsolidationStrategy {
    Auto,
    Age,
    Deprecated,
    Unused,
}

impl std::fmt::Display for ConsolidationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsolidationStrategy::Auto => write!(f, "auto"),
            ConsolidationStrategy::Age => write!(f, "age"),
            ConsolidationStrategy::Deprecated => write!(f, "deprecated"),
            ConsolidationStrategy::Unused => write!(f, "unused"),
        }
    }
}

/// Motor de consolidación.
pub struct ConsolidationEngine {
    db: std::sync::Arc<Database>,
}

impl ConsolidationEngine {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Runs consolidation: finds stale/old/deprecated memories, generates a summary,
    /// and optionally soft-deletes the originals.
    pub fn consolidate(
        &self,
        project: &str,
        strategy: ConsolidationStrategy,
        days_threshold: u64,
        dry_run: bool,
    ) -> crate::error::Result<ConsolidationResult> {
        let store = self.db.memories();
        let conn = self.db.get_conn();

        let cutoff = Utc::now() - chrono::Duration::days(days_threshold as i64);
        let cutoff_str = cutoff.to_rfc3339();
        let mut result = ConsolidationResult {
            project: project.to_string(),
            ..Default::default()
        };

        // 1. Find candidates
        let candidates: Vec<Memory> = {
            let conn_guard = conn.lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            let (where_clause, note) = match strategy {
                ConsolidationStrategy::Age => ("(m.updated_at < ?1 OR m.last_accessed_at < ?1)", "by age"),
                ConsolidationStrategy::Deprecated => ("m.deprecated_at IS NOT NULL", "by deprecation"),
                ConsolidationStrategy::Unused => ("m.access_count = 0 AND m.created_at < ?1", "by unused"),
                ConsolidationStrategy::Auto => (
                    "(m.updated_at < ?1 OR m.last_accessed_at < ?1 OR m.deprecated_at IS NOT NULL OR m.access_count = 0)",
                    "by auto (age + deprecated + unused)",
                ),
            };
            let sql = format!(
                "SELECT m.id, m.project, m.scope, m.title, m.content, m.what, m.why, m.context, m.learned,
                        m.memory_type, m.importance, m.tags, m.topic_key, m.access_count, m.revision_count,
                        m.duplicate_count, m.normalized_hash, m.created_at, m.updated_at, m.last_accessed_at, m.last_seen_at, m.deleted_at,
                        m.deprecated_at, m.deprecated_reason, m.supersedes_id, m.context_inject_count, m.origin_peer,
                        m.is_encrypted, m.encrypted_for, m.valid_from, m.valid_until, m.provenance
                 FROM memories m
                 WHERE m.project = ?2 AND m.deleted_at IS NULL
                 AND {}  LIMIT 200",
                where_clause
            );
            let mut stmt = conn_guard.prepare(&sql)?;
            let params: Vec<Box<dyn rusqlite::ToSql>> = if strategy == ConsolidationStrategy::Deprecated {
                vec![Box::new(project.to_string())]
            } else {
                vec![Box::new(cutoff_str.clone()), Box::new(project.to_string())]
            };
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), |row| {
                crate::store::memory::MemoryStore::row_to_memory(row)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })?;
            let mut mems = Vec::new();
            for m in rows.flatten() {
                mems.push(m);
            }
            result.total_analyzed = mems.len() as u32;
            if !mems.is_empty() {
                result.details.push(format!("Found {} candidates {}", mems.len(), note));
            }
            mems
        };

        if candidates.is_empty() {
            result.details.push("No candidates found".to_string());
            return Ok(result);
        }

        // 2. Generate summary memory
        let summary_content = self.generate_summary(&candidates);
        let summary_title = format!(
            "[Consolidation] {} stale memories ({})",
            candidates.len(),
            strategy
        );

        if !dry_run {
            let input = CreateMemoryInput {
                project: project.to_string(),
                scope: Some(Scope::Project),
                title: summary_title.clone(),
                content: summary_content,
                what: Some(format!("Consolidated {} stale memories", candidates.len())),
                why: Some(format!("Strategy: {}, threshold: {} days", strategy, days_threshold)),
                context: None,
                learned: Some(format!(
                    "Memories consolidated: {}",
                    candidates.iter().map(|m| m.title.as_str()).collect::<Vec<_>>().join(", ")
                )),
                memory_type: MemoryType::Note,
                importance: Importance::Low,
                tags: vec!["consolidation".to_string(), strategy.to_string()],
                topic_key: Some(format!("consolidation/{}", strategy)),
                capture_prompt: None,
                encrypt: false,
                valid_from: None,
                valid_until: None,
                provenance: Some(format!("consolidation/{}/{}", strategy, Utc::now().to_rfc3339())),
            };
            let summary = store.save(input, None, None)?;
            result.summary_memory_title = Some(summary.title);
            result.summary_memory_id = Some(summary.id.to_string());
            result.total_consolidated = candidates.len() as u32;

            // 3. Soft-delete originals
            for mem in &candidates {
                if mem.deprecated_at.is_none() {
                    store.delete(mem.id, false)?;
                    result.total_removed += 1;
                }
            }

            // 4. Log the consolidation
            let rationale = format!(
                "Consolidation of {} memories using {} strategy ({}d threshold). {} removed.",
                candidates.len(), strategy, days_threshold, result.total_removed
            );
            let conn_guard2 = conn.lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            conn_guard2.execute(
                "INSERT INTO consolidation_log (project, run_at, total_consolidated, total_removed, summary_memory_id, strategy, rationale)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    project,
                    Utc::now().to_rfc3339(),
                    result.total_consolidated as i64,
                    result.total_removed as i64,
                    result.summary_memory_id,
                    strategy.to_string(),
                    rationale,
                ],
            )?;
            result.details.push(rationale);
        } else {
            result.total_consolidated = candidates.len() as u32;
            result.details.push(format!("[dry-run] Would consolidate {} memories", candidates.len()));
        }

        Ok(result)
    }

    fn generate_summary(&self, candidates: &[Memory]) -> String {
        let mut parts = vec!["# Consolidation Summary\n".to_string()];
        parts.push(format!("Generated: {}\n", Utc::now().to_rfc3339()));
        parts.push(format!("Total memories: {}\n", candidates.len()));

        // By type
        let mut by_type: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        for m in candidates {
            *by_type.entry(m.memory_type.to_string()).or_insert(0) += 1;
        }
        parts.push("## By Type\n".to_string());
        for (t, c) in &by_type {
            parts.push(format!("- {}: {}\n", t, c));
        }

        parts.push("\n## Memories Consolidated\n".to_string());
        for m in candidates {
            let age = Utc::now().signed_duration_since(m.updated_at).num_days();
            let access = m.access_count;
            let deprecated = if m.deprecated_at.is_some() { " [DEPRECATED]" } else { "" };
            parts.push(format!(
                "- {} ({}d ago, {} accesses){}: {}",
                m.title, age, access, deprecated,
                m.content.chars().take(80).collect::<String>()
            ));
        }
        parts.join("\n")
    }

    /// Gets the consolidation log for a project.
    pub fn get_log(&self, project: &str, limit: u32) -> crate::error::Result<Vec<serde_json::Value>> {
        let conn = self.db.get_conn();
        let conn = conn.lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT run_at, total_consolidated, total_removed, summary_memory_id, strategy, rationale
             FROM consolidation_log WHERE project = ?1 ORDER BY run_at DESC LIMIT ?2"
        )?;
        let rows = stmt.query_map(rusqlite::params![project, limit as i64], |row| {
            Ok(serde_json::json!({
                "run_at": row.get::<_, String>(0)?,
                "consolidated": row.get::<_, i64>(1)?,
                "removed": row.get::<_, i64>(2)?,
                "summary_id": row.get::<_, Option<String>>(3)?,
                "strategy": row.get::<_, String>(4)?,
                "rationale": row.get::<_, String>(5)?,
            }))
        })?;
        let mut log = Vec::new();
        for row in rows {
            log.push(row?);
        }
        Ok(log)
    }

    // === Memory Blocks (Letta-inspired) ===

    pub fn set_block(
        &self,
        project: &str,
        slot: &str,
        title: &str,
        content: &str,
    ) -> crate::error::Result<MemoryBlock> {
        let store = self.db.memories();
        let input = CreateMemoryInput {
            project: project.to_string(),
            scope: Some(Scope::Project),
            title: format!("[{}] {}", slot, title),
            content: content.to_string(),
            what: Some(format!("Memory block: {}", slot)),
            why: None,
            context: None,
            learned: None,
            memory_type: MemoryType::Convention,
            importance: Importance::High,
            tags: vec![slot.to_string(), "block".to_string()],
            topic_key: Some(format!("block/{}", slot)),
            capture_prompt: None,
            encrypt: false,
            valid_from: None,
            valid_until: None,
            provenance: Some(format!("block/{}/auto", slot)),
        };
        let memory = store.save(input, None, None)?;

        let conn = self.db.get_conn();
        let conn = conn.lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT OR REPLACE INTO memory_blocks (project, slot, memory_id, title, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![project, slot, memory.id.to_string(), title, Utc::now().to_rfc3339()],
        )?;
        let id = conn.last_insert_rowid();

        Ok(MemoryBlock {
            id,
            project: project.to_string(),
            slot: slot.to_string(),
            memory_id: memory.id.to_string(),
            title: title.to_string(),
            content: content.to_string(),
            updated_at: Utc::now(),
        })
    }

    pub fn get_block(&self, project: &str, slot: &str) -> crate::error::Result<Option<MemoryBlock>> {
        let conn = self.db.get_conn();
        let conn = conn.lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let result = conn.query_row(
            "SELECT mb.id, mb.project, mb.slot, mb.memory_id, mb.title, mb.updated_at, m.content
             FROM memory_blocks mb
             JOIN memories m ON m.id = mb.memory_id
             WHERE mb.project = ?1 AND mb.slot = ?2 AND m.deleted_at IS NULL",
            rusqlite::params![project, slot],
            |row| {
                Ok(MemoryBlock {
                    id: row.get(0)?,
                    project: row.get(1)?,
                    slot: row.get(2)?,
                    memory_id: row.get(3)?,
                    title: row.get(4)?,
                    content: row.get::<_, String>(6)?,
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                        .with_timezone(&Utc),
                })
            },
        );
        match result {
            Ok(block) => Ok(Some(block)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn list_blocks(&self, project: &str) -> crate::error::Result<Vec<MemoryBlock>> {
        let conn = self.db.get_conn();
        let conn = conn.lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT mb.id, mb.project, mb.slot, mb.memory_id, mb.title, mb.updated_at, m.content
             FROM memory_blocks mb
             JOIN memories m ON m.id = mb.memory_id
             WHERE mb.project = ?1 AND m.deleted_at IS NULL
             ORDER BY mb.slot"
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            let updated_at_str: String = row.get(5)?;
            Ok(MemoryBlock {
                id: row.get(0)?,
                project: row.get(1)?,
                slot: row.get(2)?,
                memory_id: row.get(3)?,
                title: row.get(4)?,
                content: row.get::<_, String>(6)?,
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?
                    .with_timezone(&Utc),
            })
        })?;
        let mut blocks = Vec::new();
        for row in rows {
            blocks.push(row?);
        }
        Ok(blocks)
    }
}

/// Formats a consolidation result for display.
pub fn format_consolidation_result(result: &ConsolidationResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Consolidation: {}\n\n", result.project));
    out.push_str(&format!("- Analyzed: {}\n", result.total_analyzed));
    out.push_str(&format!("- Consolidated: {}\n", result.total_consolidated));
    out.push_str(&format!("- Removed: {}\n", result.total_removed));
    if let Some(ref title) = result.summary_memory_title {
        out.push_str(&format!("- Summary: {}\n", title));
    }
    if !result.details.is_empty() {
        out.push_str("\n## Details\n\n");
        for d in &result.details {
            out.push_str(&format!("- {}\n", d));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_db() -> std::sync::Arc<Database> {
        let path = std::path::PathBuf::from(format!("/tmp/mneme_consolidate_test_{}.db", uuid::Uuid::new_v4()));
        std::sync::Arc::new(Database::open(&path).unwrap())
    }

    #[test]
    fn test_consolidation_empty_project() {
        let db = make_db();
        let eng = ConsolidationEngine::new(db);
        let result = eng.consolidate("nonexistent", ConsolidationStrategy::Auto, 30, false).unwrap();
        assert_eq!(result.total_analyzed, 0);
        assert_eq!(result.total_consolidated, 0);
    }

    #[test]
    fn test_consolidation_dry_run_on_empty() {
        let db = make_db();
        let eng = ConsolidationEngine::new(db);
        let result = eng.consolidate("empty", ConsolidationStrategy::Age, 7, true).unwrap();
        assert_eq!(result.total_analyzed, 0);
    }

    #[test]
    fn test_memory_block_set_and_get() {
        let db = make_db();
        let eng = ConsolidationEngine::new(db);
        let block = eng.set_block("test-proj", "human", "User Identity", "I am Daniel, a Rust developer.").unwrap();
        assert_eq!(block.slot, "human");
        assert_eq!(block.title, "User Identity");
        assert!(block.memory_id.len() > 10);

        let fetched = eng.get_block("test-proj", "human").unwrap().unwrap();
        assert_eq!(fetched.title, "User Identity");
        assert!(fetched.content.contains("Daniel"));
    }

    #[test]
    fn test_memory_block_get_nonexistent() {
        let db = make_db();
        let eng = ConsolidationEngine::new(db);
        let result = eng.get_block("nonexistent", "human").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_memory_block_list() {
        let db = make_db();
        let eng = ConsolidationEngine::new(db);
        eng.set_block("proj1", "human", "User", "I am user").unwrap();
        eng.set_block("proj1", "persona", "Assistant", "I am assistant").unwrap();
        let blocks = eng.list_blocks("proj1").unwrap();
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_consolidation_strategy_display() {
        assert_eq!(ConsolidationStrategy::Auto.to_string(), "auto");
        assert_eq!(ConsolidationStrategy::Age.to_string(), "age");
        assert_eq!(ConsolidationStrategy::Deprecated.to_string(), "deprecated");
        assert_eq!(ConsolidationStrategy::Unused.to_string(), "unused");
    }

    #[test]
    fn test_format_consolidation_result() {
        let result = ConsolidationResult {
            project: "test".to_string(),
            total_analyzed: 10,
            total_consolidated: 5,
            total_removed: 3,
            summary_memory_title: Some("[Consolidation] 5 memories".to_string()),
            summary_memory_id: Some("uuid".to_string()),
            details: vec!["Found 5 candidates by age".to_string()],
        };
        let s = format_consolidation_result(&result);
        assert!(s.contains("Analyzed: 10"));
        assert!(s.contains("Consolidated: 5"));
        assert!(s.contains("Removed: 3"));
        assert!(s.contains("[Consolidation]"));
    }
}
