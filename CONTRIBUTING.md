# Contributing to mneme

Thanks for wanting to contribute! 🎉

## Quick Start

```bash
# Fork → clone
git clone https://github.com/YOUR_USER/mneme.git
cd mneme

# Build
cargo build

# Run tests
cargo test

# Check everything is clean
cargo clippy -- -D warnings
cargo fmt --check
```

## Development Workflow

1. Branch from `dev`
2. Make your changes
3. Add tests
4. Run `cargo clippy -- -D warnings` — zero warnings required
5. Run `cargo fmt` — consistent formatting required
6. Commit with [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` new feature
   - `fix:` bug fix
   - `docs:` documentation
   - `test:` tests
   - `refactor:` code change that is neither feat nor fix
   - `chore:` maintenance
7. Push to your fork → PR to `dev`

## Code Standards

- **Language**: Code and identifiers in English; comments in Spanish (neutral).
- **Error handling**: No `unwrap()` / `expect()` in production code — use `?` or `map_err`.
- **Logging**: Use `tracing` macros, never `println!` in production paths.
- **Safety**: No unsafe blocks unless absolutely necessary and documented.

## Feature Flags

mneme uses Cargo feature flags for optional dependencies:

| Feature     | What it enables                |
|-------------|--------------------------------|
| (default)   | Core: SQLite, CLI, MCP, HTTP   |
| `embeddings`| ONNX local embeddings (fastembed) |
| `plugins`   | WASM plugin system (extism)    |

When adding dependencies, prefer making them optional behind one of these flags.

## Testing

- Tests go in `tests/` for integration tests or inline `#[cfg(test)] mod tests` for unit tests.
- New features should include tests.
- Run the full suite: `cargo test --all-features`.

## Agent Setup

Adding a new agent to `mneme setup <agent>`? The pattern is:

1. Add a variant to `AgentSetup` enum in `src/cli/commands.rs`
2. Add a match arm in `run_command`
3. Add a `setup_<agent>()` function — most just call `write_standard_mcp_config()`
4. Document it in `README.md` under "Configuración de agentes"

## Questions?

Open an issue or PR. We're friendly.
