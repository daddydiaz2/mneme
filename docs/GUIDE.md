# mneme ecosystem — Installation & Usage Guide

Complete guide to setting up and using the mneme ecosystem: **mneme-brain** (memory), **mneme-ai** (configurator), and **mneme-guardian** (code review).

---

## Table of Contents

1. [Overview](#1-overview)
2. [Installation](#2-installation)
3. [Quick Start](#3-quick-start)
4. [mneme — Persistent Memory](#4-mneme--persistent-memory)
5. [mneme-ai — Ecosystem Configurator](#5-mneme-ai--ecosystem-configurator)
6. [mneme-guardian — Code Review](#6-mneme-guardian--code-review)
7. [Integration Guide](#7-integration-guide)
8. [CI/CD Setup](#8-cicd-setup)
9. [Troubleshooting](#9-troubleshooting)

---

## 1. Overview

The mneme ecosystem consists of three Rust tools that work together:

```
┌─────────────────────────────────────────────────────────┐
│                    Your Project                          │
│                                                         │
│  mneme-ai → configures your AI agent (opencode, claude) │
│             to use mneme as its memory brain             │
│                                                         │
│  mneme-guardian → pre-commit hook that reviews code     │
│                   with AI and saves results to mneme     │
│                                                         │
│  mneme-brain → persistent memory (MCP server, SQLite)   │
│                hybrid search, encryption, P2P sync       │
└─────────────────────────────────────────────────────────┘
```

## 2. Installation

### Option A: crates.io (recommended)

```bash
# Install all three
cargo install mneme-brain      # → binary: mneme
cargo install mneme-ai         # → binary: mneme-ai
cargo install mneme-guardian   # → binary: mneme-g
```

### Option B: Homebrew

```bash
brew tap daddydiaz2/homebrew-tap
brew install mneme
brew install mneme-ai
brew install mneme-guardian
```

### Option C: From source

```bash
# mneme-brain
git clone https://github.com/daddydiaz2/mneme.git
cd mneme && cargo install --path .

# mneme-ai
git clone https://github.com/daddydiaz2/mneme-ai.git
cd mneme-ai && cargo install --path .

# mneme-guardian
git clone https://github.com/daddydiaz2/mneme-guardian.git
cd mneme-guardian && cargo install --path .
```

### Verify installation

```bash
mneme --version
mneme-ai --version
mneme-g --version
```

## 3. Quick Start

Set up the complete ecosystem in 5 minutes:

```bash
# 1. Install (choose one method above)

# 2. Configure your AI agent
mneme-ai init
mneme-ai install opencode   # or: claude-code, cursor, windsurf, etc.

# 3. Set up code review in your project
cd /path/to/your/project
mneme-g init
mneme-g install

# 4. Save your first memory
mneme save --project my-project "Project setup" "mneme ecosystem initialized"

# 5. Done! Every commit now:
#    - Auto-reviews staged files
#    - Saves review results to mneme
#    - Your agent remembers context across sessions
```

## 4. mneme — Persistent Memory 🧠

### Basic usage

```bash
# Save a memory
mneme save --project my-app \
  --title "JWT auth middleware" \
  --type decision \
  --tags rust,auth \
  "We chose JWT Bearer for stateless auth"

# Search
mneme search "JWT auth" --project my-app

# List project memories
mneme list --project my-app

# View stats
mneme stats --project my-app
```

### Agent setup

Configure your AI agent to use mneme as MCP server:

```bash
mneme setup opencode        # OpenCode
mneme setup claude-code     # Claude Code
mneme setup cursor          # Cursor Editor
mneme setup windsurf        # Windsurf
mneme setup vscode-copilot  # VS Code Copilot Chat
mneme setup continue        # Continue
mneme setup gemini-cli      # Gemini CLI
mneme setup codex           # Codex CLI
mneme setup zed             # Zed Editor
```

### MCP Tools (64)

Once configured, your AI agent can call mneme tools directly:

| Category | Tools |
|----------|-------|
| **CRUD** | `mem_save`, `mem_get`, `mem_update`, `mem_delete`, `mem_list` |
| **Search** | `mem_search`, `mem_similar`, `mem_context`, `mem_timeline` |
| **Sessions** | `mem_session_start`, `mem_session_end`, `mem_session_summary` |
| **Quality** | `mem_audit`, `mem_health`, `mem_remind`, `mem_knowledge_gaps` |
| **Graph** | `mem_graph`, `mem_conflicts` |
| **Encryption** | `mem_encrypt`, `mem_decrypt`, `mem_keys_list` |
| **Sync** | `mem_sync_now`, `mem_sync_status` |
| **Advanced** | `mem_entity_extract`, `mem_compress`, `mem_temporal_query` |

### Memory Protocol

Tell your agent to use mneme automatically by including this in your project's `AGENTS.md`:

```markdown
## Mneme Memory Protocol
- Save important decisions: `mem_save(title, content, type, importance)`
- Search before asking: `mem_search(query)` 
- End sessions with: `mem_session_summary`
- Check health: `mem_doctor`
```

## 5. mneme-ai — Ecosystem Configurator 🤖

### Commands

```bash
mneme-ai init                  # Create config (~/.config/mneme-ai/config.toml)
mneme-ai install opencode      # Configure agent with mneme
mneme-ai install claude-code   # Configure Claude Code
mneme-ai doctor                # Health check
mneme-ai list-agents           # List supported agents
```

### Full agent list

```
opencode        → ~/.config/opencode/config.json
claude-code     → ~/.claude/settings.json
cursor          → ~/.cursor/mcp.json
windsurf        → ~/.codeium/windsurf/mcp_config.json
vscode-copilot  → ~/.config/Code/User/.../mcp_config.json
continue        → ~/.continue/config.json
gemini-cli      → ~/.config/gemini-cli/mcp.json
codex           → ~/.codex/settings.json
zed             → ~/.config/zed/settings.json
warp            → ~/.warp/mcp_config.json
```

### Health check

```bash
mneme-ai doctor
```

Output example:
```
🔍 mneme-ai ecosystem doctor
----------------------------------------
🧠 mneme binary... ✅ found at /usr/local/bin/mneme (v0.2.0)
📋 config... ✅ /home/user/.config/mneme-ai/config.toml

🤖 agent integrations:
  OpenCode... ✅ configured
  Claude Code...   not detected
  Cursor... ✅ configured
```

## 6. mneme-guardian — Code Review Guardian 😇

### Setup

```bash
# In your project directory
mneme-g init          # ~/.config/mneme-guardian/config.toml
mneme-g install       # .git/hooks/pre-commit
```

### Manual review

```bash
# Review staged files
mneme-g run

# Review last commit (CI mode)
mneme-g run --ci

# Review with specific provider
MNEME_G_PROVIDER=claude mneme-g run
MNEME_G_PROVIDER=ollama MNEME_G_MODEL=qwen2.5-coder:7b mneme-g run

# Skip mneme sync
MNEME_G_MNEME=false mneme-g run
```

### Configuration

File: `~/.config/mneme-guardian/config.toml`

```toml
provider = "opencode"
model = ""
rules_file = "./AGENTS.md"
mneme_enabled = true
exit_on_issues = true
max_lines = 0
```

Or via environment variables:
```bash
export MNEME_G_PROVIDER=claude
export MNEME_G_MNEME=true
```

### Hook types

```bash
mneme-g install                    # Pre-commit (default)
mneme-g install --hook-type commit-msg  # Commit message validation
mneme-g uninstall                  # Remove all hooks
```

## 7. Integration Guide

### Full ecosystem setup (5 min)

```bash
# 1. Install everything
cargo install mneme-brain mneme-ai mneme-guardian

# 2. Configure your agent
mneme-ai init
mneme-ai install opencode

# 3. Set up project review
cd ~/projects/my-app
mneme-g init
mneme-g install

# 4. Save initial context
mneme save --project my-app --type architecture \
  "Project architecture" "Clean Architecture with CQRS"

# 5. Make a change
echo "fn main() {}" > main.rs
git add main.rs
git commit -m "feat: initial setup"
# → mneme-g reviews automatically!
# → Results saved to mneme!
```

### Search across the ecosystem

```bash
# Find past code reviews
mneme search "code review" --project my-app

# Find architecture decisions
mneme search "architecture" --project my-app

# View knowledge graph
mneme graph --project my-app

# Check memory health
mneme audit --project my-app
```

### Recommended workflow

1. **Morning**: `mneme context` — review what was done yesterday
2. **During coding**: Agent auto-saves decisions via MCP tools
3. **Before commit**: mneme-g runs automatically (pre-commit hook)
4. **After commit**: Review results saved to mneme automatically
5. **Evening**: Agent auto-saves session summary via `mem_session_summary`

## 8. CI/CD Setup

### GitHub Actions

```yaml
# .github/workflows/review.yml
name: Code Review
on: [pull_request]
jobs:
  review:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 2
      - name: Install mneme-guardian
        run: cargo install mneme-guardian
      - name: AI Code Review
        run: mneme-g run --ci
        env:
          MNEME_G_PROVIDER: opencode
          MNEME_G_MNEME: "false"
```

### GitLab CI

```yaml
review:
  stage: test
  script:
    - cargo install mneme-guardian
    - mneme-g run --ci
  only:
    - merge_requests
```

## 9. Troubleshooting

### mneme not found

```bash
# Verify installation
which mneme
mneme --version

# Reinstall if needed
cargo install mneme-brain
```

### mneme-g hook not running

```bash
# Check hook file
cat .git/hooks/pre-commit

# Reinstall
mneme-g uninstall
mneme-g install

# Verify hook is executable
ls -la .git/hooks/pre-commit
```

### No provider available

```bash
# Check which provider is configured
mneme-g config

# Test provider availability
which opencode   # or claude, gemini, codex, ollama

# Install a provider
# OpenCode: https://opencode.ai
# Claude: https://claude.ai/code
```

### mneme-ai doctor reports issues

```bash
# Run doctor for details
mneme-ai doctor

# Common fixes:
cargo install mneme-brain      # Brain not found
mneme-ai init                  # Config missing
mneme-ai install opencode      # Agent not configured
```

---

## Quick Reference

```bash
# Install
cargo install mneme-brain mneme-ai mneme-guardian

# Memory
mneme save --project p "Fix bug" "Root cause was X"
mneme search "bug" --project p
mneme stats --project p

# Configurator
mneme-ai init
mneme-ai install opencode
mneme-ai doctor

# Review
mneme-g init
mneme-g install
mneme-g run
mneme-g run --ci
```
