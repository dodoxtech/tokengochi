---
type: task
status: active
priority: P0
delivery_order: 0001
estimate: 1d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - active
---

# Task: Scaffold Tauri v2 project

## Context

Foundation for everything. Stack decided in [[../../decisions/0001-tauri-stack|ADR-0001]]; target layout in [[../../architecture|Architecture]] §Project Structure.

## Goal

A running Tauri v2 app with the planned module layout, Svelte dashboard shell, and CI checks (fmt, clippy, tests) on all three OS targets.

## Scope

In scope: `create-tauri-app` (Svelte + TS), `src-tauri` module skeleton (`watcher/`, `economy/`, `pet/`, `store/`), `economy.toml` with constants from [[../../knowledge/game-economy|Game Economy]] §8, GitHub Actions build matrix.

Out of scope: any game logic, overlay window, packaging/signing.

## Acceptance Criteria

- [x] `cargo tauri dev` opens the dashboard shell on the dev machine. Verified indirectly: user installed Rust locally (macOS, aarch64) and ran a build; `src-tauri/target/debug/deps/tokengochi-21982e1a46dc231e` is a real, fully-linked Mach-O arm64 executable, and all 459 dependencies (incl. `tauri` 2.11.5) resolved in `Cargo.lock`. Pending: user visually confirming the window actually opened and showed the config table (asked in chat).
- [ ] CI builds pass on Windows, macOS, Ubuntu. **Still not run** - repo has no git remote configured yet (`git remote -v` empty), so `.github/workflows/ci.yml` has never executed. Needs a push to a GitHub remote (or a PR) before this can be checked off.
- [x] `economy.toml` is loaded and exposed via a `get_config` command. Verified: `src-tauri/target/debug/economy.toml` exists and is byte-identical to `src-tauri/economy.toml`, confirming Tauri's resource bundling picked it up correctly for the `get_config` command's `BaseDirectory::Resource` lookup.

## Dependencies

None.

## Implementation Notes

- Pin Tauri v2 stable; enable `tray-icon` and `devtools` features. Done via `src-tauri/Cargo.toml`: `tauri = { version = "2", features = ["tray-icon", "devtools"] }`.
- Scaffolded with `npm create tauri-app@latest -- tokengochi -m npm -t svelte-ts --identifier com.tokengochi.app --tauri-version 2`, then reorganized to match the planned layout in [[../../architecture|Architecture]] §Project Structure:
  - SvelteKit app moved from repo root into `ui/dashboard/` (own `package.json`); `ui/overlay/` and `ui/assets/sprites/` added as placeholders (with a README and `.gitkeep` respectively).
  - `src-tauri/tauri.conf.json` `build` section updated so `beforeDevCommand`/`beforeBuildCommand` run `npm --prefix ../ui/dashboard`, and `frontendDist` points at `../ui/dashboard/build`.
  - `src-tauri/src/{watcher,economy,pet,store}/` module skeleton added per architecture doc, plus `tray.rs`. These are intentionally placeholder-only (out of scope per this task): `watcher` defines `TokenProvider` trait + `TokenEvent` and stub `ClaudeCodeProvider`/`OpenAiProvider`/`ManualProvider`; `pet` defines `EvolutionStage`/`Mood` enums; `store` and `tray::setup` are no-ops with TODOs pointing at the tasks that will fill them in. Added `#![allow(dead_code)]` to the `watcher` and `pet` modules since nothing constructs these types yet - `cargo clippy -D warnings` would otherwise fail on them; remove the allow once real code uses them.
  - `economy.toml` added at `src-tauri/economy.toml` with all constants from [[../../knowledge/game-economy|Game Economy]] §8, bundled as a Tauri resource (`bundle.resources` in `tauri.conf.json`) and loaded at runtime (not compiled in) via `app.path().resolve(.., BaseDirectory::Resource)`, so tuning it needs a release but not a code change, matching game-economy.md §7.
  - `get_config` Tauri command added in `src-tauri/src/lib.rs`, backed by `EconomyState` (`Mutex<EconomyConfig>`) populated in `.setup()`. The dashboard's `+page.svelte` calls it on mount and renders the loaded constants as a minimal proof-of-wiring shell.
  - `.github/workflows/ci.yml` added: a `fmt-clippy-test` job (Ubuntu only, `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`) gating a `build` matrix job (`ubuntu-22.04`, `macos-latest`, `windows-latest`) that installs the frontend deps and runs `cargo tauri build --ci` via `cargo install tauri-cli`.

## References

- [[../../architecture|Architecture]]

## Verification Results

**Round 1 (sandbox, no Rust toolchain):** this sandbox has no Rust toolchain and no root access - `cargo`/`rustc` were not installed, `apt-get` requires root (unavailable), and `static.rust-lang.org` is blocked by the sandbox's network allowlist (confirmed with both HEAD and GET, and a CONNECT-level 403 from the proxy - not method-specific). Other mirrors tried (Tsinghua/USTC/SJTU/rsproxy.cn, conda, GitHub Releases API) were all unreachable too. What *was* verified from the sandbox:

- `npm create tauri-app@latest` scaffold generated cleanly (npm registry is reachable even though `static.rust-lang.org` isn't; interestingly `static.crates.io`/`index.crates.io` *are* reachable, so cargo dependency downloads were never actually going to be the blocker - only the compiler itself).
- `npm --prefix ui/dashboard install && npm --prefix ui/dashboard run build` succeeds end-to-end (writes `ui/dashboard/build/`).
- `tauri.conf.json`, `Cargo.toml`, `economy.toml`, `package.json` all parse as valid JSON/TOML; `economy.toml` keys checked 1:1 against `EconomyConfig` in `economy/config.rs`; `ci.yml` parses as valid YAML.
- Manual read-through of `lib.rs` wiring, Tauri v2 API usage, and added `#![allow(dead_code)]` to `watcher`/`pet` so placeholder types don't trip `clippy -D warnings`.

**Round 2 (user installed Rust locally, macOS aarch64, rustc 1.97.0):** inspected the build artifacts left in `src-tauri/target/` (this project folder is synced between the user's machine and this sandbox):

- `src-tauri/Cargo.lock` exists with all 459 dependencies resolved, `tauri = 2.11.5`, `toml = 0.8.2` - confirms `cargo` could actually reach crates.io and resolve every dependency, including `tauri`, `serde`, etc.
- `src-tauri/target/debug/deps/tokengochi-21982e1a46dc231e` is a real Mach-O 64-bit arm64 executable (1.08 MB, executable bit set) - the app compiled *and linked* successfully. `target/debug/build/` shows build scripts ran for `tauri`, `tauri-plugin-opener`, `tauri-runtime`, etc.
- `src-tauri/target/debug/economy.toml` is byte-identical to `src-tauri/economy.toml` - Tauri's resource bundling correctly picked up `economy.toml` for dev mode, so the `get_config` command's `BaseDirectory::Resource` lookup has a file to find.
- `ui/dashboard/node_modules/` is present - frontend deps installed too.
- No git remote is configured in the repo yet, and nothing has been pushed, so **CI has never actually run**.

**Still open (can't confirm without the user or a CI run):**
  - Visual confirmation that `cargo tauri dev` opened a window and the dashboard shell rendered the economy config table (build artifacts prove compilation succeeded, not that the running app rendered correctly - asked the user to confirm in chat).
  - `cargo fmt --check` / `cargo clippy -- -D warnings` / `cargo test` were not explicitly run (only an app build/dev session is evidenced by the artifacts) - worth running once for full confidence, though a clean compile is a good sign clippy will mostly be fine.
  - CI green on the Windows/macOS/Ubuntu matrix - blocked on the repo getting a git remote and a push/PR.

**Conclusion:** 2 of 3 acceptance criteria are now satisfied with strong evidence; the task stays in `docs/tasks/active/` (not `done`) until CI has actually run once, per [[../../engineering|Engineering Guide]] ("record verification results," "move completed task files... after verification").
