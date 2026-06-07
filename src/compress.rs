use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::store::memory::Memory;

/// Estrategia de compresión.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CompressionStrategy {
    /// Truncación simple: solo primeros N chars.
    Truncate,
    /// Resumen inteligente: primer párrafo + oraciones clave.
    SmartSummary,
    /// Solo extraer keywords del contenido.
    KeywordsOnly,
    /// Mínimo: solo título + tipo + keywords.
    Minimal,
}

impl std::fmt::Display for CompressionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CompressionStrategy::Truncate => "truncate",
            CompressionStrategy::SmartSummary => "smart_summary",
            CompressionStrategy::KeywordsOnly => "keywords_only",
            CompressionStrategy::Minimal => "minimal",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for CompressionStrategy {
    type Err = crate::error::MnemeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "truncate" => Ok(CompressionStrategy::Truncate),
            "smart_summary" | "smart-summary" | "smartsummary" => {
                Ok(CompressionStrategy::SmartSummary)
            }
            "keywords_only" | "keywords-only" | "keywordsonly" => {
                Ok(CompressionStrategy::KeywordsOnly)
            }
            "minimal" => Ok(CompressionStrategy::Minimal),
            other => Err(crate::error::MnemeError::Config(format!(
                "Invalid compression strategy: {}",
                other
            ))),
        }
    }
}

/// Resultado de la compresión.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemory {
    /// ID de la memoria original.
    pub memory_id: String,
    /// Título original (sin comprimir).
    pub title: String,
    /// Contenido comprimido.
    pub compressed_content: String,
    /// Estrategia utilizada.
    pub strategy: String,
    /// Ratio de compresión (0.0 - 1.0), donde 1.0 = 100% reducción.
    pub compression_ratio: f64,
    /// Keywords extraídas (todas las estrategias).
    pub keywords: Vec<String>,
    /// ¿Se puede revertir?
    pub reversible: bool,
}

/// Pipeline de compresión reversible para contenido de memorias.
/// Inspirado por Headroom (CCR — Context Compression & Retrieval).
pub struct CompressionPipeline;

impl CompressionPipeline {
    /// Comprime una memoria usando la estrategia especificada.
    pub fn compress(memory: &Memory, strategy: CompressionStrategy) -> CompressedMemory {
        let original_len = memory.content.len() as f64;

        let (compressed_content, keywords) = match strategy {
            CompressionStrategy::Truncate => Self::truncate_compress(&memory.content, 200),
            CompressionStrategy::SmartSummary => Self::smart_summary(&memory.content),
            CompressionStrategy::KeywordsOnly => Self::keywords_only(&memory.content),
            CompressionStrategy::Minimal => Self::minimal_compress(memory),
        };

        let compressed_len = compressed_content.len() as f64;
        let compression_ratio = if original_len > 0.0 {
            1.0 - (compressed_len / original_len)
        } else {
            0.0
        };

        CompressedMemory {
            memory_id: memory.id.to_string(),
            title: memory.title.clone(),
            compressed_content,
            strategy: strategy.to_string(),
            compression_ratio,
            keywords,
            reversible: true,
        }
    }

    /// Truncación simple a N caracteres.
    fn truncate_compress(content: &str, max_chars: usize) -> (String, Vec<String>) {
        if content.len() <= max_chars {
            let keywords = Self::extract_keywords(content);
            return (content.to_string(), keywords);
        }

        let mut result = String::with_capacity(max_chars + 3);
        // Keep first `max_chars` chars
        result.push_str(&content[..max_chars]);
        result.push_str("...");

        let keywords = Self::extract_keywords(&content[..max_chars]);
        (result, keywords)
    }

    /// Resumen inteligente: primer párrafo + oraciones clave.
    fn smart_summary(content: &str) -> (String, Vec<String>) {
        let mut parts: Vec<String> = Vec::new();

        // First paragraph
        if let Some(first_para) = content.split("\n\n").next() {
            if !first_para.is_empty() {
                parts.push(format!("[Intro] {}", first_para));
            }
        }

        // Extract key sentences (sentences with important keywords or patterns)
        let sentences: Vec<&str> = content
            .split(['.', '!', '?'])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let important_patterns = [
            "important",
            "critical",
            "key",
            "must",
            "should",
            "never",
            "always",
            "architecture",
            "decision",
            "chose",
            "selected",
            "implemented",
            "importante",
            "crítico",
            "clave",
            "debe",
            "nunca",
            "siempre",
            "importante",
            "decisión",
            "arquitectura",
            "seleccionó",
        ];

        let mut key_sentences: Vec<&str> = sentences
            .iter()
            .filter(|s| {
                let lower = s.to_lowercase();
                important_patterns.iter().any(|p| lower.contains(p))
            })
            .copied()
            .collect();

        // Deduplicate and limit
        key_sentences.sort();
        key_sentences.dedup();
        let max_sentences = 5.min(key_sentences.len());

        if max_sentences > 0 {
            parts.push("[Key points]".to_string());
            for sent in key_sentences.iter().take(max_sentences) {
                parts.push(format!("- {}", sent.trim()));
            }
        }

        let keywords = Self::extract_keywords(content);
        let result = parts.join("\n");
        (result, keywords)
    }

    /// Solo extraer keywords.
    fn keywords_only(content: &str) -> (String, Vec<String>) {
        let keywords = Self::extract_keywords(content);
        (format!("Keywords: {}", keywords.join(", ")), keywords)
    }

    /// Mínimo: título + tipo + keywords.
    fn minimal_compress(memory: &Memory) -> (String, Vec<String>) {
        let keywords = Self::extract_keywords(&memory.content);
        let mut parts = vec![format!("[{}] {}", memory.memory_type, memory.title)];

        if let Some(ref what) = memory.what {
            let truncated: String = what.chars().take(100).collect();
            parts.push(format!("What: {}", truncated));
        }

        if !keywords.is_empty() {
            parts.push(format!("Keywords: {}", keywords.join(", ")));
        }

        (parts.join(" | "), keywords)
    }

    /// Extrae keywords relevantes del contenido.
    pub fn extract_keywords(content: &str) -> Vec<String> {
        let stopwords: HashSet<&str> = HashSet::from([
            "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
            "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
            "shall", "can", "need", "dare", "ought", "i", "you", "he", "she", "it", "we", "they",
            "me", "him", "her", "us", "them", "my", "your", "his", "its", "our", "their", "this",
            "that", "these", "those", "some", "any", "each", "every", "all", "both", "few",
            "several", "many", "much", "no", "not", "only", "own", "same", "so", "than", "too",
            "very", "just", "because", "as", "until", "while", "of", "at", "by", "for", "with",
            "about", "against", "between", "into", "through", "during", "before", "after", "above",
            "below", "to", "from", "up", "down", "in", "out", "on", "off", "over", "under",
            "again", "further", "then", "once", "here", "there", "when", "where", "why", "how",
            "el", "la", "los", "las", "un", "una", "y", "e", "o", "u", "de", "del", "en", "al",
            "por", "para", "con", "sin", "sobre", "entre", "como", "que", "es", "se", "su", "lo",
            "le", "ha", "está", "esta", "este", "ese", "eso", "era", "ser", "han",
        ]);

        let mut word_counts: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();

        // Normalize and split
        for word in content.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_') {
            let lower = word.trim().to_lowercase();
            if lower.len() > 3
                && !stopwords.contains(lower.as_str())
                && !lower.chars().all(|c| c.is_numeric())
            {
                *word_counts.entry(lower).or_insert(0) += 1;
            }
        }

        // Also check for CamelCase/PascalCase identifiers
        for word in content.split_whitespace() {
            if word.len() > 4 {
                let has_upper = word.chars().any(|c| c.is_uppercase());
                let has_lower = word.chars().any(|c| c.is_lowercase());
                if has_upper && has_lower && !word.contains('_') {
                    let lower = word.to_lowercase();
                    if lower.len() > 3 && !stopwords.contains(lower.as_str()) {
                        *word_counts.entry(lower).or_insert(0) += 2;
                    }
                }
            }
        }

        // Sort by frequency
        let mut sorted: Vec<(String, u32)> = word_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted.into_iter().take(10).map(|(w, _)| w).collect()
    }

    /// Genera un bloque de contexto comprimido para inyección en prompts.
    /// Versión comprimida de `MemoryStore::inject_context`.
    pub fn compress_context_block(
        memories: &[Memory],
        strategy: CompressionStrategy,
        max_memories: usize,
    ) -> String {
        let mut lines = vec![
            "## Contexto comprimido del proyecto".to_string(),
            String::new(),
        ];

        for memory in memories.iter().take(max_memories) {
            let compressed = Self::compress(memory, strategy);
            lines.push(format!(
                "- **{}** [{}] ({}): {}",
                compressed.title,
                memory.memory_type,
                compressed.strategy,
                compressed
                    .compressed_content
                    .chars()
                    .take(150)
                    .collect::<String>()
            ));
        }

        lines.push(String::new());
        lines.push(format!(
            "_Contexto comprimido con estrategia '{}'. Usa mem_expand para ver contenido completo._",
            strategy
        ));

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::{Importance, Scope};

    #[test]
    fn test_truncate_compress_short() {
        let (compressed, keywords) = CompressionPipeline::truncate_compress("Short text", 200);
        assert_eq!(compressed, "Short text");
    }

    #[test]
    fn test_truncate_compress_long() {
        let long = "A".repeat(500);
        let (compressed, _) = CompressionPipeline::truncate_compress(&long, 200);
        assert!(compressed.len() < 210);
        assert!(compressed.ends_with("..."));
    }

    #[test]
    fn test_extract_keywords() {
        let text = "rust is a systems programming language focused on safety and performance";
        let keywords = CompressionPipeline::extract_keywords(text);
        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"systems".to_string()));
        assert!(keywords.contains(&"programming".to_string()));
    }

    #[test]
    fn test_minimal_compress() {
        let memory = Memory {
            id: uuid::Uuid::new_v4(),
            project: "test".to_string(),
            scope: crate::store::memory::Scope::Project,
            title: "Test Memory".to_string(),
            content: "This is a test content with some important keywords for testing purpose"
                .to_string(),
            what: Some("What was done".to_string()),
            why: None,
            context: None,
            learned: None,
            memory_type: crate::store::memory::MemoryType::Architecture,
            importance: crate::store::memory::Importance::High,
            tags: vec![],
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

        let result = CompressionPipeline::compress(&memory, CompressionStrategy::Minimal);
        assert!(result.compressed_content.contains("architecture"));
        assert!(result.compressed_content.contains("Test Memory"));
        assert_eq!(result.reversible, true);
    }
}
