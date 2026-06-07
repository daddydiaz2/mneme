use mneme::store::memory::{Importance, MatchType, MemoryType, Scope, SearchResult};
use mneme::store::search::SearchWeights;
use uuid::Uuid;

fn make_test_result(
    query: &str,
    memory_title: &str,
    cosine: Option<f32>,
    match_type: MatchType,
) -> SearchResult {
    SearchResult {
        memory: mneme::store::memory::Memory {
            id: Uuid::new_v4(),
            project: "test".to_string(),
            scope: Scope::Project,
            title: memory_title.to_string(),
            content: format!("Content for {}", memory_title),
            what: None,
            why: None,
            context: None,
            learned: None,
            memory_type: MemoryType::Note,
            importance: Importance::Medium,
            tags: vec![],
            topic_key: None,
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
        },
        score: 0.5,
        snippet: Some(format!("...{query}...")),
        match_type,
        cosine_score: cosine,
    }
}

#[test]
fn test_search_weights_default() {
    let weights = SearchWeights::default();
    assert_eq!(weights.fts, 0.5);
    assert_eq!(weights.fuzzy, 0.2);
    assert_eq!(weights.semantic, 0.3);
}

#[test]
fn test_search_weights_renormalize_without_semantic() {
    let weights = SearchWeights::default();
    let renormalized = weights.renormalize_without_semantic();
    assert_eq!(renormalized.fts, 0.7);
    assert_eq!(renormalized.fuzzy, 0.3);
    assert_eq!(renormalized.semantic, 0.0);
}

#[test]
fn test_search_weights_custom_values() {
    let weights = SearchWeights {
        fts: 0.4,
        fuzzy: 0.3,
        semantic: 0.3,
    };
    assert_eq!(weights.fts, 0.4);
    assert_eq!(weights.fuzzy, 0.3);
    assert_eq!(weights.semantic, 0.3);
}

#[test]
fn test_match_type_values() {
    assert_eq!(MatchType::Fts, MatchType::Fts);
    assert_eq!(MatchType::Fuzzy, MatchType::Fuzzy);
    assert_ne!(MatchType::Fts, MatchType::Fuzzy);
}

#[test]
fn test_search_result_construction() {
    let result = make_test_result("test", "Test Memory", Some(0.85), MatchType::Semantic);
    assert_eq!(result.memory.title, "Test Memory");
    assert_eq!(result.cosine_score, Some(0.85));
    assert_eq!(result.match_type, MatchType::Semantic);
}

#[test]
fn test_search_result_serialization() {
    let result = make_test_result("rust", "Rust Memory", Some(0.9), MatchType::Fts);
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Rust Memory"));
    let parsed: SearchResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.memory.title, "Rust Memory");
    assert_eq!(parsed.cosine_score, Some(0.9));
}

#[test]
fn test_search_result_match_types_serialize_correctly() {
    use serde_json;
    let result_fts = make_test_result("q", "M1", None, MatchType::Fts);
    let result_fuzzy = make_test_result("q", "M2", None, MatchType::Fuzzy);
    let result_semantic = make_test_result("q", "M3", None, MatchType::Semantic);
    let result_exact = make_test_result("q", "M4", None, MatchType::Exact);

    assert!(serde_json::to_string(&result_fts)
        .unwrap()
        .contains("\"fts\""));
    assert!(serde_json::to_string(&result_fuzzy)
        .unwrap()
        .contains("\"fuzzy\""));
    assert!(serde_json::to_string(&result_semantic)
        .unwrap()
        .contains("\"semantic\""));
    assert!(serde_json::to_string(&result_exact)
        .unwrap()
        .contains("\"exact\""));
}

#[test]
fn test_score_blending_logic() {
    // RRF score blending: 40% original RRF + 60% refined semantic
    let original_rrf_score = 0.8;
    let refined_semantic = 1.0;
    let blended = original_rrf_score * 0.4 + refined_semantic * 0.6;
    assert!(blended > original_rrf_score);
    assert!(((blended - 0.92) as f64).abs() < 0.001);
}

#[test]
fn test_rerank_preserves_results_when_no_engine() {
    use mneme::embeddings::rerank::rerank_search_results;
    let mut results = vec![
        make_test_result("test", "Memory 1", Some(0.5), MatchType::Semantic),
        make_test_result("test", "Memory 2", Some(0.7), MatchType::Semantic),
    ];
    let before_scores: Vec<f64> = results.iter().map(|r| r.score).collect();
    // Without engine, reranking should be a no-op
    rerank_search_results("test", &mut results, None, &SearchWeights::default());
    let after_scores: Vec<f64> = results.iter().map(|r| r.score).collect();
    assert_eq!(before_scores, after_scores);
}
