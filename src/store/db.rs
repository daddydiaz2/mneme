use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Conexión gestionada a la base de datos SQLite.
#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    crypto: Option<Arc<Mutex<crate::crypto::CryptoEngine>>>,
}

impl Database {
    /// Abre o crea la base de datos en la ruta indicada y aplica migraciones.
    pub fn open(path: &Path) -> crate::error::Result<Self> {
        let mut conn = Connection::open(path)?;

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "cache_size", -64000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;

        crate::store::migrations::run_migrations(&mut conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            crypto: None,
        })
    }

    /// Asigna el motor de encriptación.
    pub fn with_crypto(mut self, crypto: Arc<Mutex<crate::crypto::CryptoEngine>>) -> Self {
        self.crypto = Some(crypto);
        self
    }

    /// Retorna un store de memorias.
    pub fn memories(&self) -> crate::store::memory::MemoryStore {
        let store = crate::store::memory::MemoryStore::new(self.conn.clone());
        match &self.crypto {
            Some(crypto) => store.with_crypto(Arc::clone(crypto)),
            None => store,
        }
    }

    /// Retorna un store de sesiones.
    pub fn sessions(&self) -> crate::store::memory::SessionStore {
        crate::store::memory::SessionStore::new(self.conn.clone())
    }

    /// Retorna un store de embeddings.
    pub fn embeddings(&self) -> crate::embeddings::store::EmbeddingStore {
        crate::embeddings::store::EmbeddingStore::new(self.conn.clone())
    }

    /// Retorna un store de peers.
    pub fn peers(&self) -> crate::sync::peer::PeerStore {
        crate::sync::peer::PeerStore::new(self.conn.clone())
    }

    /// Retorna una copia del Arc de conexion.
    pub fn get_conn(&self) -> Arc<Mutex<Connection>> {
        self.conn.clone()
    }

    /// Ejecuta PRAGMA optimize para mejorar el rendimiento del query planner.
    pub fn optimize(&self) -> crate::error::Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| crate::error::MnemeError::Config("mutex poisoned".into()))?;
        conn.execute("PRAGMA optimize", [])?;
        Ok(())
    }
}
