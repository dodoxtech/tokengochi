---
type: task
status: active
priority: P1
delivery_order: 0008
estimate: 3d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - active
---

# Task: Packaging, signing, auto-update — MVP release

## Context

Ship the MVP (tasks 0001–0007) as installable builds. Targets and channels in [[../../architecture|Architecture]] §Runtime and Deployment.

## Goal

Tagged commits produce signed installers for all three OS with working auto-update.

## Scope

In scope: `tauri bundle` artifacts (.msi/NSIS, .dmg, .deb + AppImage); GitHub Actions release workflow; `tauri-plugin-updater` against GitHub Releases; macOS notarization and Windows code signing (or documented unsigned-MVP decision); version/changelog process.

Out of scope: Homebrew/winget/Snap (backlog later per demand).

## Acceptance Criteria

- [x] `git tag vX.Y.Z` → CI publishes installers for all 3 OS. (workflow implemented; not yet exercised with a real tag — needs repo secrets, see below)
- [ ] Auto-update from previous version verified on each OS; pet state survives update. (wired end-to-end; cross-OS rehearsal still needed after the first real release)
- [x] Install docs in README.

## Dependencies

- [[0007-tray-settings-dashboard|0007]]

## Risks

- Signing certificates cost/lead time — decide early; unsigned macOS builds trigger Gatekeeper friction. **Resolved for MVP:** see [[../../decisions/0004-unsigned-mvp-release|ADR-0004]] — ship unsigned, revisit before 1.0.

## Verification Plan

- [ ] Full release rehearsal from a test tag; record results below.

## Verification Results

### 2026-07-12 — implementation

- Added `tauri-plugin-updater` and `tauri-plugin-process` (Rust) and their JS counterparts; registered in [src-tauri/src/lib.rs](../../src-tauri/src/lib.rs), permissions granted in [src-tauri/capabilities/default.json](../../src-tauri/capabilities/default.json).
- `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`, `plugins.updater` configured with the GitHub Releases `latest.json` endpoint and the updater public key.
- Generated a minisign updater keypair (`cargo tauri signer generate`). Public key committed in `tauri.conf.json`. **Private key was generated locally in this session at `/tmp/tokengochi-updater-keys/tokengochi.key` (no password) — it must be moved into GitHub repo secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (password can stay empty or be set) and then deleted from disk before this is considered production-ready.** See [[../../knowledge/release-process|Release Process]].
- Added dashboard "Check for updates" control (Settings panel) using `check()` / `downloadAndInstall()` / `relaunch()`.
- Added [.github/workflows/release.yml](../../.github/workflows/release.yml): triggers on `v*` tags, matrix-builds macOS (arm64 + x86_64), Windows, and Linux via `tauri-apps/tauri-action`, publishes a **draft** GitHub Release with signed updater artifacts.
- Documented the unsigned-MVP decision ([[../../decisions/0004-unsigned-mvp-release|ADR-0004]]) and the release/versioning process ([[../../knowledge/release-process|Release Process]]).
- Added root [README.md](../../README.md) with install instructions per OS, including the Gatekeeper/SmartScreen workaround, and an updating section.
- Added [CHANGELOG.md](../../CHANGELOG.md) (Keep a Changelog format).

Verification:

- `cargo test --manifest-path src-tauri/Cargo.toml` passes (43 tests, unchanged by this task).
- `cargo tauri build --debug --no-bundle` succeeds from `src-tauri/` with the new plugins wired in (frontend hooks + Rust compile both complete).
- `npm --prefix ui/dashboard run check` and `npm --prefix ui/overlay run check` pass with no errors.
- Not yet run: a real `git tag vX.Y.Z` push (needs the updater secrets added to the GitHub repo first — this is a repo-state change, deferred to the user), and the actual cross-OS auto-update rehearsal, which can only happen after a real release exists.
- Pre-existing, unrelated to this task: `cargo clippy --all-targets -- -D warnings` currently fails on `main` due to dead-code lint warnings in `src/pet/mod.rs` (evolution/usage-pattern fields not yet wired up). This doesn't block `release.yml` (which doesn't run clippy), but blocks the `fmt-clippy-test` job in `ci.yml`. Flagging for a follow-up task rather than fixing here since it's outside 0008's scope.

### Follow-up before a real release

1. Move the updater private key into GitHub repo secrets, delete the local copy.
2. Push a `v0.1.0` test tag and confirm `release.yml` produces a draft release with all 5 artifacts + `latest.json`.
3. Install the pre-release build on each OS, then run a real update against a `v0.1.1` bump to confirm `tauri-plugin-updater` downloads/installs/relaunches and pet state (SQLite) survives.
4. Fix the pre-existing clippy dead-code warnings blocking `ci.yml`'s lint job (unrelated to this task).
