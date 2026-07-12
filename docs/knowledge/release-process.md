---
type: knowledge
status: active
created: 2026-07-12
updated: 2026-07-12
tags:
  - knowledge
  - packaging
  - ai-context
---

# Release Process

How to cut a Tokengochi release. See [[../tasks/active/0008-packaging-ci-updater|task 0008]] and [[../decisions/0004-unsigned-mvp-release|ADR-0004]] for background.

## Versioning

Semantic Versioning (`MAJOR.MINOR.PATCH`). Pre-1.0, treat `MINOR` bumps as potentially breaking.

Three files must carry the same version number:

- `src-tauri/tauri.conf.json` (`version`)
- `src-tauri/Cargo.toml` (`package.version`)
- `ui/dashboard/package.json` (`version`) — cosmetic, keep in sync for consistency

## Cutting a release

1. Update `CHANGELOG.md`: move `[Unreleased]` items under a new `## [X.Y.Z] - YYYY-MM-DD` heading.
2. Bump the version in the three files listed above to `X.Y.Z`.
3. Commit: `git commit -m "release: vX.Y.Z"`.
4. Tag and push: `git tag vX.Y.Z && git push origin main --tags`.
5. [.github/workflows/release.yml](../../.github/workflows/release.yml) triggers on the tag push, builds installers for macOS (arm64 + x86_64), Windows, and Linux, signs the updater artifacts, and publishes a **draft** GitHub Release with `latest.json` attached.
6. Review the draft release (binaries, changelog body), then publish it manually. `tauri-plugin-updater` polls the `latest` release's `latest.json`, so unpublished drafts are invisible to existing installs — this is the safety gate before a release goes live.
7. Verify auto-update: install the previous tagged build, launch it, click "Check for updates" in the dashboard settings panel, confirm it downloads and the "Restart now" button relaunches into the new version with pet state intact.

## Updater signing key

`tauri-plugin-updater` requires artifacts to be signed with a **minisign** keypair, separate from OS code signing (see ADR-0004). This key was generated once with:

```sh
cargo tauri signer generate -w /path/to/tokengochi-updater.key
```

- The **public key** is committed in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
- The **private key** and its password are stored as GitHub Actions repository secrets (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`) — never commit the private key file.
- If the private key is ever lost or rotated, existing installs cannot verify new updates until they're manually reinstalled with a build carrying the new pubkey.

## Unsigned OS builds (MVP)

Per [[../decisions/0004-unsigned-mvp-release|ADR-0004]], macOS and Windows builds are not notarized/code-signed yet. Users will see Gatekeeper/SmartScreen warnings on first install — documented in the root [README](../../README.md#installing).
