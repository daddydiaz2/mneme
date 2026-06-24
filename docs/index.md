# mneme ecosystem 🧠

**Persistent memory for AI coding agents.**

Three Rust projects that work together to give your AI agent memory, workflow, and code review:

## Projects

| Project | Description | Install |
|---------|-------------|---------|
| **[mneme](mneme/)** 🧠 | Persistent memory system. SQLite + FTS5 + MCP server. Hybrid search, encryption, P2P sync, plugins. | `cargo install mneme` · `brew install mneme` |
| **[mneme-ai](mneme-ai/)** 🤖 | Ecosystem configurator. Sets up any AI agent (OpenCode, Claude, Cursor, etc.) with mneme as its brain. | `cargo install mneme-ai` |
| **[mneme-guardian](mneme-guardian/)** 😇 | AI code review guardian. Provider-agnostic pre-commit reviews with optional mneme sync. | `cargo install mneme-guardian` |

## Quick Start

```bash
# 1. Install the brain
cargo install mneme

# 2. Configure your agent
cargo install mneme-ai
mneme-ai install opencode

# 3. Set up code review
cargo install mneme-guardian
cd your-project
mneme-g init
mneme-g install
```

Now every `git commit` reviews your code with AI, and every decision is saved in mneme's persistent memory.

## How It Works

```
mneme-ai              mneme-guardian
  │                       │
  │ configures            │ reviews code
  ▼                       ▼
┌─────────────────────────────────────┐
│          mneme (MCP server)         │
│  SQLite + FTS5 + Hybrid Search      │
│  Encryption + P2P Sync + Plugins    │
│  64 MCP tools                       │
└─────────────────────────────────────┘
         │
         ▼
  Persistent memory across sessions
```

## Features

- **Persistent memory**: Your agent remembers decisions, bugs, and context across sessions
- **Hybrid search**: FTS5 + fuzzy matching + semantic embeddings (ONNX local or cloud providers)
- **Encryption**: Granular age/SSH encryption per memory
- **P2P sync**: CRDT-based sync between machines (automerge)
- **WASM plugins**: Extend mneme without recompiling
- **11 agents supported**: OpenCode, Claude, Cursor, Windsurf, VS Code, Continue, Gemini, Codex, Zed, Warp
- **5 review providers**: OpenCode, Claude, Gemini, Codex, Ollama
- **Pre-commit hooks**: Automatic code review on every commit
