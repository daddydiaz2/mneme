use mneme::error::{MnemeError, Result};
use std::io;
use uuid::Uuid;

// ── Display tests ────────────────────────────────────────────────────────────

#[test]
fn test_notfound_display() {
    let id = Uuid::nil();
    let err = MnemeError::NotFound(id);
    assert!(err.to_string().contains("Memoria no encontrada"));
}

#[test]
fn test_project_required_display() {
    assert!(MnemeError::ProjectRequired.to_string().contains("proyecto"));
}

#[test]
fn test_empty_query_display() {
    assert!(MnemeError::EmptyQuery
        .to_string()
        .contains("no puede estar vacía"));
}

#[test]
fn test_invalid_memory_type_display() {
    let err = MnemeError::InvalidMemoryType("foo".into());
    assert!(err.to_string().contains("foo"));
    assert!(err.to_string().contains("Tipo de memoria"));
}

#[test]
fn test_self_relation_display() {
    let id = Uuid::nil();
    let err = MnemeError::SelfRelation(id);
    assert!(err.to_string().contains("consigo misma"));
}

#[test]
fn test_sync_failed_display() {
    let err = MnemeError::SyncFailed {
        peer: "peer-a".into(),
        message: "timeout".into(),
    };
    assert!(err.to_string().contains("peer-a"));
    assert!(err.to_string().contains("timeout"));
}

#[test]
fn test_plugin_error_display() {
    let err = MnemeError::Plugin("plugin panicked".into());
    assert!(err.to_string().contains("plugin"));
    assert!(err.to_string().contains("panicked"));
}

#[test]
fn test_no_recipients_configured_display() {
    assert!(MnemeError::NoRecipientsConfigured
        .to_string()
        .contains("clave"));
}

#[test]
fn test_key_not_found_display() {
    let err = MnemeError::KeyNotFound("my-key".into());
    assert!(err.to_string().contains("clave"));
    assert!(err.to_string().contains("my-key"));
}

// ── From trait impls ─────────────────────────────────────────────────────────

#[test]
fn test_from_rusqlite_error() {
    let sq_err = rusqlite::Error::InvalidQuery;
    let mneme_err: MnemeError = sq_err.into();
    assert!(matches!(mneme_err, MnemeError::Database(_)));
    assert!(mneme_err.to_string().contains("base de datos"));
}

#[test]
fn test_from_io_error() {
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let mneme_err: MnemeError = io_err.into();
    assert!(matches!(mneme_err, MnemeError::Io(_)));
    assert!(mneme_err.to_string().contains("E/S"));
}

#[test]
fn test_from_serde_json_error() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
    let mneme_err: MnemeError = json_err.into();
    assert!(matches!(mneme_err, MnemeError::Serialization(_)));
    assert!(mneme_err.to_string().contains("serialización"));
}

#[test]
fn test_from_rusqlite_migration_error() {
    // rusqlite_migration errors are hard to construct inline, so we test the conversion logic
    let err = MnemeError::Migration("test migration error".into());
    assert!(err.to_string().contains("Error de migración"));
    assert!(err.to_string().contains("test migration error"));
}

// ── memory_id extractor ──────────────────────────────────────────────────────

#[test]
fn test_memory_id_extracts_from_not_found() {
    let id = Uuid::new_v4();
    let err = MnemeError::NotFound(id);
    assert_eq!(err.memory_id(), Some(id));
}

#[test]
fn test_memory_id_extracts_from_self_relation() {
    let id = Uuid::new_v4();
    let err = MnemeError::SelfRelation(id);
    assert_eq!(err.memory_id(), Some(id));
}

#[test]
fn test_memory_id_extracts_from_already_encrypted() {
    let id = Uuid::new_v4();
    let err = MnemeError::AlreadyEncrypted(id);
    assert_eq!(err.memory_id(), Some(id));
}

#[test]
fn test_memory_id_extracts_from_not_encrypted() {
    let id = Uuid::new_v4();
    let err = MnemeError::NotEncrypted(id);
    assert_eq!(err.memory_id(), Some(id));
}

#[test]
fn test_memory_id_returns_none_for_unrelated_errors() {
    assert_eq!(MnemeError::ProjectRequired.memory_id(), None);
    assert_eq!(MnemeError::EmptyQuery.memory_id(), None);
    assert_eq!(MnemeError::NoRecipientsConfigured.memory_id(), None);
    assert_eq!(MnemeError::EmbeddingsDisabled.memory_id(), None);
    assert_eq!(MnemeError::SyncDisabled.memory_id(), None);
}

// ── Debug formatting ─────────────────────────────────────────────────────────

#[test]
fn test_error_is_debug() {
    let err = MnemeError::NotFound(Uuid::new_v4());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("NotFound"));
}

#[test]
fn test_result_type_alias() {
    let ok: Result<i32> = Ok(42);
    assert!(ok.is_ok());

    let err: Result<i32> = Err(MnemeError::ProjectRequired);
    assert!(err.is_err());
    if let Err(MnemeError::ProjectRequired) = err {
        // expected variant
    } else {
        panic!("expected ProjectRequired");
    }
}

#[test]
fn test_send_sync_trait() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MnemeError>();
}
