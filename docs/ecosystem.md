# Ecosystem Integration

The three mneme projects are designed to work together as a complete AI coding ecosystem.

## Installation Flow

```bash
# 1. Install all three tools
cargo install mneme
cargo install mneme-ai
cargo install mneme-guardian

# 2. (Optional) brew install
brew tap daddydiaz2/homebrew-tap
brew install mneme
brew install mneme-ai
brew install mneme-guardian
```

## Configuration Flow

```bash
# 1. Init mneme (creates database)
mneme serve &          # start HTTP server (optional)

# 2. Init mneme-ai
mneme-ai init           # ~/.config/mneme-ai/config.toml

# 3. Install your agent(s)
mneme-ai install opencode    # writes MCP config
mneme-ai install claude-code # writes MCP config
mneme-ai install cursor      # writes MCP config

# 4. Set up code review in your project
cd /path/to/your-project
mneme-g init                 # ~/.config/mneme-guardian/config.toml
mneme-g install              # .git/hooks/pre-commit
```

## How Data Flows

```
┌─────────────────────────────────────────────────────────┐
│                    Your Project                          │
│  ┌──────────────────────────────────────────────────┐   │
│  │  git commit                                      │   │
│  │    ↓                                              │   │
│  │  pre-commit hook → mneme-g run                    │   │
│  │    ↓                                              │   │
│  │  mneme-g → AI provider → review results           │   │
│  │    ↓                                              │   │
│  │  mneme save --type review (results stored)        │   │
│  └──────────────────────────────────────────────────┘   │
│                                                         │
│  Next session:                                          │
│  mneme search "auth bug" → finds past decisions         │
└─────────────────────────────────────────────────────────┘
```

## Searching Past Reviews

Since mneme-guardian automatically saves review results to mneme:

```bash
# Find all reviews for a project
mneme search "code review" --project my-project

# Find specific issues
mneme search "BLOCKER" --project my-project

# Check what was reviewed recently
mneme context --project my-project

# Visualize review history
mneme graph --project my-project
```

## CI/CD Integration

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
      - name: Review PR
        run: mneme-g run --ci
```

## Provider Configuration

```bash
# Default: OpenCode
mneme-g run

# Claude
MNEME_G_PROVIDER=claude mneme-g run

# Ollama (local)
MNEME_G_PROVIDER=ollama MNEME_G_MODEL=qwen2.5-coder:7b mneme-g run
```
