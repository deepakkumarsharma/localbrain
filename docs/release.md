# Local Brain Release Checklist

## Preflight

- Run `npm run typecheck`.
- Run `npm run build`.
- Run `cd src-tauri && cargo test`.
- Run `cd src-tauri && cargo clippy -- -D warnings`.
- Confirm `src-tauri/tauri.conf.json` has production CSP.
- Confirm file-facing commands reject paths outside workspace root.

## Packaging

- Run `npm run release:check`.
- Confirm packaged artifacts exist in `src-tauri/target/release/bundle/`.
- Confirm no `.localbrain/`, `.env*`, `.sqlite`, or `.db` files are packaged.

## Manual Smoke Test (Packaged App)

- Launch the packaged app from bundle output.
- Open a real repository and complete project indexing.
- Confirm local indexes are created under OS app-data directory.
- Open Database/Wiki/Graph views and verify content renders.
- Ask a local chat query and verify citations.
- Start Agent API and call `GET http://127.0.0.1:3737/status`, then stop API.

## Signing and Notarization

- CI workflow: `.github/workflows/release.yml` packages macOS + Windows on tag pushes (`v*`).
- Apple signing/notarization is enabled via secrets (`APPLE_*`) when available.
- Keep auto-update disabled until signed update channels and migration behavior are finalized.

## Distribution Notes

- Local-first default remains unchanged.
- No telemetry is added by release packaging.
- Cloud providers stay BYOK opt-in.
