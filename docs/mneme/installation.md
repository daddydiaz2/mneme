# Installation

## From Source (Rust)

```bash
# Requires Rust 1.75+
cargo install mneme

# With all features
cargo install mneme --features embeddings,plugins
```

## Homebrew

```bash
brew tap daddydiaz2/homebrew-tap
brew install mneme
```

## Docker

```bash
docker build -t mneme:latest https://github.com/daddydiaz2/mneme.git
docker run -d --name mneme -p 7438:7438 -v mneme-data:/app/data mneme:latest
```

## GitHub Releases

Pre-built binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows (x86_64):

[github.com/daddydiaz2/mneme/releases](https://github.com/daddydiaz2/mneme/releases)

## Verify Installation

```bash
mneme --version
mneme doctor
```
