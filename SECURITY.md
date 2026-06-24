# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.2.x   | ✅ Active development |

## Reporting a Vulnerability

mneme uses **age encryption** for sensitive memory data. If you find a vulnerability:

1. **Do NOT** open a public GitHub issue.
2. Send details to the maintainer via GitHub's private vulnerability reporting or open a [security advisory](https://github.com/daddydiaz2/mneme/security/advisories/new).

We'll respond within 48 hours and coordinate a fix before disclosure.

## Encryption Notes

- mneme encrypts memory content granularly using **age** (rage Rust implementation).
- Encryption keys default to your existing SSH key (`~/.ssh/id_ed25519`).
- Titles and tags remain in cleartext for searchability.
- When encryption is enabled, FTS5 triggers exclude encrypted fields from the full-text index.
- Sync transport uses zstd compression, not encryption — encrypt memories before syncing sensitive data.

## Best Practices

- Always register a default encryption key: `mneme keys add`
- Verify your identity: `mneme keys test`
- Use `--encrypt` flag when saving sensitive memories
- Keep your `~/.ssh/` keys safe — losing them = losing encrypted memories
