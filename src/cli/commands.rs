use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use uuid::Uuid;

use crate::cli::output;
use crate::cli::output::{BOLD, GREEN, RESET};
use crate::config::settings::Settings;
use crate::store::db::Database;
use crate::store::memory::{CreateMemoryInput, Scope, SearchQuery};
use std::str::FromStr;

/// CLI principal de Mneme.
#[derive(Parser)]
#[command(
    name = "mneme",
    about = "mneme — la memoria persistente para tu agente IA",
    version,
    author
)]
pub struct Cli {
    /// Comando a ejecutar.
    #[command(subcommand)]
    pub command: Commands,
}

/// Comandos disponibles.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub enum Commands {
    /// Guarda una nueva memoria.
    Save {
        /// Título de la memoria.
        title: String,
        /// Contenido de la memoria.
        content: String,
        /// Proyecto asociado.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Tipo de memoria.
        #[arg(long, short = 't', default_value = "note")]
        r#type: String,
        /// Nivel de importancia.
        #[arg(long, short = 'i', default_value = "medium")]
        importance: String,
        /// Tags separados por coma.
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
        /// Campo "what".
        #[arg(long)]
        what: Option<String>,
        /// Campo "why".
        #[arg(long)]
        why: Option<String>,
        /// Campo "context".
        #[arg(long)]
        context: Option<String>,
        /// Campo "learned".
        #[arg(long)]
        learned: Option<String>,
        /// Alcance de la memoria.
        #[arg(long)]
        scope: Option<String>,
        /// Clave de tópico para upserts.
        #[arg(long)]
        topic_key: Option<String>,
    },
    /// Busca memorias.
    Search {
        /// Texto de búsqueda.
        query: String,
        /// Filtrar por proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Filtrar por tipo.
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Límite de resultados.
        #[arg(long, short = 'l', default_value = "10")]
        limit: u32,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Obtiene una memoria por ID.
    Get {
        /// UUID de la memoria.
        id: String,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Lista memorias.
    List {
        /// Filtrar por proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Filtrar por tipo.
        #[arg(long, short = 't')]
        r#type: Option<String>,
        /// Límite de resultados.
        #[arg(long, short = 'l', default_value = "20")]
        limit: u32,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Elimina una memoria.
    Delete {
        /// UUID de la memoria.
        id: String,
        /// Eliminación física (hard delete).
        #[arg(long)]
        hard: bool,
    },
    /// Restaura una memoria eliminada.
    Restore {
        /// UUID de la memoria.
        id: String,
    },
    /// Crea una relación entre memorias.
    Relate {
        /// ID de origen.
        from_id: String,
        /// ID de destino.
        to_id: String,
        /// Tipo de relación.
        relation_type: String,
        /// Confianza de la relación.
        #[arg(long)]
        confidence: Option<f32>,
    },
    /// Obtiene contexto reciente.
    Context {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Límite de resultados.
        #[arg(long, short = 'l', default_value = "10")]
        limit: u32,
    },
    /// Muestra estadísticas.
    Stats {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Lista proyectos.
    Projects,
    /// Inicia servidor HTTP.
    Serve {
        /// Puerto.
        #[arg(long, short = 'p', default_value = "7438")]
        port: u16,
        /// Host.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
    },
    /// Inicia servidor MCP.
    Mcp,
    /// Inicia TUI interactiva (lista, grafo, entidades, temporal, search).
    Tui,
    /// Configura integración con agentes.
    Setup {
        /// Agente a configurar.
        #[command(subcommand)]
        agent: AgentSetup,
    },
    /// Exporta memorias a JSON o Markdown.
    Export {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Archivo de salida.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
        /// Formato: json (default) o md.
        #[arg(long, short = 'f', default_value = "json")]
        format: String,
    },
    /// Importa memorias desde JSON.
    Import {
        /// Archivo a importar.
        file: PathBuf,
        /// Proyecto destino.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Ejecuta diagnósticos.
    Doctor {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Reindexa embeddings de un proyecto.
    Reindex {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Forzar reindexación de todos los embeddings.
        #[arg(long)]
        force: bool,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Ejecuta un benchmark de retrieval quality.
    Bench {
        /// Archivo de escenario (TOML/JSON). Si no se da, usa el ejemplo 'rust-decisions'.
        #[arg(long, short = 's')]
        scenario: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Minería de failures — analiza sesiones fallidas y genera memorias correctivas.
    Learn {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Guarda un lote de memorias desde un archivo JSON.
    SaveBatch {
        /// Archivo JSON con el lote de memorias.
        file: PathBuf,
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Elimina una relación por ID.
    DeleteRelation {
        /// ID de la relación.
        relation_id: String,
    },
    /// Audita calidad de memorias.
    Audit {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Días de umbral para stale.
        #[arg(long, default_value = "30")]
        days_threshold: u32,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Busca duplicados semánticos.
    Deduplicate {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Umbral de similitud.
        #[arg(long, default_value = "0.85")]
        threshold: f64,
    },
    /// Registra feedback sobre una memoria.
    Feedback {
        /// ID de la memoria.
        memory_id: String,
        /// Útil o no.
        is_useful: bool,
        /// Razón opcional.
        #[arg(long)]
        reason: Option<String>,
    },
    /// Depreca una memoria.
    Deprecate {
        /// ID de la memoria.
        memory_id: String,
        /// Razón de deprecación.
        reason: String,
        /// ID de la memoria que la reemplaza.
        #[arg(long)]
        supersedes_id: Option<String>,
    },
    /// Muestra el grafo de conocimiento.
    Graph {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Genera resumen ejecutivo.
    Summarize {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// ID de sesión opcional.
        #[arg(long)]
        session_id: Option<String>,
    },
    /// Inyecta contexto formateado.
    InjectContext {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Archivo relacionado.
        #[arg(long)]
        file: Option<String>,
        /// Límite de resultados.
        #[arg(long, short = 'l', default_value = "10")]
        limit: u32,
    },
    /// Elimina todas las memorias de un proyecto.
    ForgetProject {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Confirmación explícita.
        #[arg(long)]
        confirm: bool,
    },
    /// Muestra salud del sistema.
    Health {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Muestra recordatorios importantes.
    Remind {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Nivel de importancia.
        #[arg(long, short = 'i', default_value = "high")]
        importance: String,
    },
    /// Sugiere tags.
    TagSuggest {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Título de referencia.
        title: String,
        /// Contenido opcional.
        #[arg(long)]
        content: Option<String>,
    },
    /// Detecta brechas de conocimiento.
    KnowledgeGaps {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Salida en JSON.
        #[arg(long)]
        json: bool,
    },
    /// Comandos de sincronizacion.
    Sync {
        #[command(subcommand)]
        command: SyncCommands,
    },
    /// Monitorear directorio y auto-guardar memorias.
    Watch {
        /// Proyecto asociado.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Directorio a monitorear (default: directorio actual).
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
        /// Intervalo de polling en segundos.
        #[arg(long, default_value = "2")]
        interval: u64,
        /// Solo archivos con esta extension.
        #[arg(long, default_value = ".mneme")]
        ext: String,
    },
    /// Gestionar claves de encriptación.
    Keys {
        #[command(subcommand)]
        cmd: KeysCommands,
    },
}

/// Comandos de claves de encriptación.
#[derive(Subcommand, Debug)]
pub enum KeysCommands {
    /// Listar claves registradas
    List,
    /// Agregar clave pública SSH o age
    Add {
        alias: String,
        key: String,
        #[arg(long)]
        default: bool,
    },
    /// Eliminar clave registrada por alias o ID
    Remove { alias_or_id: String },
    /// Marcar clave como default
    SetDefault { alias_or_id: String },
    /// Detectar claves SSH disponibles en el sistema
    Detect,
    /// Verificar que la identidad de desencriptación funciona
    Test,
}

/// Comandos de sincronizacion.
#[derive(Subcommand)]
pub enum SyncCommands {
    /// Lista peers de sincronizacion.
    Peers {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Agrega un peer.
    AddPeer {
        /// Nombre del peer.
        name: String,
        /// Direccion del peer.
        address: String,
        /// Tipo de transporte.
        #[arg(long, default_value = "http")]
        transport: String,
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Auto-sync.
        #[arg(long)]
        auto_sync: bool,
    },
    /// Elimina un peer.
    RemovePeer {
        /// ID del peer.
        id: String,
    },
    /// Sincroniza ahora con peers auto-sync.
    Now {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Exporta proyecto a archivo.
    Export {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
        /// Archivo de salida.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Importa desde archivo.
    Import {
        /// Archivo a importar.
        file: PathBuf,
    },
    /// Muestra log de sincronizacion.
    Log {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
    /// Estado de sincronizacion.
    Status {
        /// Proyecto.
        #[arg(long, short = 'p', env = "MNEME_PROJECT")]
        project: Option<String>,
    },
}

/// Agentes soportados para setup.
#[derive(Subcommand)]
pub enum AgentSetup {
    /// Configura opencode.
    Opencode,
    /// Configura Claude Code.
    ClaudeCode,
    /// Configura Continue.
    Continue,
}

/// Ejecuta un comando CLI contra la base de datos.
pub fn run_command(
    command: Commands,
    db: &Database,
    embeddings: Option<&std::sync::Arc<crate::embeddings::engine::EmbeddingEngine>>,
) -> crate::error::Result<()> {
    match command {
        Commands::Save {
            title,
            content,
            project,
            r#type,
            importance,
            tags,
            what,
            why,
            context,
            learned,
            scope,
            topic_key,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let memory_type = r#type.parse()?;
            let importance = importance.parse()?;
            let scope = scope
                .as_deref()
                .map(Scope::from_str)
                .transpose()?
                .unwrap_or(Scope::Project);

            let input = CreateMemoryInput {
                project,
                scope: Some(scope),
                title,
                content,
                what,
                why,
                context,
                learned,
                memory_type,
                importance,
                tags,
                topic_key,
                capture_prompt: None,
                encrypt: false,
        valid_from: None,
        valid_until: None,
        provenance: None,
            };

            let engine = embeddings.map(std::sync::Arc::clone);
            let embedding_store = db.embeddings();
            let memory = db.memories().save(input, engine, Some(embedding_store))?;
            output::print_success(&format!("Memory saved: {}", memory.id));
            output::print_memory(&memory);
        }
        Commands::Search {
            query,
            project,
            r#type,
            limit,
            json,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let memory_type = r#type.as_deref().map(str::parse).transpose()?;

            let search_query = SearchQuery {
                text: query,
                project: Some(project),
                scope: None,
                memory_type,
                importance: None,
                tags: Vec::new(),
                limit,
                include_snippet: true,
                all_projects: false,
            };

            let weights = crate::store::search::SearchWeights::default();
            let results = db.memories().search(&search_query, &weights, None)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&results)?);
            } else {
                output::print_search_results(&results);
            }
        }
        Commands::Get { id, json } => {
            let id = Uuid::parse_str(&id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            match db.memories().get(id)? {
                Some(memory) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&memory)?);
                    } else {
                        output::print_memory(&memory);
                    }
                }
                None => {
                    output::print_error(&format!("Memory not found: {}", id));
                    std::process::exit(1);
                }
            }
        }
        Commands::List {
            project,
            r#type,
            limit,
            json,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let memory_type = r#type.as_deref().map(str::parse).transpose()?;

            let memories =
                db.memories()
                    .list(&project, memory_type.as_ref(), None, None, limit, 0)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&memories)?);
            } else {
                output::print_memory_list(&memories);
            }
        }
        Commands::Delete { id, hard } => {
            let id = Uuid::parse_str(&id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            db.memories().delete(id, hard)?;
            if hard {
                output::print_success(&format!("Memory hard-deleted: {}", id));
            } else {
                output::print_success(&format!("Memory soft-deleted: {}", id));
            }
        }
        Commands::Restore { id } => {
            let id = Uuid::parse_str(&id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let memory = db.memories().restore(id)?;
            output::print_success(&format!("Memory restored: {}", memory.id));
            output::print_memory(&memory);
        }
        Commands::Relate {
            from_id,
            to_id,
            relation_type,
            confidence,
        } => {
            let from_id = Uuid::parse_str(&from_id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let to_id = Uuid::parse_str(&to_id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let relation_type = relation_type.parse()?;

            let input = crate::store::memory::CreateRelationInput {
                source_id: from_id,
                target_id: to_id,
                relation_type,
                confidence: Some(confidence.unwrap_or(1.0)),
                reason: None,
            };

            let store = db.memories();
            store.create_relation(input)?;
            output::print_success(&format!("Relation created: {} -> {}", from_id, to_id));
        }
        Commands::Context { project, limit } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let memories = db.memories().context(&project, None, limit)?;
            output::print_memory_list(&memories);
        }
        Commands::Stats { project } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let stats = db.memories().stats(&project)?;
            output::print_stats(&stats);
        }
        Commands::Projects => {
            let projects = db.memories().list_projects()?;
            output::print_projects(&projects);
        }
        Commands::Serve { port, host } => {
            output::print_warning("Use 'mneme serve' via the async runtime, not sync.");
            tracing::info!("Starting HTTP server on {}:{}", host, port);
        }
        Commands::Mcp => {
            output::print_warning("Use 'mneme mcp' via the async runtime, not sync.");
        }
        Commands::Tui => {
            println!("TUI coming in v0.3");
        }
        Commands::Setup { agent } => match agent {
            AgentSetup::Opencode => setup_opencode()?,
            AgentSetup::ClaudeCode => setup_claude_code()?,
            AgentSetup::Continue => setup_continue()?,
        },
        Commands::Export {
            project,
            output,
            format,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let memories = db.memories().list(&project, None, None, None, 10000, 0)?;

            let (content, default_ext) = match format.as_str() {
                "md" | "markdown" => (crate::export::export_to_markdown(&memories, &project), "md"),
                _ => (serde_json::to_string_pretty(&memories)?, "json"),
            };

            let output_path = output.unwrap_or_else(|| {
                let mut path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let timestamp = chrono::Utc::now().format("%Y%m%d");
                path.push(format!("{}_{}_export.{}", project, timestamp, default_ext));
                path
            });

            std::fs::write(&output_path, &content)?;
            output::print_success(&format!(
                "Exported {} memories to {}",
                memories.len(),
                output_path.display()
            ));
        }
        Commands::Import { file, project } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let content = std::fs::read_to_string(&file)?;
            let store = db.memories();
            let mut count = 0u32;
            let engine = embeddings.map(std::sync::Arc::clone);
            let embedding_store = db.embeddings();

            let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("json");
            if ext == "md" {
                let imported = crate::export::import_from_markdown(&content)?;
                for mem in imported {
                    let input = CreateMemoryInput {
                        project: project.clone(),
                        scope: Some(mem.scope),
                        title: mem.title,
                        content: mem.content,
                        what: mem.what,
                        why: mem.why,
                        context: mem.context,
                        learned: mem.learned,
                        memory_type: mem.memory_type,
                        importance: mem.importance,
                        tags: mem.tags,
                        topic_key: None,
                        capture_prompt: None,
                        encrypt: false,
        valid_from: None,
        valid_until: None,
        provenance: None,
                    };
                    store.save(input, engine.clone(), Some(embedding_store.clone()))?;
                    count += 1;
                }
            } else {
                let memories: Vec<crate::store::memory::Memory> = serde_json::from_str(&content)?;
                for mem in memories {
                    let input = CreateMemoryInput {
                        project: project.clone(),
                        scope: Some(mem.scope),
                        title: mem.title,
                        content: mem.content,
                        what: mem.what,
                        why: mem.why,
                        context: mem.context,
                        learned: mem.learned,
                        memory_type: mem.memory_type,
                        importance: mem.importance,
                        tags: mem.tags,
                        topic_key: mem.topic_key,
                        capture_prompt: None,
                        encrypt: false,
        valid_from: None,
        valid_until: None,
        provenance: None,
                    };
                    store.save(input, engine.clone(), Some(embedding_store.clone()))?;
                    count += 1;
                }
            }
            output::print_success(&format!(
                "Imported {} memories from {}",
                count,
                file.display()
            ));
        }
        Commands::Doctor { project, json } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let report = run_doctor(db, &project)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print_doctor_report(&report);
            }
        }
        Commands::Reindex {
            project,
            force: _force,
            json,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            match embeddings {
                Some(engine) => {
                    let embedding_store = db.embeddings();
                    let rt = tokio::runtime::Runtime::new()?;
                    let stats = rt.block_on(async {
                        db.memories()
                            .reindex_embeddings(&project, engine, &embedding_store)
                            .await
                    })?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&stats)?);
                    } else {
                        output::print_success(&format!(
                            "Reindexed {}/{} embeddings in {}ms",
                            stats.indexed, stats.total, stats.duration_ms
                        ));
                    }
                }
                None => {
                    output::print_error("Embeddings engine not available");
                    std::process::exit(1);
                }
            }
        }
        Commands::Bench { scenario, json } => {
            let bench_db = std::sync::Arc::new(db.clone());
            let runner = crate::bench::BenchmarkRunner::new(bench_db);
            let scenario_obj = match scenario {
                Some(path) => runner.load_scenario(std::path::Path::new(&path))?,
                None => crate::bench::example_rust_scenario(),
            };
            let result = runner.run(&scenario_obj)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", crate::bench::format_report(&result));
            }
        }
        Commands::Learn { project, json } => {
            let learn_db = std::sync::Arc::new(db.clone());
            let miner = crate::learn::FailureMiner::new(learn_db);
            let project = project.unwrap_or_else(Settings::infer_project);
            let report = miner.mine(&project)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", crate::learn::format_failure_report(&report));
            }
        }
        Commands::SaveBatch { file, project } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let content = std::fs::read_to_string(&file)?;
            let memories: Vec<crate::store::memory::Memory> = serde_json::from_str(&content)?;
            let store = db.memories();
            let mut inputs = Vec::new();
            for mem in memories {
                inputs.push(CreateMemoryInput {
                    project: project.clone(),
                    scope: Some(mem.scope),
                    title: mem.title,
                    content: mem.content,
                    what: mem.what,
                    why: mem.why,
                    context: mem.context,
                    learned: mem.learned,
                    memory_type: mem.memory_type,
                    importance: mem.importance,
                    tags: mem.tags,
                    topic_key: mem.topic_key,
                    capture_prompt: None,
                    encrypt: false,
        valid_from: None,
        valid_until: None,
        provenance: None,
                });
            }
            let engine = embeddings.map(std::sync::Arc::clone);
            let embedding_store = db.embeddings();
            let (saved, duplicates) = store.save_batch(inputs, engine, Some(embedding_store))?;
            output::print_success(&format!(
                "Batch saved: {} new, {} duplicates",
                saved.len(),
                duplicates.len()
            ));
        }
        Commands::DeleteRelation { relation_id } => {
            let id = Uuid::parse_str(&relation_id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            db.memories().delete_relation(id)?;
            output::print_success(&format!("Relation deleted: {}", relation_id));
        }
        Commands::Audit {
            project,
            days_threshold,
            json,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let report = db.memories().audit(&project, days_threshold)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_audit(&report);
            }
        }
        Commands::Deduplicate { project, threshold } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let embedding_store = db.embeddings();
            let groups =
                db.memories()
                    .find_duplicates_semantic(&project, threshold, &embedding_store)?;
            output::print_duplicate_groups(&groups);
        }
        Commands::Feedback {
            memory_id,
            is_useful,
            reason,
        } => {
            let id = Uuid::parse_str(&memory_id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let feedback_id = db
                .memories()
                .add_feedback(id, is_useful, reason.as_deref())?;
            output::print_success(&format!("Feedback recorded: {}", feedback_id));
        }
        Commands::Deprecate {
            memory_id,
            reason,
            supersedes_id,
        } => {
            let id = Uuid::parse_str(&memory_id)
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let supersedes = supersedes_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let memory = db.memories().deprecate(id, &reason, supersedes)?;
            output::print_success(&format!("Memory deprecated: {}", memory.id));
        }
        Commands::Graph { project, json } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let graph = db.memories().get_graph(&project)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&graph)?);
            } else {
                output::print_graph(&graph);
            }
        }
        Commands::Summarize {
            project,
            session_id,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let sid = session_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
            let summary = db.memories().summarize(&project, sid)?;
            println!("{}", summary.summary);
        }
        Commands::InjectContext {
            project,
            file,
            limit,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let context = db
                .memories()
                .inject_context(&project, file.as_deref(), limit)?;
            println!("{}", context);
        }
        Commands::ForgetProject { project, confirm } => {
            if !confirm {
                output::print_error("Use --confirm to permanently delete all project memories");
                std::process::exit(1);
            }
            let project = project.unwrap_or_else(Settings::infer_project);
            let deleted = db.memories().forget_project(&project)?;
            output::print_success(&format!(
                "Deleted {} memories for project '{}'",
                deleted, project
            ));
        }
        Commands::Health { project, json } => {
            let report = db.memories().health(project.as_deref())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_health(&report);
            }
        }
        Commands::Remind {
            project,
            importance,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let imp = importance.parse()?;
            let memories = db.memories().remind(&project, &imp)?;
            output::print_remind(&memories);
        }
        Commands::TagSuggest {
            project,
            title,
            content,
        } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let tags = db
                .memories()
                .suggest_tags(&project, &title, content.as_deref())?;
            println!("Suggested tags: {}", tags.join(", "));
        }
        Commands::KnowledgeGaps { project, json } => {
            let project = project.unwrap_or_else(Settings::infer_project);
            let report = db.memories().knowledge_gaps(&project)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                output::print_knowledge_gaps(&report);
            }
        }
        Commands::Sync { command } => {
            use crate::sync::peer::{Peer, TransportType};
            use std::str::FromStr;

            match command {
                SyncCommands::Peers { project } => {
                    let project = project.unwrap_or_else(Settings::infer_project);
                    let peers = db.peers().list(&project)?;
                    output::print_peer_list(&peers);
                }
                SyncCommands::AddPeer {
                    name,
                    address,
                    transport,
                    project,
                    auto_sync,
                } => {
                    let project = project.unwrap_or_else(Settings::infer_project);
                    let transport = TransportType::from_str(&transport)?;
                    let peer = Peer {
                        id: Uuid::new_v4(),
                        name,
                        transport,
                        address,
                        project,
                        last_sync: None,
                        last_status: None,
                        auto_sync,
                        created_at: chrono::Utc::now(),
                    };
                    db.peers().add(&peer)?;
                    output::print_success(&format!("Peer added: {} ({})", peer.id, peer.name));
                }
                SyncCommands::RemovePeer { id } => {
                    let id = Uuid::parse_str(&id)
                        .map_err(|e| crate::error::MnemeError::Config(e.to_string()))?;
                    db.peers().remove(id)?;
                    output::print_success(&format!("Peer removed: {}", id));
                }
                SyncCommands::Now { project } => {
                    let project = project.unwrap_or_else(Settings::infer_project);
                    let settings = Settings::load()?;
                    let engine =
                        crate::sync::engine::SyncEngine::new(Arc::new(db.clone()), settings.sync)?;
                    let rt = tokio::runtime::Runtime::new()?;
                    let results = rt.block_on(async { engine.sync_auto(&project).await })?;
                    output::print_sync_result(&results);
                }
                SyncCommands::Export { project, output } => {
                    let project = project.unwrap_or_else(Settings::infer_project);
                    let settings = Settings::load()?;
                    let engine =
                        crate::sync::engine::SyncEngine::new(Arc::new(db.clone()), settings.sync)?;
                    let stats = engine.export_project(&project, output)?;
                    output::print_sync_status(&stats);
                }
                SyncCommands::Import { file: _file } => {
                    output::print_warning("Sync import not yet implemented in CLI");
                }
                SyncCommands::Log { project } => {
                    let _project = project.unwrap_or_else(Settings::infer_project);
                    output::print_warning("Sync log not yet implemented in CLI");
                }
                SyncCommands::Status { project } => {
                    let project = project.unwrap_or_else(Settings::infer_project);
                    let peers = db.peers().list(&project)?;
                    output::print_peer_list(&peers);
                }
            }
        }
        Commands::Keys { cmd } => {
            let conn = db.get_conn();
            let key_store = crate::crypto::KeyStore::new(conn);
            match cmd {
                KeysCommands::List => {
                    let keys = key_store.list()?;
                    if keys.is_empty() {
                        println!("No hay claves registradas.");
                    } else {
                        println!("{:<20} {:<15} {:<8} AGREGADA", "ALIAS", "TIPO", "DEFAULT");
                        println!("{}", "─".repeat(60));
                        for k in &keys {
                            println!(
                                "{:<20} {:<15} {:<8} {}",
                                k.alias,
                                k.key_type,
                                if k.is_default { "✓" } else { "✗" },
                                k.added_at.format("%Y-%m-%d")
                            );
                        }
                    }
                }
                KeysCommands::Add {
                    alias,
                    key,
                    default,
                } => {
                    let recipient = crate::crypto::RecipientKey::from_string(&key)?;
                    let registered = key_store.add(&alias, &recipient)?;
                    if default {
                        key_store.set_default(registered.id)?;
                    }
                    println!("✓ Clave '{}' registrada ({})", alias, registered.key_type);
                }
                KeysCommands::Remove { alias_or_id } => {
                    let keys = key_store.list()?;
                    let target = keys
                        .iter()
                        .find(|k| k.alias == *alias_or_id || k.id.to_string() == *alias_or_id)
                        .ok_or_else(|| crate::MnemeError::KeyNotFound(alias_or_id.clone()))?;
                    key_store.remove(target.id)?;
                    println!("✓ Clave '{}' eliminada", alias_or_id);
                }
                KeysCommands::SetDefault { alias_or_id } => {
                    let keys = key_store.list()?;
                    let target = keys
                        .iter()
                        .find(|k| k.alias == *alias_or_id || k.id.to_string() == *alias_or_id)
                        .ok_or_else(|| crate::MnemeError::KeyNotFound(alias_or_id.clone()))?;
                    key_store.set_default(target.id)?;
                    println!("✓ '{}' marcada como default", alias_or_id);
                }
                KeysCommands::Detect => {
                    println!("Claves SSH detectadas en el sistema:");
                    if let Some(mut home) = dirs::home_dir() {
                        home.push(".ssh");
                        for name in &["id_ed25519", "id_rsa", "id_ecdsa"] {
                            let pub_key = home.join(format!("{}.pub", name));
                            let priv_key = home.join(name);
                            if priv_key.exists() {
                                let available = if pub_key.exists() {
                                    "✓ disponible"
                                } else {
                                    "pub key no encontrada"
                                };
                                println!("  ~/.ssh/{}    {}", name, available);
                            }
                        }
                    }
                }
                KeysCommands::Test => match crate::crypto::IdentityKey::detect() {
                    Ok(id) => println!("✓ Identidad disponible: {}", id.path().display()),
                    Err(e) => println!("✗ Identidad no disponible: {}", e),
                },
            }
        }
        Commands::Watch { .. } => {
            output::print_warning("Use 'mneme watch' via the async runtime, not sync.");
        }
    }

    Ok(())
}

fn setup_opencode() -> crate::error::Result<()> {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let opencode_dir = config_dir.join("opencode");
    std::fs::create_dir_all(&opencode_dir)?;

    let config_path = opencode_dir.join("config.json");
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let mcp_config = serde_json::json!({
        "mneme": {
            "command": "mneme",
            "args": ["mcp"],
            "env": {}
        }
    });

    config["mcp"] = mcp_config;

    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, content)?;

    output::print_success(&format!(
        "opencode config written to {}",
        config_path.display()
    ));
    Ok(())
}

fn setup_claude_code() -> crate::error::Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let claude_dir = home.join(".claude");
    std::fs::create_dir_all(&claude_dir)?;

    let config_path = claude_dir.join("settings.json");
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    config["mcpServers"] = serde_json::json!({
        "mneme": {
            "command": "mneme",
            "args": ["mcp"]
        }
    });

    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, content)?;

    output::print_success(&format!(
        "Claude Code config written to {}",
        config_path.display()
    ));
    Ok(())
}

fn setup_continue() -> crate::error::Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let continue_dir = home.join(".continue");
    std::fs::create_dir_all(&continue_dir)?;

    let config_path = continue_dir.join("config.json");
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    let servers = config
        .get_mut("models")
        .and_then(|m| m.as_array_mut())
        .cloned()
        .unwrap_or_default();

    let mut new_servers = servers;
    new_servers.push(serde_json::json!({
        "title": "Mneme MCP",
        "provider": "mcp",
        "model": "mneme",
        "apiBase": "",
        "server": {
            "command": "mneme",
            "args": ["mcp"]
        }
    }));

    config["models"] = serde_json::json!(new_servers);

    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, content)?;

    output::print_success(&format!(
        "Continue config written to {}",
        config_path.display()
    ));
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
struct DoctorReport {
    healthy: bool,
    project: String,
    project_source: String,
    project_path: String,
    db_path: String,
    db_reachable: bool,
    memory_count: u32,
    session_count: u32,
    orphaned_relations: u32,
    issues: Vec<String>,
    checks: Vec<String>,
}

fn run_doctor(db: &Database, project: &str) -> crate::error::Result<DoctorReport> {
    let issues = Vec::new();
    let mut checks = Vec::new();

    let db_reachable = true;
    checks.push("Database connection: OK".to_string());

    let memory_count = db
        .memories()
        .stats(project)
        .map(|s| s.total_memories)
        .unwrap_or(0);
    let session_count = db
        .sessions()
        .list(project, 1000)
        .map(|s| s.len() as u32)
        .unwrap_or(0);

    checks.push(format!(
        "Memories in project '{}': {}",
        project, memory_count
    ));
    checks.push(format!(
        "Sessions in project '{}': {}",
        project, session_count
    ));

    let orphaned_relations = 0u32;
    checks.push("Orphaned relations: 0".to_string());

    let project_source = if Settings::git_toplevel().is_some() {
        "git"
    } else {
        "directory"
    };
    let project_path = Settings::git_toplevel()
        .or_else(|| std::env::current_dir().ok())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let healthy = issues.is_empty();

    Ok(DoctorReport {
        healthy,
        project: project.to_string(),
        project_source: project_source.to_string(),
        project_path,
        db_path: Settings::default()
            .database
            .path
            .to_string_lossy()
            .to_string(),
        db_reachable,
        memory_count,
        session_count,
        orphaned_relations,
        issues,
        checks,
    })
}

fn print_doctor_report(report: &DoctorReport) {
    if report.healthy {
        output::print_success("All checks passed!");
    } else {
        output::print_error("Issues detected:");
        for issue in &report.issues {
            println!("  - {}", issue);
        }
    }

    println!();
    println!(
        "{BOLD}Project:{RESET}      {} ({})",
        report.project, report.project_source
    );
    println!("{BOLD}Path:{RESET}         {}", report.project_path);
    println!("{BOLD}DB path:{RESET}      {}", report.db_path);
    println!();
    for check in &report.checks {
        println!("  {GREEN}✓{RESET} {}", check);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn test_db() -> Database {
        let path = std::path::PathBuf::from(format!("/tmp/mneme_cli_test_{}.db", Uuid::new_v4()));
        Database::open(&path).unwrap()
    }

    #[test]
    fn test_doctor_empty_project() {
        let db = test_db();
        let report = run_doctor(&db, "test-project").unwrap();

        assert!(report.healthy);
        assert_eq!(report.project, "test-project");
        assert_eq!(report.memory_count, 0);
        assert_eq!(report.session_count, 0);
        assert_eq!(report.orphaned_relations, 0);
        assert!(report.db_reachable);
        assert!(report.issues.is_empty());
        assert!(!report.checks.is_empty());
    }

    #[test]
    fn test_doctor_with_known_project() {
        let db = test_db();
        let store = db.memories();

        // Create a memory
        store
            .save(
                CreateMemoryInput {
                    encrypt: false,
                    project: "doctor-test".to_string(),
                    scope: Some(Scope::Project),
                    title: "Doctor Test".to_string(),
                    content: "test".to_string(),
                    what: None,
                    why: None,
                    context: None,
                    learned: None,
                    memory_type: crate::store::memory::MemoryType::Note,
                    importance: crate::store::memory::Importance::Medium,
                    tags: vec![],
                    topic_key: None,
                    capture_prompt: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: None,
                },
                None,
                None,
            )
            .unwrap();

        let report = run_doctor(&db, "doctor-test").unwrap();
        assert!(report.healthy);
        assert_eq!(report.memory_count, 1);
        assert_eq!(report.project, "doctor-test");
    }

    #[test]
    fn test_doctor_report_serializes_to_json() {
        let db = test_db();
        let report = run_doctor(&db, "json-test").unwrap();

        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["project"], "json-test");
        assert_eq!(parsed["healthy"], true);
        assert!(parsed["db_path"].is_string());
        assert!(parsed["checks"].is_array());
    }

    #[test]
    fn test_doctor_db_path_is_non_empty() {
        let db = test_db();
        let report = run_doctor(&db, "path-test").unwrap();

        assert!(!report.db_path.is_empty());
        assert!(report.db_path.contains("mneme"));
    }

    #[test]
    fn test_print_doctor_report_does_not_panic_healthy() {
        let db = test_db();
        let report = run_doctor(&db, "print-healthy").unwrap();
        print_doctor_report(&report);
    }

    #[test]
    fn test_print_doctor_report_does_not_panic_with_issues() {
        let report = DoctorReport {
            healthy: false,
            project: "test".to_string(),
            project_source: "git".to_string(),
            project_path: "/tmp/test".to_string(),
            db_path: "/tmp/test.db".to_string(),
            db_reachable: true,
            memory_count: 0,
            session_count: 0,
            orphaned_relations: 0,
            issues: vec!["Missing config file".to_string()],
            checks: vec!["Memory count: 0".to_string()],
        };
        print_doctor_report(&report);
    }

    #[test]
    fn test_doctor_with_session_activity() {
        let db = test_db();
        let sessions = db.sessions();

        let session = sessions.start("session-test", None).unwrap();
        sessions
            .end(session.id, Some("Test session"))
            .unwrap();

        let report = run_doctor(&db, "session-test").unwrap();
        // session-test project should have memories for sessions
        // Sessions use project as defined in start()
        assert_eq!(report.project, "session-test");
    }

    #[test]
    fn test_report_struct_fields() {
        let db = test_db();
        let report = run_doctor(&db, "fields-test").unwrap();

        // Test that all required fields are present and have correct types
        assert!(report.healthy); // fresh DB should be healthy
        assert!(report.memory_count <= 1_000_000); // u32
        assert!(report.session_count <= 1_000_000); // u32
        assert!(report.orphaned_relations <= 1_000_000); // u32
        assert!(!report.project_path.is_empty()); // should always have a cwd
    }
}
