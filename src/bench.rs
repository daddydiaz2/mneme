//! Memory benchmarks para evaluar la calidad del retrieval.
//!
//! Inspirado por Mem0's LoCoMo / LongMemEval / BEAM. Permite definir
//! escenarios de prueba (memorias a cargar + preguntas con expected answers)
//! y medir métricas como precision@k, recall@k, MRR, y faithfulness.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::store::db::Database;
use crate::store::memory::{CreateMemoryInput, Scope, SearchQuery};

/// Un escenario de benchmark: memorias semilla + preguntas con respuestas esperadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkScenario {
    /// Nombre del escenario (ej: "rust-decisions", "auth-patterns").
    pub name: String,
    /// Descripción opcional.
    pub description: Option<String>,
    /// Proyecto al que se cargan las memorias de prueba.
    pub project: String,
    /// Memorias semilla a cargar antes de evaluar.
    #[serde(default)]
    pub seed_memories: Vec<BenchmarkSeedMemory>,
    /// Preguntas de evaluación.
    pub queries: Vec<BenchmarkQuery>,
}

/// Una memoria semilla para el benchmark.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSeedMemory {
    pub title: String,
    pub content: String,
    pub memory_type: String,
    pub importance: String,
    pub tags: Vec<String>,
}

/// Una pregunta con respuestas esperadas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuery {
    /// El query de búsqueda.
    pub query: String,
    /// IDs de memorias que se esperan encontrar (1-indexed por orden en seed).
    /// Permite expected_memory_id o expected_keywords.
    #[serde(default)]
    pub expected_titles: Vec<String>,
    /// Keywords que deberían aparecer en al menos un resultado top-k.
    #[serde(default)]
    pub expected_keywords: Vec<String>,
    /// Posición esperada (1-indexed) en los resultados. 0 = cualquier posición.
    #[serde(default)]
    pub expected_rank: u32,
    /// Profundidad k para métricas (default 5).
    #[serde(default = "default_k")]
    pub k: u32,
}

fn default_k() -> u32 { 5 }

/// Resultados de un escenario ejecutado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub scenario_name: String,
    pub total_queries: u32,
    pub metrics: BenchmarkMetrics,
    pub per_query: Vec<QueryResult>,
}

/// Métricas agregadas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    /// Mean Reciprocal Rank @ k
    pub mrr: f64,
    /// Precision @ k: promedio de (relevantos en top-k) / k
    pub precision_at_k: f64,
    /// Recall @ k: promedio de (relevantos en top-k) / total relevante
    pub recall_at_k: f64,
    /// Hit rate @ k: promedio de queries con al menos 1 relevante en top-k
    pub hit_rate: f64,
    /// F1 @ k: promedio de F1 scores
    pub f1_at_k: f64,
    /// Latencia promedio de búsqueda (ms)
    pub avg_latency_ms: f64,
}

/// Resultado individual de un query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query: String,
    pub k: u32,
    pub relevant_found: u32,
    pub total_relevant: u32,
    pub reciprocal_rank: f64,
    pub precision: f64,
    pub recall: f64,
    pub hit: bool,
    pub latency_ms: u64,
    pub top_titles: Vec<String>,
}

/// Runner de benchmarks.
pub struct BenchmarkRunner {
    db: std::sync::Arc<Database>,
}

impl BenchmarkRunner {
    pub fn new(db: std::sync::Arc<Database>) -> Self {
        Self { db }
    }

    /// Carga un escenario desde un archivo TOML o JSON.
    pub fn load_scenario(&self, path: &Path) -> crate::error::Result<BenchmarkScenario> {
        let content = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let scenario = if ext == "json" {
            serde_json::from_str(&content)?
        } else {
            // Default to TOML
            toml::from_str(&content)
                .map_err(|e| crate::error::MnemeError::Config(format!("TOML parse error: {}", e)))?
        };
        Ok(scenario)
    }

    /// Ejecuta un escenario: carga memorias, corre queries, mide métricas.
    pub fn run(&self, scenario: &BenchmarkScenario) -> crate::error::Result<BenchmarkResult> {
        // 1. Cargar memorias semilla
        let memories = self.db.memories();
        let project = scenario.project.clone();

        // Limpiar memorias existentes del proyecto (para reproducibilidad)
        let _ = memories.forget_project(&project);

        for seed in &scenario.seed_memories {
            let memory_type = seed.memory_type.parse().unwrap_or(
                crate::store::memory::MemoryType::Note,
            );
            let importance = seed.importance.parse().unwrap_or(
                crate::store::memory::Importance::Medium,
            );
            let input = CreateMemoryInput {
                project: project.clone(),
                scope: Some(Scope::Project),
                title: seed.title.clone(),
                content: seed.content.clone(),
                what: None,
                why: None,
                context: None,
                learned: None,
                memory_type,
                importance,
                tags: seed.tags.clone(),
                topic_key: None,
                capture_prompt: None,
                encrypt: false,
                valid_from: None,
                valid_until: None,
                provenance: Some("benchmark/seed".to_string()),
            };
            memories.save(input, None, None)?;
        }

        // 2. Ejecutar cada query
        let mut per_query = Vec::new();
        let mut sum_mrr = 0.0;
        let mut sum_precision = 0.0;
        let mut sum_recall = 0.0;
        let mut sum_hit = 0u32;
        let mut sum_f1 = 0.0;
        let mut sum_latency = 0u64;

        for q in &scenario.queries {
            let k = if q.k == 0 { 5 } else { q.k };
            let start = std::time::Instant::now();
            let search_query = SearchQuery {
                text: q.query.clone(),
                project: Some(project.clone()),
                scope: Some(Scope::Project),
                memory_type: None,
                importance: None,
                tags: Vec::new(),
                limit: k,
                include_snippet: false,
                all_projects: false,
            };
            let weights = crate::store::search::SearchWeights::default();
            let results = memories.search(&search_query, &weights, None)?;
            let latency_ms = start.elapsed().as_millis() as u64;

            let top_titles: Vec<String> = results.iter().take(k as usize).map(|r| r.memory.title.clone()).collect();

            // Calcular relevance
            let mut relevant_found = 0u32;
            let mut reciprocal_rank = 0.0;
            for (idx, r) in results.iter().take(k as usize).enumerate() {
                let title = &r.memory.title;
                let is_relevant = q.expected_titles.iter().any(|t| t == title) ||
                    q.expected_keywords.iter().any(|kw| {
                        r.memory.content.to_lowercase().contains(&kw.to_lowercase()) ||
                        r.memory.title.to_lowercase().contains(&kw.to_lowercase())
                    });
                if is_relevant {
                    relevant_found += 1;
                    if reciprocal_rank == 0.0 {
                        reciprocal_rank = 1.0 / (idx as f64 + 1.0);
                    }
                }
            }

            let total_relevant = if q.expected_titles.is_empty() && q.expected_keywords.is_empty() {
                0
            } else {
                q.expected_titles.len().max(q.expected_keywords.len()) as u32
            };

            let precision = if k > 0 { relevant_found as f64 / k as f64 } else { 0.0 };
            let recall = if total_relevant > 0 {
                relevant_found as f64 / total_relevant as f64
            } else if relevant_found > 0 {
                1.0
            } else {
                0.0
            };
            let f1 = if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            };
            let hit = relevant_found > 0;

            sum_mrr += reciprocal_rank;
            sum_precision += precision;
            sum_recall += recall;
            if hit { sum_hit += 1; }
            sum_f1 += f1;
            sum_latency += latency_ms;

            per_query.push(QueryResult {
                query: q.query.clone(),
                k,
                relevant_found,
                total_relevant,
                reciprocal_rank,
                precision,
                recall,
                hit,
                latency_ms,
                top_titles,
            });
        }

        let total = scenario.queries.len() as f64;
        let metrics = if total > 0.0 {
            BenchmarkMetrics {
                mrr: sum_mrr / total,
                precision_at_k: sum_precision / total,
                recall_at_k: sum_recall / total,
                hit_rate: sum_hit as f64 / total,
                f1_at_k: sum_f1 / total,
                avg_latency_ms: if total > 0.0 { sum_latency as f64 / total } else { 0.0 },
            }
        } else {
            BenchmarkMetrics::default()
        };

        Ok(BenchmarkResult {
            scenario_name: scenario.name.clone(),
            total_queries: scenario.queries.len() as u32,
            metrics,
            per_query,
        })
    }
}

/// Genera un escenario de benchmark de ejemplo para Rust.
pub fn example_rust_scenario() -> BenchmarkScenario {
    
    BenchmarkScenario {
        name: "rust-decisions".to_string(),
        description: Some("Evalúa retrieval de decisiones arquitectónicas sobre Rust".to_string()),
        project: "bench-rust".to_string(),
        seed_memories: vec![
            BenchmarkSeedMemory {
                title: "Async runtime: tokio vs async-std".to_string(),
                content: "Decidimos usar tokio como runtime async por mejor ecosystem, tracing integrado, y amplia adoption en la comunidad Rust.".to_string(),
                memory_type: "decision".to_string(),
                importance: "high".to_string(),
                tags: vec!["rust".to_string(), "async".to_string(), "architecture".to_string()],
            },
            BenchmarkSeedMemory {
                title: "Error handling: anyhow vs thiserror".to_string(),
                content: "Para errores de library usamos thiserror (typed errors), para binarios usamos anyhow (con context).".to_string(),
                memory_type: "decision".to_string(),
                importance: "medium".to_string(),
                tags: vec!["rust".to_string(), "errors".to_string()],
            },
            BenchmarkSeedMemory {
                title: "Web framework: axum vs actix-web".to_string(),
                content: "Elegimos axum por mejor integración con tower ecosystem y menor curva de aprendizaje.".to_string(),
                memory_type: "decision".to_string(),
                importance: "high".to_string(),
                tags: vec!["rust".to_string(), "web".to_string(), "architecture".to_string()],
            },
            BenchmarkSeedMemory {
                title: "Database: rusqlite con FTS5".to_string(),
                content: "Usamos rusqlite con FTS5 para full-text search nativo sin dependencias externas. Embeddings via fastembed ONNX.".to_string(),
                memory_type: "decision".to_string(),
                importance: "high".to_string(),
                tags: vec!["rust".to_string(), "database".to_string(), "sqlite".to_string()],
            },
            BenchmarkSeedMemory {
                title: "CLI: clap con derive".to_string(),
                content: "Usamos clap v4 con derive macros para type-safe argument parsing.".to_string(),
                memory_type: "convention".to_string(),
                importance: "medium".to_string(),
                tags: vec!["rust".to_string(), "cli".to_string()],
            },
        ],
        queries: vec![
            BenchmarkQuery {
                query: "qué runtime async usamos".to_string(),
                expected_titles: vec!["Async runtime: tokio vs async-std".to_string()],
                expected_keywords: vec!["tokio".to_string()],
                expected_rank: 1,
                k: 3,
            },
            BenchmarkQuery {
                query: "cómo manejamos errores".to_string(),
                expected_titles: vec!["Error handling: anyhow vs thiserror".to_string()],
                expected_keywords: vec!["anyhow".to_string(), "thiserror".to_string()],
                expected_rank: 1,
                k: 3,
            },
            BenchmarkQuery {
                query: "framework web".to_string(),
                expected_titles: vec!["Web framework: axum vs actix-web".to_string()],
                expected_keywords: vec!["axum".to_string()],
                expected_rank: 1,
                k: 3,
            },
            BenchmarkQuery {
                query: "base de datos sqlite fulltext".to_string(),
                expected_titles: vec!["Database: rusqlite con FTS5".to_string()],
                expected_keywords: vec!["rusqlite".to_string(), "fts5".to_string()],
                expected_rank: 1,
                k: 3,
            },
            BenchmarkQuery {
                query: "cli argument parsing".to_string(),
                expected_titles: vec!["CLI: clap con derive".to_string()],
                expected_keywords: vec!["clap".to_string()],
                expected_rank: 1,
                k: 3,
            },
        ],
    }
}

/// Reporta los resultados como tabla markdown.
pub fn format_report(result: &BenchmarkResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Benchmark: {}\n\n", result.scenario_name));
    out.push_str(&format!("Queries ejecutados: {}\n\n", result.total_queries));
    out.push_str("## Métricas agregadas\n\n");
    out.push_str("| Métrica | Valor |\n|---------|-------|\n");
    out.push_str(&format!("| MRR @ k | {:.4} |\n", result.metrics.mrr));
    out.push_str(&format!("| Precision @ k | {:.4} |\n", result.metrics.precision_at_k));
    out.push_str(&format!("| Recall @ k | {:.4} |\n", result.metrics.recall_at_k));
    out.push_str(&format!("| Hit Rate @ k | {:.4} |\n", result.metrics.hit_rate));
    out.push_str(&format!("| F1 @ k | {:.4} |\n", result.metrics.f1_at_k));
    out.push_str(&format!("| Avg latency (ms) | {:.2} |\n", result.metrics.avg_latency_ms));
    out.push_str("\n## Per-query\n\n");
    out.push_str("| Query | k | Rel/Total | P | R | F1 | MRR | Hit | Latency |\n");
    out.push_str("|-------|---|-----------|---|---|---|-----|-----|---------|\n");
    for q in &result.per_query {
        let f1 = if q.precision + q.recall > 0.0 {
            2.0 * q.precision * q.recall / (q.precision + q.recall)
        } else {
            0.0
        };
        out.push_str(&format!(
            "| {} | {} | {}/{} | {:.2} | {:.2} | {:.2} | {:.2} | {} | {}ms |\n",
            truncate(&q.query, 30),
            q.k,
            q.relevant_found,
            q.total_relevant,
            q.precision,
            q.recall,
            f1,
            q.reciprocal_rank,
            if q.hit { "✓" } else { "✗" },
            q.latency_ms,
        ));
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_scenario_loads() {
        let scenario = example_rust_scenario();
        assert_eq!(scenario.name, "rust-decisions");
        assert_eq!(scenario.seed_memories.len(), 5);
        assert_eq!(scenario.queries.len(), 5);
    }

    #[test]
    fn test_metrics_default() {
        let m = BenchmarkMetrics::default();
        assert_eq!(m.mrr, 0.0);
        assert_eq!(m.precision_at_k, 0.0);
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("a long string here", 10), "a long st…");
    }

    #[test]
    fn test_format_report_includes_metrics() {
        let result = BenchmarkResult {
            scenario_name: "test".to_string(),
            total_queries: 2,
            metrics: BenchmarkMetrics { mrr: 0.5, precision_at_k: 0.6, recall_at_k: 0.7, hit_rate: 0.5, f1_at_k: 0.65, avg_latency_ms: 5.0 },
            per_query: vec![],
        };
        let report = format_report(&result);
        assert!(report.contains("# Benchmark: test"));
        assert!(report.contains("MRR @ k | 0.5000"));
        assert!(report.contains("Precision @ k | 0.6000"));
    }
}
