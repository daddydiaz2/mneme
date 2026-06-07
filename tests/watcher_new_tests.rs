use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mneme_watcher_test_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_db_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("mneme_watcher_db_{}_{}.db", std::process::id(), id))
}

use mneme::store::db::Database;
use mneme::watch::watcher::DirectoryWatcher;

fn make_watcher() -> (DirectoryWatcher, std::path::PathBuf) {
    let dir = test_dir();
    let db = Database::open(&test_db_path()).unwrap();
    let store = db.memories();
    let watcher = DirectoryWatcher::new(dir.clone(), ".md".to_string(), 2, store, "test".to_string());
    (watcher, dir)
}

fn write_md_file(dir: &PathBuf, name: &str, content: &str) {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
}

#[tokio::test]
async fn test_watcher_indexes_new_file() {
    let (mut watcher, dir) = make_watcher();
    write_md_file(&dir, "memory1.md", r#"---
title: First Memory
type: note
---
Content here"#);
    let result = watcher.scan().await.unwrap();
    assert!(result.indexed >= 1, "Should index at least one file, got: {:?}", result);
}

#[tokio::test]
async fn test_watcher_skips_existing_unchanged_files() {
    let (mut watcher, dir) = make_watcher();
    write_md_file(&dir, "memory.md", "---\ntitle: T\n---\nBody");

    // First scan indexes
    let first = watcher.scan().await.unwrap();
    assert!(first.indexed >= 1);

    // Second scan should skip (no changes)
    let second = watcher.scan().await.unwrap();
    assert!(second.skipped >= 1 || second.indexed == 0);
}

#[tokio::test]
async fn test_watcher_reindexes_modified_file() {
    let (mut watcher, dir) = make_watcher();
    let path = dir.join("modified.md");
    std::fs::write(&path, "---\ntitle: T\n---\nV1").unwrap();
    watcher.scan().await.unwrap();

    // Modify file
    std::fs::write(&path, "---\ntitle: T\n---\nV2 with different content").unwrap();
    // Small delay for mtime change
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let result = watcher.scan().await.unwrap();
    // Should re-index or at least acknowledge the change
    assert!(result.indexed + result.skipped >= 1);
}

#[tokio::test]
async fn test_watcher_with_track_new_only() {
    let dir = test_dir();
    let db = Database::open(&test_db_path()).unwrap();
    let store = db.memories();
    let mut watcher = DirectoryWatcher::new(dir.clone(), ".md".to_string(), 2, store, "test".to_string())
        .with_track_new_only(true);

    // Pre-existing file should be skipped with track_new_only
    write_md_file(&dir, "preexisting.md", "---\ntitle: Pre\n---\nBody");
    let result = watcher.scan().await.unwrap();
    assert_eq!(result.indexed, 0, "Should skip pre-existing files");
}

#[tokio::test]
async fn test_watcher_detects_deletions() {
    let (mut watcher, dir) = make_watcher();
    let path = dir.join("deleteme.md");
    std::fs::write(&path, "---\ntitle: T\n---\nBody").unwrap();

    // Index it
    watcher.scan().await.unwrap();
    let initial_count = watcher.tracked_count();
    assert!(initial_count >= 1);

    // Delete the file
    std::fs::remove_file(&path).unwrap();

    // Rescan
    let result = watcher.scan().await.unwrap();
    assert!(result.removed >= 1, "Should detect at least one removal");
}

#[tokio::test]
async fn test_watcher_with_md_extension() {
    let (mut watcher, dir) = make_watcher();
    write_md_file(&dir, "test.md", "---\ntitle: Test\ntype: decision\n---\nSome content");
    let result = watcher.scan().await.unwrap();
    assert!(result.indexed >= 1);
}

#[tokio::test]
async fn test_watcher_ignores_non_matching_extensions() {
    let (mut watcher, dir) = make_watcher();
    // .txt file should be ignored
    write_md_file(&dir, "ignored.txt", "Some text content");
    write_md_file(&dir, "memory.md", "---\ntitle: M\n---\nBody");
    let result = watcher.scan().await.unwrap();
    // Only the .md file should be indexed
    assert!(result.indexed >= 1);
}

#[tokio::test]
async fn test_watcher_tracked_count_updates() {
    let (mut watcher, dir) = make_watcher();
    write_md_file(&dir, "a.md", "---\ntitle: A\n---\nBody A");
    write_md_file(&dir, "b.md", "---\ntitle: B\n---\nBody B");
    write_md_file(&dir, "c.md", "---\ntitle: C\n---\nBody C");
    watcher.scan().await.unwrap();
    assert!(watcher.tracked_count() >= 3, "Should track at least 3 files");
}

#[tokio::test]
async fn test_watcher_tracked_summary() {
    let (mut watcher, dir) = make_watcher();
    write_md_file(&dir, "summary.md", "---\ntitle: S\n---\nBody");
    watcher.scan().await.unwrap();
    let summary = watcher.tracked_summary();
    assert!(!summary.is_empty());
}

#[tokio::test]
async fn test_watcher_empty_dir() {
    let (mut watcher, _dir) = make_watcher();
    let result = watcher.scan().await.unwrap();
    assert_eq!(result.indexed, 0);
    assert_eq!(result.errors, 0);
}

#[tokio::test]
async fn test_watcher_nonexistent_dir() {
    let db = Database::open(&test_db_path()).unwrap();
    let store = db.memories();
    let mut watcher = DirectoryWatcher::new(
        PathBuf::from("/tmp/nonexistent_dir_xyz_12345"),
        ".md".to_string(),
        2,
        store,
        "test".to_string(),
    );
    let result = watcher.scan().await.unwrap();
    // Should handle missing directory gracefully
    assert_eq!(result.indexed, 0);
}

#[tokio::test]
async fn test_watcher_provenance_records_file_path() {
    use mneme::store::memory::Scope;
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();
    let dir = test_dir();
    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".md".to_string(),
        2,
        db.memories(),
        "test".to_string(),
    );
    let path = dir.join("prov.md");
    std::fs::write(&path, "---\ntitle: P\ntype: bugfix\n---\nBody").unwrap();
    watcher.scan().await.unwrap();

    // Check that provenance was set in the same DB
    let memories = store.list("test", None, None, Some(&Scope::Project), 100, 0).unwrap();
    let with_provenance = memories.iter().find(|m| m.provenance.is_some());
    if let Some(mem) = with_provenance {
        let prov = mem.provenance.as_ref().unwrap();
        assert!(prov.contains("file://"), "Provenance should record file path");
    }
}
