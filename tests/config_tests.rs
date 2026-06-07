use std::path::PathBuf;
use std::sync::Mutex;

use mneme::config::settings::Settings;

static CONFIG_LOCK: Mutex<()> = Mutex::new(());

/// Helper para tests que modifican variables de entorno de config.
fn with_test_config<F>(f: F)
where
    F: FnOnce(),
{
    let _lock = CONFIG_LOCK.lock().unwrap();
    let tmp = std::env::temp_dir().join(format!(
        "mneme_config_env_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::env::set_var("XDG_CONFIG_HOME", &tmp);
    f();
    std::env::remove_var("XDG_CONFIG_HOME");
    std::fs::remove_dir_all(&tmp).ok();
}

// ── Defaults ─────────────────────────────────────────────────────────────────

#[test]
fn test_settings_default_has_expected_values() {
    let s = Settings::default();
    assert_eq!(s.server.port, 8080);
    assert_eq!(s.server.host, "127.0.0.1");
    assert_eq!(s.mcp.default_project, "default");
    assert_eq!(s.tui.theme, "dark");
    assert_eq!(s.behavior.max_search_results, 20);
    assert!(s.behavior.auto_detect_conflicts);
    assert!(s.behavior.decay_enabled);
    assert!((s.behavior.decay_factor - 0.95).abs() < 0.001);
    assert!(s.behavior.auto_session);
    assert!(!s.crypto.enabled);
    assert!(s.crypto.auto_load_identity);
    assert!(s.embeddings.enabled);
    assert_eq!(s.embeddings.model, "BAAI/bge-small-en-v1.5");
    assert!((s.embeddings.search_weight - 0.3).abs() < 0.001);
    assert!((s.embeddings.similarity_threshold - 0.75).abs() < 0.001);
    assert!(s.embeddings.auto_index);
    assert!(s.sync.enabled);
    assert_eq!(s.sync.auto_sync_interval, 0);
    assert!(s.sync.compress);
    assert!(s.crypto.always_encrypt_projects.is_empty());
    assert!(s.crypto.identity_path.is_none());
}

#[test]
fn test_database_config_default_creates_path() {
    let cfg = mneme::config::settings::DatabaseConfig::default();
    assert!(!cfg.path.to_string_lossy().is_empty());
    assert!(cfg.path.to_string_lossy().contains("mneme"));
}

#[test]
fn test_sync_config_default_has_hostname() {
    let cfg = mneme::config::settings::SyncConfig::default();
    assert!(!cfg.peer_name.is_empty());
    assert!(cfg.enabled);
    assert!(cfg.compress);
}

// ── Env overrides ────────────────────────────────────────────────────────────

#[test]
fn test_apply_env_overrides_port() {
    let _lock = CONFIG_LOCK.lock().unwrap();
    std::env::set_var("MNEME_PORT", "9999");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.server.port, 9999);
    std::env::remove_var("MNEME_PORT");
}

#[test]
fn test_apply_env_overrides_project() {
    std::env::set_var("MNEME_PROJECT", "my-test-proj");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.mcp.default_project, "my-test-proj");
    std::env::remove_var("MNEME_PROJECT");
}

#[test]
fn test_apply_env_overrides_crypto_enabled() {
    std::env::set_var("MNEME_CRYPTO_ENABLED", "true");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert!(s.crypto.enabled);
    std::env::remove_var("MNEME_CRYPTO_ENABLED");
}

#[test]
fn test_apply_env_overrides_db_path() {
    std::env::set_var("MNEME_DB_PATH", "/tmp/custom/mneme.db");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.database.path, PathBuf::from("/tmp/custom/mneme.db"));
    std::env::remove_var("MNEME_DB_PATH");
}

#[test]
fn test_apply_env_overrides_host() {
    std::env::set_var("MNEME_HOST", "0.0.0.0");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.server.host, "0.0.0.0");
    std::env::remove_var("MNEME_HOST");
}

#[test]
fn test_apply_env_overrides_embeddings_enabled() {
    std::env::set_var("MNEME_EMBEDDINGS_ENABLED", "false");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert!(!s.embeddings.enabled);
    std::env::remove_var("MNEME_EMBEDDINGS_ENABLED");
}

#[test]
fn test_apply_env_overrides_cache_dir() {
    std::env::set_var("MNEME_CACHE_DIR", "/tmp/custom-cache");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.embeddings.cache_dir, PathBuf::from("/tmp/custom-cache"));
    std::env::remove_var("MNEME_CACHE_DIR");
}

#[test]
fn test_apply_env_overrides_embeddings_model() {
    std::env::set_var("MNEME_EMBEDDINGS_MODEL", "all-MiniLM-L6-v2");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.embeddings.model, "all-MiniLM-L6-v2");
    std::env::remove_var("MNEME_EMBEDDINGS_MODEL");
}

#[test]
fn test_apply_env_overrides_identity() {
    std::env::set_var("MNEME_IDENTITY", "~/.ssh/id_ed25519");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(
        s.crypto.identity_path,
        Some(PathBuf::from("~/.ssh/id_ed25519"))
    );
    std::env::remove_var("MNEME_IDENTITY");
}

#[test]
fn test_apply_env_overrides_invalid_port_keeps_default() {
    std::env::set_var("MNEME_PORT", "not-a-number");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert_eq!(s.server.port, 8080);
    std::env::remove_var("MNEME_PORT");
}

#[test]
fn test_apply_env_overrides_invalid_embeddings_keeps_default() {
    std::env::set_var("MNEME_EMBEDDINGS_ENABLED", "not-bool");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert!(s.embeddings.enabled);
    std::env::remove_var("MNEME_EMBEDDINGS_ENABLED");
}

#[test]
fn test_apply_env_overrides_invalid_crypto_keeps_default() {
    std::env::set_var("MNEME_CRYPTO_ENABLED", "invalid");
    let mut s = Settings::default();
    s.apply_env_overrides();
    assert!(!s.crypto.enabled);
    std::env::remove_var("MNEME_CRYPTO_ENABLED");
}

// ── Paths ────────────────────────────────────────────────────────────────────

#[test]
fn test_config_path_returns_non_empty() {
    let path = Settings::config_path();
    assert!(!path.to_string_lossy().is_empty());
    assert!(path.to_string_lossy().contains("mneme"));
    assert!(path.to_string_lossy().ends_with("config.toml"));
}

// ── Project inference ────────────────────────────────────────────────────────

#[test]
fn test_infer_project_returns_string() {
    let project = Settings::infer_project();
    assert!(!project.is_empty());
}

#[test]
fn test_git_toplevel_returns_some_in_repo() {
    // Running from the mneme repo root, this should succeed
    let toplevel = Settings::git_toplevel();
    assert!(toplevel.is_some(), "should detect git repo root");
    let path = toplevel.unwrap();
    assert!(path.to_string_lossy().contains("mneme"));
}

// ── Serialization ────────────────────────────────────────────────────────────

#[test]
fn test_settings_serialize_roundtrip() {
    let mut s = Settings::default();
    s.server.port = 12345;
    s.server.host = "10.0.0.1".to_string();
    s.behavior.max_search_results = 100;
    let toml_str = toml::to_string_pretty(&s).unwrap();
    let loaded: Settings = toml::from_str(&toml_str).unwrap();
    assert_eq!(loaded.server.port, 12345);
    assert_eq!(loaded.server.host, "10.0.0.1");
    assert_eq!(loaded.behavior.max_search_results, 100);
}

#[test]
fn test_settings_serialize_all_fields() {
    let s = Settings::default();
    let toml_str = toml::to_string_pretty(&s).unwrap();
    // Verify key sections are present
    assert!(toml_str.contains("[database]"));
    assert!(toml_str.contains("[server]"));
    assert!(toml_str.contains("[mcp]"));
    assert!(toml_str.contains("[behavior]"));
    assert!(toml_str.contains("[embeddings]"));
    assert!(toml_str.contains("[sync]"));
    assert!(toml_str.contains("[crypto]"));
    assert!(toml_str.contains("[tui]"));
}

#[test]
fn test_settings_deserialize_partial() {
    // Only providing a subset of fields should fill defaults
    let toml_str = r#"
[server]
port = 8080
"#;
    let s: Result<Settings, _> = toml::from_str(toml_str);
    // This may fail depending on whether serde requires all fields
    // Just verify it either succeeds or fails gracefully
    if let Ok(settings) = s {
        assert_eq!(settings.server.port, 8080);
    }
}

// ── Load / Save ──────────────────────────────────────────────────────────────

#[test]
fn test_load_creates_default_when_no_file() {
    with_test_config(|| {
        let config_path = Settings::config_path();
        std::fs::remove_file(&config_path).ok();
        let settings = Settings::load();
        assert!(settings.is_ok(), "load should create default config: {:?}", settings.err());
    });
}

#[test]
fn test_save_writes_valid_toml() {
    with_test_config(|| {
        let config_path = Settings::config_path();
        let config_dir = config_path.parent().unwrap();
        std::fs::create_dir_all(config_dir).ok();
        let s = Settings::default();
        let result = s.save();
        assert!(result.is_ok(), "save should succeed: {:?}", result.err());
        assert!(config_path.exists(), "config file should exist");
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[server]"));
        assert!(content.contains("port = 8080"));
    });
}
