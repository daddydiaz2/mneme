use mneme::plugins::PluginManager;
use tempfile::TempDir;

fn empty_plugin_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

#[test]
fn test_load_from_empty_dir_returns_empty_manager() {
    let dir = empty_plugin_dir();
    let pm = PluginManager::load_from_dir(dir.path()).expect("load should succeed");
    assert!(pm.is_empty());
}

#[test]
fn test_is_empty_returns_true_for_empty_manager() {
    let pm = PluginManager::empty();
    assert!(pm.is_empty());
}

#[test]
fn test_owns_tool_returns_false_for_unknown_tool() {
    let pm = PluginManager::empty();
    assert!(!pm.owns_tool("mem_save"));
    assert!(!pm.owns_tool("nonexistent_plugin_tool"));
}

#[test]
fn test_plugin_tools_returns_empty_vec_for_empty_manager() {
    let pm = PluginManager::empty();
    assert!(pm.plugin_tools().is_empty());
}

#[test]
fn test_call_tool_returns_plugin_error_for_unknown_tool() {
    let pm = PluginManager::empty();
    let result = pm.call_tool("nonexistent_tool", serde_json::json!({}), "test");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, mneme::MnemeError::Plugin(_)));
}

#[test]
fn test_run_pre_save_returns_memory_unchanged_when_no_plugins() {
    let pm = PluginManager::empty();
    let memory = serde_json::json!({ "title": "test", "content": "hello" });
    let result = pm
        .run_pre_save(memory.clone())
        .expect("pre_save should succeed");
    assert_eq!(result, memory);
}

#[test]
fn test_run_post_get_returns_memory_unchanged_when_no_plugins() {
    let pm = PluginManager::empty();
    let memory = serde_json::json!({ "id": "abc", "title": "test" });
    let result = pm
        .run_post_get(memory.clone())
        .expect("post_get should succeed");
    assert_eq!(result, memory);
}

#[test]
fn test_load_from_nonexistent_dir_returns_empty_manager() {
    let pm = PluginManager::load_from_dir(std::path::Path::new("/tmp/mneme_no_such_dir_xyz"))
        .expect("load from missing dir should not error");
    assert!(pm.is_empty());
}
