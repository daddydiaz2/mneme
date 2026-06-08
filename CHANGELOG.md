# Changelog

All notable changes to mneme are documented here.

## 0.2.0 (2026-06-07)

### 🚀 Features
- **Entity extraction**: URLs, file paths, technologies, CamelCase → auto-linking cross-memory
- **LLM-judge conflict detection**: `mem_judge` + `mem_compare` con auto-invalidation
- **Passive capture**: parsea output de sesiones → extrae Key Learnings, Decisiones
- **Multi-signal RRF search**: FTS5 + fuzzy + semantic + entities + recency (RRF k=60)
- **Progressive 3-layer retrieval**: search → expand → transcript
- **Cross-encoder reranking**: `search_reranked()` con refinamiento semántico
- **Temporal validity**: `valid_from`/`valid_until`, `mem_temporal_query`
- **Context compression**: 4 estrategias (truncate, smart_summary, keywords_only, minimal)
- **AgentFact type**: memorias auto-generadas por el agente + provenance tracking
- **Multi-provider embeddings**: ONNX / OpenAI / Ollama / Google
- **File watcher**: `mem_watch_scan` con content hash + auto-indexing de .md
- **Obsidian vault export**: frontmatter + wikilinks + graph.json
- **Cloud sync**: enrollment + autosync + Docker compose + HTTP endpoints
- **Memory benchmarks**: LoCoMo/LongMemEval style (MRR, Precision, Recall, F1)
- **Failure mining**: Headroom learn style — `mem_learn_failures`, patterns + corrective memories
- **Memory consolidation**: compacta stale memories en summary auto-generado
- **Memory blocks**: Letta-inspired `human`/`persona`/`workflow` slots
- **CLI completion**: `mneme completion bash|zsh|fish|powershell`
- **Backup/Restore**: `mneme db-export` / `mneme db-import`
- **Graceful shutdown**: SIGTERM/SIGINT handling for HTTP server

### 🎨 UI/UX
- **TUI rewrite**: diseño profesional con paleta GitHub Dark
  - Header con project + memory count pill
  - 5 tabs en detail panel: Content, Fields, Entities, Temporal, Graph
  - Shift+J/K scroll, `[`/`]` tabs, help overlay completo
  - Entity graph view + temporal view
- **Web dashboard**: filtros (type, importance, tags), search, 3 tabs, paginación
  - Favicon SVG, dark theme, skeleton loading

### 🧪 Testing
- **71 nuevos tests**: entities (17), compress (15), obsidian (9), cloud (9),
  rerank (9), watcher (12), bench (4), learn (7), consolidate (7), TUI (15)
- **~330 tests total**

### 🐛 Fixes
- Backward compat: `EmbeddingsConfig.provider` con `#[serde(default)]`
- Deadlock fix: `generate_corrective_memory` ya no mantiene lock cruzado
- Dashboard: ruta `/api/v1/sync/status` → `/api/v1/cloud/status`
- Compilation errors: clap_complete dependency added
- Memory blocks: column offset bug (content vs updated_at)

---

## 0.1.0 (2026-05-XX)

### 🚀 Features
- SQLite + FTS5 full-text search
- 11 memory types, 4 importance levels, 3 scopes
- Hybrid search: FTS5 (50%) + fuzzy (20%) + semantic (30%)
- CRDT P2P sync via Automerge (HTTP + file transport)
- Age encryption with SSH key support
- WASM plugins via Extism (feature flag)
- MCP server with 40 tools
- HTTP API (30+ endpoints)
- CLI (25+ subcommands)
- TUI with graph visualization
- Watch mode (monitor directory)
- Docker image
