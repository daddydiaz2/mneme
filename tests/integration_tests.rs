use mneme::store::db::Database;
use mneme::store::memory::{CreateMemoryInput, Importance, MemoryType, Scope};
use std::path::PathBuf;
use uuid::Uuid;

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_test_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

#[test]
fn test_import_export_roundtrip() {
    let db = setup_db();
    let store = db.memories();

    // Create some memories
    for i in 0..5 {
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: "export-test".to_string(),
                    scope: Some(Scope::Project),
                    title: format!("Memory {}", i),
                    content: format!("Content {}", i),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: MemoryType::Note,
                    importance: Importance::Medium,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                },
                None,
                None,
            )
            .unwrap();
    }

    // Export
    let memories = store.list("export-test", None, None, None, 100, 0).unwrap();
    let json = serde_json::to_string(&memories).unwrap();

    // Import (simulate)
    let db2 = setup_db();
    let store2 = db2.memories();
    let imported: Vec<mneme::store::memory::Memory> = serde_json::from_str(&json).unwrap();
    assert_eq!(imported.len(), 5);

    // Re-import into new database
    for mem in &imported {
        store2
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: mem.project.clone(),
                    scope: Some(mem.scope.clone()),
                    title: mem.title.clone(),
                    content: mem.content.clone(),
                    what: mem.what.clone(),
                    why: mem.why.clone(),
                    context: mem.context.clone(),
                    learned: mem.learned.clone(),
                    memory_type: mem.memory_type.clone(),
                    importance: mem.importance.clone(),
                    tags: mem.tags.clone(),
                    topic_key: mem.topic_key.clone(),
                    capture_prompt: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                },
                None,
                None,
            )
            .unwrap();
    }

    let reimported = store2
        .list("export-test", None, None, None, 100, 0)
        .unwrap();
    assert_eq!(reimported.len(), 5);
}

#[test]
fn test_scope_isolation() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "scope-test".to_string(),
                scope: Some(Scope::Project),
                title: "Project Note".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "scope-test".to_string(),
                scope: Some(Scope::Personal),
                title: "Personal Note".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let project_memories = store
        .list("scope-test", None, None, Some(&Scope::Project), 100, 0)
        .unwrap();
    assert_eq!(project_memories.len(), 1);
    assert_eq!(project_memories[0].scope, Scope::Project);

    let personal_memories = store
        .list("scope-test", None, None, Some(&Scope::Personal), 100, 0)
        .unwrap();
    assert_eq!(personal_memories.len(), 1);
    assert_eq!(personal_memories[0].scope, Scope::Personal);
}

#[test]
fn test_doctor_reports_health() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "doctor-test".to_string(),
                scope: Some(Scope::Project),
                title: "Test".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    // Doctor should report healthy
    let stats = store.stats("doctor-test").unwrap();
    assert_eq!(stats.total_memories, 1);
    assert!(stats.by_type.contains_key("note"));
}

#[test]
fn test_batch_save() {
    let db = setup_db();
    let store = db.memories();

    let inputs = vec![
        CreateMemoryInput {
            encrypt: false,
            project: "batch-test".to_string(),
            scope: Some(Scope::Project),
            title: "First".to_string(),
            content: "Content 1".to_string(),
            what: None,
            why: None,
            context: None,
            learned: None,
            memory_type: MemoryType::Note,
            importance: Importance::Medium,
            tags: vec![],
            topic_key: None,
            capture_prompt: None,
            valid_from: None,
            valid_until: None,
            provenance: None,
        },
        CreateMemoryInput {
            encrypt: false,
            project: "batch-test".to_string(),
            scope: Some(Scope::Project),
            title: "Second".to_string(),
            content: "Content 2".to_string(),
            what: None,
            why: None,
            context: None,
            learned: None,
            memory_type: MemoryType::Decision,
            importance: Importance::High,
            tags: vec!["tag1".to_string()],
            topic_key: None,
            capture_prompt: None,
            valid_from: None,
            valid_until: None,
            provenance: None,
        },
    ];

    let (saved, duplicates) = store.save_batch(inputs, None, None).unwrap();
    assert_eq!(saved.len(), 2);
    assert!(duplicates.is_empty());
}

#[test]
fn test_audit_finds_issues() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "audit-test".to_string(),
                scope: Some(Scope::Project),
                title: "Untagged".to_string(),
                content: "X".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let report = store.audit("audit-test", 0).unwrap();
    assert!(!report.short_memories.is_empty());
    assert!(!report.untagged_memories.is_empty());
    assert_eq!(report.type_distribution.get("note"), Some(&1));
}

#[test]
fn test_deprecate_memory() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "deprecate-test".to_string(),
                scope: Some(Scope::Project),
                title: "To Deprecate".to_string(),
                content: "Old content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let updated = store.deprecate(mem.id, "Outdated", None).unwrap();
    assert!(updated.deprecated_at.is_some());
    assert_eq!(updated.deprecated_reason, Some("Outdated".to_string()));
}

#[test]
fn test_graph_structure() {
    let db = setup_db();
    let store = db.memories();

    let m1 = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "graph-test".to_string(),
                scope: Some(Scope::Project),
                title: "Node 1".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Architecture,
                importance: Importance::High,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let m2 = store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "graph-test".to_string(),
                scope: Some(Scope::Project),
                title: "Node 2".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Decision,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    store
        .create_relation(mneme::store::memory::CreateRelationInput {
            source_id: m1.id,
            target_id: m2.id,
            relation_type: mneme::store::memory::RelationType::DependsOn,
            confidence: Some(0.9),
            reason: Some("test".to_string()),
        })
        .unwrap();

    let graph = store.get_graph("graph-test").unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].relation_type, "depends_on");
}

#[test]
fn test_health_report() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "health-test".to_string(),
                scope: Some(Scope::Project),
                title: "Health".to_string(),
                content: "Content".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Note,
                importance: Importance::Medium,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let report = store.health(Some("health-test")).unwrap();
    assert_eq!(report.total_memories, 1);
    assert!(!report.version.is_empty());
}

#[test]
fn test_remind_returns_important() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(
            CreateMemoryInput {
                encrypt: false,
                project: "remind-test".to_string(),
                scope: Some(Scope::Project),
                title: "Critical".to_string(),
                content: "Important info".to_string(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type: MemoryType::Decision,
                importance: Importance::Critical,
                tags: vec![],
                topic_key: None,
                capture_prompt: None,
                valid_from: None,
                valid_until: None,
                provenance: None,
            },
            None,
            None,
        )
        .unwrap();

    let memories = store.remind("remind-test", &Importance::High).unwrap();
    assert!(!memories.is_empty());
    assert_eq!(memories[0].importance, Importance::Critical);
}
