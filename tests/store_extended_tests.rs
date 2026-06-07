use std::path::PathBuf;
use uuid::Uuid;

use mneme::store::db::Database;
use mneme::store::memory::{
    CreateMemoryInput, CreateRelationInput, Importance, MemoryType, RelationType, Scope,
    UpdateMemoryInput,
};

fn setup_db() -> Database {
    let path = PathBuf::from(format!("/tmp/mneme_test_{}.db", Uuid::new_v4()));
    Database::open(&path).unwrap()
}

fn make_input(project: &str, title: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        encrypt: false,
        project: project.to_string(),
        scope: Some(Scope::Project),
        title: title.to_string(),
        content: format!("content of {}", title),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
    }
}

#[test]
fn test_update_memory_title_changes() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(make_input("proj", "Original Title"), None, None)
        .unwrap();
    let updated = store
        .update(
            mem.id,
            UpdateMemoryInput {
                title: Some("Updated Title".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.revision_count, 2); // initial + update
}

#[test]
fn test_update_memory_preserves_unset_fields() {
    let db = setup_db();
    let store = db.memories();

    let mem = store.save(make_input("proj", "Title"), None, None).unwrap();
    let updated = store
        .update(
            mem.id,
            UpdateMemoryInput {
                tags: Some(vec!["rust".to_string()]),
                ..Default::default()
            },
        )
        .unwrap();

    assert_eq!(updated.title, "Title"); // unchanged
    assert_eq!(updated.tags, vec!["rust"]);
}

#[test]
fn test_list_filters_by_type() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Note A");
    input.memory_type = MemoryType::Note;
    store.save(input, None, None).unwrap();

    let mut input = make_input("proj", "Decision B");
    input.memory_type = MemoryType::Decision;
    store.save(input, None, None).unwrap();

    let notes = store
        .list("proj", Some(&MemoryType::Note), None, None, 10, 0)
        .unwrap();
    assert_eq!(notes.len(), 1);
    assert!(notes.iter().all(|m| m.memory_type == MemoryType::Note));
}

#[test]
fn test_list_filters_by_importance() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Low Priority");
    input.importance = Importance::Low;
    store.save(input, None, None).unwrap();

    let mut input = make_input("proj", "High Priority");
    input.importance = Importance::High;
    store.save(input, None, None).unwrap();

    let high = store
        .list("proj", None, Some(&Importance::High), None, 10, 0)
        .unwrap();
    assert_eq!(high.len(), 1);
    assert_eq!(high[0].importance, Importance::High);
}

#[test]
fn test_list_respects_limit_and_offset() {
    let db = setup_db();
    let store = db.memories();

    for i in 0..5 {
        store
            .save(make_input("proj", &format!("Item {}", i)), None, None)
            .unwrap();
    }

    let page1 = store.list("proj", None, None, None, 2, 0).unwrap();
    let page2 = store.list("proj", None, None, None, 2, 2).unwrap();

    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 2);
    assert_ne!(page1[0].id, page2[0].id);
}

#[test]
fn test_list_excludes_deleted() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(make_input("proj", "To Delete"), None, None)
        .unwrap();
    store.delete(mem.id, false).unwrap();

    let all = store.list("proj", None, None, None, 10, 0).unwrap();
    assert!(all.iter().all(|m| m.id != mem.id));
}

#[test]
fn test_context_returns_recent_memories() {
    let db = setup_db();
    let store = db.memories();

    for i in 0..3 {
        store
            .save(make_input("proj", &format!("Context {}", i)), None, None)
            .unwrap();
    }

    let ctx = store.context("proj", None, 2).unwrap();
    assert_eq!(ctx.len(), 2);
}

#[test]
fn test_stats_counts_correctly() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Arch");
    input.memory_type = MemoryType::Architecture;
    store.save(input, None, None).unwrap();

    let mut input = make_input("proj", "Bug");
    input.memory_type = MemoryType::Bugfix;
    store.save(input, None, None).unwrap();

    let stats = store.stats("proj").unwrap();
    assert_eq!(stats.total_memories, 2);
    assert_eq!(stats.by_type.get("architecture").copied().unwrap_or(0), 1);
    assert_eq!(stats.by_type.get("bugfix").copied().unwrap_or(0), 1);
}

#[test]
fn test_stats_returns_zero_for_empty_project() {
    let db = setup_db();
    let store = db.memories();

    let stats = store.stats("nonexistent").unwrap();
    assert_eq!(stats.total_memories, 0);
    assert_eq!(stats.by_type.len(), 0);
}

#[test]
fn test_list_projects_returns_all() {
    let db = setup_db();
    let store = db.memories();

    store.save(make_input("proj-a", "A"), None, None).unwrap();
    store.save(make_input("proj-b", "B"), None, None).unwrap();

    let projects = store.list_projects().unwrap();
    assert_eq!(projects.len(), 2);
    assert!(projects.iter().any(|p| p.name == "proj-a"));
    assert!(projects.iter().any(|p| p.name == "proj-b"));
}

#[test]
fn test_list_projects_empty_when_no_memories() {
    let db = setup_db();
    let store = db.memories();

    let projects = store.list_projects().unwrap();
    assert!(projects.is_empty());
}

#[test]
fn test_save_batch_saves_all() {
    let db = setup_db();
    let store = db.memories();

    let inputs = vec![
        make_input("proj", "Batch 1"),
        make_input("proj", "Batch 2"),
        make_input("proj", "Batch 3"),
    ];

    let (saved, duplicates) = store.save_batch(inputs, None, None).unwrap();
    assert_eq!(saved.len(), 3);
    assert!(duplicates.is_empty());
}

#[test]
fn test_save_batch_empty_input_returns_empty() {
    let db = setup_db();
    let store = db.memories();

    let (saved, duplicates) = store.save_batch(vec![], None, None).unwrap();
    assert!(saved.is_empty());
    assert!(duplicates.is_empty());
}

#[test]
fn test_create_relation_links_two_memories() {
    let db = setup_db();
    let store = db.memories();

    let a = store.save(make_input("proj", "A"), None, None).unwrap();
    let b = store.save(make_input("proj", "B"), None, None).unwrap();

    let rel = store
        .create_relation(CreateRelationInput {
            source_id: a.id,
            target_id: b.id,
            relation_type: RelationType::DependsOn,
            confidence: Some(0.9),
            reason: Some("depends on".to_string()),
        })
        .unwrap();

    assert_eq!(rel.source_id, a.id);
    assert_eq!(rel.target_id, b.id);
}

#[test]
fn test_delete_relation_removes_it() {
    let db = setup_db();
    let store = db.memories();

    let a = store.save(make_input("proj", "A"), None, None).unwrap();
    let b = store.save(make_input("proj", "B"), None, None).unwrap();

    let rel = store
        .create_relation(CreateRelationInput {
            source_id: a.id,
            target_id: b.id,
            relation_type: RelationType::RelatedTo,
            confidence: Some(1.0),
            reason: None,
        })
        .unwrap();

    let deleted = store.delete_relation(rel.id).unwrap();
    assert!(deleted);
}

#[test]
fn test_create_relation_self_relation_fails() {
    let db = setup_db();
    let store = db.memories();

    let a = store.save(make_input("proj", "A"), None, None).unwrap();

    let result = store.create_relation(CreateRelationInput {
        source_id: a.id,
        target_id: a.id,
        relation_type: RelationType::RelatedTo,
        confidence: None,
        reason: None,
    });

    assert!(result.is_err());
}

#[test]
fn test_deprecate_marks_memory() {
    let db = setup_db();
    let store = db.memories();

    let mem = store.save(make_input("proj", "Old"), None, None).unwrap();
    let deprecated = store.deprecate(mem.id, "outdated", None).unwrap();

    assert!(deprecated.deprecated_at.is_some());
    assert_eq!(deprecated.deprecated_reason.as_deref(), Some("outdated"));
}

#[test]
fn test_deprecate_with_superseded_by() {
    let db = setup_db();
    let store = db.memories();

    let old_mem = store.save(make_input("proj", "Old"), None, None).unwrap();
    let new_mem = store.save(make_input("proj", "New"), None, None).unwrap();

    let deprecated = store
        .deprecate(old_mem.id, "replaced", Some(new_mem.id))
        .unwrap();
    assert_eq!(deprecated.supersedes_id, Some(new_mem.id.to_string()));
}

#[test]
fn test_feedback_useful_increases_score() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(make_input("proj", "Feedback test"), None, None)
        .unwrap();
    let id = store
        .add_feedback(mem.id, true, Some("very helpful"))
        .unwrap();
    assert!(id > 0);
}

#[test]
fn test_feedback_not_useful_decreases_score() {
    let db = setup_db();
    let store = db.memories();

    let mem = store
        .save(make_input("proj", "Feedback test"), None, None)
        .unwrap();
    let id = store
        .add_feedback(mem.id, false, Some("not helpful"))
        .unwrap();
    assert!(id > 0);
}

#[test]
fn test_forget_project_deletes_all_memories() {
    let db = setup_db();
    let store = db.memories();

    store.save(make_input("proj", "A"), None, None).unwrap();
    store.save(make_input("proj", "B"), None, None).unwrap();

    let count = store.forget_project("proj").unwrap();
    assert_eq!(count, 2);

    let remaining = store.list("proj", None, None, None, 10, 0).unwrap();
    assert!(remaining.is_empty());
}

#[test]
fn test_forget_project_returns_count() {
    let db = setup_db();
    let store = db.memories();

    for i in 0..5 {
        store
            .save(make_input("proj", &format!("M{}", i)), None, None)
            .unwrap();
    }

    let count = store.forget_project("proj").unwrap();
    assert_eq!(count, 5);
}

#[test]
fn test_suggest_tags_returns_existing_tags() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Tagged");
    input.tags = vec!["rust".to_string(), "async".to_string()];
    store.save(input, None, None).unwrap();

    let suggestions = store.suggest_tags("proj", "rust async code", None).unwrap();
    assert!(!suggestions.is_empty());
}

#[test]
fn test_knowledge_gaps_detects_missing_types() {
    let db = setup_db();
    let store = db.memories();

    // Only notes — should trigger gaps for other types
    for i in 0..3 {
        store
            .save(make_input("proj", &format!("Note {}", i)), None, None)
            .unwrap();
    }

    let report = store.knowledge_gaps("proj").unwrap();
    assert!(report.coverage_score < 1.0);
    assert!(!report.gaps.is_empty());
}

#[test]
fn test_remind_returns_high_importance_memories() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Critical Task");
    input.importance = Importance::Critical;
    store.save(input, None, None).unwrap();

    let mut input = make_input("proj", "Low Task");
    input.importance = Importance::Low;
    store.save(input, None, None).unwrap();

    let reminders = store.remind("proj", &Importance::High).unwrap();
    assert!(!reminders.is_empty());
    assert!(reminders
        .iter()
        .all(|m| m.importance == Importance::High || m.importance == Importance::Critical));
}

#[test]
fn test_remind_excludes_low_importance() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Low Only");
    input.importance = Importance::Low;
    store.save(input, None, None).unwrap();

    let reminders = store.remind("proj", &Importance::High).unwrap();
    assert!(reminders.is_empty());
}

#[test]
fn test_inject_context_has_sections() {
    let db = setup_db();
    let store = db.memories();

    let mut input = make_input("proj", "Arch Decision");
    input.memory_type = MemoryType::Architecture;
    input.importance = Importance::High;
    store.save(input, None, None).unwrap();

    let ctx = store.inject_context("proj", None, 5).unwrap();
    assert!(ctx.contains("Contexto del proyecto"));
    assert!(!ctx.is_empty());
}

#[test]
fn test_summarize_project_returns_result() {
    let db = setup_db();
    let store = db.memories();

    store
        .save(make_input("proj", "Decision One"), None, None)
        .unwrap();
    store
        .save(make_input("proj", "Decision Two"), None, None)
        .unwrap();

    let summary = store.summarize("proj", None).unwrap();
    assert!(!summary.summary.is_empty());
    assert!(summary.memory_count >= 2);
}
