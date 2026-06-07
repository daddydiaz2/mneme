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

/// Motor de búsqueda híbrida: FTS5 + fuzzy matching + semántica con boost de importancia, decaimiento y recencia.
pub struct SearchEngine;

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    /// Crea un nuevo SearchEngine.
    pub fn new() -> Self {
        Self
    }

    /// Hybrid search: FTS5 + fuzzy matching + semántica con boost de importancia, decaimiento y recencia.
    ///
    /// # Arguments
    /// * `conn` - Conexión SQLite.
    /// * `query` - Query de búsqueda.
    /// * `weights` - Pesos para cada componente.
    /// * `semantic_scores` - Mapa de ID de memoria a score coseno (opcional).
    pub fn search(
        &self,
        conn: &rusqlite::Connection,
        query: &SearchQuery,
        weights: &SearchWeights,
        semantic_scores: Option<&HashMap<Uuid, f32>>,
    ) -> crate::error::Result<Vec<SearchResult>> {
        let mut results: HashMap<Uuid, SearchResult> = HashMap::new();

        // 1. FTS5 search (if query has 3+ chars)
        if query.text.len() >= 3 {
            let fts_results = self.search_fts(conn, query)?;
            for result in fts_results {
                results.insert(result.memory.id, result);
            }
        }

        // 2. Fuzzy search on titles (always run)
        let fuzzy_results = self.search_fuzzy(conn, query)?;
        for result in fuzzy_results {
            results
                .entry(result.memory.id)
                .and_modify(|existing| {
                    if result.score > existing.score {
                        existing.score = result.score;
                        existing.match_type = result.match_type.clone();
                    }
                })
                .or_insert(result);
        }

        // 3. Merge semantic scores
        if let Some(scores) = semantic_scores {
            for (id, cosine_score) in scores {
                if let Some(existing) = results.get_mut(id) {
                    existing.cosine_score = Some(*cosine_score);
                }
            }
        }

        // 4. Apply filters (type, importance, tags)
        let mut filtered: Vec<SearchResult> = results.into_values().collect();

        if let Some(memory_type) = &query.memory_type {
            filtered.retain(|r| &r.memory.memory_type == memory_type);
        }
        if let Some(importance) = &query.importance {
            filtered.retain(|r| &r.memory.importance == importance);
        }
        if !query.tags.is_empty() {
            filtered.retain(|r| query.tags.iter().all(|tag| r.memory.tags.contains(tag)));
        }

        // 5. Apply hybrid score, importance boost and decay
        let effective_weights = if semantic_scores.is_some() {
            *weights
        } else {
            weights.renormalize_without_semantic()
        };

        for result in &mut filtered {
            let fts_component = if result.match_type == MatchType::Fts {
                result.score * effective_weights.fts
            } else {
                0.0
            };
            let fuzzy_component = if result.match_type == MatchType::Fuzzy {
                result.score * effective_weights.fuzzy
            } else {
                0.0
            };
            let semantic_component = result
                .cosine_score
                .map(|s| f64::from(s) * effective_weights.semantic)
                .unwrap_or(0.0);

            let hybrid_score = fts_component + fuzzy_component + semantic_component;
            let boost = result.memory.importance.boost_factor();
            result.score = hybrid_score * boost;

            // Decay factor: 0.95 ^ days_without_access
            if let Some(last_accessed) = result.memory.last_accessed_at {
                let days = (Utc::now() - last_accessed).num_days() as f64;
                let decay = 0.95f64.powf(days);
                result.score *= decay;
            }

            // Recency factor: 1.0 + (1.0 / (1.0 + days_since_creation * 0.1))
            let days_since_creation = (Utc::now() - result.memory.created_at).num_days() as f64;
            let recency = 1.0 + (1.0 / (1.0 + days_since_creation * 0.1));
            result.score *= recency;
        }

        // 6. Sort by score DESC and truncate
        filtered.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        filtered.truncate(query.limit as usize);

        // 7. Extract snippets if requested
        if query.include_snippet {
            for result in &mut filtered {
                result.snippet = self.extract_snippet(&result.memory.content, &query.text);
            }
        }

        Ok(filtered)
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
