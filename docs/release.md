# Local Brain Release Checklist

## Preflight

- Run `npm run build`.
- Run `cd src-tauri && cargo test`.
- Run `cd src-tauri && cargo clippy -- -D warnings`.
- Confirm `src-tauri/tauri.conf.json` has a production CSP.
- Confirm file-facing commands reject paths outside the workspace root.
- Review generated `docs/wiki/` output before committing or packaging.

## Packaging

- Run `npm run tauri -- build`.
- Launch the packaged app.
- Confirm local indexes are created under the app data directory.
- Confirm no `.localbrain/`, API keys, environment files, or local database files are bundled.

## Manual Smoke Test

- Parse and index the selected source file.
- Run incremental project indexing.
- Generate the wiki.
- Rebuild the search index.
- Ask a chat question and verify citations.
- Open graph view and select a node.
- Start the Agent API and call `GET http://127.0.0.1:3737/status`.
- Stop the Agent API.

## Distribution Notes

Signing and notarization are still release-channel decisions. Do not enable auto-update until update signatures and local-data migration behavior are explicitly designed.
