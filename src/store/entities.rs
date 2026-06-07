use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use std::str::FromStr;

use crate::store::memory::Memory;

/// Categoría de entidad extraída del contenido de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Concept,
    Person,
    Library,
    Technology,
    Framework,
    FilePath,
    Url,
    Command,
    Configuration,
    Workflow,
    Convention,
    Architecture,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EntityType::Concept => "concept",
            EntityType::Person => "person",
            EntityType::Library => "library",
            EntityType::Technology => "technology",
            EntityType::Framework => "framework",
            EntityType::FilePath => "file_path",
            EntityType::Url => "url",
            EntityType::Command => "command",
            EntityType::Configuration => "configuration",
            EntityType::Workflow => "workflow",
            EntityType::Convention => "convention",
            EntityType::Architecture => "architecture",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for EntityType {
    type Err = crate::error::MnemeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "concept" => Ok(EntityType::Concept),
            "person" => Ok(EntityType::Person),
            "library" => Ok(EntityType::Library),
            "technology" => Ok(EntityType::Technology),
            "framework" => Ok(EntityType::Framework),
            "file_path" => Ok(EntityType::FilePath),
            "url" => Ok(EntityType::Url),
            "command" => Ok(EntityType::Command),
            "configuration" => Ok(EntityType::Configuration),
            "workflow" => Ok(EntityType::Workflow),
            "convention" => Ok(EntityType::Convention),
            "architecture" => Ok(EntityType::Architecture),
            other => Err(crate::error::MnemeError::InvalidMemoryType(
                other.to_string(),
            )),
        }
    }
}

/// Entidad extraída del contenido de una memoria.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntity {
    pub id: i64,
    pub memory_id: Uuid,
    pub entity_name: String,
    pub entity_type: EntityType,
    pub confidence: f32,
    pub context: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
}

/// Link entre dos memorias que comparten una entidad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityLink {
    pub id: i64,
    pub entity_name: String,
    pub entity_type: EntityType,
    pub source_memory_id: Uuid,
    pub target_memory_id: Uuid,
    pub link_strength: f32,
}

/// Resultado de búsqueda por entidad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResult {
    pub entity: MemoryEntity,
    pub memory_title: String,
    pub memory_type: String,
    pub memory_importance: String,
}

/// Peso de entidad para boosting en búsqueda.
#[derive(Debug, Clone)]
pub struct EntityMatch {
    pub entity_name: String,
    pub entity_type: EntityType,
    pub score: f32,
}

/// Store para operaciones con entidades.
pub struct EntityStore {
    conn: Arc<Mutex<Connection>>,
}

impl EntityStore {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    /// Extrae y guarda entidades del contenido de una memoria.
    /// Usa heurísticas basadas en patrones (no LLM) para extracción inicial.
    pub fn extract_and_save(&self, memory: &Memory) -> crate::error::Result<Vec<MemoryEntity>> {
        let entities = Self::extract_entities(memory);
        let saved = self.save_entities(memory.id, &entities)?;

        // Create entity links between memories that share entities
        for entity in &saved {
            self.create_links_for_entity(&entity.entity_name, &entity.entity_type, memory.id)?;
        }

        Ok(saved)
    }

    /// Extrae entidades del contenido de una memoria usando heurísticas.
    /// Extrae entidades de un texto plano (sin guardar).
    pub fn extract_entities_from_text(text: &str) -> Vec<(String, String, f32)> {
        // Build a minimal Memory wrapper to reuse the extraction logic
        let memory = Memory {
            id: uuid::Uuid::nil(),
            project: String::new(),
            scope: crate::store::memory::Scope::Project,
            title: String::new(),
            content: text.to_string(),
            what: None,
            why: None,
            context: None,
            learned: None,
            memory_type: crate::store::memory::MemoryType::Note,
            importance: crate::store::memory::Importance::Medium,
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
        };
        Self::extract_entities(&memory)
            .into_iter()
            .map(|(name, etype, conf, _ctx)| (name, etype.to_string(), conf))
            .collect()
    }

    pub fn extract_entities(memory: &Memory) -> Vec<(String, EntityType, f32, Option<String>)> {
        let mut entities: HashMap<String, (EntityType, f32, Option<String>)> = HashMap::new();
        let content = &memory.content;
        let _text = content.to_lowercase();

        // 1. Detect URLs
        for url in Self::find_urls(content) {
            let entry = entities
                .entry(url.clone())
                .or_insert_with(|| (EntityType::Url, 0.5, None));
            entry.1 = (entry.1 + 1.0).min(1.0);
        }

        // 2. Detect file paths (patterns like /path/to/file or path/to/file.ext)
        for path in Self::find_file_paths(content) {
            let entry = entities
                .entry(path.clone())
                .or_insert_with(|| (EntityType::FilePath, 0.5, None));
            entry.1 = (entry.1 + 1.0).min(1.0);
        }

        // 3. Detect library/framework mentions (camelCase, hyphenated tech names in code context)
        for tech in Self::find_technologies(content) {
            let entry = entities
                .entry(tech.clone())
                .or_insert_with(|| (EntityType::Technology, 0.5, None));
            entry.1 = (entry.1 + 1.0).min(1.0);
        }

        // 4. Detect dependency names from Cargo.toml / package.json style mentions
        for dep in Self::find_dependencies(content) {
            let entry = entities
                .entry(dep.clone())
                .or_insert_with(|| (EntityType::Library, 0.6, None));
            entry.1 = (entry.1 + 1.0).min(1.0);
        }

        // 5. Extract named entities from "what", "why", "context", "learned" fields
        for field in [&memory.what, &memory.why, &memory.context, &memory.learned] {
            if let Some(field_text) = field {
                for concept in Self::find_key_concepts(field_text) {
                    let entry = entities.entry(concept.clone()).or_insert_with(|| {
                        (
                            EntityType::Concept,
                            0.4,
                            Some(field_text[..field_text.len().min(100)].to_string()),
                        )
                    });
                    entry.1 = (entry.1 + 0.5).min(1.0);
                    // Update context if we have a better one
                    if entry.2.is_none() {
                        entry.2 = Some(field_text[..field_text.len().min(100)].to_string());
                    }
                }
            }
        }

        // 6. Architectures and conventions from title + type
        let _title_lower = memory.title.to_lowercase();
        if matches!(
            memory.memory_type,
            crate::store::memory::MemoryType::Architecture
        ) {
            for concept in Self::find_key_concepts(&memory.title) {
                let entry = entities.entry(concept.clone()).or_insert_with(|| {
                    (
                        EntityType::Architecture,
                        0.7,
                        Some(memory.title[..memory.title.len().min(100)].to_string()),
                    )
                });
                entry.1 = (entry.1 + 0.5).min(1.0);
            }
        }

        // Filter low-confidence entities
        entities
            .into_iter()
            .filter(|(_, (_, confidence, _))| *confidence >= 0.3)
            .map(|(name, (etype, conf, ctx))| (name, etype, conf, ctx))
            .collect()
    }

    fn find_urls(content: &str) -> Vec<String> {
        // Simple URL detection: https?://... patterns
        let mut urls = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with("http://") || word.starts_with("https://") {
                let clean = word.trim_end_matches(['.', ',', ')', ']', '>']);
                if !clean.is_empty() {
                    urls.push(clean.to_string());
                }
            }
        }
        urls
    }

    fn find_file_paths(content: &str) -> Vec<String> {
        let mut paths = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            // Detect paths like src/main.rs, ./path/to/file, /absolute/path
            if line.contains('/') && !line.starts_with("http") && !line.starts_with('#') {
                // Check if it looks like a file path (has extension or common dir patterns)
                let has_ext = line.contains('.') && line.len() - line.rfind('.').unwrap() <= 6;
                let has_src = line.starts_with("src/") || line.contains("/src/");
                let is_abs = line.starts_with('/');
                if has_ext || has_src || is_abs {
                    let clean = line.trim_end_matches(['.', ',', ')', ']']);
                    if !clean.is_empty() && clean.len() > 3 {
                        paths.push(clean.to_string());
                    }
                }
            }
        }
        paths
    }

    fn find_technologies(content: &str) -> Vec<String> {
        let mut techs = Vec::new();
        let known_techs = [
            "rust",
            "python",
            "typescript",
            "javascript",
            "go",
            "react",
            "vue",
            "angular",
            "node",
            "deno",
            "bun",
            "sqlite",
            "postgresql",
            "mysql",
            "redis",
            "mongodb",
            "docker",
            "kubernetes",
            "aws",
            "gcp",
            "azure",
            "terraform",
            "ansible",
            "graphql",
            "rest",
            "grpc",
            "websocket",
            "tcp",
            "udp",
            "http",
            "linux",
            "macos",
            "windows",
            "nixos",
            "ubuntu",
            "debian",
            "alpine",
            "git",
            "github",
            "gitlab",
            "ci/cd",
            "github actions",
            "llm",
            "gpt",
            "claude",
            "gemini",
            "openai",
            "anthropic",
            "ollama",
            "mcp",
            "api",
            "sdk",
            "cli",
            "tui",
            "gui",
            "wasm",
            "webassembly",
            "docker compose",
            "nginx",
            "caddy",
            "tokio",
            "axum",
            "actix",
            "rocket",
            "diesel",
            "sqlx",
            "seaorm",
            "serde",
            "clap",
            "ratatui",
            "crossterm",
            "egui",
            "tauri",
        ];
        let lower = content.to_lowercase();
        for tech in &known_techs {
            // Match as whole word or hyphenated
            if lower.contains(tech) {
                // Check boundaries to avoid partial matches
                for window in lower.split_whitespace() {
                    let clean = window.trim_matches(|c: char| {
                        !c.is_alphanumeric() && c != '-' && c != '/' && c != '.'
                    });
                    let clean_lower = clean.to_lowercase();
                    if clean_lower == *tech
                        || clean_lower.starts_with(&format!("{}-", tech))
                        || clean_lower.starts_with(&format!("{}_", tech))
                    {
                        techs.push(clean.to_string());
                        break;
                    }
                }
            }
        }
        techs
    }

    fn find_dependencies(content: &str) -> Vec<String> {
        let mut deps = Vec::new();
        // Detect patterns like "dependency: foo", "crate: bar", "package: baz"
        let dep_indicators = [
            "dependency:",
            "crate:",
            "package:",
            "library:",
            "npm:",
            "gem:",
            "cargo:",
        ];
        let text_lower = content.to_lowercase();
        for indicator in &dep_indicators {
            if let Some(pos) = text_lower.find(indicator) {
                let after = &text_lower[pos + indicator.len()..];
                let dep_name = after
                    .split_whitespace()
                    .next()
                    .map(|s| {
                        s.trim_matches(|c: char| {
                            c == '`' || c == '"' || c == '\'' || c == ',' || c == '.'
                        })
                    })
                    .unwrap_or("")
                    .to_string();
                if dep_name.len() > 2 {
                    deps.push(dep_name);
                }
            }
        }
        // Also detect from Cargo.toml style: name = "x.y.z"
        for line in content.lines() {
            let line = line.trim();
            if line.contains(" = ") && (line.contains('"') || line.contains('\'')) {
                if let Some(eq_pos) = line.find(" = ") {
                    let name = line[..eq_pos].trim().to_string();
                    if name.len() > 2 && !name.starts_with('#') {
                        deps.push(name);
                    }
                }
            }
        }
        deps
    }

    fn find_key_concepts(text: &str) -> Vec<String> {
        let mut concepts = Vec::new();
        // Extract CamelCase and SCREAMING_SNAKE_CASE identifiers as potential concepts
        for word in text.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
            if clean.is_empty() || clean.len() < 4 {
                continue;
            }
            // CamelCase detection
            let upper_count = clean.chars().filter(|c| c.is_uppercase()).count();
            if upper_count >= 1 && clean.len() >= 4 && clean != clean.to_lowercase() {
                concepts.push(clean.to_string());
            }
            // SCREAMING_SNAKE_CASE
            if clean == clean.to_uppercase() && clean.contains('_') {
                concepts.push(clean.to_string());
            }
        }
        concepts
    }

    /// Guarda entidades en la base de datos.
    pub fn save_entities(
        &self,
        memory_id: Uuid,
        entities: &[(String, EntityType, f32, Option<String>)],
    ) -> crate::error::Result<Vec<MemoryEntity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut saved = Vec::new();

        for (name, etype, confidence, context) in entities {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO memory_entities (memory_id, entity_name, entity_type, confidence, context, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    memory_id.to_string(),
                    name,
                    etype.to_string(),
                    confidence,
                    context.as_deref(),
                    now,
                ],
            )?;

            let id = conn.last_insert_rowid();
            saved.push(MemoryEntity {
                id,
                memory_id,
                entity_name: name.clone(),
                entity_type: etype.clone(),
                confidence: *confidence,
                context: context.clone(),
                created_at: Utc::now(),
            });
        }

        Ok(saved)
    }

    /// Crea links entre memorias que comparten una entidad.
    fn create_links_for_entity(
        &self,
        entity_name: &str,
        entity_type: &EntityType,
        source_memory_id: Uuid,
    ) -> crate::error::Result<u32> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        // Find other memories that have the same entity
        let mut stmt = conn.prepare(
            "SELECT memory_id FROM memory_entities
             WHERE entity_name = ?1 AND entity_type = ?2 AND memory_id != ?3
             GROUP BY memory_id",
        )?;

        let rows = stmt.query_map(
            params![
                entity_name,
                entity_type.to_string(),
                source_memory_id.to_string()
            ],
            |row| row.get::<_, String>(0),
        )?;

        let mut link_count = 0u32;
        for row in rows {
            let target_id_str: String = row?;
            if let Ok(_target_id) = Uuid::parse_str(&target_id_str) {
                // Count co-occurrences for link strength
                let count: u32 = conn
                    .query_row(
                        "SELECT COUNT(*) FROM memory_entities
                         WHERE entity_name = ?1 AND entity_type = ?2
                         AND memory_id IN (?3, ?4)",
                        params![
                            entity_name,
                            entity_type.to_string(),
                            source_memory_id.to_string(),
                            target_id_str,
                        ],
                        |row| row.get(0),
                    )
                    .unwrap_or(1);

                let strength = (count as f32).min(5.0) / 5.0; // Normalize to 0-1

                conn.execute(
                    "INSERT OR REPLACE INTO entity_links (entity_name, entity_type, source_memory_id, target_memory_id, link_strength, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        entity_name,
                        entity_type.to_string(),
                        source_memory_id.to_string(),
                        target_id_str,
                        strength,
                        Utc::now().to_rfc3339(),
                    ],
                )?;
                link_count += 1;
            }
        }

        Ok(link_count)
    }

    /// Busca entidades por nombre (parcial).
    pub fn search_entities(
        &self,
        query: &str,
        entity_type: Option<&EntityType>,
        limit: u32,
    ) -> crate::error::Result<Vec<EntitySearchResult>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;

        let like_pattern = format!("%{}%", query);
        let limit_i64 = limit as i64;

        let sql = if let Some(_etype) = entity_type {
            "SELECT e.id, e.memory_id, e.entity_name, e.entity_type, e.confidence, e.context, e.created_at,
                        m.title, m.memory_type, m.importance
                 FROM memory_entities e
                 JOIN memories m ON m.id = e.memory_id
                 WHERE e.entity_name LIKE ?1 AND e.entity_type = ?2 AND m.deleted_at IS NULL
                 ORDER BY e.confidence DESC, LENGTH(e.entity_name) ASC
                 LIMIT ?3".to_string()
        } else {
            "SELECT e.id, e.memory_id, e.entity_name, e.entity_type, e.confidence, e.context, e.created_at,
                        m.title, m.memory_type, m.importance
                 FROM memory_entities e
                 JOIN memories m ON m.id = e.memory_id
                 WHERE e.entity_name LIKE ?1 AND m.deleted_at IS NULL
                 ORDER BY e.confidence DESC, LENGTH(e.entity_name) ASC
                 LIMIT ?2".to_string()
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<EntitySearchResult>;

        if let Some(etype) = entity_type {
            rows = stmt
                .query_map(params![like_pattern, etype.to_string(), limit_i64], |row| {
                    Ok(EntitySearchResult {
                        entity: MemoryEntity {
                            id: row.get(0)?,
                            memory_id: Uuid::parse_str(&row.get::<_, String>(1)?).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    1,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?,
                            entity_name: row.get(2)?,
                            entity_type: EntityType::from_str(&row.get::<_, String>(3)?).map_err(
                                |e| {
                                    rusqlite::Error::FromSqlConversionFailure(
                                        3,
                                        rusqlite::types::Type::Text,
                                        Box::new(e),
                                    )
                                },
                            )?,
                            confidence: row.get(4)?,
                            context: row.get(5)?,
                            created_at: chrono::DateTime::parse_from_rfc3339(
                                &row.get::<_, String>(6)?,
                            )
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    6,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?
                            .with_timezone(&Utc),
                        },
                        memory_title: row.get(7)?,
                        memory_type: row.get(8)?,
                        memory_importance: row.get(9)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
        } else {
            rows = stmt
                .query_map(params![like_pattern, limit_i64], |row| {
                    Ok(EntitySearchResult {
                        entity: MemoryEntity {
                            id: row.get(0)?,
                            memory_id: Uuid::parse_str(&row.get::<_, String>(1)?).map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    1,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?,
                            entity_name: row.get(2)?,
                            entity_type: EntityType::from_str(&row.get::<_, String>(3)?).map_err(
                                |e| {
                                    rusqlite::Error::FromSqlConversionFailure(
                                        3,
                                        rusqlite::types::Type::Text,
                                        Box::new(e),
                                    )
                                },
                            )?,
                            confidence: row.get(4)?,
                            context: row.get(5)?,
                            created_at: chrono::DateTime::parse_from_rfc3339(
                                &row.get::<_, String>(6)?,
                            )
                            .map_err(|e| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    6,
                                    rusqlite::types::Type::Text,
                                    Box::new(e),
                                )
                            })?
                            .with_timezone(&Utc),
                        },
                        memory_title: row.get(7)?,
                        memory_type: row.get(8)?,
                        memory_importance: row.get(9)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
        }

        Ok(rows)
    }

    /// Obtiene todas las entidades de una memoria.
    pub fn get_memory_entities(&self, memory_id: Uuid) -> crate::error::Result<Vec<MemoryEntity>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, memory_id, entity_name, entity_type, confidence, context, created_at
             FROM memory_entities WHERE memory_id = ?1 ORDER BY confidence DESC",
        )?;

        let rows = stmt.query_map(params![memory_id.to_string()], |row| {
            Ok(MemoryEntity {
                id: row.get(0)?,
                memory_id: Uuid::parse_str(&row.get::<_, String>(1)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                entity_name: row.get(2)?,
                entity_type: EntityType::from_str(&row.get::<_, String>(3)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                confidence: row.get(4)?,
                context: row.get(5)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;

        let mut entities = Vec::new();
        for row in rows {
            entities.push(row?);
        }
        Ok(entities)
    }

    /// Obtiene los entity links de una memoria.
    pub fn get_memory_links(
        &self,
        memory_id: Uuid,
        limit: u32,
    ) -> crate::error::Result<Vec<(EntityLink, String)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT l.id, l.entity_name, l.entity_type, l.source_memory_id, l.target_memory_id,
                    l.link_strength, m.title
             FROM entity_links l
             JOIN memories m ON m.id = l.target_memory_id
             WHERE l.source_memory_id = ?1 AND m.deleted_at IS NULL
             ORDER BY l.link_strength DESC
             LIMIT ?2",
        )?;

        let limit_i64 = limit as i64;
        let rows = stmt.query_map(params![memory_id.to_string(), limit_i64], |row| {
            let link = EntityLink {
                id: row.get(0)?,
                entity_name: row.get(1)?,
                entity_type: EntityType::from_str(&row.get::<_, String>(2)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                source_memory_id: Uuid::parse_str(&row.get::<_, String>(3)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                target_memory_id: Uuid::parse_str(&row.get::<_, String>(4)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                link_strength: row.get(5)?,
            };
            let target_title: String = row.get(6)?;
            Ok((link, target_title))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Obtiene los nombres de entidad más frecuentes en un proyecto.
    pub fn frequent_entities(
        &self,
        project: &str,
        limit: u32,
    ) -> crate::error::Result<Vec<(String, EntityType, u32)>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT e.entity_name, e.entity_type, COUNT(DISTINCT e.memory_id) as memory_count
             FROM memory_entities e
             JOIN memories m ON m.id = e.memory_id
             WHERE m.project = ?1 AND m.deleted_at IS NULL
             GROUP BY e.entity_name, e.entity_type
             ORDER BY memory_count DESC
             LIMIT ?2",
        )?;

        let limit_i64 = limit as i64;
        let rows = stmt.query_map(params![project, limit_i64], |row| {
            let entity_type = EntityType::from_str(&row.get::<_, String>(1)?).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
            Ok((row.get::<_, String>(0)?, entity_type, row.get::<_, u32>(2)?))
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }
}
