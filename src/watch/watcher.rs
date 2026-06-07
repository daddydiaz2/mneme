use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tokio::time;

use crate::store::memory::{CreateMemoryInput, Importance, MemoryStore, MemoryType, Scope};

/// Estado de un archivo conocido.
#[derive(Debug, Clone)]
struct FileState {
    modified: SystemTime,
    processed: bool,
    /// Content hash for change detection
    content_hash: u64,
    /// Memory ID if already saved
    memory_id: Option<uuid::Uuid>,
}

/// Watcher por polling con auto-indexing.
pub struct DirectoryWatcher {
    dir: PathBuf,
    ext: String,
    interval: Duration,
    known_files: HashMap<PathBuf, FileState>,
    store: MemoryStore,
    project: String,
    /// Only track new files, don't re-index existing ones
    track_new_only: bool,
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
            track_new_only: false,
        }
    }

    /// Set whether to only track new files (skip existing).
    pub fn with_track_new_only(mut self, val: bool) -> Self {
        self.track_new_only = val;
        self
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
    /// Público para testing y uso manual desde MCP.
    pub async fn scan(&mut self) -> crate::error::Result<ScanResult> {
        let mut result = ScanResult::default();

        if !self.dir.exists() {
            tracing::warn!(dir = %self.dir.display(), "watch directory does not exist");
            return Ok(result);
        }

        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(error = %e, "failed to read watch directory");
                return Ok(result);
            }
        };

        // Collect all entries first to avoid borrow issues
        let all_entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();

        for entry in &all_entries {
            let path = entry.path();
            let file_name = path.to_string_lossy();

            // Check file extension
            let is_match = self.ext == ".*"
                || file_name.ends_with(&self.ext)
                || (self.ext == ".md" && file_name.ends_with(".md"))
                || (self.ext == ".mneme" && file_name.ends_with(".mneme"));

            if !is_match {
                continue;
            }

            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            if !meta.is_file() {
                continue;
            }

            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let content_hash = Self::quick_hash(&path);

            let state = self.known_files.get(&path);
            let should_process = match state {
                None => !self.track_new_only,
                Some(st) => {
                    // Re-process if file changed
                    modified > st.modified || content_hash != st.content_hash
                }
            };

            if should_process {
                match self.process_file(&path, content_hash).await {
                    Ok(processed) => {
                        if processed {
                            result.indexed += 1;
                        } else {
                            result.skipped += 1;
                        }
                        self.known_files.insert(
                            path.clone(),
                            FileState {
                                modified,
                                processed: true,
                                content_hash,
                                memory_id: None,
                            },
                        );
                    }
                    Err(e) => {
                        tracing::warn!(path = %path.display(), error = %e, "error procesando archivo");
                        result.errors += 1;
                        self.known_files.insert(
                            path.clone(),
                            FileState {
                                modified,
                                processed: false,
                                content_hash,
                                memory_id: None,
                            },
                        );
                    }
                }
            } else {
                result.skipped += 1;
            }
        }

        // Detect deleted files
        let mut to_remove = Vec::new();
        let current_paths: std::collections::HashSet<PathBuf> = all_entries
            .iter()
            .map(|e| e.path())
            .collect();

        for tracked_path in self.known_files.keys() {
            if !current_paths.contains(tracked_path) {
                to_remove.push(tracked_path.clone());
            }
        }

        result.removed = to_remove.len() as u32;
        for path in to_remove {
            self.known_files.remove(&path);
        }

        Ok(result)
    }

    /// Compute a quick content hash for change detection.
    fn quick_hash(path: &Path) -> u64 {
        use std::hash::{Hash, Hasher};
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    async fn process_file(&self, path: &Path, _content_hash: u64) -> crate::error::Result<bool> {
        let content = std::fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(false);
        }

        let parsed = parse_mneme_file(&content);

        let input = CreateMemoryInput {
            project: self.project.clone(),
            scope: Some(Scope::Project),
            title: parsed.title,
            content: parsed.content,
            what: parsed.what,
            why: parsed.why,
            context: parsed.context,
            learned: parsed.learned,
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
            valid_from: None,
            valid_until: None,
            provenance: Some(format!("file://{}", path.to_string_lossy())),
        };

        let memory = self.store.save(input, None, None)?;
        tracing::info!(memory_id = %memory.id, title = %memory.title, "auto-indexed file");
        Ok(true)
    }

    /// Returns current tracked file count.
    pub fn tracked_count(&self) -> usize {
        self.known_files.len()
    }

    /// Returns summary of tracked files.
    pub fn tracked_summary(&self) -> Vec<(String, bool)> {
        self.known_files
            .iter()
            .map(|(path, state)| {
                (path.to_string_lossy().to_string(), state.processed)
            })
            .collect()
    }
}

/// Result of a scan cycle.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub indexed: u32,
    pub skipped: u32,
    pub errors: u32,
    pub removed: u32,
}

struct ParsedFile {
    title: String,
    content: String,
    memory_type: MemoryType,
    importance: Importance,
    tags: Vec<String>,
    what: Option<String>,
    why: Option<String>,
    context: Option<String>,
    learned: Option<String>,
}

/// Parsea un archivo .md o .mneme con frontmatter YAML.
/// Formato:
///   ---
///   title: Mi Memoria
///   type: decision
///   importance: high
///   tags: [rust, auth]
///   what: What we did
///   why: Why we did it
///   ---
///   Contenido de la memoria...
fn parse_mneme_file(content: &str) -> ParsedFile {
    if content.starts_with("---") {
        parse_with_frontmatter(content)
    } else {
        parse_simple(content)
    }
}

fn parse_with_frontmatter(content: &str) -> ParsedFile {
    // Extract block between --- and ---
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
    let mut what = None;
    let mut why = None;
    let mut context = None;
    let mut learned = None;

    for line in frontmatter.lines() {
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim();
            let v = v.trim();
            match k {
                "title" => title = v.to_string(),
                "type" => memory_type = v.parse().unwrap_or(MemoryType::Note),
                "importance" => importance = v.parse().unwrap_or(Importance::Medium),
                "tags" => {
                    let clean = v
                        .trim_start_matches('[')
                        .trim_end_matches(']');
                    tags = clean
                        .split(',')
                        .map(|t| t.trim().trim_matches('"').trim_matches('\'').to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                }
                "what" => what = Some(v.to_string()),
                "why" => why = Some(v.to_string()),
                "context" => context = Some(v.to_string()),
                "learned" => learned = Some(v.to_string()),
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
        what,
        why,
        context,
        learned,
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
        what: None,
        why: None,
        context: None,
        learned: None,
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
    fn test_parse_frontmatter_extracts_tags_array() {
        let content = "---\ntitle: T\ntags: [rust, auth, jwt]\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.tags, vec!["rust", "auth", "jwt"]);
    }

    #[test]
    fn test_parse_frontmatter_extracts_tags_csv() {
        let content = "---\ntitle: T\ntags: rust, auth, jwt\n---\nbody";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.tags, vec!["rust", "auth", "jwt"]);
    }

    #[test]
    fn test_parse_frontmatter_extracts_structured_fields() {
        let content = "---\ntitle: My Decision\ntype: decision\nwhat: Chose Rust over Go\nwhy: Better ecosystem for this project\ncontext: Team meeting Q2\nlearned: Rust's type system caught several bugs early\n---\nWe decided to use Rust for the new service.";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.what.unwrap(), "Chose Rust over Go");
        assert_eq!(parsed.why.unwrap(), "Better ecosystem for this project");
        assert_eq!(parsed.context.unwrap(), "Team meeting Q2");
        assert!(parsed.learned.unwrap().contains("Rust's"));
    }

    #[test]
    fn test_parse_simple_uses_first_line_as_title() {
        let content = "First Line Title\nRest of content\nMore content";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "First Line Title");
    }

    #[test]
    fn test_parse_simple_empty_string() {
        let content = "";
        let parsed = parse_mneme_file(content);
        assert_eq!(parsed.title, "untitled");
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
            ("agent_fact", MemoryType::AgentFact),
        ] {
            let content = format!("---\ntitle: T\ntype: {type_str}\n---\nbody");
            let parsed = parse_mneme_file(&content);
            assert_eq!(
                parsed.memory_type, expected,
                "type '{type_str}' should parse to {expected:?}"
            );
        }
    }
}
