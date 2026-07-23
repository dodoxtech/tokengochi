---
type: task
status: done
priority: P1
delivery_order: 0021
estimate: S
created: 2026-07-22
updated: 2026-07-22
owner: AI agent
sprint: null
tags:
  - task
  - packaging
  - updater
  - claude-hooks
---

# Task: Fix packaged-build hook path & missing macOS entries in updater manifest

## Status

- State: done
- Created: 2026-07-22
- Owner: AI agent
- Priority: P1
- Delivery order: 0021
- Estimate: S
- Sprint: TBD

## Context

Two bugs were reported after downloading and running the public macOS release
(v0.2.6):

1. **"Install hook" fails in the dashboard** with
   `could not resolve hook script path
   /Users/runner/work/tokengochi/tokengochi/src-tauri/../resources/claude-hooks/tokengochi-notify.sh:
   No such file or directory`. Root cause:
   [`hook_script_path`](../../../src-tauri/src/claude_hooks.rs) resolved the
   script via `env!("CARGO_MANIFEST_DIR")`, which bakes in the **CI build
   machine's** path at compile time. On any machine without the source
   checkout (i.e. every downloaded build) that path does not exist. The code
   comment already flagged bundling the script as a Tauri resource as the
   needed follow-up.
2. **"Check for updates" shows "Update check failed" on macOS.** Root cause:
   the published `latest.json` contained only `windows-x86_64` and
   `linux-x86_64` platform entries — no `darwin-*`. Tauri's `check()` looks up
   the current platform key (`darwin-aarch64`/`darwin-x86_64`), doesn't find
   it, and throws. In the release workflow the windows/linux jobs (via
   `tauri-action`) merge their `latest.json`, but the macOS job uploaded its
   own `latest.json` with a plain `gh release upload --clobber`, which does not
   merge — so the `darwin` entries were lost from the final manifest.

Related:

- [[../README|Tasks]]
- [[../active/0008-packaging-ci-updater|0008 Packaging CI & Updater]]
- [[../../knowledge/agent-status-notifications|Agent Status Notifications]]

## Goal

Make the Claude Code hook installable from a packaged/downloaded build, and
ensure the published `latest.json` always carries every platform (including
macOS) so the in-app updater works on macOS.

## Scope

In scope:

- Bundle `resources/claude-hooks/tokengochi-notify.sh` as a Tauri resource and
  resolve it at runtime via `AppHandle` (`BaseDirectory::Resource`), falling
  back to the source-tree copy for `cargo run`/`tauri dev`.
- Invoke the hook via `bash '<script>'` so a resource that loses its
  executable bit during bundling still runs.
- Change the macOS release job to merge its `darwin` platforms into the
  release's existing `latest.json` (download → union `platforms` → upload →
  verify), with a retry loop to survive the concurrent uploads from the
  windows/linux and sibling macOS-arch jobs.

Out of scope:

- Windows hook support (the script is bash-only; unchanged).
- Reworking `tauri-action`'s own windows/linux manifest merge.

## Acceptance Criteria

- [x] Hook path resolves from the bundled resource in a packaged build; dev
      builds and tests still resolve via the source tree.
- [x] `cargo test` passes (hook tests updated to use the manifest fallback).
- [x] macOS release job no longer clobbers `latest.json`; it merges and
      verifies `darwin` platform entries survived.
- [x] Workflow YAML validates; merge/verify script unit-tested locally.

## Dependencies

- None.

## Risks

- Concurrent release-asset uploads have no atomic CAS; mitigated by the
  download→merge→upload→verify retry loop (platform set only grows, so it
  converges). Cannot be fully verified until the next tagged release.

## Implementation Notes

- `src-tauri/src/claude_hooks.rs`: `hook_script_path(&AppHandle)` +
  `manifest_hook_script_path()` fallback; `status`/`install` now take
  `&AppHandle`; `hook_command` prefixes `bash`.
- `src-tauri/src/lib.rs`: the two `#[tauri::command]`s pass `AppHandle`.
- `src-tauri/tauri.conf.json`: `bundle.resources` now an object mapping
  `../resources/claude-hooks/tokengochi-notify.sh` →
  `claude-hooks/tokengochi-notify.sh`.
- `.github/workflows/release.yml`: split macOS upload into binaries + a merge
  step calling `.github/scripts/merge-updater-manifest.py`.

## References

- [[../../README|Documentation]]
- [[../../index|Project Map]]
- `src-tauri/src/claude_hooks.rs`
- `src-tauri/src/lib.rs`
- `src-tauri/tauri.conf.json`
- `.github/workflows/release.yml`
- `.github/scripts/merge-updater-manifest.py`

## Verification Plan

- [x] `cargo test claude_hooks` and `cargo build` in `src-tauri/`.
- [x] `python3 -c "import yaml; yaml.safe_load(...)"` on the workflow.
- [x] Local merge/verify self-test of the manifest script.
- [ ] Deferred: confirm on the next tagged release that `latest.json` carries
      `darwin-aarch64`/`darwin-x86_64` and that "Check for updates" works on a
      downloaded macOS build.

## Verification Results

- `cargo test claude_hooks`: 3 passed.
- `cargo build`: clean.
- Workflow YAML: valid. Merge script: merge preserves windows/linux and adds
  darwin; verify returns 0 when present, 1 when clobbered.

## Completion Notes

- Completed: 2026-07-22
- Changed files: `src-tauri/src/claude_hooks.rs`, `src-tauri/src/lib.rs`,
  `src-tauri/tauri.conf.json`, `.github/workflows/release.yml`,
  `.github/scripts/merge-updater-manifest.py`.
- Follow-ups: verify both fixes on the next tagged release (updater manifest
  can only be validated end-to-end from a real published release).
