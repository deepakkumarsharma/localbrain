# Local Brain Privacy

Local Brain is local-first. Code parsing, metadata, graph indexing, wiki generation, search, embeddings, and chat answers run against local project data by default.

## Defaults

- Cloud providers are disabled by default.
- The Agent HTTP API is stopped by default and binds only to `127.0.0.1:3737` when started.
- File commands are restricted to the active workspace root.
- Generated metadata is stored in the app data directory, not sent to a remote service.

## Local Data

Local Brain stores:

- SQLite metadata and search indexes.
- Graph database files.
- Generated wiki Markdown under `docs/wiki/` when requested.

Do not commit private generated indexes or app data. Review generated `docs/wiki/` files before pushing them.

## Cloud Providers

BYOK/cloud behavior is gated behind provider settings. The current implementation exposes the local/cloud state but does not send prompts or code to cloud providers.

## Diagnostics

Errors are shown locally in the app. There is no external telemetry or crash reporting.
