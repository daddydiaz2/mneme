#[cfg(feature = "embeddings")]
use std::path::PathBuf;
#[cfg(feature = "embeddings")]
use uuid::Uuid;

#[cfg(feature = "embeddings")]
use mneme::store::db::Database;

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_cosine_similarity_identical_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    let sim = mneme::embeddings::similarity::cosine_similarity(&a, &b);
    assert!((sim - 1.0).abs() < 0.001);
}

#[cfg(feature = "embeddings")]
#[tokio::test]
async fn test_cosine_similarity_orthogonal_vectors() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![0.0, 1.0, 0.0];
    let sim = mneme::embeddings::similarity::cosine_similarity(&a, &b);
    assert!(sim.abs() < 0.001);
}

#[cfg(feature = "embeddings")]
#[test]
fn test_embedding_serialization_roundtrip() {
    let original = vec![1.5f32, -2.0, 0.0, 3.15];
    let bytes = mneme::embeddings::store::EmbeddingStore::serialize(&original);
    let recovered = mneme::embeddings::store::EmbeddingStore::deserialize(&bytes);
    assert_eq!(original, recovered);
}

#[cfg(feature = "embeddings")]
#[test]
fn test_embedding_store_save_and_load() {
    let path = PathBuf::from(format!("/tmp/mneme_embed_test_{}.db", Uuid::new_v4()));
    let db = Database::open(&path).unwrap();
    let mem_store = db.memories();
    let store = db.embeddings();

    // Create a memory first (foreign key constraint)
    let input = mneme::store::memory::CreateMemoryInput {
        encrypt: false,
        project: "embed-test".to_string(),
        scope: Some(mneme::store::memory::Scope::Project),
        title: "Test".to_string(),
        content: "Content".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: mneme::store::memory::MemoryType::Note,
        importance: mneme::store::memory::Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
        valid_from: None,
        valid_until: None,
        provenance: None,
    };
    let mem = mem_store.save(input, None, None).unwrap();

    let embedding = vec![0.1f32, 0.2, 0.3, 0.4];
    store.save(mem.id, &embedding, "test-model").unwrap();

    let loaded = store.load(mem.id).unwrap().unwrap();
    assert_eq!(embedding, loaded);
}

#[cfg(feature = "embeddings")]
#[test]
fn test_find_unindexed_returns_correct_ids() {
    let path = PathBuf::from(format!("/tmp/mneme_unindexed_test_{}.db", Uuid::new_v4()));
    let db = Database::open(&path).unwrap();
    let mem_store = db.memories();
    let embed_store = db.embeddings();

    // Create memory
    let input = mneme::store::memory::CreateMemoryInput {
        encrypt: false,
        project: "unindexed-test".to_string(),
        scope: Some(mneme::store::memory::Scope::Project),
        title: "Test".to_string(),
        content: "Content".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: mneme::store::memory::MemoryType::Note,
        importance: mneme::store::memory::Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
        valid_from: None,
        valid_until: None,
        provenance: None,
    };
    let mem = mem_store.save(input, None, None).unwrap();

    // Should be unindexed
    let unindexed = embed_store.find_unindexed("unindexed-test").unwrap();
    assert!(unindexed.contains(&mem.id));

    // Index it
    embed_store.save(mem.id, &[0.1f32, 0.2], "test").unwrap();

    // Should no longer be unindexed
    let unindexed = embed_store.find_unindexed("unindexed-test").unwrap();
    assert!(!unindexed.contains(&mem.id));
}

#[test]
fn test_search_weights_renormalize_without_semantic() {
    let weights = mneme::store::search::SearchWeights {
        fts: 0.5,
        fuzzy: 0.2,
        semantic: 0.3,
    };
    let normalized = weights.renormalize_without_semantic();
    assert!((normalized.fts - 0.7).abs() < 0.001);
    assert!((normalized.fuzzy - 0.3).abs() < 0.001);
    assert_eq!(normalized.semantic, 0.0);
}
