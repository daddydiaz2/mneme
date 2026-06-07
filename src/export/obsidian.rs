use std::path::Path;

use crate::store::memory::{Memory, MemoryRelation, MemoryType};
use chrono::Utc;

/// Exporta el grafo de conocimiento de un proyecto a un vault de Obsidian.
///
/// Cada memoria se exporta como un archivo .md individual con:
/// - Frontmatter YAML con metadatos
/// - [[wikilinks]] a memorias relacionadas
/// - Tags de Obsidian
pub fn export_to_obsidian(
    memories: &[Memory],
    relations: &[MemoryRelation],
    project: &str,
    output_dir: &Path,
) -> crate::error::Result<ObsidianExportStats> {
    let vault_root = output_dir.join("mneme-export");
    let memories_dir = vault_root.join("memories");
    let graph_dir = vault_root.join(".graph");

    std::fs::create_dir_all(&memories_dir)?;
    std::fs::create_dir_all(&graph_dir)?;

    let mut stats = ObsidianExportStats::default();

    // Build relation map: memory_id -> linked memory ids
    let mut link_map: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    for rel in relations {
        link_map
            .entry(rel.source_id.to_string())
            .or_default()
            .push((rel.target_id.to_string(), rel.relation_type.to_string()));
        // Bidirectional for Obsidian navigation
        link_map
            .entry(rel.target_id.to_string())
            .or_default()
            .push((rel.source_id.to_string(), rel.relation_type.to_string()));
    }

    // Build title map for wikilinks
    let title_map: std::collections::HashMap<String, &Memory> =
        memories.iter().map(|m| (m.id.to_string(), m)).collect();

    // Export each memory
    for memory in memories {
        let filename = sanitize_filename(&memory.title);
        let filepath = memories_dir.join(format!("{}.md", filename));

        let mut content = String::new();

        // YAML frontmatter
        content.push_str("---\n");
        content.push_str(&format!("id: {}\n", memory.id));
        content.push_str(&format!("title: \"{}\"\n", memory.title));
        content.push_str(&format!("type: {}\n", memory.memory_type));
        content.push_str(&format!("importance: {}\n", memory.importance));
        content.push_str(&format!("scope: {}\n", memory.scope));
        content.push_str(&format!("project: {}\n", memory.project));

        if let Some(ref topic_key) = memory.topic_key {
            content.push_str(&format!("topic_key: {}\n", topic_key));
        }

        // Tags as Obsidian tags
        if !memory.tags.is_empty() {
            content.push_str(&format!("tags: [{}]\n", memory.tags.join(", ")));
        }

        // Structured fields
        if let Some(ref what) = memory.what {
            content.push_str(&format!("what: \"{}\"\n", what));
        }
        if let Some(ref why) = memory.why {
            content.push_str(&format!("why: \"{}\"\n", why));
        }
        if let Some(ref ctx) = memory.context {
            content.push_str(&format!("context: \"{}\"\n", ctx));
        }
        if let Some(ref learned) = memory.learned {
            content.push_str(&format!("learned: \"{}\"\n", learned));
        }

        content.push_str(&format!("created_at: {}\n", memory.created_at.to_rfc3339()));
        content.push_str(&format!("updated_at: {}\n", memory.updated_at.to_rfc3339()));

        // Temporal fields
        if let Some(ref vf) = memory.valid_from {
            content.push_str(&format!("valid_from: {}\n", vf.to_rfc3339()));
        }
        if let Some(ref vu) = memory.valid_until {
            content.push_str(&format!("valid_until: {}\n", vu.to_rfc3339()));
        }

        content.push_str("---\n\n");

        // Content
        content.push_str(&format!("# {}\n\n", memory.title));

        // Structured fields as markdown
        if let Some(ref what) = memory.what {
            content.push_str(&format!("**What:** {}\n\n", what));
        }
        if let Some(ref why) = memory.why {
            content.push_str(&format!("**Why:** {}\n\n", why));
        }

        content.push_str(&format!("{}\n\n", memory.content));

        if let Some(ref learned) = memory.learned {
            content.push_str(&format!("**Learned:** {}\n\n", learned));
        }

        // Tags as Obsidian inline tags
        if !memory.tags.is_empty() {
            let tags_line: String = memory
                .tags
                .iter()
                .map(|t| format!("#{}", t.replace(' ', "-")))
                .collect::<Vec<_>>()
                .join(" ");
            content.push_str(&format!("{}\n\n", tags_line));
        }

        // Wikilinks to related memories
        if let Some(links) = link_map.get(&memory.id.to_string()) {
            content.push_str("## Related\n\n");
            for (target_id, rel_type) in links {
                if let Some(target) = title_map.get(target_id) {
                    let target_filename = sanitize_filename(&target.title);
                    content.push_str(&format!(
                        "- [[{}|{}]] _({})_\n",
                        target_filename, target.title, rel_type
                    ));
                }
            }
            content.push('\n');
        }

        let content_len = content.len() as u64;
        std::fs::write(&filepath, content)?;
        stats.files_written += 1;
        stats.bytes_written += content_len;
    }

    // Generate index file
    let index_content = generate_vault_index(memories, project, &link_map, &title_map);
    stats.bytes_written += index_content.len() as u64;
    std::fs::write(vault_root.join("README.md"), index_content)?;
    stats.files_written += 1;

    // Generate graph view data
    let graph_data = generate_graph_data(memories, relations);
    let graph_json = serde_json::to_string_pretty(&graph_data)?;
    std::fs::write(graph_dir.join("graph.json"), &graph_json)?;
    stats.bytes_written += graph_json.len() as u64;
    stats.files_written += 1;

    // Create .obsidian metadata
    let obsidian_dir = vault_root.join(".obsidian");
    std::fs::create_dir_all(&obsidian_dir)?;
    let app_json = serde_json::json!({
        "baseApp": "obsidian",
        "version": "1.5.0"
    });
    std::fs::write(
        obsidian_dir.join("app.json"),
        serde_json::to_string_pretty(&app_json)?,
    )?;

    Ok(stats)
}

/// Genera el archivo README.md del vault.
fn generate_vault_index(
    memories: &[Memory],
    project: &str,
    link_map: &std::collections::HashMap<String, Vec<(String, String)>>,
    title_map: &std::collections::HashMap<String, &Memory>,
) -> String {
    let now = Utc::now().to_rfc3339();
    let mut content = String::new();

    content.push_str("---\n");
    content.push_str("title: mneme vault\n");
    content.push_str(&format!("project: {}\n", project));
    content.push_str(&format!("exported_at: {}\n", now));
    content.push_str(&format!("total_memories: {}\n", memories.len()));
    content.push_str("---\n\n");
    content.push_str(&format!("# 🧠 Mneme Vault: {}\n\n", project));
    content.push_str(&format!(
        "Export generated at {} with {} memories.\n\n",
        now,
        memories.len()
    ));

    // By type
    let mut by_type: std::collections::HashMap<&MemoryType, Vec<&Memory>> =
        std::collections::HashMap::new();
    for memory in memories {
        by_type.entry(&memory.memory_type).or_default().push(memory);
    }

    content.push_str("## By Type\n\n");
    let mut type_keys: Vec<&&MemoryType> = by_type.keys().collect();
    type_keys.sort_by_key(|t| t.to_string());
    for t in type_keys {
        if let Some(mems) = by_type.get(t) {
            content.push_str(&format!("- **{}** ({}):\n", t, mems.len()));
            for mem in mems.iter().take(5) {
                let filename = sanitize_filename(&mem.title);
                content.push_str(&format!("  - [[{}|{}]]\n", filename, mem.title));
            }
            if mems.len() > 5 {
                content.push_str(&format!("  - _... and {} more_\n", mems.len() - 5));
            }
        }
    }

    content.push_str("\n## Most Connected\n\n");
    let mut connected: Vec<(String, usize)> = link_map
        .iter()
        .map(|(id, links)| (id.clone(), links.len()))
        .collect();
    connected.sort_by(|a, b| b.1.cmp(&a.1));
    for (id, count) in connected.iter().take(10) {
        if let Some(mem) = title_map.get(id) {
            let filename = sanitize_filename(&mem.title);
            content.push_str(&format!(
                "- [[{}|{}]] ({} connections)\n",
                filename, mem.title, count
            ));
        }
    }

    content
}

/// Genera datos de grafo para visualización.
fn generate_graph_data(memories: &[Memory], relations: &[MemoryRelation]) -> serde_json::Value {
    let nodes: Vec<serde_json::Value> = memories
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id.to_string(),
                "title": m.title,
                "type": m.memory_type.to_string(),
                "importance": m.importance.to_string(),
            })
        })
        .collect();

    let edges: Vec<serde_json::Value> = relations
        .iter()
        .map(|r| {
            serde_json::json!({
                "source": r.source_id.to_string(),
                "target": r.target_id.to_string(),
                "type": r.relation_type.to_string(),
                "confidence": r.confidence,
            })
        })
        .collect();

    serde_json::json!({ "nodes": nodes, "edges": edges })
}

/// Sanitiza un string para usar como nombre de archivo.
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c == '.' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Estadísticas de la exportación.
#[derive(Debug, Clone, Default)]
pub struct ObsidianExportStats {
    pub files_written: u32,
    pub bytes_written: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::{Importance, Scope};
    use std::str::FromStr;

    fn make_test_memory(id: uuid::Uuid, title: &str, memory_type: &str) -> Memory {
        Memory {
            id,
            project: "test".to_string(),
            scope: Scope::Project,
            title: title.to_string(),
            content: format!("Content of {}", title),
            what: Some(format!("What: {}", title)),
            why: Some(format!("Why: {}", title)),
            context: None,
            learned: Some(format!("Learned: {}", title)),
            memory_type: MemoryType::from_str(memory_type).unwrap_or(MemoryType::Note),
            importance: Importance::High,
            tags: vec!["test".to_string(), "obsidian".to_string()],
            topic_key: Some(format!("test/{}", title.to_lowercase())),
            access_count: 1,
            revision_count: 1,
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
        }
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World"), "Hello World");
        assert_eq!(sanitize_filename("file/name:test"), "file-name-test");
    }

    #[test]
    fn test_export_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let mem1 = make_test_memory(uuid::Uuid::new_v4(), "Memory One", "decision");
        let mem2 = make_test_memory(uuid::Uuid::new_v4(), "Memory Two", "architecture");

        let stats = export_to_obsidian(&[mem1, mem2], &[], "test-project", dir.path()).unwrap();

        assert!(stats.files_written >= 2); // at least 2 memory files
        assert!(dir.path().join("mneme-export/memories").exists());
        assert!(dir.path().join("mneme-export/README.md").exists());
    }
}
