# mneme-ai 🤖

**Ecosystem configurator for AI coding agents.**

mneme-ai configures your AI agent (OpenCode, Claude Code, Cursor, Windsurf, etc.) to use mneme as its persistent memory brain. Inspired by Gentle-AI but built in Rust.

## What It Does

Before: your agent forgets everything between sessions.

After: your agent has persistent memory, SDD workflows, curated skills, and MCP tools.

## Quick Start

```bash
# Install
cargo install mneme-ai

# Initialize config
mneme-ai init

# Configure your agent
mneme-ai install opencode

# Check ecosystem health
mneme-ai doctor
```

## Supported Agents

| Agent | Command |
|-------|---------|
| OpenCode | `mneme-ai install opencode` |
| Claude Code | `mneme-ai install claude-code` |
| Cursor | `mneme-ai install cursor` |
| Windsurf | `mneme-ai install windsurf` |
| VS Code Copilot | `mneme-ai install vscode-copilot` |
| Continue | `mneme-ai install continue` |
| Gemini CLI | `mneme-ai install gemini-cli` |
| Codex CLI | `mneme-ai install codex` |
| Zed | `mneme-ai install zed` |
| Warp | `mneme-ai install warp` |

## Commands

| Command | Description |
|---------|-------------|
| `mneme-ai init` | Create default config |
| `mneme-ai install <agent>` | Configure agent with mneme |
| `mneme-ai doctor` | Ecosystem health check |
| `mneme-ai list-agents` | List supported agents |
| `mneme-ai version` | Show version |
