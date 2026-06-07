use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use uuid::Uuid;

use crate::store::memory::{
    Importance, MatchType, Memory, MemoryType, Scope, SearchQuery, SearchResult,
};

/// Pesos para cada componente de la búsqueda híbrida.
#[derive(Debug, Clone, Copy)]
pub struct SearchWeights {
    /// Peso para FTS5 (default 0.5).
    pub fts: f64,
    /// Peso para fuzzy matching (default 0.2).
    pub fuzzy: f64,
    /// Peso para búsqueda semántica (default 0.3).
    pub semantic: f64,
}

impl Default for SearchWeights {
    fn default() -> Self {
        Self {
            fts: 0.5,
            fuzzy: 0.2,
            semantic: 0.3,
        }
    }
}

impl SearchWeights {
    /// Renormaliza pesos cuando embeddings están deshabilitados.
    /// fts=0.7, fuzzy=0.3, semantic=0.0.
    pub fn renormalize_without_semantic(&self) -> Self {
        Self {
            fts: 0.7,
            fuzzy: 0.3,
            semantic: 0.0,
        }
    }
}

/// Motor de búsqueda multi-señal con RRF (Reciprocal Rank Fusion).
/// Combina FTS5 + fuzzy + semántica + entidades + recencia usando RRF.
pub struct SearchEngine;

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Resultado individual de un matcher.
struct RankedSignal {
    memory_id: Uuid,
    rank: usize,
    score: f64,
}

/// Constante RRF (k=60 es el valor estándar).
const RRF_K: f64 = 60.0;

impl SearchEngine {
    /// Crea un nuevo SearchEngine.
    pub fn new() -> Self {
        Self
    }

    /// Multi-signal search using Reciprocal Rank Fusion.
    ///
    /// Señales:
    /// - FTS5 full-text search
    /// - Fuzzy title matching
    /// - Semantic (cosine) similarity
    /// - Entity name matching
    /// - Temporal recency
    ///
    /// Cada señal produce un ranking. RRF fusiona los rankings en un score final.
    pub fn search(
        &self,
        conn: &rusqlite::Connection,
        query: &SearchQuery,
        _weights: &SearchWeights,
        semantic_scores: Option<&HashMap<Uuid, f32>>,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let mut all_signals: Vec<RankedSignal> = Vec::new();
        let mut seen_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

        // 1. FTS5 signal
        if query.text.len() >= 3 {
            let fts_results = self.search_fts(conn, query)?;
            for (rank, result) in fts_results.iter().enumerate() {
                all_signals.push(RankedSignal {
                    memory_id: result.memory.id,
                    rank,
                    score: result.score,
                });
                seen_ids.insert(result.memory.id);
            }
        }

        // 2. Fuzzy signal
        let fuzzy_results = self.search_fuzzy(conn, query)?;
        for (rank, result) in fuzzy_results.iter().enumerate() {
            all_signals.push(RankedSignal {
                memory_id: result.memory.id,
                rank,
                score: result.score,
            });
            seen_ids.insert(result.memory.id);
        }

        // 3. Semantic signal
        if let Some(scores) = semantic_scores {
            let mut semantic_ranked: Vec<(Uuid, f32)> =
                scores.iter().map(|(id, s)| (*id, *s)).collect();
            semantic_ranked
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for (rank, (id, _)) in semantic_ranked.iter().enumerate() {
                all_signals.push(RankedSignal {
                    memory_id: *id,
                    rank,
                    score: f64::from(*scores.get(id).unwrap_or(&0.0)),
                });
                seen_ids.insert(*id);
            }
        }

        // 4. Entity signal: search entity names matching the query
        let entity_matches = Self::search_entities(conn, &query.text, query.project.as_deref());
        for (rank, memory_id) in entity_matches.iter().enumerate() {
            all_signals.push(RankedSignal {
                memory_id: *memory_id,
                rank,
                score: 1.0,
            });
            seen_ids.insert(*memory_id);
        }

        // 5. Temporal recency signal: prefer recently accessed
        let recency_results = self.search_recency(conn, query);
        for (rank, memory_id) in recency_results.iter().enumerate() {
            if seen_ids.contains(memory_id) {
                all_signals.push(RankedSignal {
                    memory_id: *memory_id,
                    rank,
                    score: 1.0,
                });
            }
        }

        // Apply RRF: for each unique memory, sum 1/(k + rank) across all signals
        let mut rrf_scores: HashMap<Uuid, f64> = HashMap::new();
        for signal in &all_signals {
            let entry = rrf_scores.entry(signal.memory_id).or_insert(0.0);
            *entry += 1.0 / (RRF_K + signal.rank as f64);
        }

        // Build full Memory objects for scored IDs
        let mut filtered: Vec<SearchResult> = Vec::new();
        for (id, rrf_score) in &rrf_scores {
            if let Ok(Some(memory)) = Self::get_memory(conn, *id) {
                // Apply filters
                if let Some(memory_type) = &query.memory_type {
                    if &memory.memory_type != memory_type {
                        continue;
                    }
                }
                if let Some(importance) = &query.importance {
                    if &memory.importance != importance {
                        continue;
                    }
                }
                if !query.tags.is_empty() && !query.tags.iter().all(|tag| memory.tags.contains(tag))
                {
                    continue;
                }

                let cosine = semantic_scores.and_then(|s| s.get(id).copied());
                let match_type = Self::best_match_type(&all_signals, *id);

                // Apply importance boost
                let final_score = rrf_score * memory.importance.boost_factor();

                filtered.push(SearchResult {
                    memory,
                    score: final_score,
                    snippet: None,
                    match_type,
                    cosine_score: cosine,
                });
            }
        }

        // Sort by score DESC
        filtered.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        filtered.truncate(query.limit as usize);

        // Extract snippets if requested
        if query.include_snippet {
            for result in &mut filtered {
                result.snippet = self.extract_snippet(&result.memory.content, &query.text);
            }
        }

        Ok(filtered)
    }

    /// Search for memories whose entities match the query text.
    fn search_entities(
        conn: &rusqlite::Connection,
        query: &str,
        project: Option<&str>,
    ) -> Vec<Uuid> {
        let like_pattern = format!("%{}%", query);
        let mut results = Vec::new();

        let sql = if let Some(_proj) = project {
            "SELECT DISTINCT e.memory_id FROM memory_entities e
             JOIN memories m ON m.id = e.memory_id
             WHERE e.entity_name LIKE ?1 AND m.project = ?2 AND m.deleted_at IS NULL
             LIMIT 20"
        } else {
            "SELECT DISTINCT e.memory_id FROM memory_entities e
             JOIN memories m ON m.id = e.memory_id
             WHERE e.entity_name LIKE ?1 AND m.deleted_at IS NULL
             LIMIT 20"
        };

        if let Some(proj) = project {
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok(rows) = stmt.query_map(rusqlite::params![like_pattern, proj], |row| {
                    row.get::<_, String>(0)
                }) {
                    for id_str in rows.flatten() {
                        if let Ok(id) = Uuid::parse_str(&id_str) {
                            results.push(id);
                        }
                    }
                }
            }
        } else {
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok(rows) = stmt.query_map(rusqlite::params![like_pattern], |row| {
                    row.get::<_, String>(0)
                }) {
                    for id_str in rows.flatten() {
                        if let Ok(id) = Uuid::parse_str(&id_str) {
                            results.push(id);
                        }
                    }
                }
            }
        }

        results
    }

    /// Temporal recency signal: most recently accessed/updated memories.
    fn search_recency(&self, conn: &rusqlite::Connection, query: &SearchQuery) -> Vec<Uuid> {
        let mut results = Vec::new();
        let limit_i64 = (query.limit * 2) as i64;

        // Use two separate blocks to avoid Rust type incompatibility
        if let Some(ref project) = query.project {
            let sql = "SELECT id FROM memories
                       WHERE project = ?1 AND deleted_at IS NULL
                       ORDER BY last_accessed_at IS NULL, last_accessed_at DESC, updated_at DESC
                       LIMIT ?2";
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok(rows) = stmt.query_map(rusqlite::params![project, limit_i64], |row| {
                    row.get::<_, String>(0)
                }) {
                    for id_str in rows.flatten() {
                        if let Ok(id) = Uuid::parse_str(&id_str) {
                            results.push(id);
                        }
                    }
                }
            }
        } else {
            let sql = "SELECT id FROM memories
                       WHERE deleted_at IS NULL
                       ORDER BY last_accessed_at IS NULL, last_accessed_at DESC, updated_at DESC
                       LIMIT ?1";
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok(rows) =
                    stmt.query_map(rusqlite::params![limit_i64], |row| row.get::<_, String>(0))
                {
                    for id_str in rows.flatten() {
                        if let Ok(id) = Uuid::parse_str(&id_str) {
                            results.push(id);
                        }
                    }
                }
            }
        }

        results
    }

    /// Determina el mejor match type para una memoria basado en sus señales.
    fn best_match_type(signals: &[RankedSignal], memory_id: Uuid) -> MatchType {
        // Find the signal with highest score
        let best = signals
            .iter()
            .filter(|s| s.memory_id == memory_id)
            .min_by_key(|s| s.rank);

        match best {
            Some(s) if s.rank < 10 => MatchType::Fts,
            Some(_) => MatchType::Fuzzy,
            None => MatchType::Fts,
        }
    }

    /// Retrieves a single memory by ID from the connection.
    fn get_memory(conn: &rusqlite::Connection, id: Uuid) -> rusqlite::Result<Option<Memory>> {
        let mut stmt = conn.prepare(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at,
             deprecated_at, deprecated_reason, supersedes_id, context_inject_count, origin_peer,
             is_encrypted, encrypted_for, valid_from, valid_until, provenance
             FROM memories WHERE id = ?1 AND deleted_at IS NULL"
        )?;

        let result = stmt.query_row(rusqlite::params![id.to_string()], |row| {
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
                last_accessed_at: row.get::<_, Option<String>>(19)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                last_seen_at: row.get::<_, Option<String>>(20)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                deleted_at: row.get::<_, Option<String>>(21)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                deprecated_at: row.get::<_, Option<String>>(22)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                deprecated_reason: row.get(23)?,
                supersedes_id: row.get(24)?,
                context_inject_count: row.get(25)?,
                origin_peer: row.get(26)?,
                is_encrypted: row
                    .get::<_, Option<bool>>(27)
                    .unwrap_or(Some(false))
                    .unwrap_or(false),
                encrypted_for: row.get::<_, Option<String>>(28).unwrap_or(None),
                valid_from: row.get::<_, Option<String>>(29)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                valid_until: row.get::<_, Option<String>>(30)?.and_then(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .ok()
                        .map(|d| d.with_timezone(&Utc))
                }),
                provenance: row.get(31)?,
            })
        });

        match result {
            Ok(memory) => Ok(Some(memory)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn search_fts(
        &self,
        conn: &rusqlite::Connection,
        query: &SearchQuery,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let mut sql = String::from(
            "SELECT m.id, m.project, m.scope, m.title, m.content, m.what, m.why, m.context, m.learned,
             m.memory_type, m.importance, m.tags, m.topic_key, m.access_count, m.revision_count,
             m.duplicate_count, m.normalized_hash, m.created_at, m.updated_at, m.last_accessed_at, m.last_seen_at, m.deleted_at,
             0.0 as rank_score
             FROM memories_fts fts
             JOIN memories m ON m.rowid = fts.rowid
             WHERE memories_fts MATCH ?1 AND m.deleted_at IS NULL",
        );

        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        params.push(&query.text);

        if let Some(project) = &query.project {
            sql.push_str(" AND m.project = ?");
            params.push(project);
        }

        sql.push_str(" ORDER BY m.updated_at DESC LIMIT ?");
        let limit = query.limit as i64;
        params.push(&limit);

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| {
            let _rank: f64 = row.get(22)?;
            let score = 1.0; // FTS match gets base score 1.0

            Ok(SearchResult {
                memory: Memory {
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
                },
                score,
                snippet: None,
                match_type: MatchType::Fts,
                cosine_score: None,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    fn search_fuzzy(
        &self,
        conn: &rusqlite::Connection,
        query: &SearchQuery,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let matcher = SkimMatcherV2::default();

        let mut sql = String::from(
            "SELECT id, project, scope, title, content, what, why, context, learned,
             memory_type, importance, tags, topic_key, access_count, revision_count,
             duplicate_count, normalized_hash, created_at, updated_at, last_accessed_at, last_seen_at, deleted_at
             FROM memories WHERE deleted_at IS NULL",
        );
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();

        if let Some(project) = &query.project {
            sql.push_str(" AND project = ?");
            params.push(project);
        }

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params.as_slice(), |row| {
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
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            let memory = row?;
            if let Some(score) = matcher.fuzzy_match(&memory.title, &query.text) {
                let normalized_score = (score as f64) / 100.0; // Normalize to ~0-1
                results.push(SearchResult {
                    memory,
                    score: normalized_score,
                    snippet: None,
                    match_type: MatchType::Fuzzy,
                    cosine_score: None,
                });
            }
        }

        // Sort and truncate
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(query.limit as usize);

        Ok(results)
    }

    fn extract_snippet(&self, content: &str, query: &str) -> Option<String> {
        let lower_content = content.to_lowercase();
        let lower_query = query.to_lowercase();

        if let Some(pos) = lower_content.find(&lower_query) {
            let start = pos.saturating_sub(50);
            let end = (pos + query.len() + 50).min(content.len());
            Some(content[start..end].to_string())
        } else {
            Some(content[..content.len().min(100)].to_string())
        }
    }
}
