use crate::crypto::keys::RecipientKey;
use chrono::Utc;
use rusqlite::params;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegisteredKey {
    pub id: Uuid,
    pub alias: String,
    pub key_type: String,
    pub public_key: String,
    pub is_default: bool,
    pub added_at: chrono::DateTime<Utc>,
}

/// Persiste claves de encriptación en SQLite.
pub struct KeyStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl KeyStore {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>) -> Self {
        Self { conn }
    }

    /// Registra una nueva clave pública.
    pub fn add(&self, alias: &str, key: &RecipientKey) -> crate::error::Result<RegisteredKey> {
        let id = Uuid::new_v4();
        let now = Utc::now().to_rfc3339();
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute(
            "INSERT INTO encryption_keys (id, alias, key_type, public_key, is_default, added_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![
                id.to_string(),
                alias,
                key.key_type(),
                key.public_key_string(),
                now
            ],
        )?;
        Ok(RegisteredKey {
            id,
            alias: alias.to_string(),
            key_type: key.key_type().to_string(),
            public_key: key.public_key_string(),
            is_default: false,
            added_at: Utc::now(),
        })
    }

    /// Elimina una clave por ID.
    pub fn remove(&self, key_id: Uuid) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let n = conn.execute(
            "DELETE FROM encryption_keys WHERE id = ?1",
            params![key_id.to_string()],
        )?;
        if n == 0 {
            return Err(crate::error::MnemeError::KeyNotFound(key_id.to_string()));
        }
        Ok(())
    }

    /// Lista todas las claves registradas.
    pub fn list(&self) -> crate::error::Result<Vec<RegisteredKey>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        let mut stmt = conn.prepare(
            "SELECT id, alias, key_type, public_key, is_default, added_at FROM encryption_keys ORDER BY added_at DESC"
        )?;
        let keys = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let added_str: String = row.get(5)?;
                Ok((
                    id_str,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, bool>(4)?,
                    added_str,
                ))
            })?
            .filter_map(|r| r.ok())
            .map(|(id_s, alias, key_type, public_key, is_default, added_s)| {
                let id = Uuid::parse_str(&id_s).unwrap_or_else(|_| Uuid::new_v4());
                let added_at = chrono::DateTime::parse_from_rfc3339(&added_s)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                RegisteredKey {
                    id,
                    alias,
                    key_type,
                    public_key,
                    is_default,
                    added_at,
                }
            })
            .collect();
        Ok(keys)
    }

    /// Retorna la clave marcada como default.
    pub fn get_default(&self) -> crate::error::Result<Option<RegisteredKey>> {
        let all = self.list()?;
        Ok(all.into_iter().find(|k| k.is_default))
    }

    /// Marca una clave como default (desmarca las demás).
    pub fn set_default(&self, key_id: Uuid) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute("UPDATE encryption_keys SET is_default = 0", [])?;
        let n = conn.execute(
            "UPDATE encryption_keys SET is_default = 1 WHERE id = ?1",
            params![key_id.to_string()],
        )?;
        if n == 0 {
            return Err(crate::error::MnemeError::KeyNotFound(key_id.to_string()));
        }
        Ok(())
    }

    /// Carga todos los recipients para el CryptoEngine.
    pub fn load_all_recipients(&self) -> crate::error::Result<Vec<RecipientKey>> {
        let keys = self.list()?;
        let mut recipients = Vec::new();
        for key in keys {
            match RecipientKey::from_string(&key.public_key) {
                Ok(r) => recipients.push(r),
                Err(e) => {
                    tracing::warn!(key_id = %key.id, error = %e, "failed to parse registered key, skipping");
                }
            }
        }
        Ok(recipients)
    }
}
