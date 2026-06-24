# mneme 🧠

**Persistent memory for AI coding agents.**

mneme (`/ˈniːmiː/` — from Mnemosyne, Greek goddess of memory) is a Rust binary that gives your AI coding agent persistent memory across sessions.

## Architecture

```
Agent (Claude Code / OpenCode / Cursor / ...)
    ↓ MCP stdio
mneme (single Rust binary)
    ↓
SQLite + FTS5 (~/.local/share/mneme/mneme.db)
```

## Key Features

- **64 MCP tools** — save, search, audit, graph, encrypt, sync, and more
- **Hybrid search** — FTS5 (BM25) + fuzzy matching + semantic embeddings (ONNX)
- **Encryption** — granular age/SSH encryption per memory
- **P2P sync** — CRDT-based (automerge) between machines
- **WASM plugins** — extend mneme without recompiling
- **TUI with knowledge graph** — visualize memory relationships
- **HTTP API** — 30+ REST endpoints
- **Multi-provider embeddings** — ONNX, OpenAI, Ollama, Google
- **Entity extraction** — auto-detect URLs, paths, technologies
- **LLM-judge conflict detection** — detect and resolve semantic conflicts
- **Context compression** — 4 strategies for prompt injection
- **Obsidian export** — export memory graph to Obsidian vault

## Quick Links

- [Installation](installation.md)
- [Quick Start](quickstart.md)
- [CLI Reference](cli.md)
- [MCP Tools](mcp-tools.md)
- [Agent Setup](setup.md)
- [Encryption](encryption.md)
- [Architecture](architecture.md)
