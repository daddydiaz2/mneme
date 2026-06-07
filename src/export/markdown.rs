use crate::store::memory::{Importance, Memory, MemoryType, Scope};
use chrono::Utc;

/// Representa una memoria importada desde Markdown.
#[derive(Debug)]
pub struct ImportedMemory {
    pub title: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: Importance,
    pub tags: Vec<String>,
    pub scope: Scope,
    pub what: Option<String>,
    pub why: Option<String>,
    pub context: Option<String>,
    pub learned: Option<String>,
}

/// Exporta una lista de memorias al formato Markdown.
pub fn export_to_markdown(memories: &[Memory], project: &str) -> String {
    let mut out = String::new();
    let now = Utc::now().to_rfc3339();

    out.push_str(&format!("# mneme export — proyecto: {}\n", project));
    out.push_str(&format!("Exportado el: {}\n", now));
    out.push_str(&format!("Total: {} memorias\n", memories.len()));

    for memory in memories {
        out.push_str("\n---\n\n");
        out.push_str(&memory_to_markdown_section(memory));
    }
    out
}

fn memory_to_markdown_section(memory: &Memory) -> String {
    let mut s = String::new();
    s.push_str(&format!("## {}\n", memory.title));
    s.push_str(&format!("- **ID**: `{}`\n", memory.id));
    s.push_str(&format!("- **Tipo**: {}\n", memory.memory_type));
    s.push_str(&format!("- **Importancia**: {}\n", memory.importance));
    s.push_str(&format!("- **Scope**: {}\n", memory.scope));

    if !memory.tags.is_empty() {
        let tags = memory
            .tags
            .iter()
            .map(|t| format!("`{}`", t))
            .collect::<Vec<_>>()
            .join(" ");
        s.push_str(&format!("- **Tags**: {}\n", tags));
    }

    s.push_str(&format!(
        "- **Creado**: {}\n",
        memory.created_at.to_rfc3339()
    ));
    s.push_str(&format!(
        "- **Actualizado**: {}\n",
        memory.updated_at.to_rfc3339()
    ));

    if memory.is_encrypted {
        let for_whom = memory.encrypted_for.as_deref().unwrap_or("unknown");
        s.push_str(&format!("- **🔒 Encriptado**: sí ({})\n", for_whom));
        s.push_str("\n*[contenido encriptado — usar `mneme decrypt <id>` para ver]*\n");
        return s;
    }

    s.push('\n');

    if !memory.content.is_empty() {
        s.push_str(&memory.content);
        s.push('\n');
    }

    if let Some(what) = &memory.what {
        s.push_str(&format!("\n**What**: {}\n", what));
    }
    if let Some(why) = &memory.why {
        s.push_str(&format!("**Why**: {}\n", why));
    }
    if let Some(context) = &memory.context {
        s.push_str(&format!("**Context**: {}\n", context));
    }
    if let Some(learned) = &memory.learned {
        s.push_str(&format!("**Learned**: {}\n", learned));
    }

    s.push('\n');
    s
}

/// Importa memorias desde Markdown.
/// Parsea el formato generado por `export_to_markdown`.
/// Retorna los datos parseados como `ImportedMemory` (sin ID — se genera nuevo al guardar).
pub fn import_from_markdown(content: &str) -> crate::error::Result<Vec<ImportedMemory>> {
    let mut memories = Vec::new();

    let sections: Vec<&str> = content.split("\n---\n").collect();

    for section in sections.iter().skip(1) {
        let section = section.trim();
        if section.is_empty() {
            continue;
        }

        let title = section
            .lines()
            .find(|l| l.starts_with("## "))
            .map(|l| l.trim_start_matches("## ").trim().to_string())
            .unwrap_or_default();

        if title.is_empty() {
            continue;
        }

        let mut memory_type = MemoryType::Note;
        let mut importance = Importance::Medium;
        let mut tags = Vec::new();
        let mut scope = Scope::Project;

        for line in section.lines() {
            if let Some(rest) = line.strip_prefix("- **Tipo**: ") {
                let v = rest.trim();
                memory_type = v.parse().unwrap_or(MemoryType::Note);
            } else if let Some(rest) = line.strip_prefix("- **Importancia**: ") {
                let v = rest.trim();
                importance = v.parse().unwrap_or(Importance::Medium);
            } else if let Some(rest) = line.strip_prefix("- **Tags**: ") {
                let v = rest.trim();
                tags = v
                    .split_whitespace()
                    .map(|t| t.trim_matches('`').to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
            } else if let Some(rest) = line.strip_prefix("- **Scope**: ") {
                let v = rest.trim();
                scope = v.parse().unwrap_or(Scope::Project);
            }
        }

        let mut in_meta = true;
        let mut content_lines = Vec::new();
        let mut what = None;
        let mut why = None;
        let mut context_field = None;
        let mut learned = None;

        for line in section.lines().skip(1) {
            // skip title line
            if line.starts_with("- **") {
                in_meta = true;
                continue;
            }
            if in_meta && line.is_empty() {
                in_meta = false;
                continue;
            }
            if in_meta {
                continue;
            }

            if line.starts_with("**What**: ") {
                what = Some(line.trim_start_matches("**What**: ").to_string());
            } else if line.starts_with("**Why**: ") {
                why = Some(line.trim_start_matches("**Why**: ").to_string());
            } else if line.starts_with("**Context**: ") {
                context_field = Some(line.trim_start_matches("**Context**: ").to_string());
            } else if line.starts_with("**Learned**: ") {
                learned = Some(line.trim_start_matches("**Learned**: ").to_string());
            } else if !line.starts_with("*[contenido encriptado") {
                content_lines.push(line);
            }
        }

        let content = content_lines.join("\n").trim().to_string();

        memories.push(ImportedMemory {
            title,
            content,
            memory_type,
            importance,
            tags,
            scope,
            what,
            why,
            context: context_field,
            learned,
        });
    }

    Ok(memories)
}
