# mneme - Sistema de memoria persistente para agentes de IA

<div align="center">

![mneme banner](assets/imagen.png)

![Rust](https://img.shields.io/badge/Rust-2021-CE422B?logo=rust&logoColor=white)
![SQLite](https://img.shields.io/badge/SQLite-FTS5-003B57?logo=sqlite&logoColor=white)
![MCP](https://img.shields.io/badge/MCP-40_tools-6C3483?logo=anthropic&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-green)

**Sistema de memoria persistente para agentes de IA** — búsqueda híbrida (FTS5 + fuzzy + embeddings ONNX), encriptación age/SSH, sync CRDT P2P, TUI interactiva con grafo visual, plugins WASM, MCP server y HTTP API.

</div>

---

## Tabla de Contenidos

- [Caracteristicas](#caracteristicas)
- [Arquitectura](#arquitectura)
- [Stack Tecnologico](#stack-tecnologico)
- [Modelo de Datos](#modelo-de-datos)
- [Busqueda Hibrida](#busqueda-hibrida)
- [Encriptacion](#encriptacion)
- [Sync CRDT](#sync-crdt)
- [Plugins WASM](#plugins-wasm)
- [Requisitos](#requisitos)
- [Instalacion](#instalacion)
- [Uso Rapido](#uso-rapido)
- [MCP Tools](#mcp-tools)
- [HTTP API](#http-api)
- [TUI](#tui)
- [Watch Mode](#watch-mode)
- [Estructura del Proyecto](#estructura-del-proyecto)
- [Roadmap](#roadmap)
- [Licencia](#licencia)

---

## Caracteristicas

### Almacenamiento y Busqueda
- **SQLite + FTS5** con WAL mode, búsqueda full-text sobre todos los campos
- **Búsqueda híbrida**: FTS5 (50%) + fuzzy matching (20%) + semántica ONNX (30%)
- **Soft delete** — nunca se pierde contexto, `deleted_at` en lugar de DELETE
- **Deduplicación automática** — hash normalizado con ventana de 24h, detección semántica de duplicados
- **Topic keys** — upserts evolutivos por `project + scope + topic_key`
- **Scopes**: `project` / `personal` / `global`

### Embeddings (feature opcional)
- **Motor ONNX local** con `fastembed` v4 — sin costos de API, 100% offline
- **Modelo**: BAAI/bge-small-en-v1.5 (384 dimensiones)
- **Reindexación** incremental con `mneme reindex`
- **Binario sin embeddings**: 13 MB · **Con embeddings**: 37 MB

### Introspección y Calidad
- `mem_audit` — memorias obsoletas, incompletas, deprecadas
- `mem_deduplicate` — detección de duplicados semánticos
- `mem_graph` — grafo de conocimiento con nodos y aristas tipadas
- `mem_inject_context` — bloque Markdown listo para inyectar en system prompts
- `mem_health` — estado del sistema, tamaño DB, embeddings no indexados
- `mem_remind` — recordatorios de memorias críticas no accedidas en >7 días
- `mem_knowledge_gaps` — áreas del proyecto sin cobertura de memorias

### Encriptacion
- **age v0.10** con soporte SSH — sin setup, usa `~/.ssh/id_ed25519` existente
- **Granular por memoria** — cada memoria decide si está encriptada; título y tags quedan en claro para búsqueda
- **Multi-recipient** — un ciphertext, múltiples destinatarios
- **FTS5 encryption-aware** — triggers que excluyen campos cifrados del índice
- Sync de ciphertext entre peers — seguro por diseño

### Sync CRDT P2P
- **automerge 0.5** — convergencia garantizada sin servidor central
- **Transportes**: HTTP (peer-to-peer) + File (export/import `.mneme`)
- **Compresión zstd** en tránsito
- Sync bidireccional: `pull` + `push`

### Plugins WASM
- **extism 1.30** — plugins sandboxeados en WebAssembly, pure-Rust
- **Feature flag** `plugins` — off by default, zero overhead sin el flag
- **ABI JSON**: `plugin_manifest` · `call_tool` · `transform_memory`
- **Hooks**: `pre_save` / `post_get` encadenados entre plugins
- **Discovery**: `~/.config/mneme/plugins/*.wasm` al startup

### Interfaz
- **MCP server** con 40 herramientas — compatible con Claude Code, OpenCode, Continue
- **HTTP API REST** — 30+ endpoints, compatible con cualquier cliente
- **CLI** — 25+ subcomandos
- **TUI ratatui** — lista + detalle + **grafo visual interactivo** de relaciones, búsqueda inline, indicadores `🔒`
- **Watch mode** — monitorea directorio, auto-guarda archivos `.mneme`

---

## Arquitectura

```mermaid
flowchart TB
    subgraph Agents["Agentes IA"]
        Claude["Claude Code"]
        OpenCode["OpenCode"]
        Continue["Continue"]
        HTTP_Client["HTTP Client"]
    end

    subgraph Mneme["mneme (proceso local)"]
        direction TB
        MCP["MCP Server\n(stdio)"]
        HTTP["HTTP API\n(:8080)"]
        CLI["CLI\n(mneme <cmd>)"]
        TUI["TUI\n(mneme tui)"]
        Watch["Watch\n(mneme watch)"]

        Core["Core\nMemoryStore + SessionStore"]
        Search["SearchEngine\nFTS5 + Fuzzy + Semantic"]
        Crypto["CryptoEngine\nage / SSH"]
        Embed["EmbeddingEngine\n(feature flag)\nfastembed ONNX"]
        Sync["SyncEngine\nautomerge CRDT"]
        Plugins["PluginManager\n(feature flag)\nextism WASM"]
    end

    subgraph Storage["Almacenamiento"]
        SQLite["SQLite\nWAL + FTS5\nmigrations 001-008"]
        Vectors["Vectores\n(BLOB f32 LE)"]
        Peers["Peers\n(HTTP / File)"]
    end

    Claude -->|"stdio JSON-RPC"| MCP
    OpenCode -->|"stdio JSON-RPC"| MCP
    Continue -->|"stdio JSON-RPC"| MCP
    HTTP_Client -->|"REST"| HTTP

    MCP --> Core
    HTTP --> Core
    CLI --> Core
    TUI --> Core
    Watch --> Core

    Core --> Search
    Core --> Crypto
    Core --> Embed
    Core --> Sync
    Core --> Plugins

    Search --> SQLite
    Embed --> Vectors
    Sync --> Peers
    Core --> SQLite
```

### Capas

```
+----------------------------------------------------------+
|  Interface Layer                                          |
|  MCP Server (stdio) · HTTP API · CLI · TUI · Watch       |
+----------------------------------------------------------+
|  Core Layer                                               |
|  MemoryStore · SessionStore · SearchEngine               |
+----------------------------------------------------------+
|  Feature Layers                                           |
|  CryptoEngine (age) · EmbeddingEngine (ONNX) · SyncEngine|
|  PluginManager (WASM / extism)                           |
+----------------------------------------------------------+
|  Storage Layer                                            |
|  SQLite WAL + FTS5 · Migraciones 001-008                 |
+----------------------------------------------------------+
```

---

## Stack Tecnologico

| Componente | Crate | Version | Uso |
|-----------|-------|---------|-----|
| **Runtime** | tokio | 1 | Async runtime |
| **Almacenamiento** | rusqlite | 0.32 | SQLite + FTS5 + WAL |
| **Migraciones** | rusqlite_migration | 1.2 | Migraciones SQL versionadas |
| **MCP** | rmcp | 0.1 | Protocolo MCP stdio |
| **HTTP** | axum | 0.7 | API REST |
| **CLI** | clap | 4 | Subcomandos + env vars |
| **TUI** | ratatui | 0.27 | Interfaz de terminal + Canvas grafo |
| **TUI eventos** | crossterm | 0.27 | Input + raw mode |
| **Serialización** | serde / serde_json | 1 | JSON |
| **Tipos** | uuid, chrono | 1 / 0.4 | IDs y timestamps |
| **Encriptación** | age | 0.10 | age + SSH recipient |
| **Embeddings** | fastembed | 4 | ONNX local (feature flag) |
| **Sync** | automerge | 0.5 | CRDT P2P |
| **Compresión** | zstd | 0.13 | Sync transport |
| **HTTP client** | reqwest | 0.12 | Sync HTTP transport |
| **Fuzzy** | fuzzy-matcher | 0.3 | Búsqueda aproximada |
| **Plugins** | extism | 1.30 | Runtime WASM sandboxeado (feature flag) |
| **Logging** | tracing | 0.1 | Structured logging |

---

## Modelo de Datos

```mermaid
erDiagram
    MEMORY {
        text id PK
        text project
        text scope
        text title
        text content
        text what
        text why
        text context
        text learned
        text memory_type
        text importance
        text tags
        text topic_key
        int access_count
        int revision_count
        text created_at
        text updated_at
        text deleted_at
        text deprecated_at
        text deprecated_reason
        int is_encrypted
        text encrypted_for
        text origin_peer
    }

    MEMORY_RELATION {
        text id PK
        text sync_id
        text source_id FK
        text target_id FK
        text relation_type
        real confidence
        text judgment_status
        text reason
        text evidence
    }

    SESSION {
        text id PK
        text project
        text directory
        text summary
        text memory_ids
        text started_at
        text ended_at
        text status
    }

    MEMORY_EMBEDDING {
        text memory_id PK
        text model
        blob vector
        text created_at
    }

    SYNC_PEER {
        text id PK
        text name
        text transport_type
        text endpoint
        int is_enabled
        text last_sync_at
    }

    ENCRYPTION_KEY {
        text id PK
        text alias
        text key_type
        text public_key
        int is_default
        text added_at
    }

    MEMORY ||--o{ MEMORY_RELATION : "source"
    MEMORY ||--o{ MEMORY_RELATION : "target"
    MEMORY ||--o| MEMORY_EMBEDDING : "vector"
    SESSION ||--o{ MEMORY : "contains"
```

### Tipos y Enums

| Enum | Valores |
|------|---------|
| **MemoryType** | `architecture`, `decision`, `bugfix`, `pattern`, `convention`, `dependency`, `workflow`, `note`, `config`, `discovery`, `learning` |
| **Importance** | `low`, `medium`, `high`, `critical` |
| **Scope** | `project`, `personal`, `global` |
| **RelationType** | `related`, `compatible`, `scoped`, `conflicts_with`, `supersedes`, `not_conflict`, `superseded_by` |

---

## Busqueda Hibrida

La búsqueda combina tres señales con pesos configurables:

```
Score final = FTS5 (50%) + Fuzzy (20%) + Semántica ONNX (30%)
```

Sin embeddings (sin feature o deshabilitados):

```
Score final = FTS5 (70%) + Fuzzy (30%)
```

```mermaid
flowchart LR
    Q["Query del agente"] --> FTS["FTS5\nSQLite full-text\n50%"]
    Q --> Fuzzy["Fuzzy matcher\ntítulos\n20%"]
    Q --> Sem["ONNX local\ncoseno similarity\n30%"]
    FTS --> Merge["Combinar + normalizar"]
    Fuzzy --> Merge
    Sem --> Merge
    Merge --> Results["Top N resultados\nordenados por score"]
```

---

## Encriptacion

### Flujo de encriptacion

```mermaid
sequenceDiagram
    actor Agent
    participant MCP as MCP / CLI
    participant Store as MemoryStore
    participant Crypto as CryptoEngine
    participant DB as SQLite

    Agent->>MCP: mem_save { encrypt: true, ... }
    MCP->>Store: save(input)
    Store->>Crypto: encrypt_str(content)
    Crypto-->>Store: hex ciphertext
    Store->>DB: INSERT content=<hex>, is_encrypted=1
    DB-->>Store: ok
    Store-->>MCP: Memory { is_encrypted: true }

    Agent->>MCP: mem_get { id }
    MCP->>Store: get(id)
    Store->>DB: SELECT
    DB-->>Store: row { is_encrypted: 1 }
    Store->>Crypto: decrypt_str(hex)
    Crypto-->>Store: plaintext
    Store-->>MCP: Memory { content: plaintext }
```

### Setup de claves

```bash
# Registrar tu SSH key existente
mneme keys add laptop ~/.ssh/id_ed25519.pub --default

# Ver claves registradas
mneme keys list

# Verificar identidad
mneme keys test

# Detectar claves disponibles en el sistema
mneme keys detect
```

---

## Sync CRDT

```mermaid
flowchart LR
    subgraph PeerA["Peer A (laptop)"]
        SA["MemoryStore A"]
        EngA["SyncEngine A"]
    end

    subgraph PeerB["Peer B (servidor)"]
        SB["MemoryStore B"]
        EngB["SyncEngine B"]
    end

    subgraph File["Archivo .mneme"]
        FT["FileTransport\n(export/import zstd)"]
    end

    EngA -->|"Hello (heads CRDT)"| EngB
    EngB -->|"Response (delta)"| EngA
    EngA -->|"export zstd"| FT
    FT -->|"import"| EngB
```

### Comandos de sync

```bash
mneme sync status              # estado de peers y última sincronización
mneme sync add-peer <url>      # agregar peer HTTP
mneme sync now                 # sincronizar con todos los peers
mneme sync export --output sync.mneme   # exportar para transporte manual
mneme sync import sync.mneme           # importar desde archivo
```

---

## Plugins WASM

Los plugins extienden mneme sin recompilar — se descubren en `~/.config/mneme/plugins/*.wasm` al arrancar.

### Compilar con soporte de plugins

```bash
cargo build --release --features plugins
```

### ABI del plugin (3 funciones a exportar)

```
plugin_manifest()          → { name, version, tools: [...], hooks: [...] }
call_tool(json)            → { success, data, error }
transform_memory(json)     → { memory: {...} }
```

### Hooks disponibles

| Hook | Cuándo se invoca |
|------|-----------------|
| `pre_save` | Antes de persistir una memoria — puede transformar el contenido |
| `post_get` | Después de recuperar una memoria — puede enriquecer o filtrar |

Los hooks se encadenan: la salida del plugin N es la entrada del plugin N+1.

---

## Requisitos

- **Rust 1.75+** — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **SQLite 3.35+** — incluido (rusqlite bundled)
- Para embeddings: **2 GB RAM** mínimo (modelo ONNX ~90 MB)

---

## Instalacion

### Desde código fuente

```bash
git clone git@github.com:daddydiaz2/mneme.git
cd mneme

# Sin features opcionales (binario ~13 MB)
cargo build --release

# Con embeddings ONNX (binario ~37 MB)
cargo build --release --features embeddings

# Con plugins WASM
cargo build --release --features plugins

# Todo habilitado
cargo build --release --features embeddings,plugins

# Instalar globalmente
cargo install --path .
```

### Configuracion de agentes

```bash
# Claude Code
mneme setup claude-code

# OpenCode
mneme setup opencode

# Continue
mneme setup continue
```

Cada comando escribe la configuración MCP correspondiente en el directorio del agente.

---

## Uso Rapido

### CLI

```bash
# Guardar una memoria
mneme save --project mi-proyecto \
  --title "JWT auth middleware" \
  --type decision \
  --importance high \
  --tags rust,auth

# Buscar
mneme search "autenticación JWT" --project mi-proyecto

# Ver lista de memorias
mneme list --project mi-proyecto

# Auditar calidad
mneme audit --project mi-proyecto

# Iniciar TUI
mneme tui

# Watch mode (monitorea directorio)
mneme watch --project mi-proyecto --dir ./notas
```

### Como agente MCP

El servidor MCP se inicia con `mneme mcp` y se comunica por stdio. Los agentes lo configuran una sola vez y luego llaman las herramientas directamente.

```json
// Ejemplo: mem_save
{
  "tool": "mem_save",
  "params": {
    "project": "mi-app",
    "title": "Fixed N+1 en UserList",
    "content": "**What**: ...\\n**Why**: ...",
    "memory_type": "bugfix",
    "importance": "high",
    "tags": ["performance", "db"]
  }
}
```

---

## MCP Tools

40 herramientas disponibles, organizadas por categoría:

### CRUD Básico

| Tool | Descripción |
|------|-------------|
| `mem_save` | Guardar memoria (con deduplicación y topic key) |
| `mem_save_batch` | Guardar múltiples memorias en una llamada |
| `mem_save_prompt` | Guardar el prompt actual del agente |
| `mem_get` | Obtener memoria por ID |
| `mem_update` | Actualizar memoria existente |
| `mem_delete` | Soft-delete de memoria |
| `mem_restore` | Restaurar memoria eliminada |
| `mem_list` | Listar memorias del proyecto |

### Búsqueda

| Tool | Descripción |
|------|-------------|
| `mem_search` | Búsqueda híbrida (FTS5 + fuzzy + semántica) |
| `mem_similar` | Buscar memorias similares por embedding |
| `mem_timeline` | Memorias ordenadas por tiempo |
| `mem_context` | Contexto reciente de sesiones |

### Sesiones

| Tool | Descripción |
|------|-------------|
| `mem_session_start` | Iniciar sesión de trabajo |
| `mem_session_end` | Cerrar sesión |
| `mem_session_summary` | Guardar resumen de sesión |
| `mem_summarize` | Resumen ejecutivo de una sesión |

### Relaciones

| Tool | Descripción |
|------|-------------|
| `mem_conflicts` | Detectar conflictos entre memorias |
| `mem_delete_relation` | Eliminar relación por ID |
| `mem_graph` | Grafo de conocimiento del proyecto |

### Introspección y Calidad

| Tool | Descripción |
|------|-------------|
| `mem_audit` | Reporte de calidad (stale, incompletas, deprecadas) |
| `mem_deduplicate` | Detectar memorias duplicadas |
| `mem_feedback` | Registrar feedback useful / not_useful |
| `mem_deprecate` | Marcar memoria como obsoleta |
| `mem_health` | Estado del sistema |
| `mem_remind` | Recordatorios de memorias críticas no accedidas |
| `mem_tag_suggest` | Sugerir tags basado en vocabulario del proyecto |
| `mem_knowledge_gaps` | Detectar áreas sin cobertura |

### Contexto

| Tool | Descripción |
|------|-------------|
| `mem_inject_context` | Bloque Markdown listo para system prompt |
| `mem_forget_project` | Eliminar todas las memorias de un proyecto |

### Metadata

| Tool | Descripción |
|------|-------------|
| `mem_stats` | Estadísticas del proyecto |
| `mem_projects` | Listar proyectos |
| `mem_current_project` | Detectar proyecto actual (desde git) |
| `mem_doctor` | Diagnóstico del sistema |
| `mem_suggest_topic_key` | Sugerir topic key estable para upsert |

### Sincronización

| Tool | Descripción |
|------|-------------|
| `mem_sync_status` | Estado de peers y última sync |
| `mem_sync_now` | Sincronizar con todos los peers |
| `mem_sync_export` | Exportar para transporte manual |

### Encriptación

| Tool | Descripción |
|------|-------------|
| `mem_encrypt` | Encriptar memoria existente in-place |
| `mem_decrypt` | Desencriptar (retorna plaintext, no persiste) |
| `mem_keys_list` | Listar claves registradas |
| `mem_keys_status` | Estado de encriptación del sistema |

---

## HTTP API

El servidor HTTP se inicia con `mneme serve [--port 8080]`.

### Memorias

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/v1/memories` | Listar memorias |
| `POST` | `/api/v1/memories` | Crear memoria |
| `GET` | `/api/v1/memories/:id` | Obtener memoria |
| `PUT` | `/api/v1/memories/:id` | Actualizar memoria |
| `DELETE` | `/api/v1/memories/:id` | Eliminar (soft-delete) |
| `POST` | `/api/v1/memories/batch` | Crear múltiples |
| `POST` | `/api/v1/memories/:id/encrypt` | Encriptar in-place |
| `POST` | `/api/v1/memories/:id/decrypt` | Desencriptar (no persiste) |
| `POST` | `/api/v1/memories/:id/feedback` | Registrar feedback |
| `POST` | `/api/v1/memories/:id/deprecate` | Deprecar |

### Búsqueda e Introspección

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/v1/search?q=...&project=...` | Búsqueda híbrida |
| `GET` | `/api/v1/similar?id=...` | Similares por embedding |
| `GET` | `/api/v1/audit?project=...` | Reporte de calidad |
| `GET` | `/api/v1/duplicates?project=...` | Detectar duplicados |
| `GET` | `/api/v1/graph?project=...` | Grafo de conocimiento |
| `GET` | `/api/v1/context?project=...` | Contexto inyectable |
| `GET` | `/api/v1/health` | Estado del sistema |
| `GET` | `/api/v1/gaps?project=...` | Knowledge gaps |

### Claves y Sync

| Método | Ruta | Descripción |
|--------|------|-------------|
| `GET` | `/api/v1/keys` | Listar claves |
| `POST` | `/api/v1/keys` | Registrar clave |
| `DELETE` | `/api/v1/keys/:id` | Eliminar clave |
| `GET` | `/api/v1/keys/status` | Estado de encriptación |
| `GET` | `/api/v1/sync/status` | Estado de sync |
| `POST` | `/api/v1/sync/now` | Sincronizar |

---

## TUI

```
mneme tui
```

### Vista de lista + detalle

```
┌─────────────────────────────────────────────────────────────────────┐
│ mneme v0.5.0 │ proyecto: mi-app  │ [Q]uit [/]Search [Tab]Grafo [?]Help │
├────────────────────┬────────────────────────────────────────────────┤
│ MEMORIAS           │ DETALLE                                         │
│  🔒 ● [ARCH] ...  │ Título: JWT auth middleware                      │
│ >   ● [DEC]  ...  │ Tipo: decision   Importancia: high               │
│     ● [BUG]  ...  │ Proyecto: mi-app                                │
│     ● [PAT]  ...  │ Tags: [rust] [auth] [jwt]                       │
│  🔒 ● [NOTE] ...  │                                                 │
│                    │ ── Contenido ─────────────────────────────────  │
│                    │ **What**: Implementé JWT Bearer                 │
│                    │ **Why**: Auth stateless cross-instance          │
│                    │ **Where**: src/auth/middleware.rs               │
├────────────────────┴────────────────────────────────────────────────┤
│ [↑↓/jk] Navegar  [Tab] Grafo  [/] Buscar  [d] Eliminar  [Q] Salir  │
└─────────────────────────────────────────────────────────────────────┘
```

### Vista de grafo interactivo (`Tab`)

```
┌─────────────────────────────────────────────────────────────────────┐
│ mneme v0.5.0 │ proyecto: mi-app  │ [Q]uit [/]Search [Tab]Grafo [?]Help │
├─────────────────────────── GRAFO ───────────────────────────────────┤
│                                                                      │
│              [JWT middlewar]          ● CRDT sync                   │
│                    │                        │                        │
│              related ──────────────── supersedes                    │
│                    │                                                 │
│         ● bugfix N+1      ● arch-hexagonal                          │
│                    └────── conflicts_with ────┘                     │
│                                                                      │
├─────────────────────────────────────────────────────────────────────┤
│ [Tab/Esc] Volver  [j/k] Seleccionar nodo  [r] Recargar  [Q] Salir  │
└─────────────────────────────────────────────────────────────────────┘
```

**Controles lista**: `j`/`k`/`↑↓` navegar · `g`/`G` primero/último · `Tab` abrir grafo · `/` buscar inline · `r` refrescar · `d` delete con confirmación · `?` ayuda · `q` salir.

**Controles grafo**: `Tab`/`Esc` volver · `j`/`k` seleccionar nodo · `r` recargar relaciones.

**Indicadores**: `🔒` magenta = memoria encriptada · `●` rojo/amarillo/verde/gris = importancia (critical/high/medium/low) · `[TYPE]` tipo abreviado · nodo cyan = seleccionado · aristas verde/amarillo/gris = confidence ≥0.8 / ≥0.5 / <0.5.

---

## Watch Mode

Monitorea un directorio y auto-guarda archivos `.mneme` como memorias:

```bash
mneme watch --project mi-proyecto --dir ./notas --interval 2
# 👁  Watching ./notas for *.mneme files. Ctrl-C to stop.
# ✓ Guardado: JWT auth middleware
```

### Formato de archivo `.mneme`

Con frontmatter:
```
---
title: JWT auth middleware
type: decision
importance: high
tags: rust, auth, jwt
---
**What**: Implementé JWT Bearer tokens
**Why**: Auth stateless cross-instance
**Where**: src/auth/middleware.rs
```

Sin frontmatter (formato simple):
```
Título de la memoria
Contenido libre de la memoria...
```

---

## Estructura del Proyecto

```
mneme/
├── assets/
│   └── imagen.png                 # Banner y recursos visuales del README
├── src/
│   ├── main.rs                    # Entry point: init chain → dispatch
│   ├── lib.rs                     # Re-exports de módulos públicos
│   ├── error.rs                   # MnemeError (26+ variantes)
│   ├── cli/
│   │   ├── commands.rs            # 25+ subcomandos Clap
│   │   └── output.rs              # Pretty-print con colores
│   ├── config/
│   │   └── settings.rs            # Settings: DB, Server, MCP, Crypto, Sync, Embeddings
│   ├── store/
│   │   ├── db.rs                  # Database: open, WAL, PRAGMAs, stores
│   │   ├── memory.rs              # MemoryStore, SessionStore, tipos, GraphData
│   │   ├── search.rs              # SearchEngine híbrido + SearchWeights
│   │   └── migrations.rs          # Registro de migraciones 001-008
│   ├── mcp/
│   │   ├── server.rs              # MnemeServer (stdio JSON-RPC) + PluginManager
│   │   └── tools.rs               # 40 handlers MCP + dispatch dinámico de plugins
│   ├── http/
│   │   ├── router.rs              # create_router (axum)
│   │   └── handlers.rs            # 30+ handlers HTTP
│   ├── embeddings/                # Feature flag: --features embeddings
│   │   ├── mod.rs                 # Re-exports + stubs
│   │   ├── engine.rs              # EmbeddingEngine (fastembed ONNX)
│   │   ├── store.rs               # EmbeddingStore (BLOB f32 LE)
│   │   └── similarity.rs          # cosine_similarity, SemanticMatch
│   ├── crypto/
│   │   ├── engine.rs              # CryptoEngine (age encrypt/decrypt)
│   │   ├── keys.rs                # RecipientKey, IdentityKey (SSH/age)
│   │   └── store.rs               # KeyStore (SQLite)
│   ├── sync/
│   │   ├── protocol.rs            # SyncMessage, HelloMsg, ResponseMsg
│   │   ├── crdt.rs                # memory_to_doc, merge_docs (automerge)
│   │   ├── peer.rs                # Peer, PeerStore, TransportType
│   │   ├── engine.rs              # SyncEngine: build_hello, apply_response
│   │   └── transport/
│   │       ├── http.rs            # HttpTransport (reqwest)
│   │       └── file.rs            # FileTransport (zstd export/import)
│   ├── plugins/                   # Feature flag: --features plugins
│   │   ├── manifest.rs            # PluginManifest, PluginTool
│   │   ├── manager.rs             # PluginManager: discovery, load, dispatch, hooks
│   │   └── mod.rs                 # Re-exports
│   ├── tui/
│   │   ├── app.rs                 # App: estado, modos (Normal/Graph/Search/Help/Confirm)
│   │   ├── events.rs              # Manejo de teclas por AppMode
│   │   ├── graph.rs               # layout_nodes() circular, truncate_title()
│   │   └── ui.rs                  # Render: lista, detalle, grafo Canvas, overlays
│   ├── export/
│   │   └── markdown.rs            # export_to_markdown, import_from_markdown
│   └── watch/
│       └── watcher.rs             # DirectoryWatcher: polling + parse .mneme
├── migrations/
│   ├── 001_initial.sql            # Tablas base: memories, relations, sessions
│   ├── 002_fts5.sql               # Índice FTS5 full-text
│   ├── 003_vectors.sql            # Tabla embeddings (BLOB)
│   ├── 004_tools.sql              # Columnas feedback, deprecated, supersedes
│   ├── 005_sync.sql               # Tablas sync: state, peers, log
│   ├── 006_sync_origin.sql        # Columna origin_peer
│   ├── 007_encryption.sql         # Columnas is_encrypted, encrypted_for; encryption_keys
│   └── 008_fts5_encryption_aware.sql  # Triggers FTS5 encryption-aware
├── tests/
│   ├── store_tests.rs
│   ├── mcp_tests.rs
│   ├── integration_tests.rs
│   ├── embeddings_tests.rs
│   ├── sync_tests.rs
│   ├── crypto_tests.rs
│   ├── store_extended_tests.rs
│   ├── config_tests.rs
│   ├── search_tests.rs
│   ├── export_tests.rs
│   ├── plugin_tests.rs
│   └── tui_graph_tests.rs
├── Cargo.toml
└── README.md
```

---

## Roadmap

### Implementado

- [x] Store SQLite + FTS5 + WAL
- [x] Soft delete, deduplicación automática, topic keys
- [x] Scopes: project / personal / global
- [x] MCP server con 40 herramientas
- [x] HTTP API REST (30+ endpoints)
- [x] CLI con 25+ subcomandos
- [x] Embeddings ONNX local (BAAI/bge-small-en-v1.5) como feature flag
- [x] Búsqueda híbrida: FTS5 + fuzzy + semántica
- [x] Introspección: audit, deduplicate, graph, health, remind, gaps
- [x] Encriptación granular con age / SSH keys
- [x] Sync CRDT P2P con automerge (HTTP + file transport)
- [x] TUI interactiva con ratatui
- [x] Grafo visual interactivo en TUI (Canvas, layout circular, `Tab`)
- [x] Watch mode (monitoreo de directorio)
- [x] Export/import Markdown
- [x] Feature flag `embeddings` — binario base 13 MB
- [x] Plugins WASM con extism (feature flag `plugins`)
- [x] Setup automático para Claude Code, OpenCode, Continue

### Planificado

- [ ] Docker image oficial
- [ ] Documentación de API con ejemplos interactivos
- [ ] Cobertura de tests al 60%+

---

## Contribuir

1. Fork → rama desde `dev`
2. Commits en [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `chore:`, etc.
3. Push a `dev` → Pull Request a `dev` → merge a `main`

### Convenciones

- Código e identificadores en inglés; comentarios en español (neutral)
- Sin `unwrap()`/`expect()` en producción — usar `?` o `map_err`
- `tracing` para logs, nunca `println!`
- `cargo clippy -- -D warnings` debe pasar antes de cualquier PR
- `cargo fmt` aplicado

---

## Licencia

**MIT** — ver [LICENSE](LICENSE).

---

<div align="center">

Desarrollado en Rust 2021 · SQLite · automerge · age · ratatui · extism

**mneme** — Memoria que persiste, conocimiento que evoluciona.

</div>
