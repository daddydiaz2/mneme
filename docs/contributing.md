# Contributing

All three projects (mneme, mneme-ai, mneme-guardian) are open source under MIT license.

## Development Setup

```bash
# Clone all projects
git clone https://github.com/daddydiaz2/mneme.git
git clone https://github.com/daddydiaz2/mneme-ai.git
git clone https://github.com/daddydiaz2/mneme-guardian.git
```

## Code Standards

- **Language**: Code and identifiers in English
- **Commits**: Conventional Commits
- **Errors**: No `unwrap()`/`expect()` in production — use `?`
- **Logging**: Use `tracing` macros, never `println!`
- **Clippy**: Zero warnings required

## Testing

```bash
# mneme
cd mneme && cargo test

# mneme-ai
cd mneme-ai && cargo test

# mneme-guardian
cd mneme-guardian && cargo test
```

## Adding a New Agent (mneme/mneme-ai)

1. Add the agent info to `mneme-ai/src/agents.rs`
2. Add the setup function in `mneme/src/cli/commands.rs`
3. Test with: `mneme-ai install <agent-name>`
4. Document in the README

## Adding a New Provider (mneme-guardian)

1. Add a `review_with_<name>()` function in `src/providers.rs`
2. Add the case in `run_review()`
3. Add config/env support in `src/config.rs`
4. Document in README

## Documentation

Docs live in the `docs/` folder of the mneme repo and use MkDocs with Material theme.
Run locally:

```bash
pip install mkdocs-material
mkdocs serve
```
