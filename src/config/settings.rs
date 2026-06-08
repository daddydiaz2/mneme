use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuración de la base de datos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Ruta al archivo SQLite.
    pub path: PathBuf,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("mneme");
        std::fs::create_dir_all(&path).ok();
        path.push("mneme.db");
        Self { path }
    }
}

/// Configuración del servidor HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Host de escucha.
    pub host: String,
    /// Puerto de escucha.
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 8080,
        }
    }
}

/// Configuración del protocolo MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    /// Proyecto por defecto para operaciones MCP.
    pub default_project: String,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            default_project: "default".into(),
        }
    }
}

/// Configuración de la interfaz TUI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    /// Tema visual (dark, light).
    pub theme: String,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
        }
    }
}

/// Configuración de comportamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    /// Detectar conflictos automáticamente.
    pub auto_detect_conflicts: bool,
    /// Habilitar decaimiento de relevancia.
    pub decay_enabled: bool,
    /// Factor de decaimiento (0.0 - 1.0).
    pub decay_factor: f64,
    /// Máximo de resultados de búsqueda.
    pub max_search_results: u32,
    /// Crear sesiones automáticamente.
    pub auto_session: bool,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            auto_detect_conflicts: true,
            decay_enabled: true,
            decay_factor: 0.95,
            max_search_results: 20,
            auto_session: true,
        }
    }
}

/// Proveedor de embeddings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum EmbeddingProvider {
    /// ONNX local via fastembed (default, zero config).
    #[default]
    Onnx,
    /// OpenAI API (requires OPENAI_API_KEY env).
    OpenAI,
    /// Ollama local server (requires OLLAMA_HOST env, default http://localhost:11434).
    Ollama,
    /// Google Gemini API (requires GOOGLE_API_KEY env).
    Google,
}

impl std::fmt::Display for EmbeddingProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EmbeddingProvider::Onnx => "onnx",
            EmbeddingProvider::OpenAI => "openai",
            EmbeddingProvider::Ollama => "ollama",
            EmbeddingProvider::Google => "google",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for EmbeddingProvider {
    type Err = crate::error::MnemeError;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "onnx" => Ok(EmbeddingProvider::Onnx),
            "openai" => Ok(EmbeddingProvider::OpenAI),
            "ollama" => Ok(EmbeddingProvider::Ollama),
            "google" => Ok(EmbeddingProvider::Google),
            other => Err(crate::error::MnemeError::Config(format!(
                "Unknown embedding provider: {}",
                other
            ))),
        }
    }
}

/// Configuración de embeddings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// Habilitar búsqueda semántica.
    pub enabled: bool,
    /// Proveedor de embeddings (onnx, openai, ollama, google).
    #[serde(default)]
    pub provider: EmbeddingProvider,
    /// Modelo de embeddings a utilizar.
    pub model: String,
    /// Directorio de caché (usado por ONNX).
    pub cache_dir: PathBuf,
    /// Indexar automáticamente nuevas memorias.
    pub auto_index: bool,
    /// Peso en la puntuación combinada.
    pub search_weight: f64,
    /// Umbral de similitud para considerar relevante.
    pub similarity_threshold: f32,
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache_dir.push("mneme");
        std::fs::create_dir_all(&cache_dir).ok();
        Self {
            enabled: true,
            provider: EmbeddingProvider::Onnx,
            model: "BAAI/bge-small-en-v1.5".into(),
            cache_dir,
            auto_index: true,
            search_weight: 0.3,
            similarity_threshold: 0.75,
        }
    }
}

/// Configuración de sincronización.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Habilitar sync.
    pub enabled: bool,
    /// ID del peer.
    pub peer_id: String,
    /// Nombre del peer.
    pub peer_name: String,
    /// Intervalo de auto-sync en segundos (0 = deshabilitado).
    pub auto_sync_interval: u64,
    /// Comprimir documentos.
    pub compress: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            peer_id: String::new(),
            peer_name: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "mneme-peer".to_string()),
            auto_sync_interval: 0,
            compress: true,
        }
    }
}

/// Configuración de encriptación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub enabled: bool,
    pub auto_load_identity: bool,
    pub identity_path: Option<PathBuf>,
    pub always_encrypt_projects: Vec<String>,
}

impl Default for CryptoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            auto_load_identity: true,
            identity_path: None,
            always_encrypt_projects: vec![],
        }
    }
}

/// Configuración global de Mneme.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Configuración de base de datos.
    pub database: DatabaseConfig,
    /// Configuración de servidor HTTP.
    pub server: ServerConfig,
    /// Configuración MCP.
    pub mcp: McpConfig,
    /// Configuración TUI.
    pub tui: TuiConfig,
    /// Configuración de comportamiento.
    pub behavior: BehaviorConfig,
    /// Configuración de embeddings.
    pub embeddings: EmbeddingsConfig,
    /// Configuración de sincronización.
    pub sync: SyncConfig,
    /// Configuración de encriptación.
    pub crypto: CryptoConfig,
}

impl Settings {
    /// Carga la configuración desde el archivo de configuración.
    /// Si no existe, crea uno con valores por defecto.
    pub fn load() -> crate::error::Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            let settings = Self::default();
            settings.save()?;
            return Ok(settings);
        }

        let content = std::fs::read_to_string(&path)?;
        let mut settings: Settings = toml::from_str(&content)
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;

        settings.apply_env_overrides();
        Ok(settings)
    }

    /// Guarda la configuración actual en el archivo de configuración.
    pub fn save(&self) -> crate::error::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Retorna la ruta al archivo de configuración.
    pub fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("mneme");
        path.push("config.toml");
        path
    }

    /// Aplica sobre-escrituras desde variables de entorno.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("MNEME_DB_PATH") {
            self.database.path = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("MNEME_PORT") {
            if let Ok(port) = val.parse::<u16>() {
                self.server.port = port;
            }
        }
        if let Ok(val) = std::env::var("MNEME_PROJECT") {
            self.mcp.default_project = val;
        }
        if let Ok(val) = std::env::var("MNEME_HOST") {
            self.server.host = val;
        }
        if let Ok(val) = std::env::var("MNEME_EMBEDDINGS_ENABLED") {
            self.embeddings.enabled = val.parse::<bool>().unwrap_or(self.embeddings.enabled);
        }
        if let Ok(val) = std::env::var("MNEME_EMBEDDING_PROVIDER") {
            if let Ok(provider) = val.parse() {
                self.embeddings.provider = provider;
            }
        }
        if let Ok(val) = std::env::var("MNEME_CACHE_DIR") {
            self.embeddings.cache_dir = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("MNEME_EMBEDDINGS_MODEL") {
            self.embeddings.model = val;
        }
        if let Ok(val) = std::env::var("MNEME_CRYPTO_ENABLED") {
            self.crypto.enabled = val.parse::<bool>().unwrap_or(self.crypto.enabled);
        }
        if let Ok(val) = std::env::var("MNEME_IDENTITY") {
            self.crypto.identity_path = Some(PathBuf::from(val));
        }
    }

    /// Infiere el nombre del proyecto actual.
    /// Intenta obtener el directorio raíz de git; si falla, usa el directorio actual.
    pub fn infer_project() -> String {
        match Self::git_toplevel() {
            Some(path) => path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            None => std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "unknown".into()),
        }
    }

    /// Ejecuta `git rev-parse --show-toplevel` para obtener la raíz del repo.
    pub fn git_toplevel() -> Option<PathBuf> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Some(PathBuf::from(path))
        } else {
            None
        }
    }
}
