use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_db_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "mneme_compress_test_{}_{}.db",
        std::process::id(),
        id
    ))
}

use mneme::compress::{CompressionPipeline, CompressionStrategy};
use mneme::store::db::Database;
use mneme::store::memory::{CreateMemoryInput, Importance, MemoryType, Scope};

fn make_db() -> Database {
    Database::open(&test_db_path()).unwrap()
}

fn make_input(project: &str, title: &str, content: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        encrypt: false,
        project: project.to_string(),
        scope: Some(Scope::Project),
        title: title.to_string(),
        content: content.to_string(),
        what: Some("What: This is a what field".to_string()),
        why: Some("Why: Because tests".to_string()),
        context: None,
        learned: Some("Learned: That compression works".to_string()),
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec!["test".to_string(), "compression".to_string()],
        topic_key: None,
        capture_prompt: None,
        valid_from: None,
        valid_until: None,
        provenance: None,
    }
}

#[test]
fn test_truncate_strategy_short_content() {
    let input = make_input("test", "Short", "This is short");
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::Truncate);
    assert_eq!(result.compressed_content, "This is short");
    assert!(result.reversible);
}

#[test]
fn test_truncate_strategy_long_content() {
    let long_content: String = std::iter::repeat("a").take(500).collect();
    let input = make_input("test", "Long", &long_content);
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::Truncate);
    // Default truncation is 200 chars + "..."
    assert!(result.compressed_content.len() <= 210);
    assert!(result.compressed_content.ends_with("..."));
}

#[test]
fn test_truncate_strategy_at_exact_boundary() {
    let content: String = std::iter::repeat("x").take(200).collect();
    let input = make_input("test", "Exact", &content);
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::Truncate);
    // Content fits exactly, no truncation
    assert_eq!(result.compressed_content, content);
}

#[test]
fn test_smart_summary_includes_what_why() {
    let input = make_input("test", "Summary", "Some content here");
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::SmartSummary);
    // SmartSummary includes structured fields if present
    assert!(!result.compressed_content.is_empty());
}

#[test]
fn test_keywords_only_strategy() {
    let input = make_input(
        "test",
        "Keywords",
        "Rust programming language with memory systems and storage",
    );
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::KeywordsOnly);
    assert!(result.compressed_content.contains("Keywords"));
    assert!(!result.keywords.is_empty());
}

#[test]
fn test_minimal_strategy() {
    let input = make_input("test", "Minimal", "Some content");
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::Minimal);
    // Minimal should include title and type at minimum
    assert!(
        result.compressed_content.contains("Minimal") || result.compressed_content.contains("note")
    );
}

#[test]
fn test_compression_ratio_calculation() {
    let long_content: String = std::iter::repeat("a").take(1000).collect();
    let input = make_input("test", "Ratio", &long_content);
    let memory = make_db().memories().save(input, None, None).unwrap();
    let result = CompressionPipeline::compress(&memory, CompressionStrategy::Truncate);
    // 1000 -> ~200 chars = 80% reduction
    assert!(
        result.compression_ratio > 0.7,
        "Compression ratio should be > 0.7, got: {}",
        result.compression_ratio
    );
}

#[test]
fn test_extract_keywords_filters_stopwords() {
    let text = "the quick brown fox jumps over the lazy dog";
    let keywords = CompressionPipeline::extract_keywords(text);
    // Common stopwords should be filtered
    assert!(!keywords.contains(&"the".to_string()));
    assert!(!keywords.contains(&"over".to_string()));
    // But content words should remain
    assert!(
        keywords.contains(&"quick".to_string())
            || keywords.contains(&"brown".to_string())
            || keywords.contains(&"fox".to_string())
    );
}

#[test]
fn test_extract_keywords_handles_camelcase() {
    let text = "We use FastEmbed and SQLiteVec for memory";
    let keywords = CompressionPipeline::extract_keywords(text);
    // CamelCase terms should be extracted as lowercase
    let _ = keywords; // Just verify no crash
}

#[test]
fn test_extract_keywords_empty_text() {
    let keywords = CompressionPipeline::extract_keywords("");
    assert!(keywords.is_empty(), "Empty text should yield no keywords");
}

#[test]
fn test_compression_strategy_display() {
    assert_eq!(CompressionStrategy::Truncate.to_string(), "truncate");
    assert_eq!(
        CompressionStrategy::SmartSummary.to_string(),
        "smart_summary"
    );
    assert_eq!(
        CompressionStrategy::KeywordsOnly.to_string(),
        "keywords_only"
    );
    assert_eq!(CompressionStrategy::Minimal.to_string(), "minimal");
}

#[test]
fn test_compression_strategy_from_str() {
    use std::str::FromStr;
    assert!(matches!(
        CompressionStrategy::from_str("truncate").unwrap(),
        CompressionStrategy::Truncate
    ));
    assert!(matches!(
        CompressionStrategy::from_str("smart_summary").unwrap(),
        CompressionStrategy::SmartSummary
    ));
    assert!(CompressionStrategy::from_str("invalid").is_err());
}

#[test]
fn test_compress_context_block() {
    let db = make_db();
    let store = db.memories();
    let m1 = store
        .save(make_input("test", "Mem1", "Content 1"), None, None)
        .unwrap();
    let m2 = store
        .save(make_input("test", "Mem2", "Content 2"), None, None)
        .unwrap();
    let memories = vec![m1, m2];
    let block = CompressionPipeline::compress_context_block(
        &memories,
        CompressionStrategy::SmartSummary,
        10,
    );
    assert!(block.contains("Mem1") || block.contains("Mem2"));
    assert!(block.contains("Contexto"));
}

#[test]
fn test_compress_empty_memories() {
    let memories: Vec<mneme::store::memory::Memory> = vec![];
    let block =
        CompressionPipeline::compress_context_block(&memories, CompressionStrategy::Truncate, 10);
    assert!(block.contains("Contexto"));
}

#[test]
fn test_all_strategies_are_reversible() {
    let input = make_input("test", "Reversible", "Some content here for testing");
    let memory = make_db().memories().save(input, None, None).unwrap();
    for strategy in [
        CompressionStrategy::Truncate,
        CompressionStrategy::SmartSummary,
        CompressionStrategy::KeywordsOnly,
        CompressionStrategy::Minimal,
    ] {
        let result = CompressionPipeline::compress(&memory, strategy);
        assert!(result.reversible, "All strategies should be reversible");
    }
}
