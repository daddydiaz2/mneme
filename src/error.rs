use uuid::Uuid;

/// Error principal del sistema Mneme.
#[derive(Debug, thiserror::Error)]
pub enum MnemeError {
    /// Memoria no encontrada con el UUID proporcionado.
    #[error("Memoria no encontrada: {0}")]
    NotFound(Uuid),

    /// Se requiere un proyecto para esta operación.
    #[error("Se requiere un proyecto para esta operación")]
    ProjectRequired,

    /// La consulta de búsqueda está vacía.
    #[error("La consulta de búsqueda no puede estar vacía")]
    EmptyQuery,

    /// Tipo de memoria inválido.
    #[error("Tipo de memoria inválido: {0}")]
    InvalidMemoryType(String),

    /// Importancia inválida.
    #[error("Importancia inválida: {0}")]
    InvalidImportance(String),

    /// Alcance inválido.
    #[error("Alcance inválido: {0}")]
    InvalidScope(String),

    /// Tipo de relación inválido.
    #[error("Tipo de relación inválido: {0}")]
    InvalidRelationType(String),

    /// La relación entre dos memorias ya existe.
    #[error("La relación entre {0} y {1} ya existe")]
    RelationAlreadyExists(Uuid, Uuid),

    /// No se permite una relación de una memoria consigo misma.
    #[error("No se permite relacionar una memoria consigo misma: {0}")]
    SelfRelation(Uuid),

    /// Se detectó un duplicado.
    #[error("Duplicado detectado: {0}")]
    DuplicateDetected(String),

    /// Error de la base de datos SQLite.
    #[error("Error de base de datos: {0}")]
    Database(#[from] rusqlite::Error),

    /// Error durante la migración.
    #[error("Error de migración: {0}")]
    Migration(String),

    /// Error de entrada/salida.
    #[error("Error de E/S: {0}")]
    Io(#[from] std::io::Error),

    /// Error de serialización.
    #[error("Error de serialización: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Error de configuración.
    #[error("Error de configuración: {0}")]
    Config(String),

    /// Error del servidor HTTP.
    #[error("Error HTTP: {0}")]
    Http(String),

    /// Error del protocolo MCP.
    #[error("Error MCP: {0}")]
    Mcp(String),

    /// Error de embeddings.
    #[error("error de embeddings: {0}")]
    Embeddings(String),

    /// Modelo de embeddings no disponible.
    #[error("modelo de embeddings no disponible")]
    EmbeddingsDisabled,

    /// Peer no encontrado.
    #[error("peer no encontrado: {0}")]
    PeerNotFound(uuid::Uuid),

    /// Error de sync con peer.
    #[error("error de sync con peer '{peer}': {message}")]
    SyncFailed { peer: String, message: String },

    /// Sync deshabilitado.
    #[error("sync deshabilitado")]
    SyncDisabled,

    /// Transporte no soportado.
    #[error("transporte no soportado: {0}")]
    UnsupportedTransport(String),

    /// Archivo de sync invalido.
    #[error("archivo de sync invalido: {0}")]
    InvalidSyncFile(String),

    /// Error de compresion.
    #[error("error de compresion: {0}")]
    Compression(String),

    /// Encriptación no disponible — no hay recipients configurados.
    #[error("encriptación no disponible — configurar al menos una clave con 'mneme keys add'")]
    NoRecipientsConfigured,

    /// Fallo de desencriptación.
    #[error("no se pudo desencriptar — verificar que la clave privada es correcta")]
    DecryptionFailed,

    /// Identidad de desencriptación no cargada.
    #[error("identidad de desencriptación no disponible — ejecutar 'mneme keys load'")]
    IdentityNotLoaded,

    /// Memoria ya encriptada.
    #[error("la memoria {0} ya está encriptada")]
    AlreadyEncrypted(Uuid),

    /// Memoria no encriptada.
    #[error("la memoria {0} no está encriptada")]
    NotEncrypted(Uuid),

    /// Clave no encontrada.
    #[error("clave no encontrada: {0}")]
    KeyNotFound(String),
}

impl From<rusqlite_migration::Error> for MnemeError {
    fn from(err: rusqlite_migration::Error) -> Self {
        MnemeError::Migration(err.to_string())
    }
}

impl From<automerge::AutomergeError> for MnemeError {
    fn from(err: automerge::AutomergeError) -> Self {
        MnemeError::Config(format!("automerge error: {}", err))
    }
}

/// Resultado tipificado de Mneme.
pub type Result<T> = std::result::Result<T, MnemeError>;

impl MnemeError {
    /// UUID de la memoria asociada al error, si aplica.
    pub fn memory_id(&self) -> Option<Uuid> {
        match self {
            MnemeError::NotFound(id) | MnemeError::SelfRelation(id) | MnemeError::AlreadyEncrypted(id) | MnemeError::NotEncrypted(id) => Some(*id),
            _ => None,
        }
    }
}
