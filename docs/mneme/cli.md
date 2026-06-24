# CLI Reference

## Core Commands

| Command | Description |
|---------|-------------|
| `mneme save` | Save a memory |
| `mneme search` | Hybrid search (FTS5 + fuzzy + semantic) |
| `mneme get <id>` | Get memory by ID |
| `mneme list` | List memories |
| `mneme update <id>` | Update a memory |
| `mneme delete <id>` | Soft-delete a memory |
| `mneme restore <id>` | Restore a deleted memory |

## Session Management

| Command | Description |
|---------|-------------|
| `mneme context` | Recent session context |
| `mneme stats` | Memory statistics |
| `mneme projects` | List all projects |
| `mneme summarize` | Generate session summary |

## Agent Setup

| Command | Description |
|---------|-------------|
| `mneme setup opencode` | Configure OpenCode |
| `mneme setup claude-code` | Configure Claude Code |
| `mneme setup cursor` | Configure Cursor |
| `mneme setup windsurf` | Configure Windsurf |
| `mneme setup vscode-copilot` | Configure VS Code |
| `mneme setup continue` | Configure Continue |
| `mneme setup gemini-cli` | Configure Gemini CLI |
| `mneme setup codex` | Configure Codex CLI |
| `mneme setup zed` | Configure Zed |

## Server

| Command | Description |
|---------|-------------|
| `mneme serve` | Start HTTP API server |
| `mneme mcp` | Start MCP server (stdio) |
| `mneme tui` | Launch terminal UI |

## Encryption

| Command | Description |
|---------|-------------|
| `mneme keys add` | Register encryption key |
| `mneme keys list` | List registered keys |
| `mneme keys detect` | Detect available SSH keys |
| `mneme keys test` | Verify decryption identity |

## Sync

| Command | Description |
|---------|-------------|
| `mneme sync peers` | List sync peers |
| `mneme sync add-peer` | Add sync peer |
| `mneme sync now` | Sync with all peers |
| `mneme sync export` | Export for manual transport |
| `mneme sync import` | Import from file |

## Diagnostics

| Command | Description |
|---------|-------------|
| `mneme doctor` | System health check |
| `mneme audit` | Memory quality audit |
| `mneme health` | System health status |
| `mneme remind` | Critical memory reminders |
| `mneme knowledge-gaps` | Areas without coverage |
