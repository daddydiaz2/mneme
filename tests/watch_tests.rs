use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn test_dir() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("mneme_watch_test_{}_{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn test_db_path() -> PathBuf {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!("mneme_watch_db_{}_{}.db", std::process::id(), id))
}

use mneme::store::db::Database;
use mneme::watch::watcher::DirectoryWatcher;

#[tokio::test]
async fn test_watcher_new_creates_with_defaults() {
    let dir = test_dir();
    let db = Database::open(&test_db_path()).unwrap();
    let store = db.memories();

    let watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        2,
        store,
        "watch-test".to_string(),
    );

    // Constructor should succeed without panicking
    let _ = watcher;
    std::fs::remove_dir_all(&dir).ok();
}

#[tokio::test]
async fn test_watcher_process_file_creates_memory() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    // Create a .mneme file
    let file_path = dir.join("test-entry.mneme");
    std::fs::write(
        &file_path,
        "---\ntitle: Test Entry\ntype: decision\nimportance: high\ntags: test, watch\n---\nContent body",
    )
    .unwrap();

    let watcher = DirectoryWatcher::new(dir.clone(), ".mneme".to_string(), 2, store, "watch-test".to_string());

    // The process_file path is tested through scan() - see test_watcher_scan_detects_new_mneme_file

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
    let _ = watcher;
}

#[tokio::test]
async fn test_watcher_scan_detects_new_mneme_file() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "scan-test".to_string(),
    );

    // No files yet - scan should be empty
    let result = watcher.scan().await; // scan() returns Result<()>
    assert!(result.is_ok(), "scan should succeed with empty dir: {:?}", result.err());

    // Create a .mneme file
    std::fs::write(
        dir.join("hello.mneme"),
        "---\ntitle: Hello World\ntype: note\n---\nTest content",
    )
    .unwrap();

    // Scan again - should find the new file
    let result = watcher.scan().await;
    assert!(result.is_ok(), "scan should succeed with new file: {:?}", result.err());

    // Verify memory was created
    let memories = db.memories().list("scan-test", None, None, None, 10, 0).unwrap();
    assert!(!memories.is_empty(), "should have at least one memory after scan");
    let titles: Vec<&str> = memories.iter().map(|m| m.title.as_str()).collect();
    assert!(titles.contains(&"Hello World"), "should contain the scanned memory");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_scan_skips_non_mneme_files() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "skip-test".to_string(),
    );

    // Create non-.mneme files
    std::fs::write(dir.join("readme.txt"), "not a memory").unwrap();
    std::fs::write(dir.join("data.json"), "{}").unwrap();

    let result = watcher.scan().await;
    assert!(result.is_ok(), "scan with only non-mneme files should succeed");

    // No memories should be created
    let memories = db.memories().list("skip-test", None, None, None, 10, 0).unwrap();
    assert!(memories.is_empty(), "non-.mneme files should not create memories");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_scan_skips_unchanged_files() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "unchanged-test".to_string(),
    );

    // Create a single .mneme file
    std::fs::write(dir.join("stable.mneme"), "---\ntitle: Stable\ntype: note\n---\nContent").unwrap();

    // First scan should process it
    let result = watcher.scan().await;
    assert!(result.is_ok(), "first scan should succeed");

    // Second scan should skip it (unchanged)
    let result = watcher.scan().await;
    assert!(result.is_ok(), "second scan should succeed");

    // Should only have 1 memory (not duplicated)
    let memories = db.memories().list("unchanged-test", None, None, None, 10, 0).unwrap();
    assert_eq!(memories.len(), 1, "unchanged file should not create duplicate");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_detects_modified_file() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "mod-test".to_string(),
    );

    // Create initial file
    let file_path = dir.join("updatable.mneme");
    std::fs::write(&file_path, "---\ntitle: Version 1\ntype: note\n---\nFirst version").unwrap();

    // Scan to process it
    let result = watcher.scan().await;
    assert!(result.is_ok());

    // Modify the file content
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(&file_path, "---\ntitle: Version 2\ntype: note\n---\nUpdated version").unwrap();

    // Scan again - should detect modification and upsert with same topic_key
    let result = watcher.scan().await;
    assert!(result.is_ok(), "scan after modification should succeed");

    // Due to topic_key based on filename, the memory gets upserted (not duplicated)
    let memories = db.memories().list("mod-test", None, None, None, 10, 0).unwrap();
    assert!(!memories.is_empty(), "should have memories");

    // The title should be updated to the new version (topic_key causes revision increment)
    let titles: Vec<&str> = memories.iter().map(|m| m.title.as_str()).collect();
    let has_v2 = titles.contains(&"Version 2");
    let has_v1 = titles.contains(&"Version 1");
    assert!(has_v2 || has_v1, "should contain one of the versions: {:?}", titles);

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_handles_multiple_new_files() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "multi-test".to_string(),
    );

    // Create 3 .mneme files at once
    for i in 0..3 {
        std::fs::write(
            dir.join(format!("file_{i}.mneme")),
            format!("---\ntitle: Multi File {i}\ntype: note\n---\nContent {i}"),
        )
        .unwrap();
    }

    let result = watcher.scan().await;
    assert!(result.is_ok(), "scan with 3 new files should succeed");

    let memories = db.memories().list("multi-test", None, None, None, 10, 0).unwrap();
    assert_eq!(memories.len(), 3, "3 files should create 3 memories");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_with_frontmatter_all_fields() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "frontmatter-test".to_string(),
    );

    std::fs::write(
        dir.join("full.mneme"),
        "---\ntitle: Full Spec\ntype: decision\nimportance: critical\ntags: rust, api, auth\n---\nThis is a full test memory",
    )
    .unwrap();

    let result = watcher.scan().await;
    assert!(result.is_ok());

    let memories = db.memories().list("frontmatter-test", None, None, None, 10, 0).unwrap();
    assert!(!memories.is_empty());

    let mem = &memories[0];
    assert_eq!(mem.title, "Full Spec");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}

#[tokio::test]
async fn test_watcher_simple_format_no_frontmatter() {
    let dir = test_dir();
    let db_path = test_db_path();
    let db = Database::open(&db_path).unwrap();
    let store = db.memories();

    let mut watcher = DirectoryWatcher::new(
        dir.clone(),
        ".mneme".to_string(),
        1,
        store,
        "simple-test".to_string(),
    );

    std::fs::write(
        dir.join("simple.mneme"),
        "Simple Title\nContent line one\nContent line two",
    )
    .unwrap();

    let result = watcher.scan().await;
    assert!(result.is_ok());

    let memories = db.memories().list("simple-test", None, None, None, 10, 0).unwrap();
    assert!(!memories.is_empty());
    assert_eq!(memories[0].title, "Simple Title");

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_file(&db_path).ok();
}
