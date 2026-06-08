//! Failure mining — aprende de sesiones fallidas para auto-corregir memorias.
//!
//! Inspirado por Headroom's `headroom learn` (mines failed agent sessions and
//! writes corrections to `CLAUDE.md`/`AGENTS.md`).
//!
//! Proceso:
//! 1. Detectar sesiones con outcomes `failure` o memorias con feedback negativo
//! 2. Analizar patterns recurrentes (e.g. "missing_context", "outdated_advice")
//! 3. Generar memorias correctivas via `mem_corrective` con la signatura
//! 4. Persistir en `failure_patterns` + `corrective_memories` para audit

use std::collections::HashMap;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::store::db::Database;
use crate::store::memory::{CreateMemoryInput, Importance, Memory, MemoryType, Scope};

/// Outcome de una sesión: success o failure con razones.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionOutcome {
    Success,
    PartialSuccess,
    Failure { reasons: Vec<String> },
}

/// Reporte de un análisis de failures.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailureReport {
    pub project: String,
    pub sessions_analyzed: u32,
    pub failed_sessions: u32,
    pub not_useful_memories: u32,
    pub patterns_found: u32,
    pub corrective_memories_generated: u32,
    pub patterns: Vec<FailurePattern>,
}

/// Pattern de failure detectado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub id: Option<i64>,
    pub pattern_key: String,
    pub description: String,
    pub frequency: u32,
    pub confidence: f32,
    pub corrective_memory_title: Option<String>,
    pub corrective_memory_id: Option<String>,
}

/// Miner de failures.
pub struct FailureMiner {
    db: std::sync::Arc<Database>,
}

impl FailureMiner {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Registra el outcome de una sesión.
    pub fn record_session_outcome(
        &self,
        session_id: uuid::Uuid,
        outcome: SessionOutcome,
        affected_files: u32,
        bugs_introduced: u32,
        user_corrections: Option<&str>,
    ) -> crate::error::Result<()> {
        let conn = self.db.get_conn();
        let conn = conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let outcome_str = match &outcome {
            SessionOutcome::Success => "success",
            SessionOutcome::PartialSuccess => "partial",
            SessionOutcome::Failure { .. } => "failure",
        };
        let reasons_json = match &outcome {
            SessionOutcome::Failure { reasons } => Some(serde_json::to_string(reasons)?),
            _ => None,
        };

        conn.execute(
            "UPDATE sessions SET outcome = ?1, failure_reasons = ?2,
                               affected_files = ?3, bugs_introduced = ?4,
                               user_corrections = ?5
             WHERE id = ?6",
            rusqlite::params![
                outcome_str,
                reasons_json,
                affected_files as i64,
                bugs_introduced as i64,
                user_corrections,
                session_id.to_string()
            ],
        )?;
        Ok(())
    }

    /// Ejecuta el análisis completo: sesiones failure + feedback not_useful → patterns.
    /// Genera memorias correctivas automáticamente para los patterns más frecuentes.
    pub fn mine(&self, project: &str) -> crate::error::Result<FailureReport> {
        tracing::info!(project = %project, "Starting failure mining");
        let mut report = FailureReport {
            project: project.to_string(),
            ..Default::default()
        };

        // 1. Recopilar sesiones failure
        let failed_sessions = self.find_failed_sessions(project)?;
        report.sessions_analyzed = self.count_sessions(project)?;
        report.failed_sessions = failed_sessions.len() as u32;

        // 2. Recopilar memorias con feedback not_useful
        let not_useful = self.find_not_useful_memories(project)?;
        report.not_useful_memories = not_useful.len() as u32;

        // 3. Detectar patterns
        let mut pattern_signals: HashMap<String, PatternSignal> = HashMap::new();

        for session in &failed_sessions {
            // Analizar failure_reasons JSON
            if let Some(ref reasons_json) = session.failure_reasons {
                if let Ok(reasons) = serde_json::from_str::<Vec<String>>(reasons_json) {
                    for r in reasons {
                        pattern_signals
                            .entry(r.clone())
                            .or_insert_with(|| PatternSignal::new(&r))
                            .sessions
                            .push(session.id);
                    }
                }
            }
            // Si bugs_introduced > 0, signal "introduces_bugs"
            if session.bugs_introduced > 0 {
                pattern_signals
                    .entry("introduces_bugs".to_string())
                    .or_insert_with(|| PatternSignal::new("introduces_bugs"))
                    .sessions
                    .push(session.id);
            }
            // Si affected_files > 5 y no bugs, signal "scope_creep"
            if session.affected_files > 5 && session.bugs_introduced == 0 {
                pattern_signals
                    .entry("scope_creep".to_string())
                    .or_insert_with(|| PatternSignal::new("scope_creep"))
                    .sessions
                    .push(session.id);
            }
        }

        // 4. Para memorias not_useful, clasificar por tipo
        for mem in &not_useful {
            let key = match mem.memory_type {
                MemoryType::Decision => "outdated_decision",
                MemoryType::Pattern => "outdated_pattern",
                MemoryType::Convention => "outdated_convention",
                MemoryType::Architecture => "outdated_architecture",
                _ => "low_quality_memory",
            };
            pattern_signals
                .entry(key.to_string())
                .or_insert_with(|| PatternSignal::new(key))
                .memories
                .push(mem.id);
        }

        // 5. Generar o actualizar patterns en DB + corrective memories
        for (key, signal) in pattern_signals {
            let total = signal.sessions.len() + signal.memories.len();
            if total < 1 {
                continue;
            }

            let description = signal.description();
            let corrective_title = signal.corrective_title();

            // Save or update pattern
            let pattern_id = self.upsert_pattern(project, &key, &description, total as u32)?;
            report.patterns_found += 1;

            // Generate a corrective memory (if not exists)
            let corrective_id = self.generate_corrective_memory(
                project,
                &key,
                &description,
                &corrective_title,
                total as u32,
            )?;
            if corrective_id.is_some() {
                report.corrective_memories_generated += 1;
            }

            // Link corrective to pattern
            if let (Some(pid), Some(cid)) = (pattern_id, corrective_id) {
                let conn = self.db.get_conn();
                let conn = conn
                    .lock()
                    .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
                let _ = conn.execute(
                    "UPDATE failure_patterns SET corrective_memory_id = ?1 WHERE id = ?2",
                    rusqlite::params![cid.to_string(), pid],
                );
                let _ = conn.execute(
                    "INSERT OR IGNORE INTO corrective_memories (project, failure_pattern_id, generated_memory_id, rationale, user_accepted)
                     VALUES (?1, ?2, ?3, ?4, 0)",
                    rusqlite::params![project, pid, cid.to_string(), description],
                );
            }

            report.patterns.push(FailurePattern {
                id: pattern_id,
                pattern_key: key,
                description,
                frequency: total as u32,
                confidence: signal.confidence(),
                corrective_memory_title: corrective_id.map(|_| corrective_title),
                corrective_memory_id: corrective_id.map(|u| u.to_string()),
            });
        }

        Ok(report)
    }

    fn find_failed_sessions(&self, project: &str) -> crate::error::Result<Vec<SessionFailureInfo>> {
        let conn = self.db.get_conn();
        let conn = conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, outcome, failure_reasons, affected_files, bugs_introduced
             FROM sessions
             WHERE project = ?1 AND outcome = 'failure' AND ended_at IS NOT NULL
             ORDER BY started_at DESC LIMIT 100",
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            Ok(SessionFailureInfo {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                failure_reasons: row.get(2)?,
                affected_files: row.get::<_, i64>(3)? as u32,
                bugs_introduced: row.get::<_, i64>(4)? as u32,
            })
        })?;
        let mut sessions = Vec::new();
        for r in rows {
            sessions.push(r?);
        }
        Ok(sessions)
    }

    fn count_sessions(&self, project: &str) -> crate::error::Result<u32> {
        let conn = self.db.get_conn();
        let conn = conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE project = ?1",
                rusqlite::params![project],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count)
    }

    fn find_not_useful_memories(&self, project: &str) -> crate::error::Result<Vec<Memory>> {
        let conn = self.db.get_conn();
        let conn = conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        // Find memories with >= 2 negative feedbacks
        let _stmt = conn.prepare(
            "SELECT m.id, m.project, m.scope, m.title, m.content, m.what, m.why, m.context, m.learned,
                    m.memory_type, m.importance, m.tags, m.topic_key, m.access_count, m.revision_count,
                    m.duplicate_count, m.normalized_hash, m.created_at, m.updated_at, m.last_accessed_at, m.last_seen_at, m.deleted_at,
                    m.deprecated_at, m.deprecated_reason, m.supersedes_id, m.context_inject_count, m.origin_peer,
                    m.is_encrypted, m.encrypted_for
             FROM memories m
             JOIN memory_feedback f ON m.id = f.memory_id
             WHERE m.project = ?1 AND m.deleted_at IS NULL AND f.is_useful = 0
             GROUP BY m.id
             HAVING COUNT(*) >= 2
             LIMIT 100"
        )?;
        // Single efficient query: find memories with >= 2 negative feedbacks
        let mut stmt = conn.prepare(
            "SELECT id, title, content, memory_type
             FROM memories
             WHERE project = ?1 AND deleted_at IS NULL
             AND id IN (
                SELECT memory_id FROM memory_feedback WHERE is_useful = 0 GROUP BY memory_id HAVING COUNT(*) >= 2
             )
             LIMIT 100"
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            Ok(NotUsefulRef {
                id: uuid::Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                title: row.get(1)?,
                content: row.get(2)?,
                memory_type_str: row.get(3)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            let r = r?;
            result.push(Memory {
                id: r.id,
                project: project.to_string(),
                scope: Scope::Project,
                title: r.title,
                content: r.content,
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: std::str::FromStr::from_str(&r.memory_type_str)
                    .unwrap_or(MemoryType::Note),
                importance: Importance::Medium,
                tags: Vec::new(),
                topic_key: None,
                access_count: 0,
                revision_count: 0,
                duplicate_count: 0,
                normalized_hash: None,
                created_at: chrono::DateTime::UNIX_EPOCH,
                updated_at: chrono::DateTime::UNIX_EPOCH,
                last_accessed_at: None,
                last_seen_at: None,
                deleted_at: None,
                deprecated_at: None,
                deprecated_reason: None,
                supersedes_id: None,
                context_inject_count: 0,
                origin_peer: None,
                is_encrypted: false,
                encrypted_for: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            });
        }
        Ok(result)
    }

    fn upsert_pattern(
        &self,
        project: &str,
        key: &str,
        description: &str,
        freq: u32,
    ) -> crate::error::Result<Option<i64>> {
        let conn = self.db.get_conn();
        let conn = conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let now = Utc::now().to_rfc3339();

        // Check if exists
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM failure_patterns WHERE project = ?1 AND pattern_key = ?2",
                rusqlite::params![project, key],
                |row| row.get(0),
            )
            .ok();

        if let Some(id) = existing {
            conn.execute(
                "UPDATE failure_patterns SET frequency = frequency + ?1, last_seen = ?2 WHERE id = ?3",
                rusqlite::params![freq as i64, now, id],
            )?;
            Ok(Some(id))
        } else {
            conn.execute(
                "INSERT INTO failure_patterns (project, pattern_key, description, frequency, first_seen, last_seen, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 0.5)",
                rusqlite::params![project, key, description, freq as i64, now],
            )?;
            Ok(conn.last_insert_rowid().into())
        }
    }

    fn generate_corrective_memory(
        &self,
        project: &str,
        pattern_key: &str,
        description: &str,
        corrective_title: &str,
        _freq: u32,
    ) -> crate::error::Result<Option<uuid::Uuid>> {
        // Check if a corrective memory for this pattern already exists in this project.
        // Scope the lock so it's released before store.save (which also needs the lock).
        let already_exists: bool = {
            let conn = self.db.get_conn();
            let conn = conn
                .lock()
                .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
            conn.query_row(
                "SELECT id FROM memories
                 WHERE project = ?1 AND deleted_at IS NULL
                 AND provenance LIKE ?2",
                rusqlite::params![project, format!("learn/{}/%", pattern_key)],
                |row| row.get::<_, String>(0),
            )
            .is_ok()
        };

        if already_exists {
            return Ok(None);
        }

        // Save new memory with provenance pointing to the pattern
        let input = CreateMemoryInput {
            project: project.to_string(),
            scope: Some(Scope::Project),
            title: corrective_title.to_string(),
            content: format!(
                "**Patrón detectado:** `{}`\n\n**Descripción:** {}\n\n**Recomendación:** Revisar memorias relacionadas con este patrón. Considerar marcar como obsoletas las que generen este tipo de error.",
                pattern_key, description
            ),
            what: Some(format!("Pattern: {}", pattern_key)),
            why: Some(description.to_string()),
            context: None,
            learned: Some("Auto-generado por FailureMiner".to_string()),
            memory_type: MemoryType::Learning,
            importance: Importance::High,
            tags: vec!["learned".to_string(), "pattern".to_string(), pattern_key.to_string()],
            topic_key: Some(format!("learn/{}", pattern_key)),
            capture_prompt: None,
            encrypt: false,
            valid_from: None,
            valid_until: None,
            provenance: Some(format!("learn/{}/auto", pattern_key)),
        };

        let store = self.db.memories();
        let memory = store.save(input, None, None)?;
        Ok(Some(memory.id))
    }
}

/// Referencia a una memoria con feedback negativo.
struct NotUsefulRef {
    id: uuid::Uuid,
    title: String,
    content: String,
    memory_type_str: String,
}

/// Info de una sesión failure.
struct SessionFailureInfo {
    id: uuid::Uuid,
    failure_reasons: Option<String>,
    affected_files: u32,
    bugs_introduced: u32,
}

/// Señal acumulada de un pattern de failure.
struct PatternSignal {
    key: String,
    sessions: Vec<uuid::Uuid>,
    memories: Vec<uuid::Uuid>,
}

impl PatternSignal {
    fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            sessions: Vec::new(),
            memories: Vec::new(),
        }
    }

    fn total(&self) -> u32 {
        (self.sessions.len() + self.memories.len()) as u32
    }

    fn confidence(&self) -> f32 {
        let total = self.total();
        // Higher frequency = higher confidence. Cap at 0.95.
        (total as f32 * 0.15).min(0.95)
    }

    fn description(&self) -> String {
        match self.key.as_str() {
            "missing_context" => {
                "El agente no tuvo suficiente contexto al recuperar memorias".to_string()
            }
            "outdated_decision" => {
                "Decisiones arquitectónicas marcadas como outdated por feedback".to_string()
            }
            "outdated_pattern" => {
                "Patrones de código obsoletos según feedback de usuarios".to_string()
            }
            "outdated_convention" => "Convenciones del proyecto que ya no se aplican".to_string(),
            "outdated_architecture" => "Decisiones arquitectónicas que cambiaron".to_string(),
            "low_quality_memory" => {
                "Memorias con feedback negativo recurrente — revisar calidad".to_string()
            }
            "introduces_bugs" => "Sesiones donde el agente introdujo bugs".to_string(),
            "scope_creep" => "Sesiones que tocaron muchos archivos sin introducir bugs".to_string(),
            _ => format!("Pattern custom: {}", self.key),
        }
    }

    fn corrective_title(&self) -> String {
        format!("[Learn] {}", self.key)
    }
}

/// Formatea un reporte como tabla markdown.
pub fn format_failure_report(report: &FailureReport) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Failure Mining Report: {}\n\n", report.project));
    out.push_str("## Resumen\n\n");
    out.push_str(&format!(
        "- Sesiones analizadas: {}\n",
        report.sessions_analyzed
    ));
    out.push_str(&format!(
        "- Sesiones con failure: {}\n",
        report.failed_sessions
    ));
    out.push_str(&format!(
        "- Memorias con feedback negativo: {}\n",
        report.not_useful_memories
    ));
    out.push_str(&format!(
        "- Patterns detectados: {}\n",
        report.patterns_found
    ));
    out.push_str(&format!(
        "- Memorias correctivas generadas: {}\n\n",
        report.corrective_memories_generated
    ));
    if report.patterns.is_empty() {
        out.push_str("No se detectaron patterns de failure. ¡Todo bien!\n");
    } else {
        out.push_str("## Patterns detectados\n\n");
        out.push_str("| Key | Frecuencia | Confianza | Descripción | Correctiva |\n");
        out.push_str("|-----|------------|-----------|-------------|------------|\n");
        for p in &report.patterns {
            let title = p.corrective_memory_title.as_deref().unwrap_or("-");
            out.push_str(&format!(
                "| `{}` | {} | {:.2} | {} | {} |\n",
                p.pattern_key,
                p.frequency,
                p.confidence,
                truncate_str(&p.description, 40),
                title
            ));
        }
    }
    out
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::{CreateMemoryInput, Importance, MemoryType, Scope};

    fn make_db() -> std::sync::Arc<Database> {
        let path =
            std::path::PathBuf::from(format!("/tmp/mneme_learn_test_{}.db", uuid::Uuid::new_v4()));
        std::sync::Arc::new(Database::open(&path).unwrap())
    }

    #[test]
    fn test_pattern_signal_confidence() {
        let mut sig = PatternSignal::new("test");
        assert!((sig.confidence() - 0.0).abs() < 0.001);
        for _ in 0..5 {
            sig.sessions.push(uuid::Uuid::new_v4());
        }
        assert!(sig.confidence() > 0.5);
    }

    #[test]
    fn test_pattern_signal_description_known() {
        let sig = PatternSignal::new("missing_context");
        assert!(sig.description().contains("contexto"));
    }

    #[test]
    fn test_pattern_signal_description_unknown() {
        let sig = PatternSignal::new("weird_thing");
        assert!(sig.description().contains("weird_thing"));
    }

    #[test]
    fn test_format_empty_report() {
        let report = FailureReport::default();
        let s = format_failure_report(&report);
        assert!(s.contains("Failure Mining Report"));
        assert!(s.contains("No se detectaron"));
    }

    #[test]
    fn test_format_populated_report() {
        let report = FailureReport {
            project: "test-proj".to_string(),
            sessions_analyzed: 10,
            failed_sessions: 3,
            not_useful_memories: 2,
            patterns_found: 1,
            corrective_memories_generated: 1,
            patterns: vec![FailurePattern {
                id: Some(1),
                pattern_key: "introduces_bugs".to_string(),
                description: "Sesiones donde el agente introdujo bugs".to_string(),
                frequency: 3,
                confidence: 0.45,
                corrective_memory_title: Some("[Learn] introduces_bugs".to_string()),
                corrective_memory_id: Some("uuid".to_string()),
            }],
        };
        let s = format_failure_report(&report);
        assert!(s.contains("introduces_bugs"));
        assert!(s.contains("[Learn]"));
    }

    #[test]
    fn test_mine_on_empty_project() {
        let db = make_db();
        let miner = FailureMiner::new(db);
        let report = miner.mine("empty-project").unwrap();
        assert_eq!(report.sessions_analyzed, 0);
        assert_eq!(report.patterns_found, 0);
    }

    #[test]
    fn test_record_session_outcome_and_mine() {
        let db = make_db();
        let _memories = db.memories();
        let sessions = db.sessions();
        let session = sessions.start("test-proj", Some("/tmp")).unwrap();

        // Record a failure outcome
        let miner = FailureMiner::new(db.clone());
        miner
            .record_session_outcome(
                session.id,
                SessionOutcome::Failure {
                    reasons: vec![
                        "missing_context".to_string(),
                        "outdated_decision".to_string(),
                    ],
                },
                3,
                1,
                Some("Fixed typo in function signature"),
            )
            .unwrap();
        sessions.end(session.id, Some("test summary")).unwrap();

        // Mine
        let report = miner.mine("test-proj").unwrap();
        assert_eq!(report.sessions_analyzed, 1);
        assert_eq!(report.failed_sessions, 1);
        assert!(report.patterns_found >= 2); // missing_context + outdated_decision
    }
}
