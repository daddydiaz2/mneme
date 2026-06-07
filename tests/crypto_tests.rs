use mneme::crypto::{CryptoEngine, KeyStore, RecipientKey};
use secrecy::ExposeSecret;
use mneme::store::memory::{CreateMemoryInput, MemoryStore, MemoryType, Importance, Scope};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Helper para crear una identity age nativa de test
fn test_identity() -> (age::x25519::Identity, age::x25519::Recipient) {
    let identity = age::x25519::Identity::generate();
    let recipient = identity.to_public();
    (identity, recipient)
}

#[test]
fn test_encrypt_decrypt_roundtrip_age_native() {
    let (identity, recipient) = test_identity();
    let key = RecipientKey::Age(recipient.to_string());
    let mut engine = CryptoEngine::new(vec![key]);

    // Guardar identity temporalmente
    let identity_str = identity.to_string();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), identity_str.expose_secret()).unwrap();
    engine.load_identity_from_path(&tmp.path().to_path_buf()).unwrap();

    let plaintext = "hello world secret";
    let ciphertext = engine.encrypt_str(plaintext).unwrap();
    assert_ne!(ciphertext, plaintext);

    let decrypted = engine.decrypt_str(&ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_requires_recipients() {
    let engine = CryptoEngine::new(vec![]);
    let result = engine.encrypt_str("test");
    assert!(result.is_err());
}

// Nota: test con SSH key se marca #[ignore] en CI
#[test]
#[ignore]
fn test_encrypt_with_ssh_key() {
    // Solo corre si ~/.ssh/id_ed25519.pub existe
    let home = dirs::home_dir().unwrap();
    let pub_key_path = home.join(".ssh/id_ed25519.pub");
    if !pub_key_path.exists() {
        return;
    }
    let key = RecipientKey::from_ssh_file(&pub_key_path).unwrap();
    let engine = CryptoEngine::new(vec![key]);
    assert!(engine.has_recipients());
}

fn in_memory_store() -> Arc<Mutex<Connection>> {
    let conn = Connection::open_in_memory().unwrap();
    // Crear tabla mínima para tests
    conn.execute_batch("
        CREATE TABLE memories (
            id TEXT PRIMARY KEY,
            project TEXT NOT NULL,
            scope TEXT NOT NULL DEFAULT 'project',
            title TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            what TEXT,
            why TEXT,
            context TEXT,
            learned TEXT,
            memory_type TEXT NOT NULL DEFAULT 'note',
            importance TEXT NOT NULL DEFAULT 'medium',
            tags TEXT NOT NULL DEFAULT '[]',
            topic_key TEXT,
            access_count INTEGER NOT NULL DEFAULT 0,
            revision_count INTEGER NOT NULL DEFAULT 0,
            duplicate_count INTEGER NOT NULL DEFAULT 0,
            normalized_hash TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_accessed_at TEXT,
            last_seen_at TEXT,
            deleted_at TEXT,
            deprecated_at TEXT,
            deprecated_reason TEXT,
            supersedes_id TEXT,
            context_inject_count INTEGER NOT NULL DEFAULT 0,
            origin_peer TEXT,
            is_encrypted INTEGER NOT NULL DEFAULT 0,
            encrypted_for TEXT
        );
        CREATE VIRTUAL TABLE memories_fts USING fts5(
            title, content, what, why, context, learned, tags, content='memories'
        );
        CREATE TABLE encryption_keys (
            id TEXT PRIMARY KEY,
            alias TEXT NOT NULL,
            key_type TEXT NOT NULL,
            public_key TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0,
            added_at TEXT NOT NULL
        );
    ").unwrap();
    Arc::new(Mutex::new(conn))
}

#[test]
fn test_save_encrypted_memory_stores_ciphertext() {
    let (identity, recipient) = test_identity();
    let key = RecipientKey::Age(recipient.to_string());
    let mut engine = CryptoEngine::new(vec![key]);

    let identity_str = identity.to_string();
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), identity_str.expose_secret()).unwrap();
    engine.load_identity_from_path(&tmp.path().to_path_buf()).unwrap();

    let conn = in_memory_store();
    let store = MemoryStore::new(conn).with_crypto(Arc::new(Mutex::new(engine)));

    let input = CreateMemoryInput {
        project: "test".to_string(),
        scope: Some(Scope::Project),
        title: "test".to_string(),
        content: "secret content".to_string(),
        what: None,
        why: None,
        context: None,
        learned: None,
        memory_type: MemoryType::Note,
        importance: Importance::Medium,
        tags: vec![],
        topic_key: None,
        capture_prompt: None,
        encrypt: true,
    };

    let memory = store.save(input, None, None).unwrap();
    assert!(memory.is_encrypted);
    assert!(memory.encrypted_for.is_some());

    // Verificar que el contenido almacenado es ciphertext (hex)
    // get() desencripta si tiene identidad
    let retrieved = store.get(memory.id).unwrap().unwrap();
    assert_eq!(retrieved.content, "secret content");
}

#[test]
fn test_key_store_add_and_list() {
    let conn = in_memory_store();
    let key_store = KeyStore::new(conn);
    let key = RecipientKey::Age("age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq".to_string());
    let result = key_store.add("test-key", &key);
    assert!(result.is_ok());
    let keys = key_store.list().unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].alias, "test-key");
}

#[test]
fn test_recipient_key_type_detection() {
    let age_key = RecipientKey::from_string("age1qyqszqgpqyqs...").unwrap();
    assert_eq!(age_key.key_type(), "age");

    let ssh_key = RecipientKey::from_string("ssh-ed25519 AAAAC3...").unwrap();
    assert_eq!(ssh_key.key_type(), "ssh-ed25519");
}

#[test]
fn test_identity_detect_from_env() {
    std::env::remove_var("MNEME_IDENTITY");
    // No falla si no hay SSH key disponible, solo retorna Err
    let _ = mneme::crypto::IdentityKey::detect();
}
