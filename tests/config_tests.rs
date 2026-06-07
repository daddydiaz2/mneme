use mneme::config::settings::Settings;

#[test]
fn test_settings_default_has_expected_values() {
    let s = Settings::default();
    assert_eq!(s.server.port, 8080);
    assert_eq!(s.behavior.max_search_results, 20);
    assert!(!s.crypto.enabled);
    assert!(s.embeddings.enabled);
}

#[test]
fn test_apply_env_overrides_port() {
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
fn test_infer_project_returns_string() {
    let project = Settings::infer_project();
    assert!(!project.is_empty());
}

#[test]
fn test_settings_serialize_roundtrip() {
    let mut s = Settings::default();
    s.server.port = 12345;
    let toml_str = toml::to_string_pretty(&s).unwrap();
    let loaded: Settings = toml::from_str(&toml_str).unwrap();
    assert_eq!(loaded.server.port, 12345);
}
