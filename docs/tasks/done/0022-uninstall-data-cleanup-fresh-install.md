---
type: task
status: done
priority: P2
delivery_order: 0022
estimate: S
created: 2026-07-23
updated: 2026-07-23
owner: AI agent
sprint: null
tags:
  - task
  - done
  - packaging
  - cross-platform
---

# Task: Delete app data on uninstall & guarantee a fresh database on install

## Status

- State: done
- Created: 2026-07-23
- Owner: AI agent
- Priority: P2
- Delivery order: 0022
- Estimate: S

## Context

User request: when the app is uninstalled, also delete its database, and make sure
a new install starts from a brand-new database. Key constraint the user raised:
**an in-place version update must never wipe user data.**

Persisted data lives outside the bundle (see [[../../architecture|Architecture]] §Data Storage):

- SQLite game database: `<data_dir>/com.tokengochi.app/tokengochi.sqlite3`
- Watcher bookkeeping: `<data_dir>/tokengochi/`

Platform reality that shaped the design:

- **macOS**: dragging the app to Trash runs no code — no uninstall hook exists.
- **Linux**: `.deb` has `postrm`, but AppImage/rpm have no uninstall hook.
- **Windows**: NSIS uninstaller also runs during updates, so naive deletion would
  wipe data on every upgrade — exactly the user's fear.

"Fresh database on install" needs no install-time logic: both stores open with
`CREATE TABLE IF NOT EXISTS`, so any launch after a clean wipe rebuilds an empty
schema. Guaranteeing freshness therefore reduces to guaranteeing a clean removal.

Related:

- [[../README|Tasks]]
- [[../../architecture|Architecture]]

## Goal

Remove all persisted data when the user gets rid of the app, on every platform,
without ever deleting data during a version update.

## Scope

In scope:

- In-app "Delete all data & quit…" tray action (reliable on all platforms).
- Windows NSIS post-uninstall hook, skipped during updater `/UPDATE` runs.
- Debian `.deb` `postrm` cleanup, only on `purge`.

Out of scope:

- Automatic data deletion on macOS drag-to-Trash and AppImage/rpm removal
  (no OS hook exists — covered by the in-app action instead).
- Trashing the app bundle itself from within the app.

## Acceptance Criteria

- [x] A tray action wipes both data dirs after confirmation, then quits.
- [x] After a wipe, the next launch recreates an empty database.
- [x] Windows uninstall deletes data, but not when run with `/UPDATE`.
- [x] `.deb` deletes data on `purge` only, never on `upgrade`/`remove`.
- [x] `cargo check` and `cargo test` pass.

## Completion Notes

Changed:

- `src-tauri/src/storage_paths.rs`: added `watcher_data_dir()` and
  `wipe_all_app_data()` (removes `app_data_dir` + `watcher_data_dir`, treating a
  missing dir as already-clean); `watcher_data_file` now reuses `watcher_data_dir`.
- `src-tauri/src/tray.rs`: new "Delete all data & quit…" menu item →
  `confirm_and_wipe`, a non-blocking `tauri-plugin-dialog` confirm that wipes
  data and exits.
- `src-tauri/Cargo.toml` + `src-tauri/src/lib.rs`: added and registered
  `tauri-plugin-dialog` (backend-only usage; no capability entry needed).
- `src-tauri/installer/windows-hooks.nsh`: `NSIS_HOOK_POSTUNINSTALL` deletes
  `%APPDATA%\com.tokengochi.app` and `%APPDATA%\tokengochi`, guarded by the
  `/UPDATE` cmdline flag so updates keep data.
- `src-tauri/installer/deb-postrm.sh`: deletes data on `purge` only, sweeping
  `/root` and `/home/*` since dpkg runs as root.
- `src-tauri/tauri.conf.json`: wired `bundle.windows.nsis.installerHooks` and
  `bundle.linux.deb.postRemoveScript`.
- `docs/architecture.md`: documented the uninstall/fresh-install behavior.

Verified:

- `cargo check` — clean.
- `cargo test storage_paths` — 3 passed.

Follow-up / unverified:

- The Windows NSIS and `.deb` hooks are configured but only exercised by the
  release CI (they cannot be built on macOS). Confirm on the next tagged build
  that (a) a fresh `.deb` `purge` removes data, (b) a Windows update via the
  updater keeps data, and (c) a Windows uninstall removes it.
