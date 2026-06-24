# mneme-guardian 😇

**AI code review guardian.** Provider-agnostic, written in Rust.

mneme-guardian reviews staged files before every commit, catching issues early. Works with any AI provider and optionally saves results to mneme for searchable review history.

## Architecture

```
git commit  ──▶  pre-commit hook  ──▶  mneme-g run
                                            │
                                    ┌───────┴───────┐
                                    ▼               ▼
                              AI Provider     mneme (optional)
                            (review code)    (save results)
```

## Quick Start

```bash
# Install
cargo install mneme-guardian

# In your project
cd your-project
mneme-g init          # Create config
mneme-g install       # Install pre-commit hook

# Manual review
mneme-g run

# CI mode (review last commit)
mneme-g run --ci
```

## Providers

| Provider | Env Value | Required CLI |
|----------|-----------|-------------|
| OpenCode (default) | `opencode` | `opencode` |
| Claude Code | `claude` | `claude` |
| Gemini CLI | `gemini` | `gemini` |
| Codex CLI | `codex` | `codex` |
| Ollama | `ollama` | `ollama` + model |

```bash
# Use Claude
MNEME_G_PROVIDER=claude mneme-g run

# Use Ollama with specific model
MNEME_G_PROVIDER=ollama MNEME_G_MODEL=qwen2.5-coder:7b mneme-g run
```

## Mneme Integration

If mneme is installed, review results are saved as memories automatically:

```bash
mneme search "code review" --project my-project
```

Disable with: `MNEME_G_MNEME=false mneme-g run`

## Hook Types

| Command | Description |
|---------|-------------|
| `mneme-g install` | Pre-commit hook (default) |
| `mneme-g install --hook-type commit-msg` | Commit message validation |
| `mneme-g uninstall` | Remove all hooks |
