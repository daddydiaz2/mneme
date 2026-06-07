use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::time;

use crate::store::memory::{CreateMemoryInput, Importance, MemoryStore, MemoryType, Scope};

/// Estado de un archivo conocido.
struct FileState {
    modified: SystemTime,
    processed: bool,
}

/// Watcher por polling.
pub struct DirectoryWatcher {
    dir: PathBuf,
    ext: String,
    interval: Duration,
    known_files: HashMap<PathBuf, FileState>,
    store: MemoryStore,
    project: String,
}

impl DirectoryWatcher {
    pub fn new(
        dir: PathBuf,
        ext: String,
        interval_secs: u64,
        store: MemoryStore,
        project: String,
    ) -> Self {
        Self {
            dir,
            ext,
            interval: Duration::from_secs(interval_secs),
            known_files: HashMap::new(),
            store,
            project,
        }
    }

    /// Loop principal — corre hasta Ctrl-C.
    pub async fn run(&mut self) -> crate::error::Result<()> {
        tracing::info!(dir = %self.dir.display(), ext = %self.ext, "watch iniciado");
        println!(
            "👁  Watching {} for *{} files. Ctrl-C to stop.",
            self.dir.display(),
            self.ext
        );

        let mut ticker = time::interval(self.interval);
        loop {
            ticker.tick().await;
            if let Err(e) = self.scan().await {
                tracing::warn!(error = %e, "scan error");
            }
        }
    }

    /// Escanea el directorio en busca de archivos nuevos o modificados.
    /// Público para testing y uso manual.
    pub async fn scan(&mut self) -> crate::error::Result<()> {
        let entries = std::fs::read_dir(&self.dir)?;
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.to_string_lossy().ends_with(&self.ext) {
                continue;
            }
            let meta = std::fs::metadata(&path)?;
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

            let should_process = match self.known_files.get(&path) {
                None => true,
                Some(state) => !state.processed || modified > state.modified,
            };

            if should_process {
                match self.process_file(&path).await {
                    Ok(title) => {
                        println!("✓ Guardado: {}", title);
                        self.known_files.insert(
                            path,
                            FileState {
                                modified,
                                processed: true,
                            },
                        );
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "error procesando archivo");
                        self.known_files.insert(
                            path,
                            FileState {
                                modified,
                                processed: false,
                            },
                        );
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_file(&self, path: &Path) -> crate::error::Result<String> {
        let content = std::fs::read_to_string(path)?;
        let parsed = parse_mneme_file(&content);

        let input = CreateMemoryInput {
            project: self.project.clone(),
            scope: Some(Scope::Project),
            title: parsed.title,
            content: parsed.content,
            what: None,
            why: None,
            context: None,
            learned: None,
            memory_type: parsed.memory_type,
            importance: parsed.importance,
            tags: parsed.tags,
            topic_key: Some(format!(
                "watch/{}",
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
            )),
            capture_prompt: None,
            encrypt: false,
        };

        let memory = self.store.save(input, None, None)?;
        Ok(memory.title)
    }
}

struct ParsedFile {
    title: String,
    content: String,
    memory_type: MemoryType,
    importance: Importance,
    tags: Vec<String>,
}

/// Parsea un archivo .mneme con o sin frontmatter YAML.
fn parse_mneme_file(content: &str) -> ParsedFile {
    if content.starts_with("---") {
        parse_with_frontmatter(content)
    } else {
        parse_simple(content)
    }
}

fn parse_with_frontmatter(content: &str) -> ParsedFile {
    // Extraer bloque entre --- y ---
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    let (frontmatter, body) = if parts.len() >= 3 {
        (parts[1].trim(), parts[2].trim())
    } else {
        ("", content)
    };

    let mut title = String::new();
    let mut memory_type = MemoryType::Note;
    let mut importance = Importance::Medium;
    let mut tags = vec![];

    for line in frontmatter.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "title" => title = v.to_string(),
                "type" => memory_type = v.parse().unwrap_or(MemoryType::Note),
                "importance" => importance = v.parse().unwrap_or(Importance::Medium),
                "tags" => {
                    tags = v
                        .split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                }
                _ => {}
            }
        }
    }

    if title.is_empty() {
        title = body.lines().next().unwrap_or("untitled").to_string();
    }

    ParsedFile {
        title,
        content: body.to_string(),
        memory_type,
        importance,
        tags,
    }
}

fn parse_simple(content: &str) -> ParsedFile {
    let mut lines = content.lines();
    let title = lines.next().unwrap_or("untitled").to_string();
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    ParsedFile {
        title: title.clone(),
        content: if body.is_empty() { title } else { body },
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_extracts_title() {
        let content = "---\ntitle: Test Title\ntype: note\n---\ncontent here";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Test Title");
    }

    #[test]
    fn test_parse_frontmatter_extracts_type() {
        let content = "---\ntitle: T\ntype: decision\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.memory_type, MemoryType::Decision));
    }

    #[test]
    fn test_parse_frontmatter_extracts_tags() {
        let content = "---\ntitle: T\ntags: rust, auth, jwt\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.tags, vec!["rust", "auth", "jwt"]);
    }

    #[test]
    fn test_parse_frontmatter_extracts_importance() {
        let content = "---\ntitle: Critical Mem\ntype: note\nimportance: critical\n---\nimportant!";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.importance, Importance::Critical));
    }

    #[test]
    fn test_parse_frontmatter_importance_high() {
        let content = "---\ntitle: High Mem\nimportance: high\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.importance, Importance::High));
    }

    #[test]
    fn test_parse_frontmatter_importance_low() {
        let content = "---\ntitle: Low Mem\nimportance: low\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.importance, Importance::Low));
    }

    #[test]
    fn test_parse_frontmatter_invalid_type_uses_note() {
        let content = "---\ntitle: T\ntype: invalid_type_xyz\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.memory_type, MemoryType::Note));
    }

    #[test]
    fn test_parse_frontmatter_invalid_importance_uses_medium() {
        let content = "---\ntitle: T\nimportance: uber\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(matches!(parsed.importance, Importance::Medium));
    }

    #[test]
    fn test_parse_frontmatter_missing_title_falls_back_to_body() {
        let content = "---\ntype: decision\n---\nFallback Title\ncontent";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Fallback Title");
    }

    #[test]
    fn test_parse_frontmatter_empty_body() {
        let content = "---\ntitle: Only Title\ntype: note\n---";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Only Title");
        assert!(parsed.content.is_empty() || parsed.content == "Only Title");
    }

    #[test]
    fn test_parse_frontmatter_no_closing_separator() {
        let content = "---\ntitle: No Close\ntype: bugfix\nbody line";
        let parsed = parse_mneme_file(content);
        // Without closing ---, it falls back to treating the whole thing as simple format
        // The first line "---" becomes the title
        assert!(!parsed.title.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_all_types_roundtrip() {
        for (type_str, expected) in [
            ("architecture", MemoryType::Architecture),
            ("decision", MemoryType::Decision),
            ("bugfix", MemoryType::Bugfix),
            ("pattern", MemoryType::Pattern),
            ("convention", MemoryType::Convention),
            ("dependency", MemoryType::Dependency),
            ("workflow", MemoryType::Workflow),
            ("note", MemoryType::Note),
            ("config", MemoryType::Config),
            ("discovery", MemoryType::Discovery),
            ("learning", MemoryType::Learning),
        ] {
            let content = format!("---\ntitle: T\ntype: {type_str}\n---\nbody");
            let parsed = parse_mneme_file(&content);
            assert_eq!(
                parsed.memory_type, expected,
                "type '{type_str}' should parse to {expected:?}"
            );
        }
    }

    #[test]
    fn test_parse_simple_uses_first_line_as_title() {
        let content = "First Line Title\nRest of content\nMore content";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "First Line Title");
    }

    #[test]
    fn test_parse_simple_rest_is_content() {
        let content = "Title\nLine2\nLine3";
        let parsed = parse_mneme_file(content);
        assert!(parsed.content.contains("Line2"));
    }

    #[test]
    fn test_parse_simple_single_line() {
        let content = "Just a single line";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Just a single line");
        assert!(parsed.content.contains("Just a single line"));
    }

    #[test]
    fn test_parse_simple_empty_string() {
        let content = "";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "untitled");
    }

    #[test]
    fn test_parse_empty_frontmatter_title_falls_back_to_body() {
        let content = "---\n---\nfallback title\nsome content";
        let parsed = parse_mneme_file(content);
        assert!(!parsed.title.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_empty_tags_yields_empty_vec() {
        let content = "---\ntitle: T\ntags:\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert!(parsed.tags.is_empty());
    }

    #[test]
    fn test_parse_mneme_file_detects_frontmatter() {
        let content = "---\ntitle: FM\ntype: decision\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "FM");
        assert!(matches!(parsed.memory_type, MemoryType::Decision));
    }

    #[test]
    fn test_parse_mneme_file_detects_simple() {
        let content = "Simple title\nbody text";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Simple title");
    }

    #[test]
    fn test_parse_only_frontmatter_no_body_line() {
        let content = "---\ntitle: Solo\n---";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "Solo");
        assert!(parsed.memory_type == MemoryType::Note);
    }
}
