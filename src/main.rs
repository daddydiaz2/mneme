use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use tracing::info;

use mneme::cli::commands::{Cli, Commands};
use mneme::config::settings::Settings;
#[cfg(feature = "embeddings")]
use mneme::embeddings::engine::EmbeddingEngine;
use mneme::store::db::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_filter = std::env::var("MNEME_LOG").unwrap_or_else(|_| "mneme=info".to_string());

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(&log_filter)
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Mneme v{} iniciado", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();
    let settings = Settings::load()?;
    info!("Configuracion cargada correctamente");

    let data_dir = settings
        .database
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(data_dir)?;
    info!("Data directory ensured: {}", data_dir.display());

    let db = Arc::new(Database::open(&settings.database.path)?);
    info!("Database opened: {}", settings.database.path.display());

    #[cfg(feature = "embeddings")]
    let embeddings = if settings.embeddings.enabled {
        tracing::info!("inicializando motor de embeddings...");
        match EmbeddingEngine::new(&settings.embeddings.cache_dir).await {
            Ok(engine) => {
                tracing::info!(
                    model = %engine.model_name(),
                    dimensions = engine.dimensions(),
                    "motor listo"
                );
                Some(Arc::new(engine))
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "embeddings no disponibles, continuando sin busqueda semantica"
                );
                None
            }
        }
    } else {
        tracing::debug!("embeddings deshabilitados por configuracion");
        None
    };

    #[cfg(not(feature = "embeddings"))]
    let embeddings: Option<std::sync::Arc<mneme::embeddings::engine::EmbeddingEngine>> = {
        tracing::debug!("embeddings compilados sin feature — deshabilitados");
        None
    };

    let db = if settings.crypto.enabled {
        let key_store = mneme::crypto::KeyStore::new(db.get_conn());
        let recipients = key_store.load_all_recipients().unwrap_or_default();

        if recipients.is_empty() {
            tracing::warn!(
                "encriptacion habilitada pero sin claves registradas — usar 'mneme keys add'"
            );
            db
        } else {
            let mut engine = mneme::crypto::CryptoEngine::new(recipients);
            if settings.crypto.auto_load_identity {
                if let Some(ref path) = settings.crypto.identity_path {
                    match engine.load_identity_from_path(path) {
                        Ok(_) => tracing::info!("identidad cargada desde config"),
                        Err(e) => tracing::warn!(error = %e, "identidad no disponible"),
                    }
                } else {
                    match engine.load_identity() {
                        Ok(_) => tracing::info!("identidad de desencriptacion cargada"),
                        Err(e) => {
                            tracing::warn!(error = %e, "identidad no disponible — solo encriptacion activa")
                        }
                    }
                }
            }
            Arc::new(Database::with_crypto(
                Arc::try_unwrap(db).unwrap_or_else(|arc| (*arc).clone()),
                Arc::new(std::sync::Mutex::new(engine)),
            ))
        }
    } else {
        tracing::debug!("encriptacion deshabilitada por configuracion");
        db
    };

    match cli.command {
        Commands::Mcp => {
            let server = mneme::mcp::server::MnemeServer::new(db, Arc::new(settings), embeddings);
            server.run_stdio().await?;
        }
        Commands::Serve { port, host } => {
            let router = mneme::http::router::create_router(db, embeddings);
            let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
            info!("Starting HTTP server on {}", addr);
            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, router).await?;
        }
        Commands::Tui => {
            let settings_arc = Arc::new(settings.clone());
            mneme::tui::run_tui(Arc::clone(&db), settings_arc)?;
        }
        Commands::Watch {
            project,
            dir,
            interval,
            ext,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let dir = dir.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
            let store = db.memories();
            let mut watcher =
                mneme::watch::DirectoryWatcher::new(dir, ext, interval, store, project);
            watcher.run().await?;
        }
        command => {
            mneme::cli::commands::run_command(command, &db, embeddings.as_ref())?;
        }
    }

    Ok(())
}
